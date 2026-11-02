//! The `Fleet` QML element: a quick look at every machine connected to before
//! (is it up, what does it run, are there updates) and updating them. Each
//! look is a connection of its own in probe mode, so no samples flow.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        type Fleet = super::FleetRust;

        /// Looks at every `[{ address, token }]`; with `check`, also has each
        /// daemon look upstream for updates. Answers come as `probed`.
        #[qinvokable]
        fn probe(self: Pin<&mut Self>, entries: &QString, check: bool);

        /// Has the daemon at `address` update itself; progress comes as
        /// `updateEvent`, and a `probed` once it is back.
        #[qinvokable]
        fn update(self: Pin<&mut Self>, address: &QString, token: &QString);

        /// The build this app was made from.
        #[qinvokable]
        fn build(&self) -> QString;

        /// The protocol version this app speaks.
        #[qinvokable]
        fn protocol(&self) -> i32;

        /// Whether this app's binary was replaced since it started, by an
        /// update of this machine.
        #[qinvokable]
        fn app_replaced(&self) -> bool;

        /// Starts the app again from its (new) binary; QML quits after.
        #[qinvokable]
        fn relaunch(&self) -> bool;

        /// What is known about a machine (see `Status`), as JSON. A probe may
        /// answer twice: once it has the daemon's info, again after a check.
        #[qsignal]
        fn probed(self: Pin<&mut Self>, address: &QString, json: &QString);

        /// `{ state, line, error }`: the update's status changing, or a line of its output.
        #[qsignal]
        fn update_event(self: Pin<&mut Self>, address: &QString, json: &QString);
    }

    impl cxx_qt::Threading for Fleet {}
}

use std::path::PathBuf;
use std::pin::Pin;
use std::time::{Duration, Instant, SystemTime};

use cxx_qt::Threading;
use cxx_qt_lib::QString;
use plusysmon_protocol::{
    BUILD_COMMIT, ClientMsg, DaemonInfo, INFO_SINCE, OLDEST_COMPATIBLE, ServerMsg, UpdateStatus,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::client::{self, Link, OpenError};

/// A check runs `git fetch` over there; that can take a while on a slow line.
const CHECK_TIMEOUT: Duration = Duration::from_secs(150);
const ANSWER_TIMEOUT: Duration = Duration::from_secs(6);
/// How long a restarting daemon gets to come back.
const RETURN_TIMEOUT: Duration = Duration::from_secs(120);

pub struct FleetRust {
    started: SystemTime,
    exe: Option<PathBuf>,
}

impl Default for FleetRust {
    fn default() -> Self {
        Self {
            started: SystemTime::now(),
            exe: std::env::current_exe().ok(),
        }
    }
}

#[derive(Deserialize)]
struct Entry {
    address: String,
    #[serde(default)]
    token: String,
}

/// What a probe found out.
#[derive(Debug, Default, Serialize)]
struct Status {
    address: String,
    /// Something answered at the address.
    online: bool,
    /// online (talks to us), offline, untrusted, rejected, error.
    state: &'static str,
    message: String,
    hostname: String,
    /// Protocol version, when known.
    version: u32,
    build: String,
    arch: String,
    /// Whether the daemon and this app can talk.
    compatible: bool,
    /// A daemon from before version 6: no info, no remote updates.
    legacy: bool,
    checking: bool,
    updates: Option<UpdateStatus>,
}

impl Status {
    fn new(address: &str) -> Self {
        Self {
            address: address.to_owned(),
            state: "offline",
            ..Default::default()
        }
    }

    fn with_info(mut self, info: DaemonInfo) -> Self {
        self.state = "online";
        self.hostname = info.hostname;
        self.version = info.version;
        self.build = info.build;
        self.arch = info.arch;
        self.compatible = info.compatible;
        self.updates = Some(info.updates);
        self
    }
}

impl qobject::Fleet {
    pub fn probe(self: Pin<&mut Self>, entries: &QString, check: bool) {
        let entries: Vec<Entry> = serde_json::from_str(&entries.to_string()).unwrap_or_default();
        if entries.is_empty() {
            return;
        }
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            runtime().block_on(async {
                let probes = entries.iter().map(|entry| {
                    let qt_thread = qt_thread.clone();
                    async move {
                        let report = |status: &Status| {
                            let address = QString::from(&entry.address);
                            let json = QString::from(&serde_json::to_string(status).unwrap());
                            let _ = qt_thread.queue(move |fleet| fleet.probed(&address, &json));
                        };
                        probe_one(&entry.address, &entry.token, check, report).await;
                    }
                });
                futures_util::future::join_all(probes).await;
            });
        });
    }

    pub fn update(self: Pin<&mut Self>, address: &QString, token: &QString) {
        let address = address.to_string();
        let token = token.to_string();
        let qt_thread = self.qt_thread();
        std::thread::spawn(move || {
            runtime().block_on(async {
                let event = |value: serde_json::Value| {
                    let address = QString::from(&address);
                    let json = QString::from(&value.to_string());
                    let _ = qt_thread.queue(move |fleet| fleet.update_event(&address, &json));
                };
                let result = run_update(&address, &token, &event).await;
                match result {
                    Ok(()) => {
                        event(json!({ "state": "done" }));
                        let report = |status: &Status| {
                            let address = QString::from(&address);
                            let json = QString::from(&serde_json::to_string(status).unwrap());
                            let _ = qt_thread.queue(move |fleet| fleet.probed(&address, &json));
                        };
                        probe_one(&address, &token, false, report).await;
                    }
                    Err(error) => event(json!({ "state": "failed", "error": error })),
                }
            });
        });
    }

    pub fn build(&self) -> QString {
        QString::from(BUILD_COMMIT)
    }

    pub fn protocol(&self) -> i32 {
        plusysmon_protocol::PROTOCOL_VERSION as i32
    }

    pub fn app_replaced(&self) -> bool {
        self.exe_path()
            .and_then(|exe| std::fs::metadata(exe).ok()?.modified().ok())
            .is_some_and(|modified| modified > self.started)
    }

    pub fn relaunch(&self) -> bool {
        let Some(exe) = self.exe_path() else {
            return false;
        };
        std::process::Command::new(exe).spawn().is_ok()
    }

    /// Where the binary is now. Once replaced, /proc/self/exe says "(deleted)".
    fn exe_path(&self) -> Option<PathBuf> {
        let exe = self.exe.as_ref()?.to_string_lossy().into_owned();
        Some(PathBuf::from(exe.trim_end_matches(" (deleted)")))
    }
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("building fleet runtime")
}

