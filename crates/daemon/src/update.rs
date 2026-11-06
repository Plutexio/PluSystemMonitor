//! Updating the daemon from the git clone `install.sh` installed it from:
//! fetch, compare with the commit this was built from, and when asked pull,
//! run `install.sh` again and restart in place.
//!
//! Nothing happens on its own: a frontend asks for a check or an update.
//! `install.sh --no-updates` or `plusysmond --no-updates` turn it off.

use std::fs;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use plusysmon_protocol::{BUILD_COMMIT, UpdateStatus, parse_versions};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

/// Asking twice within this long gives the result of the first check.
const CHECK_INTERVAL: Duration = Duration::from_secs(60);
const GIT_TIMEOUT: Duration = Duration::from_secs(120);
/// Building on a Raspberry Pi takes a while.
const INSTALL_TIMEOUT: Duration = Duration::from_secs(3 * 3600);
/// Lines of output kept for the error message of a failed update.
const TAIL_LINES: usize = 6;

pub const IDLE: &str = "idle";
pub const CHECKING: &str = "checking";
pub const UPDATING: &str = "updating";
pub const RESTARTING: &str = "restarting";
pub const FAILED: &str = "failed";

#[derive(Clone, Debug)]
pub enum UpdateEvent {
    Status(UpdateStatus),
    Log(String),
}

/// What `install.sh` wrote into `<prefix>/share/plusysmon/install.conf`.
#[derive(Debug, Clone, PartialEq)]
pub struct InstallRecord {
    pub source: PathBuf,
    pub prefix: PathBuf,
    pub daemon_only: bool,
    pub updates: bool,
    /// The PATH install.sh ran with, so cargo is found even from a service.
    pub path: String,
}

impl InstallRecord {
    pub fn parse(text: &str) -> Option<Self> {
        let mut source = None;
        let mut prefix = None;
        let mut record = Self {
            source: PathBuf::new(),
            prefix: PathBuf::new(),
            daemon_only: false,
            updates: true,
            path: String::new(),
        };
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "source" => source = Some(PathBuf::from(value)),
                "prefix" => prefix = Some(PathBuf::from(value)),
                "daemon_only" => record.daemon_only = value == "1",
                "updates" => record.updates = value != "0",
                "path" => record.path = value.to_owned(),
                _ => {}
            }
        }
        record.source = source.filter(|p| p.is_absolute())?;
        record.prefix = prefix.filter(|p| p.is_absolute())?;
        Some(record)
    }

    /// Next to the binary: `<prefix>/bin/plusysmond` → `<prefix>/share/plusysmon/install.conf`.
    fn find(exe: &Path) -> Option<PathBuf> {
        Some(exe.parent()?.parent()?.join("share/plusysmon/install.conf"))
    }
}

pub struct Updater {
    /// Why updates can't happen, or how to do them.
    setup: Result<InstallRecord, String>,
    /// The binary to start again after installing.
    exe: PathBuf,
    status: Mutex<UpdateStatus>,
    last_check: Mutex<Option<Instant>>,
    events: broadcast::Sender<UpdateEvent>,
}

impl Updater {
    pub fn new(turned_off: bool) -> Arc<Self> {
        let exe = std::env::current_exe().unwrap_or_default();
        let setup = if turned_off {
            Err("updates are turned off on this machine (plusysmond --no-updates)".to_owned())
        } else {
            setup(&exe)
        };
        match &setup {
            Ok(record) => info!("updates from {}", record.source.display()),
            Err(reason) => info!("no updates: {reason}"),
        }
        let status = UpdateStatus {
            unavailable: setup.as_ref().err().cloned().unwrap_or_default(),
            state: IDLE.to_owned(),
            ..Default::default()
        };
        Arc::new(Self {
            setup,
            exe,
            status: Mutex::new(status),
            last_check: Mutex::new(None),
            events: broadcast::channel(256).0,
        })
    }

    pub fn status(&self) -> UpdateStatus {
        self.status.lock().unwrap().clone()
    }

    pub fn subscribe(&self) -> broadcast::Receiver<UpdateEvent> {
        self.events.subscribe()
    }

    fn set(&self, change: impl FnOnce(&mut UpdateStatus)) {
        let status = {
            let mut status = self.status.lock().unwrap();
            change(&mut status);
            status.clone()
        };
        // Err only means nobody is listening.
        let _ = self.events.send(UpdateEvent::Status(status));
    }

    fn log(&self, line: String) {
        let _ = self.events.send(UpdateEvent::Log(line));
    }

