//! Data sources for widgets: every number a sample carries, under a stable ID
//! (`cpu.total`, `gpu.0.temperature`, ...), with a name, a unit, a scale and
//! recent history. Widgets pick sources by ID and never know where they come from.

use std::collections::{HashMap, VecDeque};

use plusysmon_protocol::Sample;
use serde::Serialize;
use serde_json::json;

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
    definitions: Vec<Definition>,
    series: HashMap<String, Series>,
}

struct Reading {
    definition: Definition,
    value: f64,
    max: f64,
}

impl Metrics {
    pub fn clear(&mut self) {
        self.definitions.clear();
        self.series.clear();
    }

    pub fn record(&mut self, sample: &Sample) {
        let readings = read(sample);
        let ids: Vec<&str> = readings.iter().map(|r| r.definition.id.as_str()).collect();
        let current: Vec<&str> = self.definitions.iter().map(|d| d.id.as_str()).collect();
        if ids != current {
            self.definitions = readings.iter().map(|r| r.definition.clone()).collect();
            self.series
                .retain(|id, _| readings.iter().any(|r| &r.definition.id == id));
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
    }

    /// Every source, for the picker.
    pub fn catalog(&self) -> String {
        serde_json::to_string(&self.definitions).expect("definitions serialize")
    }

    /// One source's definition, current value, scale and history; `None` if
    /// this machine doesn't have it (e.g. a GPU source on a GPU-less box).
    pub fn get(&self, id: &str) -> Option<String> {
        let definition = self.definitions.iter().find(|d| d.id == id)?;
        let series = self.series.get(id)?;
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
                "value": series.value,
                "max": max,
                "history": series.history,
            })
            .to_string(),
        )
    }
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
                "gpu.0.core_clock"
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
        assert!(metrics.get("gpu.0.usage").is_none());
    }
}
