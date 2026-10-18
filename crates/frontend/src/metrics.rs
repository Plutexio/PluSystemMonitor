//! Data sources for widgets: every number a sample carries, under a stable ID
//! (`cpu.total`, `gpu.0.temperature`, ...), with a name, a unit, a scale and
//! recent history. Widgets pick sources by ID and never know where they come from.
//!
//! Besides the machine itself there are sources per running app
//! (`app.<app id>.gpu`) and for whichever game is running (`game.gpu`); those
//! come from the process stream rather than from samples.

use std::collections::{HashMap, VecDeque};

use plusysmon_protocol::{AppKind, Sample};
use serde::Serialize;
use serde_json::json;

use crate::apps::App;

/// Points kept per source (two minutes at the default interval).
pub const HISTORY_POINTS: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    Percent,
    Bytes,
    Celsius,
    Watts,
    Megahertz,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Definition {
    pub id: String,
    pub name: String,
    /// Groups sources in the picker: "Processor", "Graphics · RTX 5070 Ti", ...
    pub category: String,
    pub unit: Unit,
    /// Theme accent that suits the source (cpu, memory, swap, gpu).
    pub accent: &'static str,
    /// For sources of one specific app: its name, a good widget title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
}

struct Series {
    value: f64,
    /// Full scale for gauges and graphs; 0 means "scale to the highest seen".
    max: f64,
    peak: f64,
    history: VecDeque<f64>,
}

#[derive(Default)]
pub struct Metrics {
    /// From samples: CPU, memory, GPUs.
    system: Vec<Definition>,
    /// From the process stream: apps and the current game.
    apps: Vec<Definition>,
    /// Every definition seen this session, so a widget whose app has quit
    /// still knows what it was showing.
    known: HashMap<String, Definition>,
    series: HashMap<String, Series>,
    ram_total: u64,
    vram_total: u64,
}

struct Reading {
    definition: Definition,
    value: f64,
    max: f64,
}

impl Metrics {
    pub fn clear(&mut self) {
        self.system.clear();
        self.apps.clear();
        self.series.clear();
    }

    pub fn record(&mut self, sample: &Sample) {
        self.ram_total = sample.mem.total;
        self.vram_total = sample.gpus.iter().map(|g| g.memory_total).sum();
        let readings = read(sample);
        let system = std::mem::take(&mut self.system);
        self.system = self.apply(system, readings);
    }

    /// Updates the app and current-game sources; call with every process update.
    pub fn record_apps(&mut self, apps: &[App]) {
        let readings = read_apps(apps, self.ram_total, self.vram_total);
        let current = std::mem::take(&mut self.apps);
        self.apps = self.apply(current, readings);
    }

    /// Stores `readings`, replacing the definitions of one group; series of
    /// sources that disappeared from the group are dropped.
    fn apply(
        &mut self,
        mut definitions: Vec<Definition>,
        readings: Vec<Reading>,
    ) -> Vec<Definition> {
        let changed = definitions.len() != readings.len()
            || definitions
                .iter()
                .zip(&readings)
                .any(|(d, r)| *d != r.definition);
        if changed {
            for gone in definitions
                .iter()
                .filter(|d| !readings.iter().any(|r| r.definition.id == d.id))
            {
                self.series.remove(&gone.id);
            }
            definitions = readings.iter().map(|r| r.definition.clone()).collect();
            for definition in &definitions {
                self.known.insert(definition.id.clone(), definition.clone());
            }
        }
        for reading in readings {
            let series = self
                .series
                .entry(reading.definition.id)
                .or_insert_with(|| Series {
                    value: 0.0,
                    max: 0.0,
                    peak: 0.0,
                    history: VecDeque::with_capacity(HISTORY_POINTS),
                });
            if series.history.len() == HISTORY_POINTS {
                series.history.pop_front();
            }
            series.history.push_back(reading.value);
            series.value = reading.value;
            series.max = reading.max;
            series.peak = series.peak.max(reading.value);
        }
        definitions
    }

