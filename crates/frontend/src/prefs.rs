//! Frontend preferences (`frontend.toml`). Unknown keys are ignored and missing
//! ones defaulted, so the file stays compatible across versions.

use serde::{Deserialize, Serialize};

use crate::theme::DEFAULT_SOURCE;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    /// A theme ID: "system" for the desktop's Matugen scheme, a built-in, or a
    /// file in the themes folder.
    pub theme: String,
    /// auto (as the desktop prefers), light or dark.
    pub theme_mode: String,
    /// Stills what moves on its own, over and over.
    pub reduce_motion: bool,
    /// No animations at all, not even fades.
    pub performance_mode: bool,
    /// No decorative shapes; the ones that mean something stay.
    pub no_shapes: bool,
    pub sidebar_width: f64,
    /// The page shown last (0 overview, 1 apps, 2 processes), restored on start.
    pub page: u8,
    pub connection: Connection,
    pub apps: AppView,
    pub processes: ProcessView,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppView {
    /// name, cpu or memory.
    pub sort: String,
}

impl Default for AppView {
    fn default() -> Self {
        Self {
            sort: "name".to_owned(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Connection {
    pub address: String,
    /// Empty means "read the local daemon's token file".
    pub token: String,
    /// Machines connected to successfully, latest first.
    pub recent: Vec<RecentConnection>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RecentConnection {
    pub address: String,
    pub token: String,
    /// What the machine called itself when last connected.
    pub hostname: String,
}

/// How the process list was last left.
#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ProcessView {
    pub columns: Vec<String>,
    pub sort: String,
    pub descending: bool,
    pub only_mine: bool,
    pub show_kernel_threads: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            theme: DEFAULT_SOURCE.to_owned(),
            theme_mode: "auto".to_owned(),
            reduce_motion: false,
            performance_mode: false,
            no_shapes: false,
            sidebar_width: 232.0,
            page: 0,
            connection: Connection::default(),
            apps: AppView::default(),
            processes: ProcessView::default(),
        }
    }
}

impl Default for ProcessView {
    fn default() -> Self {
        Self {
            columns: ["pid", "user", "cpu", "memory", "gpu", "read", "write"]
                .map(str::to_owned)
                .to_vec(),
            sort: "cpu".to_owned(),
            descending: true,
            only_mine: false,
            show_kernel_threads: false,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recent_connections_survive_the_file() {
        let mut prefs = Prefs::default();
        prefs.connection.recent.push(RecentConnection {
            address: "192.168.1.10".to_owned(),
            token: "secret".to_owned(),
            hostname: "box".to_owned(),
        });
        let text = toml::to_string_pretty(&prefs).unwrap();
        let read: Prefs = toml::from_str(&text).unwrap();
        assert_eq!(read.connection.recent.len(), 1);
        assert_eq!(read.connection.recent[0].address, "192.168.1.10");
        assert_eq!(read.connection.recent[0].token, "secret");
        assert_eq!(read.connection.recent[0].hostname, "box");
    }

    #[test]
    fn older_files_have_no_recent_connections() {
        let read: Prefs = toml::from_str("[connection]\naddress = \"box\"\n").unwrap();
        assert_eq!(read.connection.address, "box");
        assert!(read.connection.recent.is_empty());
    }
}