/// Connects in probe mode and reports what it finds; with `check`, reports
/// again once the daemon has looked upstream.
async fn probe_one(address: &str, token: &str, check: bool, report: impl Fn(&Status)) {
    let authority = client::authority(address);
    let token = client::resolve_token(address, token);
    let mut status = Status::new(address);
    let mut link = match client::open(&authority).await {
        Ok(link) => link,
        Err(OpenError::Unreachable(e)) => {
            status.message = format!("{e:#}");
            return report(&status);
        }
        Err(OpenError::Untrusted { .. }) => {
            status.online = true;
            status.state = "untrusted";
            status.message = "its certificate isn't trusted yet; connect to it once".to_owned();
            return report(&status);
        }
        Err(OpenError::Failed(e)) => {
            status.online = true;
            status.state = "error";
            status.message = format!("{e:#}");
            return report(&status);
        }
    };
    status.online = true;
    let first = async {
        link.hello(&token, plusysmon_protocol::PROTOCOL_VERSION, true)
            .await?;
        link.recv().await
    };
    let status = match tokio::time::timeout(ANSWER_TIMEOUT, first).await {
        Ok(Ok(Some(ServerMsg::Info(info)))) => status.with_info(info),
        Ok(Ok(Some(ServerMsg::Error(reason)))) => match client::rejected_version(&reason) {
            Some(version) => {
                status.state = "online";
                status.legacy = true;
                status.version = version;
                status.compatible = version >= OLDEST_COMPATIBLE;
                status
            }
            None => {
                status.state = "rejected";
                status.message = reason;
                status
            }
        },
        Ok(Ok(_)) => {
            status.state = "error";
            status.message = "unexpected answer".to_owned();
            status
        }
        Ok(Err(e)) => {
            status.state = "error";
            status.message = format!("{e:#}");
            status
        }
        Err(_) => {
            status.state = "error";
            status.message = "no answer".to_owned();
            status
        }
    };
    let can_check = status.updates.as_ref().is_some_and(|u| u.unavailable.is_empty());
    if !(check && can_check) {
        return report(&status);
    }
    let mut status = status;
    status.checking = true;
    report(&status);
    status.checking = false;
    if let Ok(Some(updates)) = tokio::time::timeout(CHECK_TIMEOUT, check_updates(&mut link)).await {
        status.updates = Some(updates);
    }
    report(&status);
}

/// Asks for a check and waits for its outcome.
async fn check_updates(link: &mut Link) -> Option<UpdateStatus> {
    link.send(&ClientMsg::CheckUpdates).await.ok()?;
    loop {
        match link.recv().await.ok()?? {
            ServerMsg::Updates(status) if status.state != "checking" => return Some(status),
            _ => continue,
        }
    }
}