    /// Every source available right now, for the picker. The current-game
    /// sources are always offered, so a game widget can be set up beforehand.
    pub fn catalog(&self) -> String {
        let placeholders: Vec<Definition> = if self.apps.iter().any(|d| d.id.starts_with("game.")) {
            Vec::new()
        } else {
            GAME_SOURCES
                .iter()
                .filter_map(|(key, _)| game_placeholder(&format!("game.{key}")))
                .collect()
        };
        let all: Vec<&Definition> = self
            .system
            .iter()
            .chain(&placeholders)
            .chain(&self.apps)
            .collect();
        serde_json::to_string(&all).expect("definitions serialize")
    }

    /// One source's definition, current value, scale and history. A source seen
    /// earlier but gone now (its app quit) comes back with `available: false`;
    /// one never seen gives `None`.
    pub fn get(&self, id: &str) -> Option<String> {
        let current = self.system.iter().chain(&self.apps).find(|d| d.id == id);
        let Some((definition, series)) = current.zip(self.series.get(id)) else {
            // Without a game, its sources keep their generic names rather than
            // the last game's.
            let placeholder = game_placeholder(id);
            let definition = placeholder.as_ref().or_else(|| self.known.get(id))?;
            return Some(
                json!({
                    "id": definition.id,
                    "name": definition.name,
                    "category": definition.category,
                    "unit": definition.unit,
                    "accent": definition.accent,
                    "available": false,
                })
                .to_string(),
            );
        };
        let max = if series.max > 0.0 {
            series.max
        } else {
            series.peak
        };
        Some(
            json!({
                "id": definition.id,
                "name": definition.name,
                "category": definition.category,
                "unit": definition.unit,
                "accent": definition.accent,
                "available": true,
                "value": series.value,
                "max": max,
                "history": series.history,
            })
            .to_string(),
        )
    }

    pub fn history(&self, id: &str) -> Vec<f64> {
        self.series
            .get(id)
            .map(|s| s.history.iter().copied().collect())
            .unwrap_or_default()
    }
}

/// The per-app metrics: ID suffix and the word used in names.
const GAME_SOURCES: [(&str, &str); 4] = [
    ("cpu", "CPU"),
    ("memory", "memory"),
    ("gpu", "GPU"),
    ("vram", "VRAM"),
];

/// "game.gpu" while no game runs: named "Game GPU".
fn game_placeholder(id: &str) -> Option<Definition> {
    let key = id.strip_prefix("game.")?;
    let (_, what) = GAME_SOURCES.iter().find(|(k, _)| *k == key)?;
    let (unit, accent) = match key {
        "cpu" => (Unit::Percent, "cpu"),
        "gpu" => (Unit::Percent, "gpu"),
        _ => (Unit::Bytes, "memory"),
    };
    Some(Definition {
        id: id.to_owned(),
        name: format!("Game {what}"),
        category: "Current game".to_owned(),
        unit,
        accent,
        app: None,
    })
}

