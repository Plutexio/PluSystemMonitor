//! Icons and artwork fetched from a daemon, cached on disk per host so they
//! work for remote machines and survive restarts.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};

/// What Qt's image plugins read; the daemon's file extension must be one of these.
const FORMATS: &[&str] = &["png", "jpg", "jpeg", "svg", "webp", "gif"];

#[derive(Default)]
pub struct AssetCache {
    dir: Option<PathBuf>,
    /// Asset key → file:// URL.
    urls: HashMap<String, String>,
    requested: HashSet<String>,
}

impl AssetCache {
    pub fn for_host(host: &str) -> Self {
        let dir = std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
            .map(|cache| cache.join("plusysmon/assets").join(hex(host)));
        let mut urls = HashMap::new();
        for entry in dir
            .iter()
            .flat_map(|d| fs::read_dir(d).into_iter().flatten().flatten())
        {
            let path = entry.path();
            if let Some(key) = path.file_stem().and_then(|s| s.to_str()).and_then(unhex) {
                urls.insert(key, url(&path));
            }
        }
        Self {
            dir,
            urls,
            requested: HashSet::new(),
        }
    }

    pub fn url(&self, key: &str) -> Option<&str> {
        self.urls.get(key).map(String::as_str)
    }

    /// True the first time an uncached key is seen: the caller should fetch it.
    pub fn should_fetch(&mut self, key: &str) -> bool {
        !self.urls.contains_key(key) && self.requested.insert(key.to_owned())
    }

    pub fn store(&mut self, key: &str, format: &str, data: &[u8]) -> Result<()> {
        if data.is_empty() {
            // Unavailable on the daemon; asking again won't help.
            return Ok(());
        }
        let format = format.to_lowercase();
        if !FORMATS.contains(&format.as_str()) {
            bail!("unsupported image format {format:?} for {key}");
        }
        let dir = self.dir.as_ref().context("no cache directory")?;
        fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        let path = dir.join(format!("{}.{format}", hex(key)));
        fs::write(&path, data).with_context(|| format!("writing {}", path.display()))?;
        self.urls.insert(key.to_owned(), url(&path));
        Ok(())
    }
}

fn url(path: &std::path::Path) -> String {
    format!("file://{}", path.display())
}

/// Keys contain `:` and arbitrary IDs; hex keeps file names safe and reversible.
fn hex(text: &str) -> String {
    text.bytes().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Option<String> {
    let bytes: Option<Vec<u8>> = (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect();
    String::from_utf8(bytes?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_roundtrip() {
        assert_eq!(
            unhex(&hex("steam-art:1062090")).as_deref(),
            Some("steam-art:1062090")
        );
        assert_eq!(unhex("zz"), None);
    }

    #[test]
    fn stores_and_finds_again() {
        let dir = std::env::temp_dir().join(format!("plusysmon-assets-{}", std::process::id()));
        let mut cache = AssetCache {
            dir: Some(dir.clone()),
            ..Default::default()
        };
        assert!(cache.should_fetch("icon:x"));
        assert!(!cache.should_fetch("icon:x"));
        cache.store("icon:x", "PNG", b"data").unwrap();
        assert!(cache.url("icon:x").unwrap().ends_with(".png"));
        assert!(cache.store("icon:y", "exe", b"data").is_err());

        // A fresh cache for the same directory picks the file up.
        let entries: Vec<_> = fs::read_dir(&dir).unwrap().flatten().collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            unhex(entries[0].path().file_stem().unwrap().to_str().unwrap()).as_deref(),
            Some("icon:x")
        );
    }
}
