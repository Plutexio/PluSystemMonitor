//! The fans: finding them, setting them, and putting them back.

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// A fan this machine lets us drive.
#[derive(Debug, Clone, PartialEq)]
pub struct Output {
    /// `<hwmon device>.pwm<N>` or `nvidia.<GPU UUID>.fan<N>`.
    pub id: String,
    /// The hwmon driver (`it8689`, `amdgpu`, ...) or the card's name.
    pub driver: String,
    /// The sensor ID with this fan's RPM, when the chip has one.
    pub rpm_sensor: Option<String>,
    /// The lowest percent the hardware takes.
    pub min_percent: u32,
    pub target: Target,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    Hwmon {
        pwm: PathBuf,
        /// `pwmN_enable`; chips without one can't be put back to automatic,
        /// only to the duty they had.
        enable: Option<PathBuf>,
        max: u32,
    },
    #[cfg_attr(not(feature = "nvidia"), allow(dead_code))]
    Nvidia { uuid: String, fan: u32 },
}

/// What a fan was doing before we took it, to give it back that way.
#[derive(Debug, Clone, PartialEq)]
pub enum Original {
    Hwmon {
        pwm: PathBuf,
        value: String,
        enable: Option<(PathBuf, String)>,
    },
    Nvidia {
        uuid: String,
        fan: u32,
    },
}

/// `pwmN_enable` value for "the software sets the duty", the same in every
/// hwmon driver that has the file.
const MANUAL: &str = "1";

pub struct Hardware {
    /// `/sys`, or a fake tree in tests.
    sysfs: PathBuf,
    #[cfg(feature = "nvidia")]
    nvml: Option<nvml_wrapper::Nvml>,
    #[cfg(feature = "nvidia")]
    nvml_tried: bool,
}

impl Hardware {
    pub fn new(sysfs: PathBuf) -> Self {
        Self {
            sysfs,
            #[cfg(feature = "nvidia")]
            nvml: None,
            #[cfg(feature = "nvidia")]
            nvml_tried: false,
        }
    }

    pub fn sysfs(&self) -> &Path {
        &self.sysfs
    }

    pub fn scan(&mut self) -> BTreeMap<String, Output> {
        #[cfg_attr(not(feature = "nvidia"), allow(unused_mut))]
        let mut outputs: BTreeMap<String, Output> = self
            .hwmon_outputs()
            .into_iter()
            .map(|o| (o.id.clone(), o))
            .collect();
        #[cfg(feature = "nvidia")]
        for output in self.nvidia_outputs() {
            outputs.insert(output.id.clone(), output);
        }
        outputs
    }

