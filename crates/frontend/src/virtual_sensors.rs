//! Virtual sensors: data sources of your own (`virtual.<n>`) that take their
//! value from whatever real source each machine has for it. One machine
//! reports its CPU voltage as `sensor.it87.2624.in0`, another as
//! `sensor.nct6798.656.in1`; a widget showing `virtual.1` works on both.
//!
//! They live with the dashboard, in `~/.config/plusysmon/virtual_sensors.toml`.
//! Machines are told apart by certificate fingerprint, which stays put when
//! addresses change.

use serde::{Deserialize, Serialize};

use crate::metrics::Unit;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct VirtualSensors {
    pub sensors: Vec<VirtualSensor>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VirtualSensor {
    /// `virtual.<n>`; stays when the name changes, so widgets keep it.
    pub id: String,
    pub name: String,
    /// Only sources in this unit can be linked.
    pub unit: Unit,
    #[serde(default)]
    pub mode: Mode,
    #[serde(default)]
    pub bindings: Vec<Binding>,
}

/// What a machine's linked sources make up.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// The first linked source the machine has right now.
    #[default]
    First,
    Average,
    Highest,
    Lowest,
    Sum,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Binding {
    /// Certificate fingerprint; empty for "every other machine".
    pub machine: String,
    /// What that machine was called, for showing the link elsewhere.
    pub hostname: String,
    /// Source IDs, in order of preference.
    pub sources: Vec<String>,
}

/// A source's current value and full scale, as `Metrics` has them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Value {
    pub value: f64,
    pub max: f64,
}

impl VirtualSensor {
    /// The links that apply on `machine`: its own, or the ones for every machine.
    pub fn binding_for(&self, machine: &str) -> Option<&Binding> {
        self.bindings
            .iter()
            .find(|b| !machine.is_empty() && b.machine == machine)
            .or_else(|| self.bindings.iter().find(|b| b.machine.is_empty()))
    }

    /// The value on `machine`, from the sources `lookup` knows; `None` when
    /// none of the linked sources is there.
    pub fn resolve(&self, machine: &str, lookup: impl Fn(&str) -> Option<Value>) -> Option<Value> {
        let binding = self.binding_for(machine)?;
        let present: Vec<Value> = binding.sources.iter().filter_map(|id| lookup(id)).collect();
        let first = *present.first()?;
        let count = present.len() as f64;
        let values = present.iter().map(|v| v.value);
        // The scale follows: a sum fills up to the sum of the scales.
        let max_scale = present.iter().map(|v| v.max).fold(0.0, f64::max);
        Some(match self.mode {
            Mode::First => first,
            Mode::Average => Value {
                value: values.sum::<f64>() / count,
                max: max_scale,
            },
            Mode::Highest => Value {
                value: values.fold(f64::MIN, f64::max),
                max: max_scale,
            },
            Mode::Lowest => Value {
                value: values.fold(f64::MAX, f64::min),
                max: max_scale,
            },
            Mode::Sum => Value {
                value: values.sum(),
                max: present.iter().map(|v| v.max).sum(),
            },
        })
    }
}

