//! plusysfand: the part of fan control that needs root, and nothing more.
//!
//! plusysmond (a user service) works out fan speeds from the samples it
//! already has and sends them here over a Unix socket; this sets them. A fan
//! is taken from the firmware on the first `set` and given back, as it was,
//! when its controller says `auto` or `release`, disconnects, or stays quiet
//! for [`HELPER_WATCHDOG_SECS`]; and when this stops. What it took is also
//! written to a state file, so `plusysfand --restore` (systemd's
//! ExecStopPost) and the next start give fans back even after a crash.
//!
//! The protocol is lines of text, one answer per command:
//!
//! ```text
//! hello <version>        → ok plusysfand <HELPER_VERSION>
//! ping                   → ok
//! list                   → out\t<id>\t<driver>\t<percent|->\t<min percent>\t<rpm sensor|->  …  end
//! set <id> <percent>     → ok | err <why>
//! auto <id>              → ok | err <why>
//! release                → ok      (every fan this connection took)
//! ```
//!
//! Anyone may connect and list; only root and the users in
//! `/etc/plusysmon/fand.conf` may set.

mod hardware;

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::{FileTypeExt, PermissionsExt};
use std::os::unix::io::AsRawFd;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use std::{fs, thread};

use hardware::{Hardware, Original, Output};
use plusysmon_protocol::fans::{HELPER_SOCKET, HELPER_VERSION, HELPER_WATCHDOG_SECS};

const USAGE: &str = "\
Usage: plusysfand [OPTIONS]

Sets fan speeds for plusysmond. Runs as root, normally as the plusysfand
system service.

Options:
  --socket <PATH>   Where to listen [default: /run/plusysfand/plusysfand.sock]
  --config <PATH>   Who may set speeds [default: /etc/plusysmon/fand.conf]
  --state <PATH>    What to give back after a crash [default: /run/plusysfand/state]
  --sysfs <DIR>     The sysfs tree, for testing [default: /sys]
  --restore         Give back the fans the state file names, and exit
  --list            Print the fans there are, and exit
  --version         Print the protocol version, and exit
  -h, --help        Print this help";

const CONFIG: &str = "/etc/plusysmon/fand.conf";
const STATE: &str = "/run/plusysfand/state";
/// Commands are short; a longer line is someone else talking.
const MAX_LINE: u64 = 512;
const MAX_CONNECTIONS: usize = 16;

struct Args {
    socket: PathBuf,
    config: PathBuf,
    state: PathBuf,
    sysfs: PathBuf,
    restore: bool,
    list: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        socket: HELPER_SOCKET.into(),
        config: CONFIG.into(),
        state: STATE.into(),
        sysfs: "/sys".into(),
        restore: false,
        list: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut value = || it.next().ok_or_else(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--socket" => args.socket = value()?.into(),
            "--config" => args.config = value()?.into(),
            "--state" => args.state = value()?.into(),
            "--sysfs" => args.sysfs = value()?.into(),
            "--restore" => args.restore = true,
            "--list" => args.list = true,
            "--version" => {
                println!("plusysfand {HELPER_VERSION}");
                std::process::exit(0);
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument {other}\n\n{USAGE}")),
        }
    }
    Ok(args)
}

fn main() {
    let args = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let mut state = State::new(Hardware::new(args.sysfs.clone()), args.state.clone());
    // Whatever a crashed run left behind goes back first.
    state.restore_from_file();
    if args.restore {
        return;
    }
    if args.list {
        for line in state.list() {
            println!("{line}");
        }
        return;
    }

    let allowed = read_allowed(&args.config);
    let state = Arc::new(Mutex::new(state));
    handle_signals(state.clone(), args.socket.clone());
    let listener = match bind(&args.socket) {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("can't listen on {}: {e}", args.socket.display());
            std::process::exit(1);
        }
    };
    eprintln!(
        "plusysfand {HELPER_VERSION} listening on {}; users allowed to set speeds: {allowed:?}",
        args.socket.display()
    );
    serve(
        listener,
        state,
        Arc::new(allowed),
        Duration::from_secs(HELPER_WATCHDOG_SECS),
    );
}

