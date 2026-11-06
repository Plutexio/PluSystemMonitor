//! Wire protocol shared by the daemon and its frontends.
//!
//! Connections are TLS 1.3, always; the daemon's certificate is self-signed and
//! clients pin its [`cert_fingerprint`], so the token in the hello stays secret.
//!
//! Every WebSocket binary frame carries one MessagePack-encoded message.
//! The client opens with [`ClientMsg::Hello`]; the daemon answers with
//! [`ServerMsg::Welcome`] (including buffered history) and then streams
//! [`ServerMsg::Sample`] at its sampling interval.
//!
//! Process data is opt-in: after [`ClientMsg::Subscribe`] with `processes`, the
//! daemon sends one full [`ProcessUpdate`] and then only the differences.
//!
//! # Versions
//!
//! Both sides send their [`PROTOCOL_VERSION`] and the oldest version of the
//! other side they still work with ([`OLDEST_COMPATIBLE`]). Additions bump the
//! version only: new fields get `#[serde(default)]` (named fields keep older
//! readers working), and new messages only go to peers whose version knows
//! them. A change that can't be made that way also raises `OLDEST_COMPATIBLE`,
//! so the older side is told it needs an update instead of misreading things.
//!
//! What lets two such builds still talk is *stable*: the hello, the daemon's
//! [`ServerMsg::Info`], and the update messages. They exist from version 6 on
//! and only ever gain optional fields, so a frontend can update any daemon,
//! however far apart the two have drifted.

pub mod fans;

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Bumped with every change to the message types, compatible or not.
pub const PROTOCOL_VERSION: u32 = 7;

/// The oldest peer this build still talks to. Raised (to `PROTOCOL_VERSION`)
/// only by changes older peers can't cope with.
pub const OLDEST_COMPATIBLE: u32 = 5;

/// The first version with [`ServerMsg::Info`], updates and sensor settings.
pub const INFO_SINCE: u32 = 6;

/// The first version with fan control messages.
pub const FANS_SINCE: u32 = 7;

/// The git commit this was built from (short hash), or "unknown".
pub const BUILD_COMMIT: &str = env!("PLUSYSMON_COMMIT");

