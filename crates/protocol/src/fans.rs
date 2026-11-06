//! Fan control: what the daemon (feature `fans`) keeps in `fans.toml`, what
//! it tells frontends, and how it talks to `plusysfand`, the small root
//! helper that does the writing.
//!
//! The daemon turns its own samples into a speed for every controlled fan;
//! the helper only sets speeds, and gives the fans back to the firmware when
//! the daemon goes quiet.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Where `plusysfand` listens (systemd's `RuntimeDirectory=plusysfand`).
pub const HELPER_SOCKET: &str = "/run/plusysfand/plusysfand.sock";

/// The helper's line protocol. Raised on changes the other side must know of;
/// see `crates/fand/src/main.rs` for the commands.
pub const HELPER_VERSION: u32 = 1;

/// A helper that hears nothing from the daemon for this long gives the fans
/// back to the firmware.
pub const HELPER_WATCHDOG_SECS: u64 = 10;

pub const DEFAULT_INTERVAL_MS: u32 = 3000;
/// Ticks further apart than this would trip the helper's watchdog.
pub const MAX_INTERVAL_MS: u32 = 5000;
pub const MIN_INTERVAL_MS: u32 = 1000;

/// How several sources make up one value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// The first source there is right now.
    #[default]
    First,
    Average,
    Highest,
    Lowest,
    Sum,
}

/// `values` combined as `mode` says; `None` when there are none.
pub fn combine(mode: Mode, values: &[f64]) -> Option<f64> {
    let first = *values.first()?;
    Some(match mode {
        Mode::First => first,
        Mode::Average => values.iter().sum::<f64>() / values.len() as f64,
        Mode::Highest => values.iter().copied().fold(f64::MIN, f64::max),
        Mode::Lowest => values.iter().copied().fold(f64::MAX, f64::min),
        Mode::Sum => values.iter().sum(),
    })
}

/// A machine's fan control, kept by its daemon (`~/.config/plusysmon/fans.toml`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FanSettings {
    /// How often speeds are worked out and sent, ms.
    pub interval_ms: u32,
    pub curves: Vec<Curve>,
    /// Per fan output ([`FanOutput::id`]). Fans not listed stay with the firmware.
    pub fans: BTreeMap<String, FanControl>,
}

impl Default for FanSettings {
    fn default() -> Self {
        Self {
            interval_ms: DEFAULT_INTERVAL_MS,
            curves: Vec::new(),
            fans: BTreeMap::new(),
        }
    }
}

/// Fan speed (percent) by an input value, usually a temperature.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Curve {
    /// Stays when the name changes, so fans keep their curve.
    pub id: String,
    pub name: String,
    /// Data source IDs (`cpu.temperature`, `gpu.0.temperature`,
    /// `sensor.<id>`, ...), combined as `combine` says.
    pub sources: Vec<String>,
    pub combine: Mode,
    /// The virtual sensor `sources` were taken from, for showing it; the
    /// daemon only reads `sources`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virtual_id: Option<String>,
    /// `[input, percent]`, ascending by input. Between points the speed is
    /// interpolated; outside, it is the nearest end's.
    pub points: Vec<[f64; 2]>,
    /// Speeds only drop once the input fell this far below where they rose.
    pub hysteresis: f64,
    /// The input is averaged over this many seconds of samples (0: the latest).
    pub average_secs: u32,
    /// Most the speed may rise and fall per second, percent; 0 for no limit.
    pub step_up: f64,
    pub step_down: f64,
}

impl Default for Curve {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            sources: Vec::new(),
            combine: Mode::Highest,
            virtual_id: None,
            points: vec![[30.0, 25.0], [50.0, 35.0], [70.0, 60.0], [85.0, 100.0]],
            hysteresis: 2.0,
            average_secs: 0,
            step_up: 0.0,
            step_down: 5.0,
        }
    }
}