/// Who may set speeds besides root: `users = 1000, 1001` in the config.
fn read_allowed(path: &Path) -> Vec<u32> {
    let Ok(text) = fs::read_to_string(path) else {
        eprintln!(
            "{} is missing; only root may set fan speeds",
            path.display()
        );
        return Vec::new();
    };
    parse_allowed(&text)
}

fn parse_allowed(text: &str) -> Vec<u32> {
    text.lines()
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key.trim() == "users").then_some(value)
        })
        .flat_map(|value| value.split(','))
        .filter_map(|uid| uid.trim().parse().ok())
        .collect()
}

fn bind(socket: &Path) -> std::io::Result<UnixListener> {
    // A socket left by an earlier run; anything else there is not ours to remove.
    if fs::symlink_metadata(socket).is_ok_and(|m| m.file_type().is_socket()) {
        fs::remove_file(socket)?;
    }
    let listener = UnixListener::bind(socket)?;
    // Everyone may connect; who may set speeds is checked per connection.
    fs::set_permissions(socket, fs::Permissions::from_mode(0o666))?;
    Ok(listener)
}

/// Gives every fan back and exits on SIGTERM, SIGINT and SIGHUP.
fn handle_signals(state: Arc<Mutex<State>>, socket: PathBuf) {
    // SAFETY: plain libc signal-mask calls on a local, zeroed sigset_t. The
    // mask is set before any other thread starts, so all of them inherit it
    // and only the waiting thread ever receives these signals.
    let set = unsafe {
        let mut set: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut set);
        for signal in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
            libc::sigaddset(&mut set, signal);
        }
        libc::pthread_sigmask(libc::SIG_BLOCK, &set, std::ptr::null_mut());
        set
    };
    thread::spawn(move || {
        let mut signal = 0;
        // SAFETY: `set` is a valid, initialised signal set.
        unsafe { libc::sigwait(&set, &mut signal) };
        eprintln!("stopping (signal {signal}); giving the fans back");
        lock(&state).release_all();
        let _ = fs::remove_file(&socket);
        std::process::exit(0);
    });
}

fn serve(
    listener: UnixListener,
    state: Arc<Mutex<State>>,
    allowed: Arc<Vec<u32>>,
    watchdog: Duration,
) {
    let active = Arc::new(AtomicUsize::new(0));
    for (number, stream) in listener.incoming().enumerate() {
        let Ok(stream) = stream else { continue };
        if active.load(Ordering::Relaxed) >= MAX_CONNECTIONS {
            continue;
        }
        active.fetch_add(1, Ordering::Relaxed);
        let (state, allowed, active) = (state.clone(), allowed.clone(), active.clone());
        thread::spawn(move || {
            let uid = peer_uid(&stream);
            let may_set = uid.is_some_and(|uid| uid == 0 || allowed.contains(&uid));
            if let Err(e) = converse(stream, number as u64, may_set, &state, watchdog) {
                eprintln!("connection {number} (uid {uid:?}): {e}");
            }
            // However the connection ended, its fans go back.
            lock(&state).release(number as u64);
            active.fetch_sub(1, Ordering::Relaxed);
        });
    }
}

fn converse(
    stream: UnixStream,
    owner: u64,
    may_set: bool,
    state: &Mutex<State>,
    watchdog: Duration,
) -> std::io::Result<()> {
    // Silence for this long gets the connection, and its fans, dropped.
    stream.set_read_timeout(Some(watchdog))?;
    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        let read = match (&mut reader).take(MAX_LINE).read_line(&mut line) {
            Ok(read) => read,
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                if lock(state).owns_any(owner) {
                    eprintln!("connection {owner} went quiet; giving its fans back");
                }
                return Ok(());
            }
            Err(e) => return Err(e),
        };
        if read == 0 {
            return Ok(());
        }
        if !line.ends_with('\n') {
            writer.write_all(b"err line too long\n")?;
            return Ok(());
        }
        let reply = lock(state).command(owner, may_set, line.trim());
        writer.write_all(reply.as_bytes())?;
    }
}

