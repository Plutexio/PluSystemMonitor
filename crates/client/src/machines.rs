//! The machines the app connected to, as kept in its preferences
//! (`frontend.toml`). The app owns the file; others only read the
//! `[connection]` table, ignoring the rest.

use serde::{Deserialize, Serialize};

use crate::client;
use crate::paths;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Connection {
    pub address: String,
    /// Empty means "read the local daemon's token file".
    pub token: String,
    /// Machines connected to successfully, latest first.
    pub recent: Vec<RecentConnection>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RecentConnection {
    pub address: String,
    pub token: String,
    /// What the machine called itself when last connected.
    pub hostname: String,
}

impl Default for Connection {
    fn default() -> Self {
        Self {
            address: "127.0.0.1".to_owned(),
            token: String::new(),
            recent: Vec::new(),
        }
    }
}

impl Connection {
    /// The `[connection]` table of the app's preferences; empty when there
    /// are none or they can't be read.
    pub fn load() -> Self {
        #[derive(Default, Deserialize)]
        #[serde(default)]
        struct Prefs {
            connection: Connection,
        }
        paths::prefs_file()
            .ok()
            .and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| toml::from_str::<Prefs>(&text).ok())
            .unwrap_or_default()
            .connection
    }

    /// Every other machine connected to, latest first, one entry each.
    pub fn remote(&self) -> Vec<RecentConnection> {
        let mut out: Vec<RecentConnection> = Vec::new();
        let current = RecentConnection {
            address: self.address.clone(),
            token: self.token.clone(),
            hostname: String::new(),
        };
        for entry in self.recent.iter().chain([&current]) {
            let authority = client::authority(entry.address.trim());
            if entry.address.trim().is_empty()
                || client::is_local(&authority)
                || out
                    .iter()
                    .any(|e| client::authority(&e.address) == authority)
            {
                continue;
            }
            out.push(entry.clone());
        }
        out
    }

    /// The token saved for the machine at `authority`.
    pub fn token_for(&self, authority: &str) -> Option<String> {
        self.remote()
            .into_iter()
            .find(|e| client::authority(&e.address) == authority)
            .map(|e| e.token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(address: &str, token: &str) -> RecentConnection {
        RecentConnection {
            address: address.to_owned(),
            token: token.to_owned(),
            hostname: String::new(),
        }
    }

    #[test]
    fn lists_other_machines_once() {
        let connection = Connection {
            address: "pi".to_owned(),
            token: "current".to_owned(),
            recent: vec![
                entry("127.0.0.1", ""),
                entry("box:7878", "a"),
                entry("box", "b"),
                entry("192.168.1.5", "c"),
            ],
        };
        let remote: Vec<String> = connection.remote().into_iter().map(|e| e.address).collect();
        assert_eq!(remote, ["box:7878", "192.168.1.5", "pi"]);
        assert_eq!(connection.token_for("box:7878").as_deref(), Some("a"));
        assert_eq!(connection.token_for("pi:7878").as_deref(), Some("current"));
        assert_eq!(connection.token_for("elsewhere:7878"), None);
    }
}
