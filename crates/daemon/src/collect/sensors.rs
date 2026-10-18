//! Hardware sensors (hwmon), CPU clocks and CPU package power.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use plusysmon_protocol::{SensorKind, SensorReading};
use tracing::info;

/// hwmon devices rarely come and go; look for new ones this often.
const RESCAN_AFTER: Duration = Duration::from_secs(60);

/// GPU drivers' hwmon chips duplicate what the GPU collector reports.
const SKIPPED_DRIVERS: &[&str] = &["amdgpu", "nouveau", "radeon"];

struct Feature {
    input: PathBuf,
    /// `temp1`, `in0`, ...
    name: String,
    label: String,
    kind: SensorKind,
    limit: Option<f64>,
    /// Unused fan headers and voltage inputs read zero forever; a feature only
    /// shows up once it has reported something.
    alive: bool,
}

struct Chip {
    /// Stable across reboots, unlike the hwmonN number.
    device_id: String,
    device: String,
    driver: String,
    features: Vec<Feature>,
}

#[derive(Debug, Default, PartialEq)]
pub struct CpuSensors {
    pub temperature: Option<f32>,
    pub frequency: Option<f32>,
    pub frequency_peak: Option<f32>,
    pub frequency_max: Option<f32>,
    pub power: Option<f32>,
}

pub struct Sensors {
    chips: Vec<Chip>,
    scanned_at: Instant,
    rapl: Option<Rapl>,
    frequency_max: Option<f32>,
}

impl Sensors {
    pub fn new() -> Self {
        let rapl = Rapl::open();
        if rapl.is_none() && Path::new(RAPL_ENERGY).exists() {
            info!("CPU power needs read access to {RAPL_ENERGY} (root only by default)");
        }
        Self {
            chips: scan(),
            scanned_at: Instant::now(),
            rapl,
            frequency_max: max_frequency(),
        }
    }

    pub fn sample(&mut self) -> (Vec<SensorReading>, CpuSensors) {
        if self.scanned_at.elapsed() > RESCAN_AFTER {
            self.rescan();
        }
        let mut readings = Vec::new();
        let mut cpu = CpuSensors::default();
        for chip in &mut self.chips {
            for feature in &mut chip.features {
                let Some(value) = read_value(&feature.input, feature.kind) else {
                    continue;
                };
                feature.alive |= value.abs() >= dead_threshold(feature.kind);
                if !feature.alive {
                    continue;
                }
                if cpu.temperature.is_none() && is_cpu_package(&chip.driver, &feature.label) {
                    cpu.temperature = Some(value as f32);
                }
                readings.push(SensorReading {
                    id: format!("{}.{}", chip.device_id, feature.name),
                    device: chip.device.clone(),
                    driver: chip.driver.clone(),
                    label: feature.label.clone(),
                    kind: feature.kind,
                    value,
                    limit: feature.limit,
                });
            }
        }
        if let Some((average, peak)) = current_frequencies() {
            cpu.frequency = Some(average);
            cpu.frequency_peak = Some(peak);
        }
        cpu.frequency_max = self.frequency_max;
        cpu.power = self.rapl.as_mut().and_then(Rapl::watts);
        (readings, cpu)
    }

    /// Keeps the "alive" state of features that are still there.
    fn rescan(&mut self) {
        let mut fresh = scan();
        for chip in &mut fresh {
            let Some(old) = self.chips.iter().find(|c| c.device_id == chip.device_id) else {
                continue;
            };
            for feature in &mut chip.features {
                feature.alive = old
                    .features
                    .iter()
                    .any(|f| f.name == feature.name && f.alive);
            }
        }
        self.chips = fresh;
        self.scanned_at = Instant::now();
    }
}

fn scan() -> Vec<Chip> {
    let Ok(entries) = fs::read_dir("/sys/class/hwmon") else {
        return Vec::new();
    };
    let mut chips: Vec<Chip> = entries
        .flatten()
        .filter_map(|entry| read_chip(&entry.path()))
        .collect();
    chips.sort_by(|a, b| a.device.cmp(&b.device).then(a.device_id.cmp(&b.device_id)));
    chips
}