    fn hwmon_outputs(&self) -> Vec<Output> {
        let Ok(chips) = fs::read_dir(self.sysfs.join("class/hwmon")) else {
            return Vec::new();
        };
        let mut outputs = Vec::new();
        for chip in chips.flatten() {
            let dir = chip.path();
            let Some(driver) = read_trimmed(&dir.join("name")) else {
                continue;
            };
            let device_id = plusysmon_protocol::fans::hwmon_device_id(&dir, &driver);
            let Ok(files) = fs::read_dir(&dir) else {
                continue;
            };
            for file in files.flatten() {
                let name = file.file_name();
                let Some(name) = name.to_str() else { continue };
                let Some(number) = name.strip_prefix("pwm") else {
                    continue;
                };
                if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
                    continue;
                }
                let pwm = file.path();
                // Read-only duty: the driver reports it but won't take one.
                let writable =
                    fs::metadata(&pwm).is_ok_and(|m| m.permissions().mode() & 0o200 != 0);
                if !writable {
                    continue;
                }
                let enable = dir.join(format!("pwm{number}_enable"));
                let max = read_trimmed(&dir.join(format!("pwm{number}_max")))
                    .and_then(|m| m.parse().ok())
                    .filter(|&m| m > 0)
                    .unwrap_or(255);
                let rpm = dir.join(format!("fan{number}_input"));
                outputs.push(Output {
                    id: format!("{device_id}.pwm{number}"),
                    driver: driver.clone(),
                    rpm_sensor: rpm.exists().then(|| format!("{device_id}.fan{number}")),
                    min_percent: 0,
                    target: Target::Hwmon {
                        pwm,
                        enable: enable.exists().then_some(enable),
                        max,
                    },
                });
            }
        }
        outputs
    }

    #[cfg(feature = "nvidia")]
    fn nvml(&mut self) -> Option<&nvml_wrapper::Nvml> {
        if !self.nvml_tried {
            self.nvml_tried = true;
            // Only on the real tree: tests must not touch the machine's GPU.
            if self.sysfs == Path::new("/sys") {
                self.nvml = nvml_wrapper::Nvml::init().ok();
            }
        }
        self.nvml.as_ref()
    }

    #[cfg(feature = "nvidia")]
    fn nvidia_outputs(&mut self) -> Vec<Output> {
        let Some(nvml) = self.nvml() else {
            return Vec::new();
        };
        let mut outputs = Vec::new();
        for index in 0..nvml.device_count().unwrap_or(0) {
            let Ok(device) = nvml.device_by_index(index) else {
                continue;
            };
            let Ok(uuid) = device.uuid() else { continue };
            let name = device.name().unwrap_or_else(|_| "NVIDIA GPU".to_owned());
            let min = device.min_max_fan_speed().map_or(0, |(min, _)| min);
            for fan in 0..device.num_fans().unwrap_or(0) {
                outputs.push(Output {
                    id: format!("nvidia.{uuid}.fan{fan}"),
                    driver: name.clone(),
                    rpm_sensor: None,
                    min_percent: min,
                    target: Target::Nvidia {
                        uuid: uuid.clone(),
                        fan,
                    },
                });
            }
        }
        outputs
    }

    /// What the fan runs at now, percent.
    pub fn percent(&mut self, output: &Output) -> Option<f64> {
        match &output.target {
            Target::Hwmon { pwm, max, .. } => {
                let raw: f64 = read_trimmed(pwm)?.parse().ok()?;
                Some((raw / *max as f64 * 100.0).clamp(0.0, 100.0))
            }
            #[cfg(feature = "nvidia")]
            Target::Nvidia { uuid, fan } => {
                let nvml = self.nvml()?;
                let device = nvml.device_by_uuid(uuid.as_str()).ok()?;
                device.fan_speed(*fan).ok().map(f64::from)
            }
            #[cfg(not(feature = "nvidia"))]
            Target::Nvidia { .. } => None,
        }
    }

    /// How to give this fan back later; read before the first change.
    pub fn original(&mut self, output: &Output) -> Result<Original, String> {
        match &output.target {
            Target::Hwmon { pwm, enable, .. } => Ok(Original::Hwmon {
                pwm: pwm.clone(),
                value: read_trimmed(pwm).ok_or("can't read the duty")?,
                enable: match enable {
                    Some(path) => Some((
                        path.clone(),
                        read_trimmed(path).ok_or("can't read the mode")?,
                    )),
                    None => None,
                },
            }),
            Target::Nvidia { uuid, fan } => Ok(Original::Nvidia {
                uuid: uuid.clone(),
                fan: *fan,
            }),
        }
    }

    /// Sets the fan to `percent`. `last` is the raw value written before, if
    /// any: unchanged values aren't written again, unless the firmware took
    /// the fan back in the meantime. Returns the raw value now set.
    pub fn set(&mut self, output: &Output, percent: f64, last: Option<u32>) -> Result<u32, String> {
        let percent = percent.clamp(output.min_percent as f64, 100.0);
        match &output.target {
            Target::Hwmon { pwm, enable, max } => {
                let raw = (percent / 100.0 * *max as f64).round() as u32;
                let mut taken_back = false;
                if let Some(enable) = enable {
                    // BIOS, resume and some drivers put fans back on automatic.
                    if read_trimmed(enable).as_deref() != Some(MANUAL) {
                        write(enable, MANUAL)?;
                        taken_back = true;
                    }
                }
                if taken_back || last != Some(raw) {
                    write(pwm, &raw.to_string())?;
                }
                Ok(raw)
            }
            #[cfg(feature = "nvidia")]
            Target::Nvidia { uuid, fan } => {
                let raw = percent.round() as u32;
                let nvml = self.nvml().ok_or("NVML is not available")?;
                let mut device = nvml
                    .device_by_uuid(uuid.as_str())
                    .map_err(|e| format!("GPU gone: {e}"))?;
                // NVML has no "taken back" to read cheaply; the driver keeps
                // manual speeds until told otherwise, so writing on change does.
                if last != Some(raw) {
                    device
                        .set_fan_speed(*fan, raw)
                        .map_err(|e| format!("setting the fan speed: {e}"))?;
                }
                Ok(raw)
            }
            #[cfg(not(feature = "nvidia"))]
            Target::Nvidia { .. } => Err("built without NVIDIA support".to_owned()),
        }
    }

    /// Hands a fan back as it was. Best effort: tries every step.
    pub fn restore(&mut self, original: &Original) -> Result<(), String> {
        match original {
            Original::Hwmon { pwm, value, enable } => {
                // The duty first, while it's still ours to set; then the mode.
                let duty = write(pwm, value);
                let mode = match enable {
                    Some((path, mode)) => write(path, mode),
                    None => Ok(()),
                };
                duty.and(mode)
            }
            #[cfg(feature = "nvidia")]
            Original::Nvidia { uuid, fan } => {
                let nvml = self.nvml().ok_or("NVML is not available")?;
                let mut device = nvml
                    .device_by_uuid(uuid.as_str())
                    .map_err(|e| format!("GPU gone: {e}"))?;
                device
                    .set_default_fan_speed(*fan)
                    .map_err(|e| format!("restoring the fan: {e}"))
            }
            #[cfg(not(feature = "nvidia"))]
            Original::Nvidia { .. } => Err("built without NVIDIA support".to_owned()),
        }
    }
}

