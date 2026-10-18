//! Frontend preferences (`frontend.toml`). Unknown keys are ignored and missing
//! ones defaulted, so the file stays compatible across versions.

use serde::{Deserialize, Serialize};

use crate::theme::DEFAULT_THEME;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub theme: String,
    pub connection: Connection,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Connection {
    pub address: String,
    /// Empty means "read the local daemon's token file".
    pub token: String,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            theme: DEFAULT_THEME.to_owned(),
            connection: Connection::default(),
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
