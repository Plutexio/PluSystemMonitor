//! Theme files: TOML, resolved through an `extends` chain down to a built-in base.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value as Json, json};
use toml::{Table, Value};

pub const DEFAULT_THEME: &str = "aurora";

const BUILTIN: &[(&str, &str)] = &[
    ("aurora", include_str!("../themes/aurora.toml")),
    ("frost", include_str!("../themes/frost.toml")),
    ("daylight", include_str!("../themes/daylight.toml")),
];

/// A fully resolved theme. Every field is required here; partial theme files get
/// the rest from the themes they extend.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    pub name: String,
    pub colors: Colors,
    pub accents: Accents,
    pub shape: Shape,
    pub effects: Effects,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Colors {
    pub background: Color,
    /// Buttons, focus rings, selection: the colour of the UI itself.
    pub primary: Color,
    pub card: Color,
    pub card_border: Color,
    pub card_highlight: Color,
    pub track: Color,
    pub grid: Color,
    pub text: Color,
    pub text_dim: Color,
    pub good: Color,
    pub warn: Color,
    pub bad: Color,
    pub vignette: Color,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Accents {
    pub cpu: Color,
    pub memory: Color,
    pub swap: Color,
    pub gpu: Color,
    pub network: Color,
    pub disk: Color,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shape {
    pub radius: f64,
    pub spacing: f64,
    pub padding: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effects {
    pub animated_background: bool,
    pub background_glow: f64,
    pub glow: f64,
}

/// Parsed from CSS-style `#rgb` / `#rrggbb` / `#rrggbbaa`, stored as Qt's `#aarrggbb`.
#[derive(Debug, Serialize)]
#[serde(transparent)]
pub struct Color(String);

impl<'de> Deserialize<'de> for Color {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        parse_color(&raw).map(Color).ok_or_else(|| {
            D::Error::custom(format!(
                "invalid colour {raw:?}, expected #rgb, #rrggbb or #rrggbbaa"
            ))
        })
    }
}

fn parse_color(raw: &str) -> Option<String> {
    let hex = raw.strip_prefix('#')?;
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let rgba = match hex.len() {
        3 => hex
            .chars()
            .flat_map(|c| [c, c])
            .chain("ff".chars())
            .collect(),
        6 => format!("{hex}ff"),
        8 => hex.to_owned(),
        _ => return None,
    };
    Some(format!("#{}{}", &rgba[6..], &rgba[..6]).to_ascii_lowercase())
}

pub struct Resolved {
    pub theme: Theme,
    /// User files the theme was built from; edits to any of them should reload it.
    pub files: Vec<PathBuf>,
}

pub fn resolve(id: &str, user_dir: Option<&Path>) -> Result<Resolved> {
    let mut visited = HashSet::new();
    let mut chain = Vec::new();
    let mut files = Vec::new();
    let mut next = Some(id.to_owned());

    while let Some(current) = next.take() {
        let (mut table, file, builtin) = load_one(&current, user_dir, &mut visited)?;
        next = match table.remove("extends") {
            Some(Value::String(parent)) => Some(parent),
            Some(_) => bail!("theme {current:?}: `extends` must be a theme name"),
            None if builtin && current == DEFAULT_THEME => None,
            None => Some(DEFAULT_THEME.to_owned()),
        };
        files.extend(file);
        chain.push((current, table));
    }

    // A user file shadowing a built-in keeps the built-in's display name;
    // names of themes further down the chain must not leak upwards.
    let name = chain
        .iter()
        .filter(|(chain_id, _)| chain_id == id)
        .find_map(|(_, table)| table.get("name").and_then(Value::as_str))
        .unwrap_or(id)
        .to_owned();
    let mut merged = Table::new();
    for (_, table) in chain.into_iter().rev() {
        merge(&mut merged, table);
    }
    merged.insert("name".to_owned(), Value::String(name));
    let theme = Value::Table(merged)
        .try_into()
        .with_context(|| format!("theme {id:?}"))?;
    Ok(Resolved { theme, files })
}

/// A user file shadows the built-in of the same name, except when the user file
/// itself extends that name, which then means the built-in.
fn load_one(
    id: &str,
    user_dir: Option<&Path>,
    visited: &mut HashSet<String>,
) -> Result<(Table, Option<PathBuf>, bool)> {
    if let Some(path) = user_dir.map(|dir| dir.join(format!("{id}.toml")))
        && path.is_file()
        && visited.insert(format!("user:{id}"))
    {
        let text =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let table = text
            .parse()
            .with_context(|| format!("{}", path.display()))?;
        return Ok((table, Some(path), false));
    }
    if let Some((_, text)) = BUILTIN.iter().find(|(name, _)| *name == id) {
        if visited.insert(format!("builtin:{id}")) {
            return Ok((
                text.parse().expect("built-in themes are valid TOML"),
                None,
                true,
            ));
        }
        bail!("theme {id:?} extends itself");
    }
    if visited.contains(&format!("user:{id}")) {
        bail!("theme {id:?} is part of an `extends` cycle");
    }
    bail!("there is no theme named {id:?}")
}

fn merge(base: &mut Table, over: Table) {
    for (key, value) in over {
        match (base.get_mut(&key), value) {
            (Some(Value::Table(base)), Value::Table(over)) => merge(base, over),
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}

/// Flattens the theme into the camelCase property names of `Theme.qml`.
pub fn to_qml(theme: &Theme) -> Json {
    let mut flat = Map::new();
    let Json::Object(sections) = serde_json::to_value(theme).expect("themes serialize") else {
        unreachable!("a struct serializes to an object");
    };
    for (key, value) in sections {
        match value {
            Json::Object(fields) => {
                flat.extend(fields.into_iter().map(|(k, v)| (camel_case(&k), v)));
            }
            value => {
                flat.insert(camel_case(&key), value);
            }
        }
    }
    Json::Object(flat)
}

fn camel_case(snake: &str) -> String {
    let mut out = String::with_capacity(snake.len());
    let mut upper = false;
    for c in snake.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            out.push(c.to_ascii_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// Every available theme with enough colours to draw a preview swatch.
/// Broken user themes are listed too, with their error, so they can be fixed.
pub fn list(user_dir: Option<&Path>) -> Vec<Json> {
    let mut ids: Vec<String> = BUILTIN.iter().map(|(id, _)| (*id).to_owned()).collect();
    let mut user: Vec<String> = user_dir
        .and_then(|dir| fs::read_dir(dir).ok())
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            (path.extension()? == "toml").then(|| path.file_stem()?.to_str().map(str::to_owned))?
        })
        .filter(|id| !ids.contains(id))
        .collect();
    user.sort();
    ids.extend(user);

    ids.into_iter()
        .map(|id| {
            let custom = user_dir.is_some_and(|dir| dir.join(format!("{id}.toml")).is_file());
            match resolve(&id, user_dir) {
                Ok(Resolved { theme, .. }) => json!({
                    "id": id,
                    "name": theme.name,
                    "custom": custom,
                    "background": theme.colors.background,
                    "card": theme.colors.card,
                    "text": theme.colors.text,
                    "accents": [theme.accents.cpu, theme.accents.memory, theme.accents.swap],
                }),
                Err(e) => {
                    json!({ "id": id, "name": id, "custom": custom, "error": format!("{e:#}") })
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("plusysmon-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn colours_become_qt_argb() {
        assert_eq!(parse_color("#abc").as_deref(), Some("#ffaabbcc"));
        assert_eq!(parse_color("#8B5CF6").as_deref(), Some("#ff8b5cf6"));
        assert_eq!(parse_color("#ffffff0b").as_deref(), Some("#0bffffff"));
        assert_eq!(parse_color("8b5cf6"), None);
        assert_eq!(parse_color("#12345"), None);
        assert_eq!(parse_color("#gggggg"), None);
    }

    #[test]
    fn builtins_resolve() {
        for (id, _) in BUILTIN {
            resolve(id, None).unwrap_or_else(|e| panic!("{id}: {e:#}"));
        }
        let frost = resolve("frost", None).unwrap().theme;
        assert_eq!(frost.name, "Frost");
        assert_eq!(frost.shape.radius, 14.0);
        // Inherited from Aurora.
        assert_eq!(frost.shape.spacing, 16.0);
    }

    #[test]
    fn partial_user_theme_inherits_default() {
        let dir = temp_dir("partial");
        fs::write(dir.join("mine.toml"), "[accents]\ncpu = \"#ff0000\"\n").unwrap();
        let resolved = resolve("mine", Some(&dir)).unwrap();
        assert_eq!(resolved.theme.name, "mine");
        assert_eq!(resolved.theme.accents.cpu.0, "#ffff0000");
        assert_eq!(resolved.theme.accents.memory.0, "#ff22d3ee");
        assert_eq!(resolved.files, vec![dir.join("mine.toml")]);
    }

    #[test]
    fn user_file_can_shadow_and_extend_builtin() {
        let dir = temp_dir("shadow");
        fs::write(dir.join("aurora.toml"), "[shape]\nradius = 4\n").unwrap();
        let theme = resolve("aurora", Some(&dir)).unwrap().theme;
        assert_eq!(theme.shape.radius, 4.0);
        assert_eq!(theme.name, "Aurora");
    }

    #[test]
    fn reports_typos_and_cycles() {
        let dir = temp_dir("errors");
        fs::write(dir.join("typo.toml"), "[colors]\nbakground = \"#000\"\n").unwrap();
        let err = format!("{:#}", resolve("typo", Some(&dir)).err().unwrap());
        assert!(err.contains("bakground"), "{err}");

        fs::write(dir.join("a.toml"), "extends = \"b\"\n").unwrap();
        fs::write(dir.join("b.toml"), "extends = \"a\"\n").unwrap();
        let err = format!("{:#}", resolve("a", Some(&dir)).err().unwrap());
        assert!(err.contains("cycle"), "{err}");
    }

    #[test]
    fn flattens_for_qml() {
        let qml = to_qml(&resolve("aurora", None).unwrap().theme);
        assert_eq!(qml["textDim"], "#ff8b90b2");
        assert_eq!(qml["memory"], "#ff22d3ee");
        assert_eq!(qml["animatedBackground"], true);
        assert_eq!(qml["name"], "Aurora");
    }

    #[test]
    fn lists_builtins_and_broken_user_themes() {
        let dir = temp_dir("list");
        fs::write(dir.join("broken.toml"), "[colors\n").unwrap();
        let list = list(Some(&dir));
        let ids: Vec<_> = list.iter().map(|t| t["id"].as_str().unwrap()).collect();
        assert_eq!(ids, ["aurora", "frost", "daylight", "broken"]);
        assert!(list[3]["error"].is_string());
    }
}