impl Original {
    /// One line of the state file.
    pub fn to_line(&self) -> String {
        match self {
            Original::Hwmon { pwm, value, enable } => {
                let (path, mode) = match enable {
                    Some((path, mode)) => (path.to_string_lossy().into_owned(), mode.clone()),
                    None => ("-".to_owned(), "-".to_owned()),
                };
                format!("hwmon\t{}\t{value}\t{path}\t{mode}", pwm.display())
            }
            Original::Nvidia { uuid, fan } => format!("nvidia\t{uuid}\t{fan}"),
        }
    }

    /// Reads a state file line back, accepting only paths to fan files under
    /// `sysfs`: the file is root's, but it's still no place to take paths from blindly.
    pub fn from_line(line: &str, sysfs: &Path) -> Option<Self> {
        let fields: Vec<&str> = line.split('\t').collect();
        let fan_file = |path: &str, suffix: &str| -> Option<PathBuf> {
            let path = PathBuf::from(path);
            let name = path.file_name()?.to_str()?;
            let number = name.strip_prefix("pwm")?.strip_suffix(suffix)?;
            let plain = !number.is_empty() && number.chars().all(|c| c.is_ascii_digit());
            let inside =
                path.starts_with(sysfs) && !path.components().any(|c| c.as_os_str() == "..");
            (plain && inside).then_some(path)
        };
        let number = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
        match fields.as_slice() {
            ["hwmon", pwm, value, enable, mode] if number(value) => Some(Original::Hwmon {
                pwm: fan_file(pwm, "")?,
                value: value.to_string(),
                enable: match (*enable, *mode) {
                    ("-", "-") => None,
                    (path, mode) if number(mode) => {
                        Some((fan_file(path, "_enable")?, mode.to_string()))
                    }
                    _ => return None,
                },
            }),
            ["nvidia", uuid, fan] => Some(Original::Nvidia {
                uuid: uuid
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-')
                    .then(|| uuid.to_string())?,
                fan: fan.parse().ok()?,
            }),
            _ => None,
        }
    }
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_owned())
}