fn read_chip(dir: &Path) -> Option<Chip> {
    let driver = read_trimmed(&dir.join("name"))?;
    if SKIPPED_DRIVERS.contains(&driver.as_str()) {
        return None;
    }
    // Most chips sit under a device; virtual ones (acpitz) keep only a name.
    let device_dir = fs::canonicalize(dir.join("device")).ok();
    let device_id = device_dir
        .as_ref()
        .and_then(|d| d.file_name()?.to_str().map(str::to_owned))
        .unwrap_or_else(|| driver.clone());
    let model = device_dir
        .as_ref()
        .and_then(|d| read_trimmed(&d.join("model")));

    let mut features: Vec<Feature> = fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let file = entry.file_name();
            let name = file.to_str()?.strip_suffix("_input")?.to_owned();
            let (kind, number) = parse_feature(&name)?;
            let label = read_trimmed(&dir.join(format!("{name}_label")))
                .map(|l| tidy_label(&driver, &l))
                .unwrap_or_else(|| generic_label(kind, number));
            let limit = ["crit", "max"]
                .iter()
                .find_map(|suffix| read_value(&dir.join(format!("{name}_{suffix}")), kind))
                // 127 °C is an unset 8-bit register; some drives report garbage
                // like 65261 °C. Neither is a limit worth scaling a gauge to.
                .filter(|&limit| kind == SensorKind::Temperature && limit > 0.0 && limit < 127.0);
            Some(Feature {
                input: entry.path(),
                name,
                label,
                kind,
                limit,
                alive: false,
            })
        })
        .collect();
    if features.is_empty() {
        return None;
    }
    features.sort_by_key(|f| {
        let (kind, number) = parse_feature(&f.name).unwrap_or((SensorKind::Temperature, 0));
        (kind as u8, number)
    });
    Some(Chip {
        device: device_name(&driver, model.as_deref()),
        driver,
        device_id,
        features,
    })
}

/// `temp3` → (Temperature, 3).
fn parse_feature(name: &str) -> Option<(SensorKind, u32)> {
    let split = name.find(|c: char| c.is_ascii_digit())?;
    let number = name[split..].parse().ok()?;
    let kind = match &name[..split] {
        "temp" => SensorKind::Temperature,
        "in" => SensorKind::Voltage,
        "fan" => SensorKind::Fan,
        "power" => SensorKind::Power,
        "curr" => SensorKind::Current,
        _ => return None,
    };
    Some((kind, number))
}

/// Raw sysfs value in the sensor's display unit.
fn read_value(path: &Path, kind: SensorKind) -> Option<f64> {
    let raw: f64 = read_trimmed(path)?.parse().ok()?;
    // sysfs units: millidegrees, millivolts, RPM, microwatts, milliamperes.
    Some(match kind {
        SensorKind::Temperature | SensorKind::Voltage | SensorKind::Current => raw / 1000.0,
        SensorKind::Fan => raw,
        SensorKind::Power => raw / 1e6,
    })
}

/// Below this, a fan or voltage input is taken to be unconnected.
fn dead_threshold(kind: SensorKind) -> f64 {
    match kind {
        SensorKind::Fan => 1.0,
        SensorKind::Voltage => 0.05,
        _ => f64::MIN_POSITIVE,
    }
}

fn generic_label(kind: SensorKind, number: u32) -> String {
    let what = match kind {
        SensorKind::Temperature => "Temperature",
        SensorKind::Voltage => "Voltage",
        SensorKind::Fan => "Fan",
        SensorKind::Power => "Power",
        SensorKind::Current => "Current",
    };
    format!("{what} {number}")
}

fn tidy_label(driver: &str, label: &str) -> String {
    match (driver, label) {
        ("coretemp", "Package id 0") => "Package".to_owned(),
        _ => label.to_owned(),
    }
}

fn is_cpu_package(driver: &str, label: &str) -> bool {
    matches!(
        (driver, label),
        ("coretemp", "Package") | ("k10temp" | "zenpower", "Tctl" | "Tdie")
    )
}

/// What a chip belongs to, in words a user recognises.
fn device_name(driver: &str, model: Option<&str>) -> String {
    let board = [
        "it87", "it86", "nct6", "w83", "f71", "asus", "gigabyte", "dell_smm", "thinkpad",
    ];
    match driver {
        "coretemp" | "k10temp" | "zenpower" => "CPU".to_owned(),
        "nvme" => model.map_or_else(|| "NVMe drive".to_owned(), |m| format!("NVMe {m}")),
        "drivetemp" => model.map_or_else(|| "Drive".to_owned(), |m| format!("Drive {m}")),
        "acpitz" => "ACPI".to_owned(),
        "spd5118" | "jc42" => "Memory".to_owned(),
        d if d.starts_with("iwlwifi") || d.starts_with("mt7") => "Wi-Fi".to_owned(),
        d if ["r8169", "igc", "e1000", "atlantic", "ixgbe"]
            .iter()
            .any(|n| d.starts_with(n)) =>
        {
            "Network adapter".to_owned()
        }
        d if board.iter().any(|b| d.starts_with(b)) => "Motherboard".to_owned(),
        d => d.to_owned(),
    }
}

/// Average and highest current core clock in MHz.
fn current_frequencies() -> Option<(f32, f32)> {
    let mut sum = 0.0;
    let mut peak: f32 = 0.0;
    let mut count = 0;
    for entry in fs::read_dir("/sys/devices/system/cpu").ok()?.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("cpu")
            || !name[3..].chars().all(|c| c.is_ascii_digit())
            || name.len() == 3
        {
            continue;
        }
        let Some(khz) = read_trimmed(&entry.path().join("cpufreq/scaling_cur_freq"))
            .and_then(|f| f.parse::<f32>().ok())
        else {
            continue;
        };
        let mhz = khz / 1000.0;
        sum += mhz;
        peak = peak.max(mhz);
        count += 1;
    }
    (count > 0).then(|| (sum / count as f32, peak))
}