pub const DEFAULT_PORT: u16 = 7878;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMsg {
    /// Stable.
    Hello {
        version: u32,
        token: String,
        /// The oldest daemon this client works with (since 6).
        #[serde(default)]
        oldest: u32,
        /// Only [`ServerMsg::Info`] and the update messages, no samples: for a
        /// quick look at a machine (since 6).
        #[serde(default)]
        probe: bool,
    },
    /// Replaces the set of optional streams this client wants.
    Subscribe {
        processes: bool,
    },
    Signal {
        pid: u32,
        signal: Signal,
    },
    /// Fetch an icon or artwork named by an [`AppInfo`].
    Asset {
        key: String,
    },
    /// Stable. Look upstream for updates; answered with [`ServerMsg::Updates`].
    CheckUpdates,
    /// Stable. Pull, build and install the update, then restart. Progress comes
    /// as [`ServerMsg::UpdateLog`] and [`ServerMsg::Updates`].
    Update,
    /// Since 6: answered with [`ServerMsg::SensorSettings`].
    GetSensorSettings,
    /// Since 6: replaces the machine's sensor settings; answered like `GetSensorSettings`.
    SetSensorSettings(SensorSettings),
    /// Since 7: answered with [`ServerMsg::Fans`].
    GetFans,
    /// Since 7: replaces the machine's fan control; answered like `GetFans`.
    SetFans(fans::FanSettings),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Signal {
    /// SIGTERM: ask the process to exit.
    Terminate,
    /// SIGKILL: end it immediately.
    Kill,
    /// SIGSTOP
    Stop,
    /// SIGCONT
    Continue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMsg {
    Welcome {
        hostname: String,
        interval_ms: u32,
        /// The daemon's user, so frontends can tell "my processes" apart.
        uid: u32,
        history: Vec<Sample>,
    },
    Sample(Sample),
    Processes(ProcessUpdate),
    SignalResult {
        pid: u32,
        signal: Signal,
        error: Option<String>,
    },
    /// Answer to [`ClientMsg::Asset`]; `data` is empty if the asset is unavailable.
    Asset {
        key: String,
        /// File extension: png, svg, jpg, ...
        format: String,
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    },
    /// Sent right before the daemon closes the connection.
    Error(String),
    /// Stable. The first answer to a hello from version 6 on, before `Welcome`.
    /// When `compatible` is false (or for a probe) no `Welcome` follows; only
    /// the update messages work.
    Info(DaemonInfo),
    /// Stable. Sent when asked and whenever the update status changes.
    Updates(UpdateStatus),
    /// Stable. A line of output from a running update.
    UpdateLog(String),
    /// Since 6. `sensors` are the readings before the settings were applied,
    /// hidden ones included, so they can be shown and set up.
    SensorSettings {
        settings: SensorSettings,
        sensors: Vec<SensorReading>,
        /// Why the settings just sent weren't taken; `settings` are the old ones.
        #[serde(default)]
        error: Option<String>,
    },
    /// Since 7. The machine's fan control and the fans there are to control.
    Fans {
        settings: fans::FanSettings,
        outputs: Vec<fans::FanOutput>,
        status: fans::FanStatus,
        /// Why the settings just sent weren't taken; `settings` are the old ones.
        #[serde(default)]
        error: Option<String>,
    },
}

/// Who a daemon is. Stable: fields may be added, never changed.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DaemonInfo {
    pub hostname: String,
    pub version: u32,
    pub oldest_compatible: u32,
    /// [`BUILD_COMMIT`] of the daemon.
    pub build: String,
    /// CPU architecture: x86_64, aarch64, ...
    pub arch: String,
    /// Whether the daemon streams to this client. When not, one side needs an update.
    pub compatible: bool,
    pub updates: UpdateStatus,
    /// The daemon was built with fan control (since 7).
    pub fans: bool,
}

/// What a daemon knows about updates for itself. Stable: fields may be added,
/// never changed. `state` is a string so new states don't break old readers.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateStatus {
    /// Why this daemon can't update itself; empty when it can.
    pub unavailable: String,
    /// idle, checking, updating, restarting or failed.
    pub state: String,
    /// When upstream was last looked at, ms since the Unix epoch; 0 if never.
    pub checked_ms: u64,
    /// Commits upstream has that this build doesn't.
    pub behind: u32,
    /// Upstream's newest commit: short hash and subject.
    pub latest: String,
    /// The protocol version upstream has, and the oldest peer it works with;
    /// 0 when unknown. Tells whether updating breaks other machines.
    pub latest_version: u32,
    pub latest_oldest_compatible: u32,
    /// Why the last check or update failed.
    pub error: String,
}

/// How a machine's sensors are presented, kept by its daemon
/// (`~/.config/plusysmon/sensors.toml`) so every frontend sees the same.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SensorSettings {
    /// The sensor (a [`SensorReading::id`]) `cpu.temperature` comes from,
    /// instead of the one found automatically.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_temperature: Option<String>,
    /// The same for `cpu.power`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_power: Option<String>,
    /// Per sensor ID.
    pub sensors: BTreeMap<String, SensorOverride>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SensorOverride {
    /// Shown instead of the driver's label.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// The value shown is `raw * multiplier + offset`: voltage dividers on
    /// motherboards, sensors that report in odd units.
    pub multiplier: f64,
    pub offset: f64,
    /// Left out of the sensor list (it can still feed `cpu.temperature`).
    pub hidden: bool,
}

impl Default for SensorOverride {
    fn default() -> Self {
        Self {
            label: None,
            multiplier: 1.0,
            offset: 0.0,
            hidden: false,
        }
    }
}