impl VirtualSensors {
    /// Refuses what makes no sense, so the file and the UI never hold it.
    pub fn validated(mut self) -> Result<Self, String> {
        let mut seen = std::collections::HashSet::new();
        for sensor in &mut self.sensors {
            if !sensor.id.starts_with("virtual.") || !seen.insert(sensor.id.clone()) {
                return Err(format!("{:?} is not a usable ID", sensor.id));
            }
            sensor.name = sensor.name.trim().to_owned();
            if sensor.name.is_empty() {
                sensor.name = "Virtual sensor".to_owned();
            }
            for binding in &mut sensor.bindings {
                let mut sources = std::collections::HashSet::new();
                binding
                    .sources
                    .retain(|id| !id.starts_with("virtual.") && sources.insert(id.clone()));
            }
            // One set of links per machine; empty ones say nothing.
            let mut machines = std::collections::HashSet::new();
            sensor
                .bindings
                .retain(|b| !b.sources.is_empty() && machines.insert(b.machine.clone()));
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn voltage(bindings: Vec<Binding>, mode: Mode) -> VirtualSensor {
        VirtualSensor {
            id: "virtual.1".into(),
            name: "CPU voltage".into(),
            unit: Unit::Volts,
            mode,
            bindings,
        }
    }

    fn bind(machine: &str, sources: &[&str]) -> Binding {
        Binding {
            machine: machine.into(),
            hostname: String::new(),
            sources: sources.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn lookup(values: &[(&str, f64)]) -> impl Fn(&str) -> Option<Value> {
        let map: HashMap<String, f64> = values.iter().map(|(k, v)| (k.to_string(), *v)).collect();
        move |id| map.get(id).map(|&value| Value { value, max: 2.0 })
    }

    #[test]
    fn each_machine_uses_its_own_sources() {
        let sensor = voltage(
            vec![
                bind("AA", &["sensor.it87.in0"]),
                bind("BB", &["sensor.nct.in1"]),
            ],
            Mode::First,
        );
        let both = lookup(&[("sensor.it87.in0", 1.2), ("sensor.nct.in1", 0.9)]);
        assert_eq!(sensor.resolve("AA", &both).unwrap().value, 1.2);
        assert_eq!(sensor.resolve("BB", &both).unwrap().value, 0.9);
        // Not linked on this machine, and no catch-all.
        assert_eq!(sensor.resolve("CC", &both), None);
    }

    #[test]
    fn other_machines_fall_back_to_the_catch_all() {
        let sensor = voltage(
            vec![bind("", &["sensor.a", "sensor.b"]), bind("AA", &["sensor.c"])],
            Mode::First,
        );
        let values = lookup(&[("sensor.b", 1.0), ("sensor.c", 3.0)]);
        // The first linked source that exists here.
        assert_eq!(sensor.resolve("ZZ", &values).unwrap().value, 1.0);
        assert_eq!(sensor.resolve("AA", &values).unwrap().value, 3.0);
        assert_eq!(sensor.resolve("ZZ", lookup(&[])), None);
    }

    #[test]
    fn combines_several_sources() {
        let values = lookup(&[("a", 40.0), ("b", 60.0), ("c", 50.0)]);
        let sources = vec![bind("", &["a", "b", "c", "missing"])];
        let resolve = |mode| voltage(sources.clone(), mode).resolve("X", &values).unwrap();
        assert_eq!(resolve(Mode::First).value, 40.0);
        assert_eq!(resolve(Mode::Average).value, 50.0);
        assert_eq!(resolve(Mode::Highest).value, 60.0);
        assert_eq!(resolve(Mode::Lowest).value, 40.0);
        assert_eq!(resolve(Mode::Sum).value, 150.0);
        assert_eq!(resolve(Mode::Sum).max, 6.0);
        assert_eq!(resolve(Mode::Highest).max, 2.0);
    }

    #[test]
    fn validation_tidies_up() {
        let sensors = VirtualSensors {
            sensors: vec![voltage(
                vec![
                    bind("AA", &["a", "a", "virtual.2"]),
                    bind("AA", &["b"]),
                    bind("BB", &[]),
                ],
                Mode::First,
            )],
        }
        .validated()
        .unwrap();
        assert_eq!(sensors.sensors[0].bindings, vec![bind("AA", &["a"])]);

        let duplicate = VirtualSensors {
            sensors: vec![voltage(vec![], Mode::First), voltage(vec![], Mode::First)],
        };
        assert!(duplicate.validated().is_err());
    }

    #[test]
    fn survives_the_file() {
        let sensors = VirtualSensors {
            sensors: vec![voltage(vec![bind("AA:BB", &["sensor.x.in0"])], Mode::Highest)],
        };
        let text = toml::to_string_pretty(&sensors).unwrap();
        assert_eq!(toml::from_str::<VirtualSensors>(&text).unwrap(), sensors);
        assert!(text.contains("unit = \"volts\""), "{text}");
    }
}
