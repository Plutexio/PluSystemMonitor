//! The widget service: data sources for desktop widgets (the Plasma applet
//! in packaging/plasmoid), on the session D-Bus.
//!
//! A widget names a machine and the sources it shows. This machine is read
//! through its own listening socket, other machines the way the app reads
//! them: with the tokens and certificates the app saved. So a widget shows
//! everything the app can, virtual sensors included, and needs no app running.
//!
//! Connections open when a widget first asks and close a minute after the
//! last one stopped asking.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use plusysmon_client::apps;
use plusysmon_client::client::{self, Command, Event};
use plusysmon_client::machines::Connection;
use plusysmon_client::metrics::Metrics;
use plusysmon_client::paths;
use plusysmon_client::processes::ProcessTable;
use plusysmon_client::virtual_sensors::VirtualSensors;
use serde_json::{Value, json};
use tokio::sync::Notify;
use tokio::sync::mpsc::{self, UnboundedSender};
use tracing::{info, warn};

pub const BUS_NAME: &str = "io.github.plutexio.PluSystemMonitor";
const OBJECT_PATH: &str = "/Widgets";

/// A machine nobody asked about for this long is let go.
const IDLE: Duration = Duration::from_secs(60);
/// App and game sources stop being collected this long after the last
/// widget showing one asked.
const PROCESSES_IDLE: Duration = Duration::from_secs(30);
/// A machine that turned us away (untrusted, bad token) is tried again after this.
const RETRY_ENDED: Duration = Duration::from_secs(20);

/// Serves the widget service until the daemon stops. Without a session bus
/// (a server) it says so once and does nothing.
pub async fn run(listen: SocketAddr, token: String, hostname: String) {
    let hub = Arc::new(Hub {
        local: local_authority(listen),
        local_token: token,
        hostname,
        machines: Mutex::default(),
        virtuals: Mutex::default(),
    });
    let service = Service { hub: hub.clone() };
    let connection = match zbus::connection::Builder::session()
        .and_then(|b| b.name(BUS_NAME))
        .and_then(|b| b.serve_at(OBJECT_PATH, service))
    {
        Ok(builder) => builder.build().await,
        Err(e) => Err(e),
    };
    let _connection = match connection {
        Ok(connection) => connection,
        Err(e) => {
            info!("widget service off (no session bus, or another daemon has it): {e}");
            return;
        }
    };
    info!("widget service on the session bus as {BUS_NAME}");

    let mut ticker = tokio::time::interval(Duration::from_secs(10));
    loop {
        ticker.tick().await;
        hub.tidy();
    }
}

/// Where this daemon can be reached from here.
fn local_authority(listen: SocketAddr) -> String {
    let ip = match listen.ip() {
        IpAddr::V4(ip) if ip.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(ip) if ip.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
        ip => ip,
    };
    SocketAddr::new(ip, listen.port()).to_string()
}

struct Service {
    hub: Arc<Hub>,
}

#[zbus::interface(name = "io.github.plutexio.PluSystemMonitor.Widgets")]
impl Service {
    /// `[{ address, name, local }]`: this machine (address "") and every
    /// other one the app connected to.
    fn machines(&self) -> String {
        self.hub.machines_json().to_string()
    }

    /// `{ state, connected, status, hostname, sources: [definition] }`: what
    /// `machine` offers, for picking sources.
    fn catalog(&self, machine: &str) -> String {
        self.hub.catalog(machine).to_string()
    }

    /// `{ state, connected, status, offlineSince, hostname, intervalMs,
    /// sources: [source] }`, a source being what the app's widgets get
    /// (`Metrics::get`). `state` is connecting, online, offline (it went
    /// away; being retried) or refused (it turned us away; `status` says
    /// why). Offline, the sources keep their last values, and
    /// `offlineSince` is when (Unix milliseconds). Widgets call this once
    /// per interval; it also keeps the machine connected.
    fn read(&self, machine: &str, ids: Vec<String>) -> String {
        self.hub.read(machine, &ids).to_string()
    }
}

struct Hub {
    local: String,
    local_token: String,
    hostname: String,
    machines: Mutex<HashMap<String, Arc<Mutex<Machine>>>>,
    virtuals: Mutex<Virtuals>,
}

/// The app's virtual sensors, read again when the file changes.
#[derive(Default)]
struct Virtuals {
    modified: Option<SystemTime>,
    version: u64,
    sensors: VirtualSensors,
}

#[derive(Clone, Copy, PartialEq)]
enum State {
    Connecting,
    Online,
    Offline,
    Refused,
}

impl State {
    fn name(self) -> &'static str {
        match self {
            State::Connecting => "connecting",
            State::Online => "online",
            State::Offline => "offline",
            State::Refused => "refused",
        }
    }
}

