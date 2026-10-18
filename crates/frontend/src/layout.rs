//! Dashboard layout: widgets placed on a grid, stored as `layout.toml`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

const MAX_COLUMNS: u32 = 48;
const MAX_ROWS: u32 = 200;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub columns: u32,
    #[serde(default)]
    pub widgets: Vec<Widget>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Widget {
    /// Registry id such as `cpu.graph`. Unknown ids are kept so a layout made by a
    /// newer version survives a round trip through an older one.
    #[serde(rename = "type")]
    pub kind: String,
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub options: BTreeMap<String, OptionValue>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OptionValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

impl Layout {
    pub fn builtin_default() -> Self {
        toml::from_str(include_str!("../layouts/default.toml")).expect("default layout is valid")
    }

    pub fn from_json(json: &str) -> Result<Self> {
        let layout: Self = serde_json::from_str(json).context("malformed layout")?;
        layout.validate()?;
        Ok(layout)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("layouts serialize")
    }

    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).expect("layouts serialize")
    }

    fn validate(&self) -> Result<()> {
        if !(1..=MAX_COLUMNS).contains(&self.columns) {
            bail!("columns must be between 1 and {MAX_COLUMNS}");
        }
        for widget in &self.widgets {
            let Widget {
                kind, x, y, w, h, ..
            } = widget;
            if *w == 0 || *h == 0 || x + w > self.columns || y + h > MAX_ROWS {
                bail!("widget {kind:?} at {x},{y} sized {w}x{h} does not fit the grid");
            }
        }
        Ok(())
    }
}

pub enum Loaded {
    Ok(Layout),
    Missing,
    /// The file was unreadable and has been moved aside to `<name>.broken`.
    Broken(String),
}

pub fn load(path: &Path) -> Loaded {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Loaded::Missing,
        Err(e) => return Loaded::Broken(format!("{}: {e}", path.display())),
    };
    let parsed = toml::from_str::<Layout>(&text)
        .map_err(anyhow::Error::from)
        .and_then(|layout| layout.validate().map(|()| layout));
    match parsed {
        Ok(layout) => Loaded::Ok(layout),
        Err(e) => {
            // Keep the user's file instead of silently overwriting it on the next save.
            let _ = fs::rename(path, path.with_extension("toml.broken"));
            Loaded::Broken(format!("{}: {e:#}", path.display()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_valid() {
        let layout = Layout::builtin_default();
        layout.validate().unwrap();
        assert_eq!(layout.widgets.len(), 6);
    }

    #[test]
    fn roundtrips_json_and_toml() {
        let json = r#"{"columns":12,"widgets":[{"type":"cpu.graph","x":3,"y":0,"w":9,"h":4,
            "options":{"showCores":false,"accent":"swap","points":90}}]}"#;
        let layout = Layout::from_json(json).unwrap();
        assert_eq!(
            layout.widgets[0].options["accent"],
            OptionValue::Text("swap".into())
        );
        assert_eq!(layout.widgets[0].options["points"], OptionValue::Int(90));
        let back: Layout = toml::from_str(&layout.to_toml()).unwrap();
        assert_eq!(back, layout);
    }

    #[test]
    fn rejects_widgets_off_the_grid() {
        let json = r#"{"columns":12,"widgets":[{"type":"cpu.gauge","x":10,"y":0,"w":3,"h":4}]}"#;
        assert!(Layout::from_json(json).is_err());
        let json = r#"{"columns":12,"widgets":[{"type":"cpu.gauge","x":0,"y":0,"w":0,"h":4}]}"#;
        assert!(Layout::from_json(json).is_err());
    }

    #[test]
    fn broken_file_is_moved_aside() {
        let dir = std::env::temp_dir().join(format!("plusysmon-layout-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("layout.toml");
        fs::write(&path, "columns = \"twelve\"\n").unwrap();
        assert!(matches!(load(&path), Loaded::Broken(_)));
        assert!(!path.exists());
        assert!(dir.join("layout.toml.broken").exists());
        assert!(matches!(load(&path), Loaded::Missing));
    }
}