/// Per app: CPU, memory, GPU and VRAM. Plus the same four for the current game
/// (the running game using the most GPU), which follow whatever game that is.
fn read_apps(apps: &[App], ram_total: u64, vram_total: u64) -> Vec<Reading> {
    let mut readings = Vec::new();
    let mut add_four =
        |prefix: &str, name: &str, category: &str, app_name: Option<&str>, app: &App| {
            let metrics = [
                ("cpu", "CPU", Unit::Percent, "cpu", app.cpu, 100.0),
                (
                    "memory",
                    "memory",
                    Unit::Bytes,
                    "memory",
                    app.memory as f64,
                    ram_total as f64,
                ),
                ("gpu", "GPU", Unit::Percent, "gpu", app.gpu, 100.0),
                (
                    "vram",
                    "VRAM",
                    Unit::Bytes,
                    "memory",
                    app.gpu_memory as f64,
                    vram_total as f64,
                ),
            ];
            for (key, what, unit, accent, value, max) in metrics {
                readings.push(Reading {
                    definition: Definition {
                        id: format!("{prefix}.{key}"),
                        name: format!("{name} {what}"),
                        category: category.to_owned(),
                        unit,
                        accent,
                        app: app_name.map(str::to_owned),
                    },
                    value,
                    max,
                });
            }
        };

    let game = apps
        .iter()
        .filter(|a| a.info.kind == AppKind::Game)
        .max_by(|a, b| a.gpu.total_cmp(&b.gpu).then(a.cpu.total_cmp(&b.cpu)));
    if let Some(game) = game {
        // The name changes with the game, so the widget title does too.
        add_four("game", &game.info.name, "Current game", None, game);
    }

    let mut sorted: Vec<&App> = apps.iter().collect();
    sorted.sort_by_key(|app| app.info.name.to_lowercase());
    for app in sorted {
        let category = match app.info.kind {
            AppKind::Game => format!("Games · {}", app.info.name),
            AppKind::App => format!("Apps · {}", app.info.name),
        };
        let prefix = format!("app.{}", app.info.id);
        add_four(
            &prefix,
            &app.info.name,
            &category,
            Some(&app.info.name),
            app,
        );
    }
    readings
}

fn read(sample: &Sample) -> Vec<Reading> {
    let mut readings = Vec::new();
    let mut add = |id: String, name: String, category: &str, unit, accent, value: f64, max: f64| {
        readings.push(Reading {
            definition: Definition {
                id,
                name,
                category: category.to_owned(),
                unit,
                accent,
                app: None,
            },
            value,
            max,
        })
    };

    let cpu = "Processor";
    add(
        "cpu.total".into(),
        "CPU usage".into(),
        cpu,
        Unit::Percent,
        "cpu",
        sample.cpu.total as f64,
        100.0,
    );
    for (i, core) in sample.cpu.per_core.iter().enumerate() {
        let category = "Processor cores";
        add(
            format!("cpu.core.{i}"),
            format!("Core {i}"),
            category,
            Unit::Percent,
            "cpu",
            *core as f64,
            100.0,
        );
    }

    let mem = &sample.mem;
    let memory = "Memory";
    add(
        "memory.used".into(),
        "RAM".into(),
        memory,
        Unit::Bytes,
        "memory",
        mem.used as f64,
        mem.total as f64,
    );
    if mem.swap_total > 0 {
        add(
            "memory.swap".into(),
            "Swap".into(),
            memory,
            Unit::Bytes,
            "swap",
            mem.swap_used as f64,
            mem.swap_total as f64,
        );
    }

    for (i, gpu) in sample.gpus.iter().enumerate() {
        let short = short_gpu_name(&gpu.name);
        let category = format!("Graphics · {short}");
        let id = |key: &str| format!("gpu.{i}.{key}");
        let name = |what: &str| format!("{short} {what}");
        add(
            id("usage"),
            name("usage"),
            &category,
            Unit::Percent,
            "gpu",
            gpu.usage as f64,
            100.0,
        );
        add(
            id("vram"),
            name("VRAM"),
            &category,
            Unit::Bytes,
            "memory",
            gpu.memory_used as f64,
            gpu.memory_total as f64,
        );
        let optional = [
            (
                "temperature",
                "temperature",
                Unit::Celsius,
                gpu.temperature.map(f64::from),
                100.0,
            ),
            (
                "power",
                "power",
                Unit::Watts,
                gpu.power.map(f64::from),
                gpu.power_limit.unwrap_or(0.0) as f64,
            ),
            (
                "core_clock",
                "core clock",
                Unit::Megahertz,
                gpu.core_clock.map(f64::from),
                0.0,
            ),
            (
                "memory_clock",
                "memory clock",
                Unit::Megahertz,
                gpu.memory_clock.map(f64::from),
                0.0,
            ),
            ("fan", "fan", Unit::Percent, gpu.fan.map(f64::from), 100.0),
            (
                "encoder",
                "video encoder",
                Unit::Percent,
                gpu.encoder.map(f64::from),
                100.0,
            ),
            (
                "decoder",
                "video decoder",
                Unit::Percent,
                gpu.decoder.map(f64::from),
                100.0,
            ),
        ];
        for (key, what, unit, value, max) in optional {
            if let Some(value) = value {
                add(id(key), name(what), &category, unit, "gpu", value, max);
            }
        }
    }
    readings
}

