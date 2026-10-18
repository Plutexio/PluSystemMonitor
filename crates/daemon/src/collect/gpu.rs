//! Graphics cards. NVIDIA through NVML, which is loaded at runtime so the daemon
//! still runs on machines without the driver; AMD through amdgpu's sysfs files.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use nvml_wrapper::Nvml;
use nvml_wrapper::enum_wrappers::device::{Clock, TemperatureSensor};
use nvml_wrapper::enums::device::UsedGpuMemory;
use plusysmon_protocol::GpuSample;
use tracing::info;

/// What one process uses, summed over all GPUs.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct GpuUse {
    pub usage: f32,
    pub memory: u64,
}

pub struct GpuCollector {
    nvml: Option<Nvml>,
    /// Per NVIDIA device: timestamp (µs) of the newest process sample seen.
    nvidia_seen: HashMap<u32, u64>,
    /// `/sys/class/drm/cardN/device` of each amdgpu card, with its name.
    amd: Vec<(PathBuf, String)>,
}

impl GpuCollector {
    pub fn new() -> Self {
        let nvml = Nvml::init().ok();
        let amd = find_amd_cards();
        if let Some(nvml) = &nvml {
            info!("NVIDIA GPUs: {}", nvml.device_count().unwrap_or(0));
        }
        if !amd.is_empty() {
            info!("AMD GPUs: {}", amd.len());
        }
        Self {
            nvml,
            nvidia_seen: HashMap::new(),
            amd,
        }
    }

    pub fn sample(&self) -> Vec<GpuSample> {
        let mut gpus = self.nvidia_sample();
        gpus.extend(
            self.amd
                .iter()
                .filter_map(|(dir, name)| amd_sample(dir, name)),
        );
        gpus
    }

    /// Per-process GPU use. NVIDIA only: amdgpu reports it per DRM file, which
    /// needs a different approach.
    pub fn processes(&mut self) -> HashMap<u32, GpuUse> {
        let mut usage: HashMap<u32, GpuUse> = HashMap::new();
        let Some(nvml) = &self.nvml else {
            return usage;
        };
        for index in 0..nvml.device_count().unwrap_or(0) {
            let Ok(device) = nvml.device_by_index(index) else {
                continue;
            };
            // Engine time: average the samples taken since the last call.
            let seen = self.nvidia_seen.get(&index).copied();
            let mut samples: HashMap<u32, (u32, u32)> = HashMap::new();
            for sample in device.process_utilization_stats(seen).unwrap_or_default() {
                let (sum, count) = samples.entry(sample.pid).or_default();
                *sum += sample.sm_util;
                *count += 1;
                let newest = self.nvidia_seen.entry(index).or_default();
                *newest = (*newest).max(sample.timestamp);
            }
            for (pid, (sum, count)) in samples {
                usage.entry(pid).or_default().usage += sum as f32 / count as f32;
            }
            // Memory: graphics and compute contexts are listed separately.
            let processes = device
                .running_graphics_processes()
                .unwrap_or_default()
                .into_iter()
                .chain(device.running_compute_processes().unwrap_or_default());
            let mut memory: HashMap<u32, u64> = HashMap::new();
            for process in processes {
                if let UsedGpuMemory::Used(bytes) = process.used_gpu_memory {
                    // A process with both contexts is listed twice with the same total.
                    let entry = memory.entry(process.pid).or_default();
                    *entry = (*entry).max(bytes);
                } else {
                    memory.entry(process.pid).or_default();
                }
            }
            for (pid, bytes) in memory {
                usage.entry(pid).or_default().memory += bytes;
            }
        }
        for gpu in usage.values_mut() {
            gpu.usage = gpu.usage.min(100.0);
        }
        usage
    }