impl SensorOverride {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

impl SensorSettings {
    /// Applies the settings to readings as the sensors collector made them.
    /// Returns the readings to show and what `cpu.temperature` and
    /// `cpu.power` should be, when the settings name their sources.
    pub fn apply(&self, raw: &[SensorReading]) -> (Vec<SensorReading>, Option<f32>, Option<f32>) {
        let mut shown = Vec::with_capacity(raw.len());
        let mut temperature = None;
        let mut power = None;
        for reading in raw {
            let mut reading = reading.clone();
            if let Some(o) = self.sensors.get(&reading.id) {
                reading.value = reading.value * o.multiplier + o.offset;
                reading.limit = reading.limit.map(|l| l * o.multiplier + o.offset);
                if let Some(label) = o.label.as_ref().filter(|l| !l.trim().is_empty()) {
                    reading.label = label.trim().to_owned();
                }
            }
            if self.cpu_temperature.as_deref() == Some(reading.id.as_str()) {
                temperature = Some(reading.value as f32);
            }
            if self.cpu_power.as_deref() == Some(reading.id.as_str()) {
                power = Some(reading.value as f32);
            }
            if !self.sensors.get(&reading.id).is_some_and(|o| o.hidden) {
                shown.push(reading);
            }
        }
        (shown, temperature, power)
    }