impl Curve {
    /// The speed for `input`, before hysteresis, steps and limits.
    pub fn speed_at(&self, input: f64) -> f64 {
        let points = &self.points;
        let Some(first) = points.first() else {
            return 100.0;
        };
        if input <= first[0] {
            return first[1];
        }
        for pair in points.windows(2) {
            let ([x0, y0], [x1, y1]) = (pair[0], pair[1]);
            if input <= x1 {
                if x1 <= x0 {
                    return y1;
                }
                return y0 + (y1 - y0) * (input - x0) / (x1 - x0);
            }
        }
        points[points.len() - 1][1]
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FanMode {
    /// Left to the firmware (BIOS, GPU driver).
    #[default]
    Auto,
    /// Always [`FanControl::percent`].
    Fixed,
    /// Follows [`FanControl::curve`].
    Curve,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FanControl {
    /// Shown instead of the generated name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub mode: FanMode,
    /// For [`FanMode::Fixed`].
    pub percent: f64,
    /// A [`Curve::id`], for [`FanMode::Curve`].
    pub curve: String,
    /// The speed never goes below `min` (or stops, see `allow_stop`) or above `max`.
    pub min: f64,
    pub max: f64,
    /// Lets the fan stop when the curve asks for less than `min`.
    pub allow_stop: bool,
}

impl Default for FanControl {
    fn default() -> Self {
        Self {
            label: None,
            mode: FanMode::Auto,
            percent: 50.0,
            curve: String::new(),
            min: 20.0,
            max: 100.0,
            allow_stop: false,
        }
    }
}

impl FanControl {
    /// What a fan asked to run at `percent` gets.
    pub fn limit(&self, percent: f64) -> f64 {
        if self.allow_stop && percent < self.min {
            return 0.0;
        }
        percent.clamp(self.min, self.max)
    }

    fn is_default(&self) -> bool {
        self.mode == FanMode::Auto && self.label.is_none()
    }
}

impl FanSettings {
    /// Whether any fan is taken from the firmware.
    pub fn controls_any(&self) -> bool {
        self.fans.values().any(|f| f.mode != FanMode::Auto)
    }

    pub fn curve(&self, id: &str) -> Option<&Curve> {
        self.curves.iter().find(|c| c.id == id)
    }

    /// Refuses what makes no sense or could cook something, and tidies the rest.
    pub fn validated(mut self) -> Result<Self, String> {
        const MAX_CURVES: usize = 100;
        const MAX_FANS: usize = 200;
        const MAX_POINTS: usize = 32;
        const MAX_NAME: usize = 80;
        if self.curves.len() > MAX_CURVES || self.fans.len() > MAX_FANS {
            return Err("too many curves or fans".to_owned());
        }
        if !(MIN_INTERVAL_MS..=MAX_INTERVAL_MS).contains(&self.interval_ms) {
            return Err(format!(
                "the interval must be {MIN_INTERVAL_MS} to {MAX_INTERVAL_MS} ms"
            ));
        }
        let mut ids = std::collections::HashSet::new();
        for curve in &mut self.curves {
            curve.id = curve.id.trim().to_owned();
            if curve.id.is_empty() || !ids.insert(curve.id.clone()) {
                return Err(format!("{:?} is not a usable curve ID", curve.id));
            }
            curve.name = curve.name.trim().to_owned();
            if curve.name.is_empty() {
                curve.name = "Curve".to_owned();
            }
            if curve.name.chars().count() > MAX_NAME {
                return Err(format!("{}: the name is too long", curve.name));
            }
            let mut seen = std::collections::HashSet::new();
            curve
                .sources
                .retain(|s| !s.is_empty() && seen.insert(s.clone()));
            if curve.points.is_empty() || curve.points.len() > MAX_POINTS {
                return Err(format!(
                    "{}: a curve needs 1 to {MAX_POINTS} points",
                    curve.name
                ));
            }
            if curve.points.iter().flatten().any(|v| !v.is_finite()) {
                return Err(format!("{}: points must be numbers", curve.name));
            }
            for point in &mut curve.points {
                point[1] = point[1].clamp(0.0, 100.0);
            }
            curve.points.sort_by(|a, b| a[0].total_cmp(&b[0]));
            for value in [curve.hysteresis, curve.step_up, curve.step_down] {
                if !value.is_finite() || value < 0.0 {
                    return Err(format!(
                        "{}: hysteresis and steps can't be negative",
                        curve.name
                    ));
                }
            }
            curve.average_secs = curve.average_secs.min(600);
        }
        for (id, fan) in &mut self.fans {
            if [fan.percent, fan.min, fan.max]
                .iter()
                .any(|v| !v.is_finite())
            {
                return Err(format!("{id}: speeds must be numbers"));
            }
            fan.min = fan.min.clamp(0.0, 100.0);
            fan.max = fan.max.clamp(fan.min, 100.0);
            fan.percent = fan.percent.clamp(0.0, 100.0);
            if fan.label.as_ref().is_some_and(|l| l.trim().is_empty()) {
                fan.label = None;
            }
            if fan
                .label
                .as_ref()
                .is_some_and(|l| l.chars().count() > MAX_NAME)
            {
                return Err(format!("{id}: the name is too long"));
            }
            if fan.mode == FanMode::Curve && !ids.contains(&fan.curve) {
                return Err(format!("{id}: there is no curve {:?}", fan.curve));
            }
        }
        self.fans.retain(|_, f| !f.is_default());
        Ok(self)
    }
}

/// A fan the helper can drive.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FanOutput {
    /// `<hwmon device>.pwm<N>` or `nvidia.<GPU UUID>.fan<N>`.
    pub id: String,
    /// "Motherboard Fan 2", "RTX 5070 Ti Fan 1", ...
    pub name: String,
    /// The chip or card: "Motherboard", "RTX 5070 Ti", ...
    pub device: String,
    /// The data source with its speed in RPM, when there is one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpm_source: Option<String>,
    /// What it runs at now, percent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub percent: Option<f64>,
    /// The lowest speed the hardware takes (NVIDIA cards have one).
    pub min_percent: f64,
}

/// A controlled fan, as of the latest tick (part of every sample).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FanState {
    pub id: String,
    /// As [`FanOutput::name`], or the user's label.
    #[serde(default)]
    pub name: String,
    /// The speed sent to the helper, percent.
    pub percent: f64,
    /// The curve's input value, when the fan follows a curve.
    pub input: Option<f64>,
    /// What went wrong: no input, helper unreachable, ...; empty when fine.
    #[serde(default)]
    pub problem: String,
}

/// Whether the daemon can control fans right now.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FanStatus {
    /// The helper answered.
    pub helper: bool,
    /// Why fans can't be controlled, in words for the user; empty when fine.
    pub problem: String,
}

