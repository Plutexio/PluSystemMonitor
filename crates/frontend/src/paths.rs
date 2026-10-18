use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

fn config_dir() -> Result<PathBuf> {
    plusysmon_protocol::config_dir().context("cannot determine the config directory")
}

pub fn themes_dir() -> Result<PathBuf> {
    Ok(config_dir()?.join("themes"))
}

pub fn layout_file() -> Result<PathBuf> {
    Ok(config_dir()?.join("layout.toml"))
}

pub fn prefs_file() -> Result<PathBuf> {
    Ok(config_dir()?.join("frontend.toml"))
}

/// Writes via a temporary file and rename, so a crash never leaves a half-written
/// config behind. Owner-only, as the preferences hold daemon tokens.
pub fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    }
    let tmp = path.with_extension("tmp");
    OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&tmp)
        .and_then(|mut f| f.write_all(contents.as_bytes()))
        .with_context(|| format!("writing {}", tmp.display()))?;
    fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))
}