    fn nvidia_sample(&self) -> Vec<GpuSample> {
        let Some(nvml) = &self.nvml else {
            return Vec::new();
        };
        (0..nvml.device_count().unwrap_or(0))
            .filter_map(|index| nvml.device_by_index(index).ok())
            .map(|device| {
                let memory = device.memory_info().ok();
                let fans = device.num_fans().unwrap_or(0);
                GpuSample {
                    name: device.name().unwrap_or_else(|_| "NVIDIA GPU".to_owned()),
                    vendor: "NVIDIA".to_owned(),
                    usage: device.utilization_rates().map_or(0.0, |u| u.gpu as f32),
                    memory_used: memory.as_ref().map_or(0, |m| m.used),
                    memory_total: memory.as_ref().map_or(0, |m| m.total),
                    temperature: device
                        .temperature(TemperatureSensor::Gpu)
                        .ok()
                        .map(|t| t as f32),
                    power: device.power_usage().ok().map(|mw| mw as f32 / 1000.0),
                    power_limit: device
                        .enforced_power_limit()
                        .ok()
                        .map(|mw| mw as f32 / 1000.0),
                    core_clock: device.clock_info(Clock::Graphics).ok(),
                    memory_clock: device.clock_info(Clock::Memory).ok(),
                    fan: (fans > 0)
                        .then(|| device.fan_speed(0).ok())
                        .flatten()
                        .map(|f| f as f32),
                    encoder: device
                        .encoder_utilization()
                        .ok()
                        .map(|u| u.utilization as f32),
                    decoder: device
                        .decoder_utilization()
                        .ok()
                        .map(|u| u.utilization as f32),
                }
            })
            .collect()
    }
}

fn find_amd_cards() -> Vec<(PathBuf, String)> {
    let Ok(entries) = fs::read_dir("/sys/class/drm") else {
        return Vec::new();
    };
    let pci_ids = ["/usr/share/hwdata/pci.ids", "/usr/share/misc/pci.ids"]
        .iter()
        .find_map(|p| fs::read_to_string(p).ok())
        .unwrap_or_default();
    let mut cards: Vec<(PathBuf, String)> = entries
        .flatten()
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            // cardN only; cardN-DP-1 and friends are its connectors.
            name.starts_with("card") && !name.contains('-')
        })
        .map(|e| e.path().join("device"))
        .filter(|dir| read_trimmed(&dir.join("vendor")).as_deref() == Some("0x1002"))
        .map(|dir| {
            let device = read_trimmed(&dir.join("device")).unwrap_or_default();
            let name = pci_device_name(&pci_ids, "1002", device.trim_start_matches("0x"))
                .unwrap_or_else(|| "AMD Radeon GPU".to_owned());
            (dir, name)
        })
        .collect();
    cards.sort();
    cards
}

fn amd_sample(dir: &Path, name: &str) -> Option<GpuSample> {
    let number = |file: &str| read_trimmed(&dir.join(file))?.parse::<u64>().ok();
    let hwmon = fs::read_dir(dir.join("hwmon"))
        .ok()?
        .flatten()
        .map(|e| e.path())
        .next();
    let sensor =
        |file: &str| -> Option<f64> { read_trimmed(&hwmon.as_ref()?.join(file))?.parse().ok() };
    let fan = match (sensor("pwm1"), sensor("pwm1_max")) {
        (Some(pwm), max) => Some((pwm / max.unwrap_or(255.0) * 100.0) as f32),
        _ => None,
    };
    Some(GpuSample {
        name: name.to_owned(),
        vendor: "AMD".to_owned(),
        usage: number("gpu_busy_percent")? as f32,
        memory_used: number("mem_info_vram_used").unwrap_or(0),
        memory_total: number("mem_info_vram_total").unwrap_or(0),
        // hwmon units: millidegrees, microwatts, hertz.
        temperature: sensor("temp1_input").map(|t| (t / 1000.0) as f32),
        power: sensor("power1_average")
            .or_else(|| sensor("power1_input"))
            .map(|uw| (uw / 1e6) as f32),
        power_limit: sensor("power1_cap").map(|uw| (uw / 1e6) as f32),
        core_clock: sensor("freq1_input").map(|hz| (hz / 1e6) as u32),
        memory_clock: sensor("freq2_input").map(|hz| (hz / 1e6) as u32),
        fan,
        encoder: None,
        decoder: None,
    })
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_owned())
}

