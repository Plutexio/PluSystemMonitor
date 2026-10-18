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
