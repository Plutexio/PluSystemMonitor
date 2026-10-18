//! Finding application icons in the user's icon theme (freedesktop Icon Theme spec).

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use super::desktop::data_dirs;

struct Theme {
    /// The same theme can live in several base directories (user and system).
    roots: Vec<PathBuf>,
    /// Application icon directories, best (scalable, then largest) first.
    app_dirs: Vec<String>,
}

#[derive(Default)]
pub struct IconLookup {
    /// Built on first use: the user's theme, what it inherits, then hicolor.
    themes: Option<Vec<Theme>>,
    found: HashMap<String, Option<PathBuf>>,
}

impl IconLookup {
    pub fn find(&mut self, name: &str) -> Option<PathBuf> {
        if let Some(found) = self.found.get(name) {
            return found.clone();
        }
        let found = self.search(name);
        self.found.insert(name.to_owned(), found.clone());
        found
    }

    fn search(&mut self, name: &str) -> Option<PathBuf> {
        if name.starts_with('/') {
            return Path::new(name).is_file().then(|| PathBuf::from(name));
        }
        let themes = self.themes.get_or_insert_with(load_themes);
        for theme in themes.iter() {
            for dir in &theme.app_dirs {
                for root in &theme.roots {
                    for ext in ["svg", "png"] {
                        let path = root.join(dir).join(format!("{name}.{ext}"));
                        if path.is_file() {
                            return Some(path);
                        }
                    }
                }
            }
        }
        ["svg", "png"]
            .iter()
            .map(|ext| PathBuf::from(format!("/usr/share/pixmaps/{name}.{ext}")))
            .find(|path| path.is_file())
    }
}

fn icon_bases() -> Vec<PathBuf> {
    let mut bases: Vec<PathBuf> = std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join(".icons"))
        .into_iter()
        .collect();
    bases.extend(data_dirs().into_iter().map(|d| d.join("icons")));
    bases
}

fn user_theme() -> Option<String> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    let kde = fs::read_to_string(config.join("kdeglobals"))
        .ok()
        .and_then(|text| ini_value(&text, "Icons", "Theme"));
    kde.or_else(|| {
        let gtk = fs::read_to_string(config.join("gtk-3.0/settings.ini")).ok()?;
        ini_value(&gtk, "Settings", "gtk-icon-theme-name")
    })
}

fn ini_value(text: &str, section: &str, key: &str) -> Option<String> {
    let header = format!("[{section}]");
    let mut inside = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            inside = line == header;
        } else if inside
            && let Some((k, v)) = line.split_once('=')
            && k.trim() == key
        {
            return Some(v.trim().to_owned());
        }
    }
    None
}

fn load_themes() -> Vec<Theme> {
    let bases = icon_bases();
    let index_of = |name: &str| -> (Vec<PathBuf>, Option<String>) {
        let roots: Vec<PathBuf> = bases
            .iter()
            .map(|b| b.join(name))
            .filter(|p| p.is_dir())
            .collect();
        let index = roots
            .iter()
            .find_map(|r| fs::read_to_string(r.join("index.theme")).ok());
        (roots, index)
    };

    // The user's theme and everything it inherits, breadth first; hicolor is
    // every theme's fallback, so it goes last regardless of where it's named.
    let mut names: Vec<String> = Vec::new();
    let mut queue: VecDeque<String> = user_theme().into_iter().collect();
    while let Some(name) = queue.pop_front() {
        if name == "hicolor" || names.contains(&name) {
            continue;
        }
        if let (_, Some(index)) = index_of(&name)
            && let Some(parents) = ini_value(&index, "Icon Theme", "Inherits")
        {
            queue.extend(parents.split(',').map(|p| p.trim().to_owned()));
        }
        names.push(name);
    }
    names.push("hicolor".to_owned());

    names
        .iter()
        .filter_map(|name| {
            let (roots, index) = index_of(name);
            let mut app_dirs: Vec<String> = ini_value(&index?, "Icon Theme", "Directories")
                .unwrap_or_default()
                .split(',')
                .map(|d| d.trim().to_owned())
                .filter(|d| d.split('/').any(|part| part == "apps") && !d.contains("symbolic"))
                .collect();
            app_dirs.sort_by_key(|d| std::cmp::Reverse(dir_size(d)));
            Some(Theme { roots, app_dirs })
        })
        .collect()
}

/// "scalable/apps" beats "256x256/apps" beats "apps/48"; a bare "apps" is
/// usually a scalable set too.
fn dir_size(dir: &str) -> u32 {
    if dir.contains("scalable") {
        return u32::MAX;
    }
    dir.split('/')
        .filter_map(|part| part.split(['x', '@']).next()?.parse().ok())
        .next()
        .unwrap_or(512)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_ini_sections() {
        let text = "[General]\nTheme=wrong\n[Icons]\nTheme=Papirus-Dark\n[Other]\nTheme=x\n";
        assert_eq!(
            ini_value(text, "Icons", "Theme").as_deref(),
            Some("Papirus-Dark")
        );
        assert_eq!(ini_value(text, "Missing", "Theme"), None);
    }

    #[test]
    fn ranks_icon_dirs() {
        let mut dirs = vec!["16x16/apps", "apps/48", "scalable/apps", "256x256@2/apps"];
        dirs.sort_by_key(|d| std::cmp::Reverse(dir_size(d)));
        assert_eq!(
            dirs,
            ["scalable/apps", "256x256@2/apps", "apps/48", "16x16/apps"]
        );
    }
}
