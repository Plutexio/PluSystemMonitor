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

/// Where Google Sans Flex and Material Symbols may be installed beside the app
/// (`install.sh` puts them in the prefix's share folder), when the system has
/// none of its own. Swiftify ships the same two faces.
pub fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(prefix) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent()?.parent().map(Path::to_path_buf))
    {
        dirs.push(prefix.join("share/plusysmon/fonts"));
    }
    let data_home = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")));
    if let Some(data_home) = data_home {
        dirs.push(data_home.join("plusysmon/fonts"));
    }
    for prefix in ["/usr/local/share", "/usr/share"] {
        dirs.push(Path::new(prefix).join("plusysmon/fonts"));
        dirs.push(Path::new(prefix).join("swiftify/fonts"));
    }
    dirs
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
