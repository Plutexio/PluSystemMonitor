//! Installed applications, from .desktop files (freedesktop Desktop Entry spec).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Re-scan at most this often, and only when a lookup misses.
const RESCAN_AFTER: Duration = Duration::from_secs(60);

/// Steam's own desktop IDs (native, Flatpak).
const STEAM_IDS: &[&str] = &["steam", "com.valvesoftware.steam"];

#[derive(Debug, Clone)]
pub struct DesktopEntry {
    /// Desktop file ID, e.g. `org.kde.dolphin`.
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
}

#[derive(Default)]
pub struct DesktopIndex {
    entries: Vec<DesktopEntry>,
    /// Lowercased desktop ID → entry.
    ids: HashMap<String, usize>,
    /// Lowercased StartupWMClass and executable name → entry. Kept apart from
    /// `ids` so they can never shadow a real ID: Steam's game shortcuts all run
    /// `steam steam://rungameid/...` and would otherwise claim "steam".
    hints: HashMap<String, usize>,
    /// Entries that are Steam game shortcuts (`steam://rungameid/...`).
    steam_shortcuts: HashSet<usize>,
    scanned_at: Option<Instant>,
}

impl DesktopIndex {
    pub fn lookup(&mut self, key: &str) -> Option<DesktopEntry> {
        let key = key.to_lowercase();
        let stale = self.scanned_at.is_none_or(|at| at.elapsed() > RESCAN_AFTER);
        let known = |index: &Self| index.ids.contains_key(&key) || index.hints.contains_key(&key);
        if !known(self) && stale {
            self.scan();
        }
        let mut index = *self.ids.get(&key).or_else(|| self.hints.get(&key))?;
        // Launched from a game's shortcut while Steam wasn't running, the Steam
        // client itself ends up in the game's unit and stays there after the
        // game quits. The game is recognized by its reaper anyway, so whatever
        // else runs in that unit is Steam.
        if self.steam_shortcuts.contains(&index) {
            index = STEAM_IDS.iter().find_map(|id| self.ids.get(*id).copied())?;
        }
        Some(self.entries[index].clone())
    }

    fn scan(&mut self) {
        self.entries.clear();
        self.ids.clear();
        self.hints.clear();
        self.steam_shortcuts.clear();
        self.scanned_at = Some(Instant::now());
        // Earlier directories take precedence, so the first entry for an ID wins.
        for dir in data_dirs() {
            let root = dir.join("applications");
            let mut files = Vec::new();
            collect_desktop_files(&root, &mut files, 0);
            for path in files {
                let Some(id) = desktop_id(&root, &path) else {
                    continue;
                };
                if self.ids.contains_key(&id.to_lowercase()) {
                    continue;
                }
                let Some(parsed) = fs::read_to_string(&path).ok().and_then(|t| parse(&t)) else {
                    continue;
                };
                let index = self.entries.len();
                self.entries.push(DesktopEntry {
                    id: id.clone(),
                    name: parsed.name,
                    icon: parsed.icon,
                });
                self.ids.insert(id.to_lowercase(), index);
                if parsed.steam_game {
                    self.steam_shortcuts.insert(index);
                }
                for key in parsed.wm_class.into_iter().chain(parsed.exec) {
                    self.hints.entry(key.to_lowercase()).or_insert(index);
                }
            }
        }
    }
}

/// `$XDG_DATA_HOME` and `$XDG_DATA_DIRS`, plus Flatpak's exports in case the
/// daemon runs without a full desktop environment.
pub fn data_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut dirs: Vec<PathBuf> = Vec::new();
    match std::env::var_os("XDG_DATA_HOME") {
        Some(dir) => dirs.push(dir.into()),
        None => dirs.extend(home.as_ref().map(|h| h.join(".local/share"))),
    }
    let system = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".to_owned());
    dirs.extend(system.split(':').map(PathBuf::from));
    dirs.extend(
        home.map(|h| h.join(".local/share/flatpak/exports/share"))
            .into_iter()
            .chain([PathBuf::from("/var/lib/flatpak/exports/share")]),
    );
    let mut seen = std::collections::HashSet::new();
    dirs.retain(|d| seen.insert(d.clone()));
    dirs
}

