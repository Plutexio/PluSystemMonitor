//! Steam: telling games apart from the client, and finding their names and artwork.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Steam starts every game through `<steam>/ubuntu12_32/reaper SteamLaunch AppId=<id> -- ...`,
/// and the reaper stays the ancestor (and subreaper) of everything the game
/// spawns. The match is anchored on the program itself so that unrelated
/// commands merely mentioning it (a `grep SteamLaunch`, a shell script) don't count.
pub fn launch_app_id(command: &str) -> Option<u32> {
    const MARKER: &str = "reaper SteamLaunch ";
    let at = command.find(MARKER)?;
    // The program path may contain spaces, so check what precedes "reaper"
    // rather than splitting: it must be the start of argv[0] or a directory.
    if at > 0 && (!command[..at].ends_with('/') || command[..at].contains(" -")) {
        return None;
    }
    command[at + MARKER.len()..]
        .split_whitespace()
        .take_while(|arg| *arg != "--")
        .find_map(|arg| arg.strip_prefix("AppId="))
        .and_then(|id| id.parse().ok())
        .filter(|&id| id != 0)
}

#[derive(Debug, Default, PartialEq)]
pub struct Art {
    pub icon: Option<PathBuf>,
    pub header: Option<PathBuf>,
    pub logo: Option<PathBuf>,
}

#[derive(Default)]
pub struct Steam {
    /// `None` until first used; `Some(None)` when Steam isn't installed.
    root: Option<Option<PathBuf>>,
    names: HashMap<u32, Option<String>>,
}

impl Steam {
    fn root(&mut self) -> Option<&Path> {
        self.root
            .get_or_insert_with(|| {
                let home = PathBuf::from(std::env::var_os("HOME")?);
                [
                    ".local/share/Steam",
                    ".steam/steam",
                    ".var/app/com.valvesoftware.Steam/.local/share/Steam",
                ]
                .iter()
                .map(|p| home.join(p))
                .find(|p| p.join("steamapps").is_dir())
            })
            .as_deref()
    }

    pub fn game_name(&mut self, app_id: u32) -> Option<String> {
        if let Some(name) = self.names.get(&app_id) {
            return name.clone();
        }
        let name = self.root().and_then(|root| {
            let libraries = fs::read_to_string(root.join("steamapps/libraryfolders.vdf"))
                .map(|vdf| library_paths(&vdf))
                .unwrap_or_default();
            std::iter::once(root.to_path_buf())
                .chain(libraries)
                .find_map(|lib| {
                    let manifest = lib.join(format!("steamapps/appmanifest_{app_id}.acf"));
                    manifest_name(&fs::read_to_string(manifest).ok()?)
                })
        });
        self.names.insert(app_id, name.clone());
        name
    }

    pub fn art(&mut self, app_id: u32) -> Art {
        let Some(cache) = self.root().map(|r| r.join("appcache/librarycache")) else {
            return Art::default();
        };
        // Current clients: librarycache/<id>/<hash>/library_header.jpg etc., with
        // the small icon directly in librarycache/<id>/.
        let dir = cache.join(app_id.to_string());
        let mut art = Art::default();
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    for file in fs::read_dir(&path).into_iter().flatten().flatten() {
                        match file.file_name().to_str() {
                            Some("library_header.jpg") => art.header = Some(file.path()),
                            Some("logo.png") => art.logo = Some(file.path()),
                            _ => {}
                        }
                    }
                } else if path.extension().is_some_and(|e| e == "jpg") {
                    art.icon = Some(path);
                }
            }
        }
        // Older clients kept everything flat.
        let flat =
            |suffix: &str| Some(cache.join(format!("{app_id}_{suffix}"))).filter(|p| p.is_file());
        art.icon = art.icon.or_else(|| flat("icon.jpg"));
        art.header = art.header.or_else(|| flat("header.jpg"));
        art.logo = art.logo.or_else(|| flat("logo.png"));
        art
    }
}

/// `"path"  "/mnt/games/SteamLibrary"` lines of libraryfolders.vdf.
fn library_paths(vdf: &str) -> Vec<PathBuf> {
    vdf.lines()
        .filter_map(|line| quoted_value(line, "path"))
        .map(PathBuf::from)
        .collect()
}

fn manifest_name(acf: &str) -> Option<String> {
    acf.lines().find_map(|line| quoted_value(line, "name"))
}

/// `"key"  "value"` with VDF's backslash escapes.
fn quoted_value(line: &str, key: &str) -> Option<String> {
    let rest = line.trim().strip_prefix(&format!("\"{key}\""))?;
    let value = rest.trim().strip_prefix('"')?.strip_suffix('"')?;
    Some(value.replace("\\\\", "\\").replace("\\\"", "\""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_launch_app_id() {
        let cmd = "/home/u/.local/share/Steam/ubuntu12_32/reaper SteamLaunch AppId=1062090 -- \
                   /home/u/.local/share/Steam/steamapps/common/SteamLinuxRuntime_sniper/_v2-entry-point";
        assert_eq!(launch_app_id(cmd), Some(1062090));
        assert_eq!(
            launch_app_id("/opt/My Games/Steam/reaper SteamLaunch AppId=7 --"),
            Some(7)
        );
        assert_eq!(launch_app_id("/usr/bin/steam -silent"), None);
        assert_eq!(launch_app_id("reaper SteamLaunch AppId=0 --"), None);
        // Mentions of the marker elsewhere in a command are not a reaper.
        assert_eq!(launch_app_id("grep reaper SteamLaunch AppId=5"), None);
        assert_eq!(
            launch_app_id("zsh -c sh -c sleep fake-reaper SteamLaunch AppId=5"),
            None
        );
        assert_eq!(launch_app_id("zsh -c /x/reaper SteamLaunch AppId=5"), None);
        assert_eq!(launch_app_id("/x/reaper SteamLaunch -- game AppId=5"), None);
    }

    #[test]
    fn parses_vdf_values() {
        let vdf = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"/home/u/.local/share/Steam\"\n\
                   \t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"/mnt/diskGames/SteamLibrary\"\n\t}\n}\n";
        assert_eq!(
            library_paths(vdf),
            [
                PathBuf::from("/home/u/.local/share/Steam"),
                PathBuf::from("/mnt/diskGames/SteamLibrary")
            ]
        );
        let acf = "\"AppState\"\n{\n\t\"appid\"\t\t\"1062090\"\n\t\"name\"\t\t\"Timberborn\"\n}\n";
        assert_eq!(manifest_name(acf).as_deref(), Some("Timberborn"));
        assert_eq!(
            quoted_value(r#"  "name"  "Say \"Hi\" C:\\Games"  "#, "name").as_deref(),
            Some(r#"Say "Hi" C:\Games"#)
        );
    }
}