/// The connecting process' user.
fn peer_uid(stream: &UnixStream) -> Option<u32> {
    let mut cred: libc::ucred = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: getsockopt writes at most `len` bytes into `cred`, which is that big.
    let ok = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut cred as *mut libc::ucred).cast(),
            &mut len,
        )
    } == 0;
    ok.then_some(cred.uid)
}

/// A panicked connection thread must not keep the fans from going back.
fn lock(state: &Mutex<State>) -> MutexGuard<'_, State> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct Claim {
    owner: u64,
    original: Original,
    /// The raw value last written.
    last: Option<u32>,
}

struct State {
    hardware: Hardware,
    outputs: BTreeMap<String, Output>,
    claims: BTreeMap<String, Claim>,
    state_file: PathBuf,
}

impl State {
    fn new(mut hardware: Hardware, state_file: PathBuf) -> Self {
        let outputs = hardware.scan();
        Self {
            hardware,
            outputs,
            claims: BTreeMap::new(),
            state_file,
        }
    }

    fn command(&mut self, owner: u64, may_set: bool, line: &str) -> String {
        let words: Vec<&str> = line.split_whitespace().collect();
        let result = match words.as_slice() {
            ["hello", ..] => return format!("ok plusysfand {HELPER_VERSION}\n"),
            ["ping"] => Ok(()),
            ["list"] => {
                let mut out = self.list().join("\n");
                out.push_str(if out.is_empty() { "end\n" } else { "\nend\n" });
                return out;
            }
            ["set" | "auto" | "release", ..] if !may_set => {
                Err("this user may not set fan speeds (see /etc/plusysmon/fand.conf)".to_owned())
            }
            ["set", id, percent] => match percent.parse::<f64>() {
                Ok(percent) if (0.0..=100.0).contains(&percent) => self.set(owner, id, percent),
                _ => Err("the speed must be 0 to 100".to_owned()),
            },
            ["auto", id] => self.auto(owner, id),
            ["release"] => {
                self.release(owner);
                Ok(())
            }
            _ => Err("unknown command".to_owned()),
        };
        match result {
            Ok(()) => "ok\n".to_owned(),
            Err(e) => format!("err {}\n", e.replace('\n', " ")),
        }
    }

    fn list(&mut self) -> Vec<String> {
        self.outputs = self.hardware.scan();
        let outputs: Vec<Output> = self.outputs.values().cloned().collect();
        outputs
            .iter()
            .map(|output| {
                let percent = self
                    .hardware
                    .percent(output)
                    .map_or_else(|| "-".to_owned(), |p| format!("{p:.1}"));
                let clean = |s: &str| s.replace(['\t', '\n'], " ");
                format!(
                    "out\t{}\t{}\t{percent}\t{}\t{}",
                    output.id,
                    clean(&output.driver),
                    output.min_percent,
                    output.rpm_sensor.as_deref().unwrap_or("-")
                )
            })
            .collect()
    }

    /// The output called `id`, looking again if it's new (hotplug, a GPU
    /// driver loaded late).
    fn output(&mut self, id: &str) -> Result<Output, String> {
        if !self.outputs.contains_key(id) {
            self.outputs = self.hardware.scan();
        }
        self.outputs
            .get(id)
            .cloned()
            .ok_or_else(|| format!("no fan {id}"))
    }

    fn set(&mut self, owner: u64, id: &str, percent: f64) -> Result<(), String> {
        let output = self.output(id)?;
        let last = match self.claims.get(id) {
            Some(claim) if claim.owner != owner => {
                return Err("another controller has this fan".to_owned());
            }
            Some(claim) => claim.last,
            None => {
                let original = self.hardware.original(&output)?;
                eprintln!("taking {id}");
                self.claims.insert(
                    id.to_owned(),
                    Claim {
                        owner,
                        original,
                        last: None,
                    },
                );
                // Recorded before the first write, so a crash can't lose it.
                self.persist();
                None
            }
        };
        let raw = self.hardware.set(&output, percent, last)?;
        if let Some(claim) = self.claims.get_mut(id) {
            claim.last = Some(raw);
        }
        Ok(())
    }

