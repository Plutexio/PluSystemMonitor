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

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Bumped on any incompatible change to the message types.
pub const PROTOCOL_VERSION: u32 = 5;

pub const DEFAULT_PORT: u16 = 7878;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMsg {
    Hello {
        version: u32,
        token: String,
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
        });
        let ServerMsg::Sample(s) = decode::<ServerMsg>(&encode(&msg)).unwrap() else {
            panic!("wrong variant");
        };
        assert_eq!(s.timestamp_ms, 42);
        assert_eq!(s.cpu.per_core, vec![10.0, 15.0]);
        assert_eq!(s.mem.swap_used, 1);
    }
}