/// Starts the update and follows it until the daemon restarted and came back.
async fn run_update(
    address: &str,
    token: &str,
    event: &impl Fn(serde_json::Value),
) -> Result<(), String> {
    let authority = client::authority(address);
    let token = client::resolve_token(address, token);
    let mut link = client::open(&authority).await.map_err(|e| match e {
        OpenError::Unreachable(e) | OpenError::Failed(e) => format!("{e:#}"),
        OpenError::Untrusted { .. } => "its certificate isn't trusted".to_owned(),
    })?;
    link.hello(&token, plusysmon_protocol::PROTOCOL_VERSION, true)
        .await
        .map_err(|e| format!("{e:#}"))?;
    let build = match link.recv().await {
        Ok(Some(ServerMsg::Info(info))) if info.version >= INFO_SINCE => {
            if !info.updates.unavailable.is_empty() {
                return Err(info.updates.unavailable);
            }
            info.build
        }
        Ok(Some(ServerMsg::Error(reason))) => {
            return Err(if client::rejected_version(&reason).is_some() {
                "this daemon is too old to update itself; run install.sh there once".to_owned()
            } else {
                reason
            });
        }
        Ok(_) => return Err("unexpected answer".to_owned()),
        Err(e) => return Err(format!("{e:#}")),
    };
    link.send(&ClientMsg::Update)
        .await
        .map_err(|e| format!("{e:#}"))?;
    event(json!({ "state": "updating" }));

    let mut restarting = false;
    loop {
        let msg = match link.recv().await {
            Ok(Some(msg)) => msg,
            // The daemon went away: it restarts, or something broke.
            Ok(None) | Err(_) => break,
        };
        match msg {
            ServerMsg::UpdateLog(line) => event(json!({ "line": line })),
            ServerMsg::Updates(status) => {
                event(json!({ "state": status.state, "error": status.error }));
                match status.state.as_str() {
                    "failed" => return Err(status.error),
                    "restarting" => restarting = true,
                    _ => {}
                }
            }
            _ => {}
        }
    }
    if !restarting {
        return Err("the connection dropped during the update".to_owned());
    }

    // Wait for the new daemon, and make sure it's a new one.
    event(json!({ "state": "restarting", "line": "Waiting for the daemon to come back…" }));
    let deadline = Instant::now() + RETURN_TIMEOUT;
    while Instant::now() < deadline {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let Ok(mut link) = client::open(&authority).await else {
            continue;
        };
        if link
            .hello(&token, plusysmon_protocol::PROTOCOL_VERSION, true)
            .await
            .is_err()
        {
            continue;
        }
        // The old connection only ends once the daemon replaced itself, so
        // whoever answers now is the new one.
        if let Ok(Ok(Some(ServerMsg::Info(info)))) =
            tokio::time::timeout(ANSWER_TIMEOUT, link.recv()).await
        {
            event(json!({ "line": format!("Back: was {build}, now {}", info.build) }));
            return Ok(());
        }
    }
    Err("the daemon didn't come back; see journalctl --user -u plusysmond over there".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> (String, String) {
        let address = std::env::var("PLUSYSMON_ADDRESS").unwrap_or_else(|_| "127.0.0.1".into());
        let token = std::env::var("PLUSYSMON_TOKEN").unwrap_or_default();
        (address, token)
    }

    /// `PLUSYSMON_ADDRESS=host[:port] PLUSYSMON_TOKEN=… cargo test -p plusysmon
    /// live_probe -- --ignored --nocapture` shows what the Updates dialog sees.
    #[test]
    #[ignore]
    fn live_probe() {
        let (address, token) = target();
        runtime().block_on(probe_one(&address, &token, true, |status| {
            println!("{}", serde_json::to_string_pretty(status).unwrap());
        }));
    }

    /// The same, but updates that machine.
    #[test]
    #[ignore]
    fn live_update() {
        let (address, token) = target();
        let result = runtime().block_on(run_update(&address, &token, &|event| println!("{event}")));
        println!("{result:?}");
        assert!(result.is_ok());
    }

    #[test]
    fn an_unreachable_machine_is_offline() {
        let seen = std::cell::RefCell::new(Vec::new());
        // Port 9 (discard) is closed nearly everywhere.
        runtime().block_on(probe_one("127.0.0.1:9", "", false, |status| {
            seen.borrow_mut().push((status.online, status.state));
        }));
        assert_eq!(seen.into_inner(), [(false, "offline")]);
    }
}