fn collect_desktop_files(dir: &Path, out: &mut Vec<PathBuf>, depth: usize) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && depth < 3 {
            collect_desktop_files(&path, out, depth + 1);
        } else if path.extension().is_some_and(|e| e == "desktop") {
            out.push(path);
        }
    }
}

/// The spec's ID: path below `applications/`, `/` replaced by `-`, no extension.
fn desktop_id(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?.to_str()?;
    Some(relative.strip_suffix(".desktop")?.replace('/', "-"))
}

#[derive(Debug, PartialEq)]
struct Parsed {
    name: String,
    icon: Option<String>,
    wm_class: Option<String>,
    exec: Option<String>,
    /// A Steam game shortcut: runs `steam steam://rungameid/<id>` or similar.
    steam_game: bool,
}

/// Launchers that say nothing about which application they start.
const GENERIC_EXECS: &[&str] = &["env", "flatpak", "sh", "bash", "python", "python3", "java"];

fn parse(text: &str) -> Option<Parsed> {
    let mut in_entry = false;
    let (mut name, mut icon, mut wm_class, mut exec) = (None, None, None, None);
    let mut application = false;
    let mut steam_game = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            // Only [Desktop Entry]; actions repeat Name= for their own labels.
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().to_owned();
        match key.trim() {
            "Name" => name = Some(value),
            "Icon" if !value.is_empty() => icon = Some(value),
            "StartupWMClass" if !value.is_empty() => wm_class = Some(value),
            "Exec" => {
                steam_game = value.contains("steam://rungameid/");
                exec = exec_name(&value);
            }
            "Type" => application = value == "Application",
            "Hidden" if value == "true" => return None,
            _ => {}
        }
    }
    if !application {
        return None;
    }
    Some(Parsed {
        name: name?,
        icon,
        wm_class,
        exec,
        steam_game,
    })
}

fn exec_name(exec: &str) -> Option<String> {
    let program = exec
        .split_whitespace()
        .find(|token| !token.contains('=') && *token != "env")?;
    let name = program.trim_matches('"').rsplit('/').next()?;
    (!GENERIC_EXECS.contains(&name)).then(|| name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_the_main_entry() {
        let text = "[Desktop Entry]\nType=Application\nName=Zen Browser\nIcon=zen-browser\n\
                    Exec=/usr/bin/zen-browser %u\nStartupWMClass=zen\n\n\
                    [Desktop Action new-window]\nName=New Window\nExec=zen --new-window\n";
        assert_eq!(
            parse(text),
            Some(Parsed {
                name: "Zen Browser".into(),
                icon: Some("zen-browser".into()),
                wm_class: Some("zen".into()),
                exec: Some("zen-browser".into()),
                steam_game: false,
            })
        );
    }

    #[test]
    fn recognizes_steam_game_shortcuts() {
        let text = "[Desktop Entry]\nName=Timberborn\nExec=steam steam://rungameid/1062090\n\
                    Icon=steam_icon_1062090\nType=Application\n";
        assert!(parse(text).is_some_and(|p| p.steam_game));
        let steam = "[Desktop Entry]\nName=Steam\nExec=/usr/bin/steam %U\nType=Application\n";
        assert!(parse(steam).is_some_and(|p| !p.steam_game));
    }

    #[test]
    fn skips_non_applications_and_generic_launchers() {
        assert_eq!(parse("[Desktop Entry]\nType=Link\nName=Docs\n"), None);
        assert_eq!(
            parse("[Desktop Entry]\nType=Application\nName=X\nHidden=true\n"),
            None
        );
        assert_eq!(exec_name("env FOO=1 flatpak run org.app"), None);
        assert_eq!(
            exec_name("env GDK_BACKEND=x11 /opt/app/bin/app --flag"),
            Some("app".into())
        );
    }

    #[test]
    fn builds_ids_from_subdirectories() {
        let root = Path::new("/usr/share/applications");
        assert_eq!(
            desktop_id(root, &root.join("kde/org.kde.konsole.desktop")).as_deref(),
            Some("kde-org.kde.konsole")
        );
    }
}