/// Marketing name from the PCI ID database: the bracketed part of
/// `\t7480  Navi 33 [Radeon RX 7600/7600 XT]`, or the whole name.
fn pci_device_name(pci_ids: &str, vendor: &str, device: &str) -> Option<String> {
    let mut in_vendor = false;
    for line in pci_ids.lines() {
        if !line.starts_with('\t') && !line.starts_with('#') && !line.is_empty() {
            in_vendor = line.starts_with(vendor);
        } else if in_vendor
            && !line.starts_with("\t\t")
            && let Some(rest) = line.trim_start_matches('\t').strip_prefix(device)
        {
            let name = rest.trim();
            let bracketed = name.split_once('[').and_then(|(_, b)| b.strip_suffix(']'));
            return Some(bracketed.unwrap_or(name).to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const PCI_IDS: &str = "\
# comment
1000  Broadcom / LSI
\t7480  Wrong vendor
1002  Advanced Micro Devices, Inc. [AMD/ATI]
\t73bf  Navi 21 [Radeon RX 6800/6800 XT / 6900 XT]
\t\t1002 0e3a  Radeon RX 6900 XT
\t7480  Navi 33 [Radeon RX 7600/7600 XT]
\t9999  Plain Name
10de  NVIDIA Corporation
";

    #[test]
    fn looks_up_pci_names() {
        assert_eq!(
            pci_device_name(PCI_IDS, "1002", "7480").as_deref(),
            Some("Radeon RX 7600/7600 XT")
        );
        assert_eq!(
            pci_device_name(PCI_IDS, "1002", "9999").as_deref(),
            Some("Plain Name")
        );
        assert_eq!(pci_device_name(PCI_IDS, "1002", "0e3a"), None);
        assert_eq!(pci_device_name(PCI_IDS, "10de", "7480"), None);
    }

    #[test]
    fn reads_amdgpu_sysfs() {
        let dir = std::env::temp_dir().join(format!("plusysmon-amd-{}", std::process::id()));
        let hwmon = dir.join("hwmon/hwmon3");
        fs::create_dir_all(&hwmon).unwrap();
        for (file, value) in [
            ("gpu_busy_percent", "37\n"),
            ("mem_info_vram_used", "2147483648\n"),
            ("mem_info_vram_total", "8589934592\n"),
        ] {
            fs::write(dir.join(file), value).unwrap();
        }
        for (file, value) in [
            ("temp1_input", "54000"),
            ("power1_average", "123000000"),
            ("power1_cap", "200000000"),
            ("freq1_input", "2450000000"),
            ("pwm1", "102"),
        ] {
            fs::write(hwmon.join(file), value).unwrap();
        }
        let gpu = amd_sample(&dir, "Radeon RX 7600").unwrap();
        assert_eq!(gpu.usage, 37.0);
        assert_eq!(gpu.memory_used, 2 << 30);
        assert_eq!(gpu.temperature, Some(54.0));
        assert_eq!(gpu.power, Some(123.0));
        assert_eq!(gpu.power_limit, Some(200.0));
        assert_eq!(gpu.core_clock, Some(2450));
        assert_eq!(gpu.memory_clock, None);
        assert_eq!(gpu.fan, Some(40.0));
    }

    /// `cargo test -p plusysmond live_gpus -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn live_gpus() {
        let mut gpus = GpuCollector::new();
        println!("{:#?}", gpus.sample());
        let _ = gpus.processes();
        std::thread::sleep(std::time::Duration::from_secs(1));
        let mut users: Vec<_> = gpus.processes().into_iter().collect();
        users.sort_by_key(|(pid, _)| *pid);
        for (pid, gpu) in users {
            let name = fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
            println!(
                "{pid:>8} {:<20} {:>5.1}% {:>6} MiB",
                name.trim(),
                gpu.usage,
                gpu.memory >> 20
            );
        }
    }
}
