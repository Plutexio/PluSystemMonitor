mod apps;
mod cpu;
mod desktop;
mod icons;
mod mem;
mod process;
mod steam;
mod users;

pub use apps::Assets;

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use plusysmon_protocol::{AppInfo, ProcessInfo, ProcessStats, Sample};

pub struct Collector {
    cpu: cpu::CpuCollector,
    processes: process::ProcessCollector,
}

impl Collector {
    pub fn new(assets: Assets) -> Result<Self> {
        Ok(Self {
            cpu: cpu::CpuCollector::new()?,
            processes: process::ProcessCollector::new(assets),
        })
    }

    pub fn sample(&mut self) -> Result<Sample> {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Ok(Sample {
            timestamp_ms,
            cpu: self.cpu.sample()?,
            mem: mem::sample()?,
        })
    }

    /// Call right after [`Self::sample`], whose CPU totals it divides by.
    pub fn sample_processes(&mut self) -> (Vec<(ProcessInfo, ProcessStats)>, Vec<AppInfo>) {
        self.processes.sample(self.cpu.elapsed_ticks())
    }

    pub fn processes_active(&self) -> bool {
        self.processes.is_active()
    }

    pub fn reset_processes(&mut self) {
        self.processes.reset();
    }
}