fn write(path: &Path, value: &str) -> Result<(), String> {
    fs::write(path, value).map_err(|e| format!("writing {}: {e}", path.display()))
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// A fake sysfs with one chip: pwm1 (with mode and RPM), pwm2 (without a
    /// mode file) and a read-only pwm3.
    pub fn fake_sysfs(tag: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("plusysfand-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let chip = root.join("class/hwmon/hwmon3");
        fs::create_dir_all(&chip).unwrap();
        for (file, value) in [
            ("name", "it8689"),
            ("pwm1", "128"),
            ("pwm1_enable", "2"),
            ("fan1_input", "900"),
            ("pwm2", "200"),
            ("pwm3", "50"),
        ] {
            fs::write(chip.join(file), value).unwrap();
        }
        fs::set_permissions(chip.join("pwm3"), fs::Permissions::from_mode(0o444)).unwrap();
        root
    }

    pub fn read(root: &Path, file: &str) -> String {
        fs::read_to_string(root.join("class/hwmon/hwmon3").join(file))
            .unwrap()
            .trim()
            .to_owned()
    }

    #[test]
    fn finds_writable_fans() {
        let root = fake_sysfs("scan");
        let outputs = Hardware::new(root.clone()).scan();
        let ids: Vec<_> = outputs.keys().map(String::as_str).collect();
        assert_eq!(ids, ["it8689.pwm1", "it8689.pwm2"]);
        let fan = &outputs["it8689.pwm1"];
        assert_eq!(fan.rpm_sensor.as_deref(), Some("it8689.fan1"));
        assert_eq!(outputs["it8689.pwm2"].rpm_sensor, None);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sets_and_restores() {
        let root = fake_sysfs("set");
        let mut hw = Hardware::new(root.clone());
        let outputs = hw.scan();
        let fan = &outputs["it8689.pwm1"];
        assert!((hw.percent(fan).unwrap() - 50.2).abs() < 0.1);

        let original = hw.original(fan).unwrap();
        let raw = hw.set(fan, 40.0, None).unwrap();
        assert_eq!(raw, 102);
        assert_eq!(read(&root, "pwm1_enable"), "1");
        assert_eq!(read(&root, "pwm1"), "102");

        // Unchanged: nothing written. Firmware took it back: written again.
        fs::write(root.join("class/hwmon/hwmon3/pwm1"), "77").unwrap();
        hw.set(fan, 40.0, Some(raw)).unwrap();
        assert_eq!(read(&root, "pwm1"), "77");
        fs::write(root.join("class/hwmon/hwmon3/pwm1_enable"), "2").unwrap();
        hw.set(fan, 40.0, Some(raw)).unwrap();
        assert_eq!(read(&root, "pwm1"), "102");
        assert_eq!(read(&root, "pwm1_enable"), "1");

        hw.restore(&original).unwrap();
        assert_eq!(read(&root, "pwm1"), "128");
        assert_eq!(read(&root, "pwm1_enable"), "2");
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn state_lines_round_trip_and_stay_in_sysfs() {
        let sysfs = Path::new("/sys");
        let original = Original::Hwmon {
            pwm: "/sys/class/hwmon/hwmon6/pwm2".into(),
            value: "90".into(),
            enable: Some(("/sys/class/hwmon/hwmon6/pwm2_enable".into(), "2".into())),
        };
        assert_eq!(
            Original::from_line(&original.to_line(), sysfs),
            Some(original)
        );
        let nvidia = Original::Nvidia {
            uuid: "GPU-12ab-34".into(),
            fan: 1,
        };
        assert_eq!(Original::from_line(&nvidia.to_line(), sysfs), Some(nvidia));
        for bad in [
            "hwmon\t/etc/passwd\t1\t-\t-",
            "hwmon\t/sys/class/hwmon/hwmon6/../../../etc/pwm1\t1\t-\t-",
            "hwmon\t/sys/class/hwmon/hwmon6/pwm1\tx\t-\t-",
            "hwmon\t/sys/class/hwmon/hwmon6/pwm1\t1\t/sys/kernel/x\t1",
            "nvidia\tGPU;rm\t0",
        ] {
            assert_eq!(Original::from_line(bad, sysfs), None, "{bad}");
        }
    }
}
