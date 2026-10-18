//! Which application a process belongs to.
//!
//! Desktop sessions start every application in its own systemd unit
//! (`app-<id>@<random>.service` or `app-<launcher>-<id>-<pid>.scope`), so the
//! cgroup names the application. Steam games are the exception: they run inside
//! Steam's unit, but always below a `reaper SteamLaunch AppId=<id>` process.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use plusysmon_protocol::{AppInfo, AppKind};

use super::desktop::DesktopIndex;
use super::icons::IconLookup;
use super::steam::Steam;

/// Asset key → file. Only files registered here are ever served to clients.
pub type Assets = Arc<Mutex<HashMap<String, PathBuf>>>;

pub struct Apps {
    desktop: DesktopIndex,
    icons: IconLookup,
    steam: Steam,
    /// Unit name → app ID, including the misses (background services).
    units: HashMap<String, Option<String>>,
    infos: HashMap<String, AppInfo>,
    /// App IDs whose `AppInfo` clients have already been sent.
    sent: HashSet<String>,
    assets: Assets,
}

impl Apps {
    pub fn new(assets: Assets) -> Self {
        Self {
            desktop: DesktopIndex::default(),
            icons: IconLookup::default(),
            steam: Steam::default(),
            units: HashMap::new(),
            infos: HashMap::new(),
            sent: HashSet::new(),
            assets,
        }
    }

    /// Clients will start from scratch, so every app must be described again.
    pub fn reset(&mut self) {
        self.sent.clear();
    }

    /// The app of a process in systemd unit `unit` (see [`app_unit`]).
    pub fn for_unit(&mut self, unit: &str) -> Option<String> {
        if let Some(id) = self.units.get(unit) {
            return id.clone();
        }
        // Units that don't lead to a desktop entry are D-Bus activated helpers
        // and the like: background work, not something the user launched.
        let id = unit_candidates(unit).find_map(|candidate| {
            let entry = self.desktop.lookup(&candidate)?;
            let id = format!("desktop:{}", entry.id);
            if !self.infos.contains_key(&id) {
                let icon = entry
                    .icon
                    .and_then(|name| self.icons.find(&name))
                    .map(|path| self.register(format!("icon:{}", entry.id), path));
                self.infos.insert(
                    id.clone(),
                    AppInfo {
                        id: id.clone(),
                        name: entry.name,
                        kind: AppKind::App,
                        icon,
                        art: None,
                        logo: None,
                    },
                );
            }
            Some(id)
        });
        self.units.insert(unit.to_owned(), id.clone());
        id
    }

    pub fn for_game(&mut self, app_id: u32) -> String {
        let id = format!("steam:{app_id}");
        if !self.infos.contains_key(&id) {
            let name = self
                .steam
                .game_name(app_id)
                .unwrap_or_else(|| format!("Steam game {app_id}"));
            let art = self.steam.art(app_id);
            let register = |kind: &str, path: Option<PathBuf>| {
                path.map(|p| self.register(format!("steam-{kind}:{app_id}"), p))
            };
            let info = AppInfo {
                id: id.clone(),
                name,
                kind: AppKind::Game,
                icon: register("icon", art.icon),
                art: register("art", art.header),
                logo: register("logo", art.logo),
            };
            self.infos.insert(id.clone(), info);
        }
        id
    }

    /// Descriptions of the given apps that haven't been sent since the last reset.
    pub fn take_new<'a>(&mut self, ids: impl Iterator<Item = &'a str>) -> Vec<AppInfo> {
        let mut new = Vec::new();
        for id in ids {
            if !self.sent.contains(id)
                && let Some(info) = self.infos.get(id)
            {
                self.sent.insert(id.to_owned());
                new.push(info.clone());
            }
        }
        new
    }

    fn register(&self, key: String, path: PathBuf) -> String {
        self.assets.lock().unwrap().insert(key.clone(), path);
        key
    }
}

/// The application unit from `/proc/<pid>/cgroup`, if the process runs in one.
pub fn app_unit(cgroup: &str) -> Option<String> {
    let path = cgroup.lines().find_map(|line| line.strip_prefix("0::"))?;
    let unit = path.rsplit('/').next()?;
    (path.contains("/app.slice/") && unit.starts_with("app-")).then(|| unit.to_owned())
}

/// Possible desktop file IDs for a unit, most specific first:
/// `app-gnome-org.gnome.Nautilus-1234.scope` → `gnome-org.gnome.Nautilus`, then
/// `org.gnome.Nautilus` (without the launcher prefix).
fn unit_candidates(unit: &str) -> impl Iterator<Item = String> {
    let name = unescape(unit);
    let name = name
        .strip_suffix(".service")
        .map(|n| n.split('@').next().unwrap_or(n).to_owned())
        .or_else(|| {
            // Scopes end in -<pid> (or another random number).
            let scope = name.strip_suffix(".scope")?;
            Some(match scope.rsplit_once('-') {
                Some((head, tail)) if tail.chars().all(|c| c.is_ascii_digit()) => head.to_owned(),
                _ => scope.to_owned(),
            })
        })
        .unwrap_or(name);
    let name = name.strip_prefix("app-").unwrap_or(&name).to_owned();
    let mut rest = Some(name);
    std::iter::from_fn(move || {
        let current = rest.take()?;
        rest = current.split_once('-').map(|(_, tail)| tail.to_owned());
        Some(current)
    })
}

/// systemd escapes unit names as `\xNN` (e.g. `-` inside an ID becomes `\x2d`).
fn unescape(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut rest = name;
    while let Some(pos) = rest.find("\\x") {
        out.push_str(&rest[..pos]);
        let code = rest.get(pos + 2..pos + 4);
        match code.and_then(|c| u8::from_str_radix(c, 16).ok()) {
            Some(byte) => {
                out.push(byte as char);
                rest = &rest[pos + 4..];
            }
            None => {
                out.push_str("\\x");
                rest = &rest[pos + 2..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidates(unit: &str) -> Vec<String> {
        unit_candidates(unit).collect()
    }

    #[test]
    fn finds_app_units() {
        let kitty =
            "0::/user.slice/user-1000.slice/user@1000.service/app.slice/app-kitty@c5d9.service\n";
        assert_eq!(app_unit(kitty).as_deref(), Some("app-kitty@c5d9.service"));
        let shell = "0::/user.slice/user-1000.slice/user@1000.service/session.slice/plasma-plasmashell.service";
        assert_eq!(app_unit(shell), None);
        assert_eq!(app_unit("0::/system.slice/sshd.service"), None);
    }

    #[test]
    fn derives_desktop_ids() {
        assert_eq!(candidates("app-kitty@c5d95240.service"), ["kitty"]);
        assert_eq!(candidates("app-discord-107511.scope"), ["discord"]);
        assert_eq!(
            candidates("app-gnome-org.gnome.Nautilus-4321.scope"),
            ["gnome-org.gnome.Nautilus", "org.gnome.Nautilus"]
        );
        assert_eq!(
            candidates("app-flatpak-com.valvesoftware.Steam-99.scope"),
            ["flatpak-com.valvesoftware.Steam", "com.valvesoftware.Steam"]
        );
        assert_eq!(candidates(r"app-my\x2dapp@1.service"), ["my-app", "app"]);
    }

    #[test]
    fn unescapes_unit_names() {
        assert_eq!(unescape(r"dbus\x2d:1.2\x2dorg"), "dbus-:1.2-org");
        assert_eq!(unescape(r"broken\xZZ"), r"broken\xZZ");
    }
}