    /// Starts a check unless one ran just now or something is under way.
    /// Returns whether it did; if not, the current status is the answer.
    pub fn check(self: &Arc<Self>) -> bool {
        let Ok(record) = self.setup.clone() else {
            return false;
        };
        {
            let mut last = self.last_check.lock().unwrap();
            let state = self.status.lock().unwrap().state.clone();
            let busy = state != IDLE && state != FAILED;
            if busy || last.is_some_and(|at| at.elapsed() < CHECK_INTERVAL) {
                return false;
            }
            *last = Some(Instant::now());
        }
        self.set(|s| s.state = CHECKING.to_owned());
        let updater = self.clone();
        tokio::spawn(async move {
            let result = updater.look_upstream(&record).await;
            updater.finish_check(result);
        });
        true
    }

    fn finish_check(&self, result: Result<Upstream, String>) {
        self.set(|s| {
            s.checked_ms = now_ms();
            match result {
                Ok(upstream) => {
                    s.state = IDLE.to_owned();
                    s.error.clear();
                    s.behind = upstream.behind;
                    s.latest = upstream.latest;
                    (s.latest_version, s.latest_oldest_compatible) =
                        upstream.versions.unwrap_or((0, 0));
                }
                Err(e) => {
                    warn!("checking for updates: {e}");
                    s.state = FAILED.to_owned();
                    s.error = e;
                }
            }
        });
    }

    /// Starts an update unless one can't or already runs. Returns whether it did.
    pub fn start(self: &Arc<Self>) -> bool {
        let Ok(record) = self.setup.clone() else {
            return false;
        };
        {
            let mut status = self.status.lock().unwrap();
            if status.state == UPDATING || status.state == RESTARTING {
                return false;
            }
            status.state = UPDATING.to_owned();
            status.error.clear();
        }
        self.set(|_| {});
        let updater = self.clone();
        tokio::spawn(async move {
            match updater.install(&record).await {
                Ok(()) => updater.restart(),
                Err(e) => {
                    error!("update failed: {e}");
                    updater.set(|s| {
                        s.state = FAILED.to_owned();
                        s.error = e;
                    });
                }
            }
        });
        true
    }

    async fn look_upstream(&self, record: &InstallRecord) -> Result<Upstream, String> {
        let git = Git::new(record);
        git.run(&["fetch", "--quiet"]).await?;
        // Compare with what runs, not with what's checked out: the clone may
        // have been pulled without installing.
        let base = if BUILD_COMMIT != "unknown"
            && git
                .run(&["cat-file", "-e", &format!("{BUILD_COMMIT}^{{commit}}")])
                .await
                .is_ok()
        {
            BUILD_COMMIT
        } else {
            "HEAD"
        };
        let behind = git
            .run(&["rev-list", "--count", &format!("{base}..@{{upstream}}")])
            .await?
            .trim()
            .parse()
            .map_err(|_| "git rev-list gave no number".to_owned())?;
        let latest = git
            .run(&["log", "-1", "--format=%h %s", "@{upstream}"])
            .await?
            .trim()
            .to_owned();
        let versions = git
            .run(&["show", "@{upstream}:crates/protocol/src/lib.rs"])
            .await
            .ok()
            .and_then(|source| parse_versions(&source));
        Ok(Upstream {
            behind,
            latest,
            versions,
        })
    }

