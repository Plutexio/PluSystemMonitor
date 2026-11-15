mod apps;
mod cpu;
mod desktop;
mod gpu;
mod icons;
mod mem;
mod process;
mod sensors;
mod steam;
mod users;

pub use apps::Assets;

/// What a hwmon chip belongs to, in words a user recognises.
#[cfg(feature = "fans")]
pub fn device_name(driver: &str) -> String {
    sensors::device_name(driver, None)
}

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use plusysmon_protocol::{AppInfo, ProcessInfo, ProcessStats, Sample};

pub struct Collector {
    cpu: cpu::CpuCollector,
    gpu: gpu::GpuCollector,
    sensors: sensors::Sensors,
    processes: process::ProcessCollector,
}

impl Collector {
    pub fn new(assets: Assets) -> Result<Self> {
        Ok(Self {
            cpu: cpu::CpuCollector::new()?,
            gpu: gpu::GpuCollector::new(),
            sensors: sensors::Sensors::new(),
            processes: process::ProcessCollector::new(assets),
        })
    }

    pub fn sample(&mut self) -> Result<Sample> {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        let mut cpu = self.cpu.sample()?;
        let (sensors, extra) = self.sensors.sample();
        cpu.temperature = extra.temperature;
        cpu.frequency = extra.frequency;
        cpu.frequency_peak = extra.frequency_peak;
        cpu.frequency_max = extra.frequency_max;
        cpu.power = extra.power;
        Ok(Sample {
            timestamp_ms,
            cpu,
            mem: mem::sample()?,
            gpus: self.gpu.sample(),
            sensors,
            fans: Vec::new(),
            uptime: uptime(),
        })
    }

    /// Call right after [`Self::sample`], whose CPU totals it divides by.
    pub fn sample_processes(&mut self) -> (Vec<(ProcessInfo, ProcessStats)>, Vec<AppInfo>) {
        let gpu = self.gpu.processes();
        self.processes.sample(self.cpu.elapsed_ticks(), &gpu)
    }

    pub fn processes_active(&self) -> bool {
        self.processes.is_active()
    }

    pub fn reset_processes(&mut self) {
        self.processes.reset();
    }
}

/// Whole seconds since boot, from /proc/uptime ("12345.67 98765.43").
fn uptime() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/uptime").ok()?;
    parse_uptime(&text)
}

fn parse_uptime(text: &str) -> Option<u64> {
    let seconds: f64 = text.split_whitespace().next()?.parse().ok()?;
    (seconds.is_finite() && seconds >= 0.0).then_some(seconds as u64)
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_uptime() {
        assert_eq!(super::parse_uptime("350735.47 234388.90\n"), Some(350735));
        assert_eq!(super::parse_uptime(""), None);
        assert_eq!(super::parse_uptime("x y"), None);
    }
}