fn max_frequency() -> Option<f32> {
    fs::read_dir("/sys/devices/system/cpu")
        .ok()?
        .flatten()
        .filter_map(|e| read_trimmed(&e.path().join("cpufreq/cpuinfo_max_freq")))
        .filter_map(|f| f.parse::<f32>().ok())
        .reduce(f32::max)
        .map(|khz| khz / 1000.0)
}

const RAPL_ENERGY: &str = "/sys/class/powercap/intel-rapl:0/energy_uj";
const RAPL_RANGE: &str = "/sys/class/powercap/intel-rapl:0/max_energy_range_uj";

/// CPU package power from the RAPL energy counter (Intel, and AMD Zen).
struct Rapl {
    range: u64,
    last: Option<(u64, Instant)>,
}

impl Rapl {
    fn open() -> Option<Self> {
        // Unreadable for normal users since the PLATYPUS side channel.
        read_trimmed(Path::new(RAPL_ENERGY))?.parse::<u64>().ok()?;
        let range = read_trimmed(Path::new(RAPL_RANGE))
            .and_then(|r| r.parse().ok())
            .unwrap_or(u64::MAX);
        Some(Self { range, last: None })
    }

    fn watts(&mut self) -> Option<f32> {
        let energy: u64 = read_trimmed(Path::new(RAPL_ENERGY))?.parse().ok()?;
        let now = Instant::now();
        let watts = self.last.map(|(before, at)| {
            // The counter wraps at `range`.
            let used = if energy >= before {
                energy - before
            } else {
                self.range - before + energy
            };
            (used as f64 / 1e6 / (now - at).as_secs_f64()) as f32
        });
        self.last = Some((energy, now));
        watts
    }
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_feature_names() {
        assert_eq!(parse_feature("temp12"), Some((SensorKind::Temperature, 12)));
        assert_eq!(parse_feature("in0"), Some((SensorKind::Voltage, 0)));
        assert_eq!(parse_feature("fan3"), Some((SensorKind::Fan, 3)));
        assert_eq!(parse_feature("pwm1"), None);
        assert_eq!(parse_feature("temp"), None);
    }

    #[test]
    fn names_devices() {
        assert_eq!(device_name("coretemp", None), "CPU");
        assert_eq!(device_name("it8689", None), "Motherboard");
        assert_eq!(
            device_name("nvme", Some("Lexar SSD NM620 2TB")),
            "NVMe Lexar SSD NM620 2TB"
        );
        assert_eq!(device_name("r8169_0_500:00", None), "Network adapter");
        assert_eq!(device_name("something_new", None), "something_new");
    }

    #[test]
    fn reads_a_fake_chip() {
        let root = std::env::temp_dir().join(format!("plusysmon-hwmon-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let dir = root.join("hwmon9");
        fs::create_dir_all(&dir).unwrap();
        for (file, value) in [
            ("name", "coretemp"),
            ("temp1_input", "52000"),
            ("temp1_label", "Package id 0"),
            ("temp1_crit", "100000"),
            ("temp2_input", "47000"),
            ("temp2_max", "127000"),
            ("in0_input", "1164"),
            ("fan2_input", "0"),
        ] {
            fs::write(dir.join(file), value).unwrap();
        }
        let chip = read_chip(&dir).unwrap();
        assert_eq!(chip.device, "CPU");
        assert_eq!(chip.device_id, "coretemp");
        let names: Vec<_> = chip.features.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["temp1", "temp2", "in0", "fan2"]);
        assert_eq!(chip.features[0].label, "Package");
        assert_eq!(chip.features[0].limit, Some(100.0));
        assert_eq!(chip.features[1].label, "Temperature 2");
        assert_eq!(chip.features[1].limit, None);
        assert_eq!(
            read_value(&chip.features[2].input, SensorKind::Voltage),
            Some(1.164)
        );

        let mut sensors = Sensors {
            chips: vec![chip],
            scanned_at: Instant::now(),
            rapl: None,
            frequency_max: None,
        };
        let (readings, cpu) = sensors.sample();
        // The unconnected fan header stays hidden.
        let ids: Vec<_> = readings.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids, ["coretemp.temp1", "coretemp.temp2", "coretemp.in0"]);
        assert_eq!(cpu.temperature, Some(52.0));
    }

    /// `cargo test -p plusysmond live_sensors -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn live_sensors() {
        let mut sensors = Sensors::new();
        let (readings, cpu) = sensors.sample();
        for r in &readings {
            println!(
                "{:<36} {:<24} {:<12} {:<16} {:>9.3} {:?}",
                r.id, r.device, r.driver, r.label, r.value, r.limit
            );
        }
        println!("{cpu:?}");
    }
}