    async fn install(&self, record: &InstallRecord) -> Result<(), String> {
        let git = Git::new(record);
        self.log(format!("Updating {}", record.source.display()));
        git.run(&["fetch", "--quiet"]).await?;
        git.run(&["merge", "--ff-only", "@{upstream}"])
            .await
            .map_err(|e| {
                format!("can't fast-forward the source folder (local commits or changes?): {e}")
            })?;
        let head = git.run(&["log", "-1", "--format=%h %s"]).await?;
        self.log(format!("Now at {}", head.trim()));

        let mut command = Command::new("bash");
        command
            .arg(record.source.join("install.sh"))
            .arg("--no-service")
            // The fan helper needs sudo, and nobody is there to answer; the
            // one installed keeps working (its protocol is versioned).
            .arg("--keep-fan-helper")
            .arg("--prefix")
            .arg(&record.prefix);
        if record.daemon_only {
            command.arg("--daemon-only");
        }
        command
            .current_dir(&record.source)
            .env("CARGO_TERM_COLOR", "never")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        if !record.path.is_empty() {
            command.env("PATH", &record.path);
        }
        // Builds are heavy; the machine's own work comes first.
        // SAFETY: nice is async-signal-safe.
        unsafe {
            command.pre_exec(|| {
                libc::nice(10);
                Ok(())
            });
        }
        let mut child = command
            .spawn()
            .map_err(|e| format!("starting install.sh: {e}"))?;
        let tail = Arc::new(Mutex::new(Vec::new()));
        let stdout = self.forward(child.stdout.take(), tail.clone());
        let stderr = self.forward(child.stderr.take(), tail.clone());
        let run = async {
            let (_, _, status) = tokio::join!(stdout, stderr, child.wait());
            status
        };
        let status = tokio::time::timeout(INSTALL_TIMEOUT, run)
            .await
            .map_err(|_| "install.sh took too long".to_owned())?
            .map_err(|e| format!("install.sh: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            let tail = tail.lock().unwrap().join("\n");
            Err(format!("install.sh failed ({status}):\n{tail}"))
        }
    }

    /// Passes a child's output on, line by line, keeping the last few.
    async fn forward(&self, pipe: Option<impl AsyncRead + Unpin>, tail: Arc<Mutex<Vec<String>>>) {
        let Some(pipe) = pipe else { return };
        let mut lines = BufReader::new(pipe).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let line = strip_ansi(&line);
            if line.trim().is_empty() {
                continue;
            }
            {
                let mut tail = tail.lock().unwrap();
                if tail.len() == TAIL_LINES {
                    tail.remove(0);
                }
                tail.push(line.clone());
            }
            self.log(line);
        }
    }

    /// Becomes the new binary: same PID, so systemd doesn't notice, same
    /// arguments. Clients see the connection drop and reconnect.
    fn restart(self: &Arc<Self>) {
        self.set(|s| {
            s.state = RESTARTING.to_owned();
            s.behind = 0;
        });
        self.log("Installed; restarting the daemon".to_owned());
        let updater = self.clone();
        tokio::spawn(async move {
            // Let the last messages reach the clients.
            tokio::time::sleep(Duration::from_millis(800)).await;
            info!("restarting into the updated {}", updater.exe.display());
            let error = std::process::Command::new(&updater.exe)
                .args(std::env::args_os().skip(1))
                .exec();
            error!("couldn't restart: {error}");
            updater.set(|s| {
                s.state = FAILED.to_owned();
                s.error = format!("installed, but couldn't restart ({error}); restart it by hand");
            });
        });
    }
}

struct Upstream {
    behind: u32,
    latest: String,
    versions: Option<(u32, u32)>,
}

struct Git<'a> {
    record: &'a InstallRecord,
}

impl<'a> Git<'a> {
    fn new(record: &'a InstallRecord) -> Self {
        Self { record }
    }

    async fn run(&self, args: &[&str]) -> Result<String, String> {
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(&self.record.source)
            .args(args)
            // Never wait for a password nobody can type.
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("LC_ALL", "C")
            .stdin(Stdio::null())
            .kill_on_drop(true);
        if !self.record.path.is_empty() {
            command.env("PATH", &self.record.path);
        }
        let output = tokio::time::timeout(GIT_TIMEOUT, command.output())
            .await
            .map_err(|_| format!("git {} took too long", args[0]))?
            .map_err(|e| format!("running git: {e}"))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let reason = stderr.lines().rfind(|l| !l.trim().is_empty()).unwrap_or("");
            Err(format!("git {}: {}", args[0], reason.trim()))
        }
    }
}

/// Whether this daemon can update itself, and from where.
fn setup(exe: &Path) -> Result<InstallRecord, String> {
    let not_installed = || "not installed with install.sh".to_owned();
    let path = InstallRecord::find(exe).ok_or_else(not_installed)?;
    let text = fs::read_to_string(&path).map_err(|_| not_installed())?;
    let record = InstallRecord::parse(&text)
        .ok_or_else(|| format!("{} can't be read; run install.sh again", path.display()))?;
    if !record.updates {
        return Err("updates are turned off on this machine (install.sh --no-updates)".to_owned());
    }
    if !record.source.join(".git").exists() || !record.source.join("install.sh").is_file() {
        return Err(format!(
            "{} is no longer a clone of PluSystemMonitor",
            record.source.display()
        ));
    }
    // install.sh would ask for sudo, and nobody is there to answer.
    if !writable(&record.prefix.join("bin")) {
        return Err(format!(
            "installed to {}, which only root may change",
            record.prefix.display()
        ));
    }
    Ok(record)
}