    fn auto(&mut self, owner: u64, id: &str) -> Result<(), String> {
        match self.claims.get(id) {
            None => Ok(()),
            Some(claim) if claim.owner != owner => {
                Err("another controller has this fan".to_owned())
            }
            Some(_) => {
                let claim = self.claims.remove(id).expect("checked above");
                let result = self.give_back(id, &claim.original);
                self.persist();
                result
            }
        }
    }

    fn owns_any(&self, owner: u64) -> bool {
        self.claims.values().any(|c| c.owner == owner)
    }

    fn release(&mut self, owner: u64) {
        let ids: Vec<String> = self
            .claims
            .iter()
            .filter(|(_, c)| c.owner == owner)
            .map(|(id, _)| id.clone())
            .collect();
        if ids.is_empty() {
            return;
        }
        for id in ids {
            let claim = self.claims.remove(&id).expect("listed above");
            let _ = self.give_back(&id, &claim.original);
        }
        self.persist();
    }

    fn release_all(&mut self) {
        for (id, claim) in std::mem::take(&mut self.claims) {
            let _ = self.give_back(&id, &claim.original);
        }
        self.persist();
    }

    fn give_back(&mut self, id: &str, original: &Original) -> Result<(), String> {
        let result = self.hardware.restore(original);
        match &result {
            Ok(()) => eprintln!("gave {id} back"),
            Err(e) => eprintln!("giving {id} back failed: {e}"),
        }
        result
    }

    /// Writes what is held now, or removes the file when nothing is.
    fn persist(&self) {
        if self.claims.is_empty() {
            let _ = fs::remove_file(&self.state_file);
            return;
        }
        let text: String = self
            .claims
            .values()
            .map(|c| c.original.to_line() + "\n")
            .collect();
        let tmp = self.state_file.with_extension("tmp");
        if let Err(e) = fs::write(&tmp, text).and_then(|()| fs::rename(&tmp, &self.state_file)) {
            eprintln!("can't write {}: {e}", self.state_file.display());
        }
    }

