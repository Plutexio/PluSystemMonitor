//! Wire protocol shared by the daemon and its frontends.
//!
//! Every WebSocket binary frame carries one MessagePack-encoded message.
//! The client opens with [`ClientMsg::Hello`]; the daemon answers with
//! [`ServerMsg::Welcome`] (including buffered history) and then streams
//! [`ServerMsg::Sample`] at its sampling interval.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Bumped on any incompatible change to the message types.
pub const PROTOCOL_VERSION: u32 = 1;

pub const DEFAULT_PORT: u16 = 7878;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMsg {
    Hello { version: u32, token: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMsg {
    Welcome {
        hostname: String,
        interval_ms: u32,
        history: Vec<Sample>,
    },
    Sample(Sample),
    /// Sent right before the daemon closes the connection.
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sample {
    /// Milliseconds since the Unix epoch.
    pub timestamp_ms: u64,
    pub cpu: CpuSample,
    pub mem: MemSample,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuSample {
    /// Overall utilisation, 0.0..=100.0.
    pub total: f32,
    pub per_core: Vec<f32>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let msg = ServerMsg::Sample(Sample {
            timestamp_ms: 42,
            cpu: CpuSample {
                total: 12.5,
                per_core: vec![10.0, 15.0],
            },
            mem: MemSample {
                total: 16,
                used: 8,
                available: 8,
                swap_total: 4,
                swap_used: 1,
            },
        });
        let ServerMsg::Sample(s) = decode::<ServerMsg>(&encode(&msg)).unwrap() else {
            panic!("wrong variant");
        };
        assert_eq!(s.timestamp_ms, 42);
        assert_eq!(s.cpu.per_core, vec![10.0, 15.0]);
        assert_eq!(s.mem.swap_used, 1);
    }
}