    /// Refuses what makes no sense, and drops overrides that change nothing.
    pub fn validated(mut self) -> Result<Self, String> {
        const MAX_SENSORS: usize = 1000;
        const MAX_LABEL: usize = 80;
        if self.sensors.len() > MAX_SENSORS {
            return Err("too many sensors".to_owned());
        }
        for (id, o) in &mut self.sensors {
            if !o.multiplier.is_finite() || !o.offset.is_finite() {
                return Err(format!("{id}: multiplier and offset must be numbers"));
            }
            if o.label.as_ref().is_some_and(|l| l.chars().count() > MAX_LABEL) {
                return Err(format!("{id}: the name is too long"));
            }
            if o.label.as_ref().is_some_and(|l| l.trim().is_empty()) {
                o.label = None;
            }
        }
        self.sensors.retain(|_, o| !o.is_default());
        self.cpu_temperature = self.cpu_temperature.filter(|id| !id.is_empty());
        self.cpu_power = self.cpu_power.filter(|id| !id.is_empty());
        Ok(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sample {
    /// Milliseconds since the Unix epoch.
    pub timestamp_ms: u64,
    pub cpu: CpuSample,
    pub mem: MemSample,
    pub gpus: Vec<GpuSample>,
    /// Every hardware sensor the kernel exposes (hwmon), except GPU ones,
    /// which `gpus` covers.
    pub sensors: Vec<SensorReading>,
    /// Fans the daemon controls (since 7).
    #[serde(default)]
    pub fans: Vec<fans::FanState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSample {
    /// Overall utilisation, 0.0..=100.0.
    pub total: f32,
    pub per_core: Vec<f32>,
    /// Package temperature, °C.
    pub temperature: Option<f32>,
    /// Average and highest current core clock, and the hardware maximum, MHz.
    pub frequency: Option<f32>,
    pub frequency_peak: Option<f32>,
    pub frequency_max: Option<f32>,
    /// Package power, watts (needs read access to the RAPL energy counter).
    pub power: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SensorKind {
    /// °C
    Temperature,
    /// Volts.
    Voltage,
    /// RPM.
    Fan,
    /// Watts.
    Power,
    /// Amperes.
    Current,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SensorReading {
    /// Stable across reboots: `<device>.<feature>`, e.g. `it87.2624.fan1`.
    pub id: String,
    /// What the chip is part of: "CPU", "Motherboard", "NVMe Lexar SSD ...".
    pub device: String,
    /// The kernel driver, e.g. "coretemp", "it8689".
    pub driver: String,
    /// The driver's label, or a generic one like "Voltage 3".
    pub label: String,
    pub kind: SensorKind,
    pub value: f64,
    /// The driver's critical (or maximum) limit, when it has one.
    pub limit: Option<f64>,
}

/// One graphics card. Optional fields are what not every driver reports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuSample {
    pub name: String,
    /// NVIDIA, AMD, ...
    pub vendor: String,
    /// 0.0..=100.0
    pub usage: f32,
    /// Bytes.
    pub memory_used: u64,
    pub memory_total: u64,
    /// °C
    pub temperature: Option<f32>,
    /// Watts.
    pub power: Option<f32>,
    pub power_limit: Option<f32>,
    /// MHz.
    pub core_clock: Option<u32>,
    pub memory_clock: Option<u32>,
    /// Percent of maximum fan speed.
    pub fan: Option<f32>,
    /// Video engine utilisation, 0.0..=100.0.
    pub encoder: Option<f32>,
    pub decoder: Option<f32>,
}

/// All values in bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemSample {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub swap_total: u64,
    pub swap_used: u64,
}

/// Changes to the process table since the previous update. Apply `removed`,
/// then `added`, then `stats`. A reused PID shows up as removed and added.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProcessUpdate {
    /// Drop everything known so far before applying this update.
    pub full: bool,
    pub removed: Vec<u32>,
    pub added: Vec<ProcessInfo>,
    /// Only processes whose numbers changed (all of them for added processes).
    pub stats: Vec<ProcessStats>,
    /// Applications referenced by processes for the first time.
    pub apps: Vec<AppInfo>,
}

/// An application processes are grouped under.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppInfo {
    /// `desktop:<desktop file id>` or `steam:<app id>`.
    pub id: String,
    pub name: String,
    pub kind: AppKind,
    /// Asset keys, see [`ClientMsg::Asset`].
    pub icon: Option<String>,
    /// Wide artwork (games).
    pub art: Option<String>,
    /// Transparent title logo (games).
    pub logo: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppKind {
    App,
    Game,
}

/// What doesn't change over a process' lifetime (or rarely matters if it does).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    /// Full command line, arguments separated by spaces; empty for kernel threads.
    pub command: String,
    pub uid: u32,
    pub user: String,
    /// Milliseconds since the Unix epoch.
    pub started_ms: u64,
    pub kernel_thread: bool,
    /// [`AppInfo::id`] of the application this process belongs to, if any.
    pub app: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessStats {
    pub pid: u32,
    /// Share of the whole machine, 0.0..=100.0, like the total CPU gauge.
    pub cpu: f32,
    /// Resident memory not shared with other processes, in bytes.
    pub memory: u64,
    /// Disk bytes per second; `None` when the daemon may not read it.
    pub read_rate: Option<u64>,
    pub write_rate: Option<u64>,
    pub threads: u32,
    /// The kernel's state letter: R, S, D, T, Z, I, ...
    pub state: char,
    pub nice: i32,
    /// Share of the GPU's engine time (summed over GPUs), for processes on a GPU.
    pub gpu: Option<f32>,
    /// Video memory in bytes, for processes holding any.
    pub gpu_memory: Option<u64>,
}

pub fn encode<T: Serialize>(msg: &T) -> Vec<u8> {
    // Named fields keep older clients working when fields are added.
    rmp_serde::to_vec_named(msg).expect("protocol messages are always serializable")
}

pub fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, rmp_serde::decode::Error> {
    rmp_serde::from_slice(bytes)
}

/// `~/.config/plusysmon`, shared by the daemon and the frontend.
pub fn config_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("plusysmon"))
}

/// Where the daemon keeps its access token, so a local frontend can pick it up
/// without the user copying it around.
pub fn token_path() -> Option<PathBuf> {
    Some(config_dir()?.join("token"))
}

/// The daemon's TLS certificate (DER). A local frontend trusts whoever holds
/// the key to this one, so it never has to ask about its own machine's daemon.
pub fn tls_cert_path() -> Option<PathBuf> {
    Some(config_dir()?.join("tls-cert.der"))
}

/// The daemon's sensor settings.
pub fn sensor_settings_path() -> Option<PathBuf> {
    Some(config_dir()?.join("sensors.toml"))
}

/// The daemon's fan control.
pub fn fan_settings_path() -> Option<PathBuf> {
    Some(config_dir()?.join("fans.toml"))
}

/// Whether two builds can talk: each must be at least as new as the other
/// one's oldest compatible version.
pub fn compatible(client: u32, client_oldest: u32, daemon: u32, daemon_oldest: u32) -> bool {
    client >= daemon_oldest && daemon >= client_oldest
}

/// The protocol version and oldest compatible version a copy of this file
/// declares, for telling whether an update upstream breaks compatibility.
pub fn parse_versions(source: &str) -> Option<(u32, u32)> {
    let constant = |name: &str| {
        source.lines().find_map(|line| {
            let rest = line.trim().strip_prefix(&format!("pub const {name}: u32 = "))?;
            rest.trim_end_matches(';').trim().parse().ok()
        })
    };
    Some((constant("PROTOCOL_VERSION")?, constant("OLDEST_COMPATIBLE")?))
}

/// SHA-256 of a DER certificate as `AB:CD:…`, the form `openssl x509 -fingerprint`
/// prints. This is what identifies a daemon: certificates are self-signed.
pub fn cert_fingerprint(der: &[u8]) -> String {
    let digest = Sha256::digest(der);
    let hex: Vec<String> = digest.iter().map(|byte| format!("{byte:02X}")).collect();
    hex.join(":")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprints_like_openssl() {
        // printf abc | sha256sum
        assert_eq!(
            cert_fingerprint(b"abc"),
            "BA:78:16:BF:8F:01:CF:EA:41:41:40:DE:5D:AE:22:23:\
             B0:03:61:A3:96:17:7A:9C:B4:10:FF:61:F2:00:15:AD"
        );
    }

    /// What every frontend since version 5 sends must stay readable.
    #[test]
    fn reads_a_version_5_hello() {
        #[derive(Serialize)]
        enum OldClientMsg {
            Hello { version: u32, token: String },
        }
        let old = encode(&OldClientMsg::Hello {
            version: 5,
            token: "t".into(),
        });
        let ClientMsg::Hello {
            version,
            token,
            oldest,
            probe,
        } = decode(&old).unwrap()
        else {
            panic!("wrong variant");
        };
        assert_eq!((version, token.as_str(), oldest, probe), (5, "t", 0, false));
    }

    /// Info may gain fields; readers that don't know them must not mind.
    #[test]
    fn info_tolerates_new_fields() {
        #[derive(Serialize)]
        struct FutureInfo {
            hostname: String,
            version: u32,
            something_new: Vec<u32>,
        }
        #[derive(Serialize)]
        enum FutureMsg {
            Info(FutureInfo),
        }
        let bytes = encode(&FutureMsg::Info(FutureInfo {
            hostname: "pi".into(),
            version: 9,
            something_new: vec![1],
        }));
        let ServerMsg::Info(info) = decode(&bytes).unwrap() else {
            panic!("wrong variant");
        };
        assert_eq!(info.hostname, "pi");
        assert_eq!(info.version, 9);
        assert_eq!(info.updates, UpdateStatus::default());
    }

    #[test]
    fn decides_compatibility() {
        assert!(compatible(6, 5, 6, 5));
        // A version 5 frontend (oldest unknown: 0) and a daemon that still serves it.
        assert!(compatible(5, 0, 6, 5));
        // A daemon that dropped version 5.
        assert!(!compatible(5, 0, 8, 8));
        // A frontend that needs a newer daemon.
        assert!(!compatible(9, 9, 6, 5));
    }

    #[test]
    fn reads_versions_from_source() {
        let source = include_str!("lib.rs");
        assert_eq!(
            parse_versions(source),
            Some((PROTOCOL_VERSION, OLDEST_COMPATIBLE))
        );
        assert_eq!(parse_versions("nothing here"), None);
    }

    fn reading(id: &str, kind: SensorKind, value: f64) -> SensorReading {
        SensorReading {
            id: id.into(),
            device: "dev".into(),
            driver: "drv".into(),
            label: "Label".into(),
            kind,
            value,
            limit: None,
        }
    }

    #[test]
    fn applies_sensor_settings() {
        let raw = vec![
            reading("thermal_zone0.temp1", SensorKind::Temperature, 48.0),
            reading("it87.in3", SensorKind::Voltage, 2.0),
            reading("it87.in4", SensorKind::Voltage, 1.0),
        ];
        let mut settings = SensorSettings {
            cpu_temperature: Some("thermal_zone0.temp1".into()),
            ..Default::default()
        };
        settings.sensors.insert(
            "thermal_zone0.temp1".into(),
            SensorOverride {
                hidden: true,
                ..Default::default()
            },
        );
        settings.sensors.insert(
            "it87.in3".into(),
            SensorOverride {
                label: Some(" +12V ".into()),
                multiplier: 6.0,
                offset: 0.1,
                hidden: false,
            },
        );
        let (shown, temperature, power) = settings.apply(&raw);
        assert_eq!(temperature, Some(48.0));
        assert_eq!(power, None);
        let ids: Vec<_> = shown.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["it87.in3", "it87.in4"]);
        assert_eq!(shown[0].label, "+12V");
        assert!((shown[0].value - 12.1).abs() < 1e-9);
        assert_eq!(shown[1], raw[2]);
    }