    fn restore_from_file(&mut self) {
        let Ok(text) = fs::read_to_string(&self.state_file) else {
            return;
        };
        eprintln!("giving back the fans an earlier run held");
        for line in text.lines() {
            match Original::from_line(line, self.hardware.sysfs()) {
                Some(original) => {
                    if let Err(e) = self.hardware.restore(&original) {
                        eprintln!("{e}");
                    }
                }
                None => eprintln!("ignoring a state line: {line:?}"),
            }
        }
        let _ = fs::remove_file(&self.state_file);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::tests::{fake_sysfs, read};

    struct Helper {
        root: PathBuf,
        socket: PathBuf,
        state_file: PathBuf,
    }

    impl Helper {
        fn start(tag: &str, allowed: Vec<u32>, watchdog: Duration) -> Self {
            let root = fake_sysfs(tag);
            let socket = root.join("fand.sock");
            let state_file = root.join("state");
            let state = Arc::new(Mutex::new(State::new(
                Hardware::new(root.clone()),
                state_file.clone(),
            )));
            let listener = bind(&socket).unwrap();
            thread::spawn(move || serve(listener, state, Arc::new(allowed), watchdog));
            Self {
                root,
                socket,
                state_file,
            }
        }

        fn connect(&self) -> Conn {
            let stream = UnixStream::connect(&self.socket).unwrap();
            Conn {
                reader: BufReader::new(stream.try_clone().unwrap()),
                writer: stream,
            }
        }
    }

    impl Drop for Helper {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    struct Conn {
        reader: BufReader<UnixStream>,
        writer: UnixStream,
    }

    impl Conn {
        fn ask(&mut self, line: &str) -> String {
            writeln!(self.writer, "{line}").unwrap();
            let mut reply = String::new();
            self.reader.read_line(&mut reply).unwrap();
            reply.trim().to_owned()
        }
    }

    fn me() -> u32 {
        // SAFETY: getuid cannot fail.
        unsafe { libc::getuid() }
    }

    fn wait_for(what: impl Fn() -> bool) {
        for _ in 0..200 {
            if what() {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
        panic!("timed out");
    }

    #[test]
    fn reads_allowed_users() {
        assert_eq!(
            parse_allowed("# who\nusers = 1000, 1001\nother = 5\n"),
            [1000, 1001]
        );
        assert!(parse_allowed("").is_empty());
    }

    #[test]
    fn takes_fans_and_gives_them_back_on_disconnect() {
        let helper = Helper::start("disconnect", vec![me()], Duration::from_secs(10));
        let mut conn = helper.connect();
        assert_eq!(
            conn.ask("hello 1"),
            format!("ok plusysfand {HELPER_VERSION}")
        );
        assert_eq!(conn.ask("set it8689.pwm1 40"), "ok");
        assert_eq!(read(&helper.root, "pwm1"), "102");
        assert_eq!(read(&helper.root, "pwm1_enable"), "1");
        // What to give back is on disk.
        assert!(
            fs::read_to_string(&helper.state_file)
                .unwrap()
                .contains("pwm1")
        );

        assert!(conn.ask("set it8689.pwm9 40").starts_with("err no fan"));
        assert!(conn.ask("set it8689.pwm1 140").starts_with("err"));
        assert!(conn.ask("set it8689.pwm3 40").starts_with("err no fan"));

        drop(conn);
        wait_for(|| read(&helper.root, "pwm1_enable") == "2");
        assert_eq!(read(&helper.root, "pwm1"), "128");
        assert!(!helper.state_file.exists());
    }

    #[test]
    fn the_watchdog_gives_fans_back() {
        let helper = Helper::start("watchdog", vec![me()], Duration::from_millis(300));
        let mut conn = helper.connect();
        assert_eq!(conn.ask("set it8689.pwm2 20"), "ok");
        assert_eq!(read(&helper.root, "pwm2"), "51");
        // Keeping in touch keeps the fan.
        for _ in 0..3 {
            thread::sleep(Duration::from_millis(150));
            assert_eq!(conn.ask("ping"), "ok");
        }
        assert_eq!(read(&helper.root, "pwm2"), "51");
        // Then silence.
        wait_for(|| read(&helper.root, "pwm2") == "200");
    }

    #[test]
    fn one_controller_per_fan_and_only_allowed_users() {
        let helper = Helper::start("owners", vec![me()], Duration::from_secs(10));
        let mut first = helper.connect();
        let mut second = helper.connect();
        assert_eq!(first.ask("set it8689.pwm1 50"), "ok");
        assert!(second.ask("set it8689.pwm1 90").starts_with("err another"));
        assert!(second.ask("auto it8689.pwm1").starts_with("err another"));
        assert_eq!(first.ask("auto it8689.pwm1"), "ok");
        assert_eq!(read(&helper.root, "pwm1_enable"), "2");
        assert_eq!(second.ask("set it8689.pwm1 90"), "ok");

        let strangers = Helper::start("strangers", vec![], Duration::from_secs(10));
        let mut conn = strangers.connect();
        if me() != 0 {
            assert!(conn.ask("set it8689.pwm1 50").starts_with("err this user"));
        }
        // Looking is fine.
        writeln!(conn.writer, "list").unwrap();
        let mut lines = Vec::new();
        loop {
            let mut line = String::new();
            conn.reader.read_line(&mut line).unwrap();
            if line.trim() == "end" {
                break;
            }
            lines.push(line.trim().to_owned());
        }
        assert_eq!(
            lines,
            [
                "out\tit8689.pwm1\tit8689\t50.2\t0\tit8689.fan1",
                "out\tit8689.pwm2\tit8689\t78.4\t0\t-"
            ]
        );
    }

    #[test]
    fn a_new_run_gives_back_what_a_crashed_one_held() {
        let root = fake_sysfs("crash");
        let state_file = root.join("state");
        let mut state = State::new(Hardware::new(root.clone()), state_file.clone());
        state.set(1, "it8689.pwm1", 100.0).unwrap();
        assert_eq!(read(&root, "pwm1"), "255");
        // Crash: the state is never released.
        std::mem::forget(state);

        let mut next = State::new(Hardware::new(root.clone()), state_file.clone());
        next.restore_from_file();
        assert_eq!(read(&root, "pwm1"), "128");
        assert_eq!(read(&root, "pwm1_enable"), "2");
        assert!(!state_file.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
