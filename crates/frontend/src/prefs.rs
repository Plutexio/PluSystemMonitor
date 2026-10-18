//! Frontend preferences (`frontend.toml`). Unknown keys are ignored and missing
//! ones defaulted, so the file stays compatible across versions.

use serde::{Deserialize, Serialize};

use crate::theme::DEFAULT_THEME;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub theme: String,
    /// The page shown last (0 overview, 1 processes), restored on start.
    pub page: u8,
    pub connection: Connection,
    pub processes: ProcessView,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Connection {
    pub address: String,
    /// Empty means "read the local daemon's token file".
    pub token: String,
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
            theme: DEFAULT_THEME.to_owned(),
            page: 0,
            connection: Connection::default(),
            processes: ProcessView::default(),
        }
    }
}

impl Default for ProcessView {
    fn default() -> Self {
        Self {
            columns: ["pid", "user", "cpu", "memory", "read", "write"]
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
        }
    }
}