    #[test]
    fn validates_sensor_settings() {
        let mut settings = SensorSettings::default();
        settings
            .sensors
            .insert("a".into(), SensorOverride::default());
        settings.sensors.insert(
            "b".into(),
            SensorOverride {
                label: Some("  ".into()),
                ..Default::default()
            },
        );
        settings.cpu_power = Some(String::new());
        let clean = settings.validated().unwrap();
        assert!(clean.sensors.is_empty());
        assert_eq!(clean.cpu_power, None);

        let mut bad = SensorSettings::default();
        bad.sensors.insert(
            "a".into(),
            SensorOverride {
                multiplier: f64::NAN,
                ..Default::default()
            },
        );
        assert!(bad.validated().is_err());
    }

    #[test]
    fn sensor_settings_survive_toml() {
        let mut settings = SensorSettings {
            cpu_temperature: Some("thermal_zone0.temp1".into()),
            ..Default::default()
        };
        settings.sensors.insert(
            "it87.2624.in3".into(),
            SensorOverride {
                label: Some("+12V".into()),
                multiplier: 6.0,
                ..Default::default()
            },
        );
        let text = toml::to_string_pretty(&settings).unwrap();
        assert_eq!(toml::from_str::<SensorSettings>(&text).unwrap(), settings);
        // A hand-written file may leave out whatever isn't changed.
        let partial: SensorSettings =
            toml::from_str("[sensors.\"x.in0\"]\nmultiplier = 2.0\n").unwrap();
        assert_eq!(partial.sensors["x.in0"].offset, 0.0);
        assert!(!partial.sensors["x.in0"].hidden);
    }

    #[test]
    fn roundtrip() {
        let msg = ServerMsg::Sample(Sample {
            timestamp_ms: 42,
            cpu: CpuSample {
                total: 12.5,
                per_core: vec![10.0, 15.0],
                temperature: None,
                frequency: None,
                frequency_peak: None,
                frequency_max: None,
                power: None,
            },
            mem: MemSample {
                total: 16,
                used: 8,
                available: 8,
                swap_total: 4,
                swap_used: 1,
            },
            gpus: vec![],
            sensors: vec![],
            fans: vec![],
        });
        let ServerMsg::Sample(s) = decode::<ServerMsg>(&encode(&msg)).unwrap() else {
            panic!("wrong variant");
        };
        assert_eq!(s.timestamp_ms, 42);
        assert_eq!(s.cpu.per_core, vec![10.0, 15.0]);
        assert_eq!(s.mem.swap_used, 1);
    }
}
