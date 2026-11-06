//! Data sources for widgets: every number a sample carries, under a stable ID
//! (`cpu.total`, `gpu.0.temperature`, ...), with a name, a unit, a scale and
//! recent history. Widgets pick sources by ID and never know where they come from.
//!
//! Besides the machine itself there are sources per running app
//! (`app.<app id>.gpu`) and for whichever game is running (`game.gpu`); those
//! come from the process stream rather than from samples. Virtual sensors
//! (`virtual.<n>`, see `virtual_sensors`) take their values from other sources.

use std::collections::{HashMap, VecDeque};

use plusysmon_protocol::{AppKind, Sample, SensorKind};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::apps::App;
use crate::virtual_sensors::{self, VirtualSensors};

/// Points kept per source (two minutes at the default interval).
pub const HISTORY_POINTS: usize = 120;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Unit {
    Percent,
    Bytes,
    Celsius,
    Watts,
    Megahertz,
    Volts,
    Rpm,
    Amps,
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
    virtuals: VirtualSensors,
    /// The connected machine's certificate fingerprint, which virtual
    /// sensors are linked by.
    machine: String,
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

    pub fn set_virtuals(&mut self, virtuals: VirtualSensors) {
        self.virtuals = virtuals;
    }

    pub fn set_machine(&mut self, fingerprint: &str) {
        self.machine = fingerprint.to_owned();
    }

    pub fn record(&mut self, sample: &Sample) {
        self.ram_total = sample.mem.total;
        self.vram_total = sample.gpus.iter().map(|g| g.memory_total).sum();
        let mut readings = read(sample);
        let virtual_readings = self.read_virtuals(&readings);
        readings.extend(virtual_readings);
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

    /// The virtual sensors that have a linked source on this machine.
    fn read_virtuals(&self, readings: &[Reading]) -> Vec<Reading> {
        self.virtuals
            .sensors
            .iter()
            .filter_map(|sensor| {
                let found = sensor.resolve(&self.machine, |id| {
                    let reading = readings.iter().find(|r| r.definition.id == id)?;
                    (reading.definition.unit == sensor.unit).then_some(virtual_sensors::Value {
                        value: reading.value,
                        max: reading.max,
                    })
                })?;
                Some(Reading {
                    definition: virtual_definition(sensor),
                    value: found.value,
                    max: found.max,
                })
            })
            .collect()
    }

    /// Every source available right now, for the picker. The current-game
    /// sources are always offered, so a game widget can be set up beforehand;
    /// so are virtual sensors, which may only be linked on other machines.
    pub fn catalog(&self) -> String {
        let mut placeholders: Vec<Definition> =
            if self.apps.iter().any(|d| d.id.starts_with("game.")) {
                Vec::new()
            } else {
                GAME_SOURCES
                    .iter()
                    .filter_map(|(key, _)| game_placeholder(&format!("game.{key}")))
                    .collect()
            };
        placeholders.extend(
            self.virtuals
                .sensors
                .iter()
                .filter(|v| !self.system.iter().any(|d| d.id == v.id))
                .map(virtual_definition),
        );
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
        self.get_value(id).map(|value| value.to_string())
    }

    /// `get`, as a JSON value.
    pub fn get_value(&self, id: &str) -> Option<serde_json::Value> {
        let current = self.system.iter().chain(&self.apps).find(|d| d.id == id);
        let Some((definition, series)) = current.zip(self.series.get(id)) else {
            // Without a game, its sources keep their generic names rather than
            // the last game's.
            let placeholder = game_placeholder(id).or_else(|| {
                let sensor = self.virtuals.sensors.iter().find(|v| v.id == id)?;
                Some(virtual_definition(sensor))
            });
            let definition = placeholder.as_ref().or_else(|| self.known.get(id))?;
            return Some(json!({
                "id": definition.id,
                "name": definition.name,
                "category": definition.category,
                "unit": definition.unit,
                "accent": definition.accent,
                "available": false,
            }));
        };
        let max = if series.max > 0.0 {
            series.max
        } else {
            series.peak
        };
        Some(json!({
            "id": definition.id,
            "name": definition.name,
            "category": definition.category,
            "unit": definition.unit,
            "accent": definition.accent,
            "available": true,
            "value": series.value,
            "max": max,
            "history": series.history,
        }))
    }

    pub fn history(&self, id: &str) -> Vec<f64> {
        self.series
            .get(id)
            .map(|s| s.history.iter().copied().collect())
            .unwrap_or_default()
    }
}

fn virtual_definition(sensor: &virtual_sensors::VirtualSensor) -> Definition {
    Definition {
        id: sensor.id.clone(),
        name: sensor.name.clone(),
        category: "Virtual sensors".to_owned(),
        unit: sensor.unit,
        accent: unit_accent(sensor.unit),
        app: None,
    }
}

/// The colour sources of a unit are usually drawn in.
fn unit_accent(unit: Unit) -> &'static str {
    match unit {
        Unit::Percent | Unit::Megahertz => "cpu",
        Unit::Bytes | Unit::Rpm => "memory",
        Unit::Celsius => "disk",
        Unit::Watts => "gpu",
        Unit::Volts => "network",
        Unit::Amps => "swap",
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
    let extras = [
        (
            "cpu.temperature",
            "CPU temperature",
            Unit::Celsius,
            "disk",
            sample.cpu.temperature,
            100.0,
        ),
        (
            "cpu.frequency",
            "CPU clock (average)",
            Unit::Megahertz,
            "cpu",
            sample.cpu.frequency,
            sample.cpu.frequency_max.unwrap_or(0.0),
        ),
        (
            "cpu.frequency_peak",
            "CPU clock (fastest core)",
            Unit::Megahertz,
            "cpu",
            sample.cpu.frequency_peak,
            sample.cpu.frequency_max.unwrap_or(0.0),
        ),
        (
            "cpu.power",
            "CPU power",
            Unit::Watts,
            "gpu",
            sample.cpu.power,
            0.0,
        ),
    ];
    for (id, name, unit, accent, value, max) in extras {
        if let Some(value) = value {
            add(
                id.into(),
                name.into(),
                cpu,
                unit,
                accent,
                value as f64,
                max as f64,
            );
        }
    }
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

    for sensor in &sample.sensors {
        // Each kind gets a colour of its own, so mixed graphs stay readable.
        let (unit, accent, max) = match sensor.kind {
            SensorKind::Temperature => (Unit::Celsius, "disk", sensor.limit.unwrap_or(100.0)),
            SensorKind::Voltage => (Unit::Volts, "network", 0.0),
            SensorKind::Fan => (Unit::Rpm, "memory", 0.0),
            SensorKind::Power => (Unit::Watts, "gpu", 0.0),
            SensorKind::Current => (Unit::Amps, "swap", 0.0),
        };
        add(
            format!("sensor.{}", sensor.id),
            format!("{} {}", sensor.device, sensor.label),
            &format!("Sensors · {} ({})", sensor.device, sensor.driver),
            unit,
            accent,
            sensor.value,
            max,
        );
    }

    for fan in &sample.fans {
        add(
            format!("fan.{}", fan.id),
            format!("{} speed", fan.name),
            "Fan control",
            Unit::Percent,
            "memory",
            fan.percent,
            100.0,
        );
    }
    readings
}

/// One source's value in `sample`, as [`Metrics`] would show it, without
/// building every other source too (fan control asks every few seconds).
pub fn value_of(sample: &Sample, id: &str) -> Option<f64> {
    let (head, rest) = id.split_once('.')?;
    let number = |v: Option<f32>| v.map(f64::from);
    match head {
        "cpu" => match rest {
            "total" => Some(sample.cpu.total.into()),
            "temperature" => number(sample.cpu.temperature),
            "frequency" => number(sample.cpu.frequency),
            "frequency_peak" => number(sample.cpu.frequency_peak),
            "power" => number(sample.cpu.power),
            _ => {
                let core: usize = rest.strip_prefix("core.")?.parse().ok()?;
                sample.cpu.per_core.get(core).map(|&v| v.into())
            }
        },
        "memory" => match rest {
            "used" => Some(sample.mem.used as f64),
            "swap" if sample.mem.swap_total > 0 => Some(sample.mem.swap_used as f64),
            _ => None,
        },
        "gpu" => {
            let (index, key) = rest.split_once('.')?;
            let gpu = sample.gpus.get(index.parse::<usize>().ok()?)?;
            match key {
                "usage" => Some(gpu.usage.into()),
                "vram" => Some(gpu.memory_used as f64),
                "temperature" => number(gpu.temperature),
                "power" => number(gpu.power),
                "core_clock" => gpu.core_clock.map(f64::from),
                "memory_clock" => gpu.memory_clock.map(f64::from),
                "fan" => number(gpu.fan),
                "encoder" => number(gpu.encoder),
                "decoder" => number(gpu.decoder),
                _ => None,
            }
        }
        "sensor" => sample.sensors.iter().find(|s| s.id == rest).map(|s| s.value),
        "fan" => sample.fans.iter().find(|f| f.id == rest).map(|f| f.percent),
        _ => None,
    }
}

/// "NVIDIA GeForce RTX 5070 Ti" → "RTX 5070 Ti"; the vendor is implied.
pub fn short_gpu_name(name: &str) -> &str {
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
                temperature: None,
                frequency: None,
                frequency_peak: None,
                frequency_max: None,
                power: None,
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
            sensors: vec![],
            fans: vec![],
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

    #[test]
    fn virtual_sensors_follow_the_machine() {
        use crate::virtual_sensors::{Binding, Mode, VirtualSensor};
        let mut metrics = Metrics::default();
        metrics.set_virtuals(VirtualSensors {
            sensors: vec![VirtualSensor {
                id: "virtual.1".into(),
                name: "GPU heat".into(),
                unit: Unit::Celsius,
                mode: Mode::First,
                bindings: vec![
                    Binding {
                        machine: "AA".into(),
                        hostname: "desk".into(),
                        sources: vec!["gpu.0.temperature".into()],
                    },
                    Binding {
                        machine: "BB".into(),
                        hostname: "server".into(),
                        // Wrong unit: never used.
                        sources: vec!["gpu.0.usage".into()],
                    },
                ],
            }],
        });

        metrics.set_machine("AA");
        metrics.record(&sample(10.0, 2000));
        metrics.record(&sample(10.0, 2000));
        let heat = get(&metrics, "virtual.1");
        assert_eq!(heat["value"], 55.0);
        assert_eq!(heat["max"], 100.0);
        assert_eq!(heat["unit"], "celsius");
        assert_eq!(heat["category"], "Virtual sensors");
        assert_eq!(metrics.history("virtual.1"), [55.0, 55.0]);

        // On a machine without a link it's still offered, but has no value.
        metrics.clear();
        metrics.set_machine("BB");
        metrics.record(&sample(10.0, 2000));
        let heat = get(&metrics, "virtual.1");
        assert_eq!(heat["available"], false);
        assert_eq!(heat["name"], "GPU heat");
        let catalog: Vec<Value> = serde_json::from_str(&metrics.catalog()).unwrap();
        assert!(catalog.iter().any(|d| d["id"] == "virtual.1"));
    }

    #[test]
    fn offers_cpu_extras_and_sensors() {
        let mut with_sensors = sample(10.0, 2000);
        with_sensors.cpu.temperature = Some(53.0);
        with_sensors.cpu.frequency = Some(4000.0);
        with_sensors.cpu.frequency_max = Some(4900.0);
        with_sensors.sensors = vec![
            plusysmon_protocol::SensorReading {
                id: "it87.2624.fan1".into(),
                device: "Motherboard".into(),
                driver: "it8689".into(),
                label: "Fan 1".into(),
                kind: SensorKind::Fan,
                value: 450.0,
                limit: None,
            },
            plusysmon_protocol::SensorReading {
                id: "nvme0.temp1".into(),
                device: "NVMe Lexar".into(),
                driver: "nvme".into(),
                label: "Composite".into(),
                kind: SensorKind::Temperature,
                value: 48.0,
                limit: Some(95.0),
            },
        ];
        let mut metrics = Metrics::default();
        metrics.record(&with_sensors);

        let temp = get(&metrics, "cpu.temperature");
        assert_eq!(temp["value"], 53.0);
        assert_eq!(temp["unit"], "celsius");
        assert_eq!(get(&metrics, "cpu.frequency")["max"], 4900.0);
        assert!(metrics.get("cpu.power").is_none());

        let fan = get(&metrics, "sensor.it87.2624.fan1");
        assert_eq!(fan["name"], "Motherboard Fan 1");
        assert_eq!(fan["category"], "Sensors · Motherboard (it8689)");
        assert_eq!(fan["unit"], "rpm");
        // No natural scale: the highest value seen.
        assert_eq!(fan["max"], 450.0);
        assert_eq!(get(&metrics, "sensor.nvme0.temp1")["max"], 95.0);
    }

    /// Fan control reads single values; they must be the ones widgets show.
    #[test]
    fn single_values_match_the_catalog() {
        let mut full = sample(37.5, 2100);
        full.cpu.temperature = Some(61.0);
        full.cpu.frequency = Some(4100.0);
        full.cpu.frequency_peak = Some(5200.0);
        full.cpu.power = Some(88.0);
        full.mem.swap_total = 8;
        full.gpus[0].fan = Some(44.0);
        full.sensors = vec![plusysmon_protocol::SensorReading {
            id: "nvme0.temp1".into(),
            device: "NVMe".into(),
            driver: "nvme".into(),
            label: "Composite".into(),
            kind: SensorKind::Temperature,
            value: 48.0,
            limit: None,
        }];
        full.fans = vec![plusysmon_protocol::fans::FanState {
            id: "it87.2624.pwm2".into(),
            name: "Motherboard Fan 2".into(),
            percent: 42.0,
            input: Some(61.0),
            problem: String::new(),
        }];
        let readings = read(&full);
        assert!(readings.len() > 15);
        for reading in &readings {
            assert_eq!(
                value_of(&full, &reading.definition.id),
                Some(reading.value),
                "{}",
                reading.definition.id
            );
        }
        assert_eq!(value_of(&full, "fan.it87.2624.pwm2"), Some(42.0));
        assert_eq!(value_of(&full, "gpu.3.usage"), None);
        assert_eq!(value_of(&full, "cpu.core.99"), None);
        assert_eq!(value_of(&full, "nothing"), None);
    }
}