fn writable(dir: &Path) -> bool {
    let Ok(path) = std::ffi::CString::new(dir.as_os_str().as_encoded_bytes()) else {
        return false;
    };
    // SAFETY: a valid C string; access has no other preconditions.
    unsafe { libc::access(path.as_ptr(), libc::W_OK) == 0 }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// install.sh colours its messages; the log is plain text.
fn strip_ansi(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                // Parameters, then one final byte in @..~.
                for c in chars.by_ref() {
                    if ('@'..='~').contains(&c) {
                        break;
                    }
                }
            }
            continue;
        }
        if c != '\r' {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_install_record() {
        let text = "# written by install.sh\n\
                    source=/home/me/PluSystemMonitor\n\
                    prefix=/home/me/.local\n\
                    daemon_only=1\n\
                    updates=1\n\
                    path=/home/me/.cargo/bin:/usr/bin\n";
        let record = InstallRecord::parse(text).unwrap();
        assert_eq!(record.source, Path::new("/home/me/PluSystemMonitor"));
        assert_eq!(record.prefix, Path::new("/home/me/.local"));
        assert!(record.daemon_only);
        assert!(record.updates);
        assert_eq!(record.path, "/home/me/.cargo/bin:/usr/bin");

        let off = InstallRecord::parse("source=/s\nprefix=/p\nupdates=0\n").unwrap();
        assert!(!off.updates);
        assert!(!off.daemon_only);
        assert!(InstallRecord::parse("prefix=/p\n").is_none());
        assert!(InstallRecord::parse("source=relative\nprefix=/p\n").is_none());
    }

    #[test]
    fn finds_the_record_beside_the_binary() {
        assert_eq!(
            InstallRecord::find(Path::new("/home/me/.local/bin/plusysmond")).unwrap(),
            Path::new("/home/me/.local/share/plusysmon/install.conf")
        );
    }

    #[test]
    fn strips_colours() {
        assert_eq!(
            strip_ansi("\u{1b}[1;35m::\u{1b}[0m Building (release)\r"),
            ":: Building (release)"
        );
        assert_eq!(strip_ansi("plain"), "plain");
    }

    #[test]
    fn without_a_record_updates_are_unavailable() {
        let reason = setup(Path::new("/nonexistent/bin/plusysmond")).unwrap_err();
        assert_eq!(reason, "not installed with install.sh");
    }

    fn git_ok(dir: &Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .unwrap();
        assert!(status.status.success(), "git {args:?}: {status:?}");
    }

    /// A clone two commits behind its upstream.
    #[test]
    fn counts_commits_behind_upstream() {
        let root = std::env::temp_dir().join(format!("plusysmon-update-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let upstream = root.join("upstream");
        fs::create_dir_all(upstream.join("crates/protocol/src")).unwrap();
        git_ok(&upstream, &["init", "-q", "-b", "master"]);
        fs::write(upstream.join("install.sh"), "").unwrap();
        fs::write(
            upstream.join("crates/protocol/src/lib.rs"),
            "pub const PROTOCOL_VERSION: u32 = 6;\npub const OLDEST_COMPATIBLE: u32 = 5;\n",
        )
        .unwrap();
        git_ok(&upstream, &["add", "."]);
        git_ok(&upstream, &["commit", "-q", "-m", "first"]);
        git_ok(
            &root,
            &["clone", "-q", upstream.to_str().unwrap(), "clone"],
        );
        fs::write(
            upstream.join("crates/protocol/src/lib.rs"),
            "pub const PROTOCOL_VERSION: u32 = 9;\npub const OLDEST_COMPATIBLE: u32 = 9;\n",
        )
        .unwrap();
        git_ok(&upstream, &["commit", "-q", "-am", "breaking"]);
        git_ok(&upstream, &["commit", "-q", "--allow-empty", "-m", "latest"]);

        let record = InstallRecord {
            source: root.join("clone"),
            prefix: root.join("prefix"),
            daemon_only: true,
            updates: true,
            path: String::new(),
        };
        let updater = Updater {
            setup: Ok(record.clone()),
            exe: PathBuf::new(),
            status: Mutex::default(),
            last_check: Mutex::default(),
            events: broadcast::channel(4).0,
        };
        let upstream = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(updater.look_upstream(&record))
            .unwrap();
        assert_eq!(upstream.behind, 2);
        assert!(upstream.latest.ends_with(" latest"), "{}", upstream.latest);
        assert_eq!(upstream.versions, Some((9, 9)));
        let _ = fs::remove_dir_all(&root);
    }
}