struct Machine {
    state: State,
    connected: bool,
    status: String,
    /// When it went offline (or first couldn't be reached), Unix milliseconds.
    offline_since: Option<u64>,
    hostname: String,
    interval_ms: u32,
    metrics: Metrics,
    processes: ProcessTable,
    /// Virtual sensors version applied to `metrics`.
    virtuals: u64,
    /// When a widget last asked for app or game sources, if one did lately.
    processes_wanted: Option<Instant>,
    last_read: Instant,
    /// Set once the daemon turned us away for good (until retried).
    ended: Option<Instant>,
    commands: UnboundedSender<Command>,
    stop: Arc<Notify>,
}

impl Drop for Machine {
    fn drop(&mut self) {
        self.stop.notify_one();
    }
}

impl Hub {
    /// The address of the machine a widget names: "" is this one.
    fn authority(&self, machine: &str) -> String {
        let machine = machine.trim();
        if machine.is_empty() || client::is_local(machine) {
            self.local.clone()
        } else {
            client::authority(machine)
        }
    }

    fn machines_json(&self) -> Value {
        let mut list = vec![json!({
            "address": "",
            "name": format!("This computer ({})", self.hostname),
            "local": true,
        })];
        for entry in Connection::load().remote() {
            let name = if entry.hostname.is_empty() {
                entry.address.clone()
            } else {
                format!("{} ({})", entry.hostname, entry.address)
            };
            list.push(json!({ "address": entry.address, "name": name, "local": false }));
        }
        Value::Array(list)
    }

    fn catalog(&self, machine: &str) -> Value {
        let machine = self.machine(machine);
        let machine = machine.lock().unwrap();
        let sources: Value = serde_json::from_str(&machine.metrics.catalog()).unwrap_or_default();
        json!({
            "state": machine.state.name(),
            "connected": machine.connected,
            "status": machine.status,
            "hostname": machine.hostname,
            "sources": sources,
        })
    }

    fn read(&self, machine: &str, ids: &[String]) -> Value {
        let machine = self.machine(machine);
        let mut machine = machine.lock().unwrap();
        if ids
            .iter()
            .any(|id| id.starts_with("app.") || id.starts_with("game."))
        {
            if machine.processes_wanted.is_none() {
                let _ = machine
                    .commands
                    .send(Command::Subscribe { processes: true });
            }
            machine.processes_wanted = Some(Instant::now());
        }
        let sources: Vec<Value> = ids
            .iter()
            .map(|id| {
                machine
                    .metrics
                    .get_value(id)
                    .unwrap_or_else(|| json!({ "id": id, "name": id, "available": false }))
            })
            .collect();
        json!({
            "state": machine.state.name(),
            "connected": machine.connected,
            "status": machine.status,
            "offlineSince": machine.offline_since,
            "hostname": machine.hostname,
            "intervalMs": machine.interval_ms,
            "sources": sources,
        })
    }

    /// The machine, connecting to it if nobody asked lately; marked as read.
    fn machine(&self, name: &str) -> Arc<Mutex<Machine>> {
        let authority = self.authority(name);
        let (version, sensors) = self.virtuals();
        let mut machines = self.machines.lock().unwrap();
        let retry = machines.get(&authority).is_some_and(|m| {
            m.lock()
                .unwrap()
                .ended
                .is_some_and(|at| at.elapsed() >= RETRY_ENDED)
        });
        if retry {
            machines.remove(&authority);
        }
        let machine = machines
            .entry(authority.clone())
            .or_insert_with(|| self.connect(&authority))
            .clone();
        drop(machines);
        {
            let mut m = machine.lock().unwrap();
            m.last_read = Instant::now();
            if m.virtuals != version {
                m.virtuals = version;
                m.metrics.set_virtuals(sensors);
            }
        }
        machine
    }

    fn connect(&self, authority: &str) -> Arc<Mutex<Machine>> {
        let token = if *authority == self.local {
            self.local_token.clone()
        } else {
            Connection::load().token_for(authority).unwrap_or_default()
        };
        let stop = Arc::new(Notify::new());
        let (commands, command_rx) = mpsc::unbounded_channel();
        let machine = Arc::new(Mutex::new(Machine {
            state: State::Connecting,
            connected: false,
            status: "Connecting…".to_owned(),
            offline_since: None,
            hostname: String::new(),
            interval_ms: 1000,
            metrics: Metrics::default(),
            processes: ProcessTable::default(),
            virtuals: 0,
            processes_wanted: None,
            last_read: Instant::now(),
            ended: None,
            commands,
            stop: stop.clone(),
        }));
        let weak: Weak<Mutex<Machine>> = Arc::downgrade(&machine);
        let has_token = !token.is_empty();
        client::spawn(
            authority.to_owned(),
            token,
            stop,
            command_rx,
            move |event| {
                // Gone means no widget wants it any more: the connection ends.
                let Some(machine) = weak.upgrade() else {
                    return false;
                };
                machine.lock().unwrap().apply(event, has_token);
                true
            },
        );
        machine
    }