/// "NVIDIA GeForce RTX 5070 Ti" → "RTX 5070 Ti"; the vendor is implied.
fn short_gpu_name(name: &str) -> &str {
    ["NVIDIA GeForce ", "NVIDIA ", "AMD Radeon ", "AMD "]
        .iter()
        .find_map(|prefix| name.strip_prefix(prefix))
        .filter(|rest| !rest.is_empty())
        .unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use plusysmon_protocol::{CpuSample, GpuSample, MemSample};
    use serde_json::Value;

    fn sample(cpu: f32, gpu_clock: u32) -> Sample {
        Sample {
            timestamp_ms: 0,
            cpu: CpuSample {
                total: cpu,
                per_core: vec![cpu, cpu],
            },
            mem: MemSample {
                total: 1000,
                used: 250,
                available: 750,
                swap_total: 0,
                swap_used: 0,
            },
            gpus: vec![GpuSample {
                name: "NVIDIA GeForce RTX 5070 Ti".into(),
                vendor: "NVIDIA".into(),
                usage: 40.0,
                memory_used: 4,
                memory_total: 16,
                temperature: Some(55.0),
                power: Some(100.0),
                power_limit: Some(300.0),
                core_clock: Some(gpu_clock),
                memory_clock: None,
                fan: None,
                encoder: None,
                decoder: None,
            }],
        }
    }

    fn get(metrics: &Metrics, id: &str) -> Value {
        serde_json::from_str(&metrics.get(id).unwrap()).unwrap()
    }

    #[test]
    fn lists_what_the_sample_has() {
        let mut metrics = Metrics::default();
        metrics.record(&sample(10.0, 2000));
        let catalog: Vec<Value> = serde_json::from_str(&metrics.catalog()).unwrap();
        let ids: Vec<&str> = catalog.iter().map(|d| d["id"].as_str().unwrap()).collect();
        assert_eq!(
            ids,
            [
                "cpu.total",
                "cpu.core.0",
                "cpu.core.1",
                "memory.used",
                "gpu.0.usage",
                "gpu.0.vram",
                "gpu.0.temperature",
                "gpu.0.power",
                "gpu.0.core_clock",
                "game.cpu",
                "game.memory",
                "game.gpu",
                "game.vram"
            ]
        );
        assert_eq!(catalog[6]["name"], "RTX 5070 Ti temperature");
        assert_eq!(catalog[6]["category"], "Graphics · RTX 5070 Ti");
        assert!(metrics.get("memory.swap").is_none());
    }

    #[test]
    fn keeps_values_scales_and_history() {
        let mut metrics = Metrics::default();
        metrics.record(&sample(10.0, 2000));
        metrics.record(&sample(30.0, 2500));
        let cpu = get(&metrics, "cpu.total");
        assert_eq!(cpu["value"], 30.0);
        assert_eq!(cpu["max"], 100.0);
        assert_eq!(cpu["history"], json!([10.0, 30.0]));
        assert_eq!(get(&metrics, "memory.used")["max"], 1000.0);
        assert_eq!(get(&metrics, "gpu.0.power")["max"], 300.0);
        // No natural maximum: scale to the highest value seen.
        assert_eq!(get(&metrics, "gpu.0.core_clock")["max"], 2500.0);
        assert_eq!(get(&metrics, "gpu.0.vram")["unit"], "bytes");
    }

    #[test]
    fn follows_hardware_changes() {
        let mut metrics = Metrics::default();
        metrics.record(&sample(10.0, 2000));
        let catalog = metrics.catalog();
        metrics.record(&sample(20.0, 2000));
        assert_eq!(metrics.catalog(), catalog);
        let mut without_gpu = sample(20.0, 2000);
        without_gpu.gpus.clear();
        metrics.record(&without_gpu);
        assert_ne!(metrics.catalog(), catalog);
        // Known from before, so widgets can still name it.
        let gone = get(&metrics, "gpu.0.usage");
        assert_eq!(gone["available"], false);
        assert_eq!(gone["name"], "RTX 5070 Ti usage");
        assert!(metrics.get("never.seen").is_none());
    }

    fn app(id: &str, name: &str, kind: AppKind, cpu: f64, gpu: f64) -> App {
        App {
            info: plusysmon_protocol::AppInfo {
                id: id.into(),
                name: name.into(),
                kind,
                icon: None,
                art: None,
                logo: None,
            },
            cpu,
            memory: 512,
            read_rate: 0,
            write_rate: 0,
            gpu,
            gpu_memory: 4,
            pids: vec![1],
        }
    }

    #[test]
    fn offers_app_and_current_game_sources() {
        let mut metrics = Metrics::default();
        metrics.record(&sample(10.0, 2000));
        metrics.record_apps(&[
            app("desktop:zen", "Zen Browser", AppKind::App, 3.0, 1.0),
            app("steam:1", "Old Game", AppKind::Game, 1.0, 5.0),
            app("steam:2", "Big Game", AppKind::Game, 20.0, 90.0),
        ]);

        let gpu = get(&metrics, "app.steam:2.gpu");
        assert_eq!(gpu["name"], "Big Game GPU");
        assert_eq!(gpu["value"], 90.0);
        assert_eq!(get(&metrics, "app.desktop:zen.memory")["max"], 1000.0);
        assert_eq!(get(&metrics, "app.steam:2.vram")["max"], 16.0);

        // The current game is the one working the GPU hardest.
        let game = get(&metrics, "game.gpu");
        assert_eq!(game["name"], "Big Game GPU");
        assert_eq!(game["category"], "Current game");

        let catalog: Vec<Value> = serde_json::from_str(&metrics.catalog()).unwrap();
        let zen = catalog
            .iter()
            .find(|d| d["id"] == "app.desktop:zen.cpu")
            .unwrap();
        assert_eq!(zen["app"], "Zen Browser");
        assert_eq!(zen["category"], "Apps · Zen Browser");
        let current = catalog.iter().find(|d| d["id"] == "game.cpu").unwrap();
        assert!(current.get("app").is_none());
        // System sources stay.
        assert_eq!(get(&metrics, "cpu.total")["value"], 10.0);
    }

    #[test]
    fn offers_game_sources_before_any_game_runs() {
        let mut metrics = Metrics::default();
        metrics.record(&sample(10.0, 2000));
        let catalog: Vec<Value> = serde_json::from_str(&metrics.catalog()).unwrap();
        let game: Vec<&str> = catalog
            .iter()
            .filter(|d| d["category"] == "Current game")
            .map(|d| d["name"].as_str().unwrap())
            .collect();
        assert_eq!(game, ["Game CPU", "Game memory", "Game GPU", "Game VRAM"]);
        assert_eq!(get(&metrics, "game.vram")["available"], false);
    }

    #[test]
    fn app_sources_go_and_keep_their_names() {
        let mut metrics = Metrics::default();
        metrics.record(&sample(10.0, 2000));
        metrics.record_apps(&[app("steam:2", "Big Game", AppKind::Game, 20.0, 90.0)]);
        metrics.record_apps(&[app("steam:2", "Big Game", AppKind::Game, 30.0, 80.0)]);
        assert_eq!(metrics.history("app.steam:2.cpu"), [20.0, 30.0]);

        metrics.record_apps(&[]);
        let gone = get(&metrics, "app.steam:2.gpu");
        assert_eq!(gone["available"], false);
        assert_eq!(gone["name"], "Big Game GPU");
        let game = get(&metrics, "game.gpu");
        assert_eq!(game["available"], false);
        assert_eq!(game["name"], "Game GPU");
        assert!(metrics.history("app.steam:2.cpu").is_empty());
    }
}