/// A stable name for a hwmon chip (`/sys/class/hwmon/hwmonN`): its device's
/// name (`it87.2624`, `0000:03:00.0`), which survives reboots unlike the
/// hwmonN number, or the driver's name for chips without a device.
pub fn hwmon_device_id(hwmon_dir: &Path, driver: &str) -> String {
    std::fs::canonicalize(hwmon_dir.join("device"))
        .ok()
        .and_then(|d| d.file_name()?.to_str().map(str::to_owned))
        .unwrap_or_else(|| driver.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn curve(points: &[[f64; 2]]) -> Curve {
        Curve {
            id: "c".into(),
            name: "CPU".into(),
            points: points.to_vec(),
            ..Default::default()
        }
    }

    #[test]
    fn interpolates_curves() {
        let c = curve(&[[40.0, 20.0], [60.0, 40.0], [80.0, 100.0]]);
        assert_eq!(c.speed_at(10.0), 20.0);
        assert_eq!(c.speed_at(40.0), 20.0);
        assert_eq!(c.speed_at(50.0), 30.0);
        assert_eq!(c.speed_at(70.0), 70.0);
        assert_eq!(c.speed_at(95.0), 100.0);
        // Two points at one input: a step.
        let step = curve(&[[50.0, 20.0], [50.0, 80.0]]);
        assert_eq!(step.speed_at(49.0), 20.0);
        assert_eq!(step.speed_at(50.5), 80.0);
    }

    #[test]
    fn combines_sources() {
        assert_eq!(combine(Mode::Highest, &[40.0, 70.0, 55.0]), Some(70.0));
        assert_eq!(combine(Mode::Lowest, &[40.0, 70.0]), Some(40.0));
        assert_eq!(combine(Mode::Average, &[40.0, 60.0]), Some(50.0));
        assert_eq!(combine(Mode::First, &[3.0, 1.0]), Some(3.0));
        assert_eq!(combine(Mode::Sum, &[3.0, 1.0]), Some(4.0));
        assert_eq!(combine(Mode::Highest, &[]), None);
    }

    #[test]
    fn limits_speeds() {
        let fan = FanControl {
            min: 30.0,
            max: 90.0,
            ..Default::default()
        };
        assert_eq!(fan.limit(10.0), 30.0);
        assert_eq!(fan.limit(95.0), 90.0);
        let stopping = FanControl {
            allow_stop: true,
            ..fan
        };
        assert_eq!(stopping.limit(10.0), 0.0);
        assert_eq!(stopping.limit(50.0), 50.0);
    }

    #[test]
    fn validates_settings() {
        let mut settings = FanSettings {
            curves: vec![curve(&[[80.0, 120.0], [40.0, 20.0]])],
            ..Default::default()
        };
        settings.fans.insert(
            "it87.2624.pwm1".into(),
            FanControl {
                mode: FanMode::Curve,
                curve: "c".into(),
                min: 50.0,
                max: 10.0,
                ..Default::default()
            },
        );
        settings
            .fans
            .insert("it87.2624.pwm2".into(), FanControl::default());
        let clean = settings.clone().validated().unwrap();
        // Sorted, clamped, and fans left to the firmware forgotten.
        assert_eq!(clean.curves[0].points, vec![[40.0, 20.0], [80.0, 100.0]]);
        assert_eq!(clean.fans.len(), 1);
        assert_eq!(clean.fans["it87.2624.pwm1"].max, 50.0);

        let mut dangling = settings.clone();
        dangling.fans.get_mut("it87.2624.pwm1").unwrap().curve = "gone".into();
        assert!(dangling.validated().is_err());

        let mut fast = settings.clone();
        fast.interval_ms = 20_000;
        assert!(fast.validated().is_err());

        let mut empty = settings;
        empty.curves[0].points.clear();
        assert!(empty.validated().is_err());
    }

    #[test]
    fn settings_survive_toml() {
        let mut settings = FanSettings {
            curves: vec![Curve {
                sources: vec!["cpu.temperature".into(), "gpu.0.temperature".into()],
                virtual_id: Some("virtual.2".into()),
                ..curve(&[[40.0, 20.0], [80.0, 100.0]])
            }],
            ..Default::default()
        };
        settings.fans.insert(
            "nvidia.GPU-1234.fan0".into(),
            FanControl {
                mode: FanMode::Fixed,
                percent: 45.0,
                ..Default::default()
            },
        );
        let text = toml::to_string_pretty(&settings).unwrap();
        assert_eq!(toml::from_str::<FanSettings>(&text).unwrap(), settings);
        // Hand-written files may leave out what isn't changed.
        let partial: FanSettings =
            toml::from_str("[fans.\"x.pwm1\"]\nmode = \"fixed\"\npercent = 60.0\n").unwrap();
        assert_eq!(partial.interval_ms, DEFAULT_INTERVAL_MS);
        assert_eq!(partial.fans["x.pwm1"].max, 100.0);
    }
}