    /// The virtual sensors and their version, read again if the file changed.
    fn virtuals(&self) -> (u64, VirtualSensors) {
        let mut virtuals = self.virtuals.lock().unwrap();
        let modified = paths::virtual_sensors_file()
            .ok()
            .and_then(|path| std::fs::metadata(path).ok())
            .and_then(|meta| meta.modified().ok());
        if virtuals.version == 0 || modified != virtuals.modified {
            virtuals.modified = modified;
            virtuals.version += 1;
            virtuals.sensors = VirtualSensors::load().unwrap_or_else(|e| {
                warn!("widgets: {e:#}");
                VirtualSensors::default()
            });
        }
        (virtuals.version, virtuals.sensors.clone())
    }

    /// Lets go of machines nobody looks at, and of process streams no widget needs.
    fn tidy(&self) {
        let mut machines = self.machines.lock().unwrap();
        machines.retain(|authority, machine| {
            let mut machine = machine.lock().unwrap();
            if machine.last_read.elapsed() >= IDLE {
                info!("widgets: no longer watching {authority}");
                return false;
            }
            if machine
                .processes_wanted
                .is_some_and(|at| at.elapsed() >= PROCESSES_IDLE)
            {
                machine.processes_wanted = None;
                let _ = machine
                    .commands
                    .send(Command::Subscribe { processes: false });
                machine.processes.clear();
                machine.metrics.record_apps(&[]);
            }
            true
        });
    }
}

impl Machine {
    fn apply(&mut self, event: Event, has_token: bool) {
        match event {
            // Offline stays offline until it answers; each retry says Connecting.
            Event::Connecting(_) => {
                if self.state != State::Offline {
                    self.state = State::Connecting;
                    self.status = "Connecting…".to_owned();
                }
            }
            Event::Connected {
                hostname,
                interval_ms,
                history,
                fingerprint,
                ..
            } => {
                self.metrics.clear();
                self.metrics.set_machine(&fingerprint);
                for sample in &history {
                    self.metrics.record(sample);
                }
                self.hostname = hostname;
                self.interval_ms = interval_ms;
                self.connected = true;
                self.state = State::Online;
                self.offline_since = None;
                self.status = "Connected".to_owned();
            }
            Event::Sample(sample) => self.metrics.record(&sample),
            Event::Processes(update) => {
                self.processes.apply(update);
                let (apps, _) = apps::group(&self.processes);
                self.metrics.record_apps(&apps);
            }
            Event::Lost(reason) => {
                self.disconnected();
                self.state = State::Offline;
                self.offline_since.get_or_insert_with(now_ms);
                self.status = offline_reason(&reason).to_owned();
            }
            Event::Rejected(reason) => {
                self.disconnected();
                self.state = State::Refused;
                self.ended = Some(Instant::now());
                self.status = if has_token {
                    format!("Rejected by the daemon: {reason}")
                } else {
                    "No token for this machine. Connect to it once in PluSystemMonitor.".to_owned()
                };
            }
            Event::Incompatible(info) => {
                self.disconnected();
                self.state = State::Refused;
                self.ended = Some(Instant::now());
                self.status = client::incompatibility(&info.hostname, info.version);
            }
            Event::Untrusted { authority, .. } => {
                self.disconnected();
                self.state = State::Refused;
                self.ended = Some(Instant::now());
                self.status = format!(
                    "The certificate of {authority} isn't trusted yet. Connect to it once in \
                     PluSystemMonitor."
                );
            }
            Event::SignalResult { .. } | Event::Asset { .. } | Event::SensorSettings { .. }
            | Event::Fans { .. } => {}
        }
    }

    fn disconnected(&mut self) {
        self.connected = false;
        self.processes.clear();
        self.metrics.record_apps(&[]);
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// Why a machine can't be reached, in a few words rather than the OS error.
fn offline_reason(error: &str) -> &'static str {
    let error = error.to_lowercase();
    if error.contains("refused") {
        "It's up, but its daemon isn't running."
    } else if error.contains("no route") || error.contains("unreachable") {
        "It can't be reached from here."
    } else if error.contains("timed out") {
        "It doesn't answer."
    } else if error.contains("lookup")
        || error.contains("resolve")
        || error.contains("name or service")
    {
        "Its name can't be found."
    } else {
        "The connection broke."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn says_why_briefly() {
        assert_eq!(
            offline_reason("Connection refused (os error 111)"),
            "It's up, but its daemon isn't running."
        );
        assert_eq!(offline_reason("timed out"), "It doesn't answer.");
        assert_eq!(
            offline_reason("No route to host (os error 113)"),
            "It can't be reached from here."
        );
        assert_eq!(offline_reason("connection closed"), "The connection broke.");
    }

    #[test]
    fn reaches_itself_on_loopback() {
        let at = |s: &str| local_authority(s.parse().unwrap());
        assert_eq!(at("0.0.0.0:7878"), "127.0.0.1:7878");
        assert_eq!(at("[::]:7879"), "[::1]:7879");
        assert_eq!(at("192.168.1.4:7878"), "192.168.1.4:7878");
        assert_eq!(at("127.0.0.1:7880"), "127.0.0.1:7880");
    }
}
