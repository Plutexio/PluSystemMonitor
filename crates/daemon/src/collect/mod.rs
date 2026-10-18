mod cpu;
mod mem;

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use plusysmon_protocol::Sample;

pub struct Collector {
    cpu: cpu::CpuCollector,
}

impl Collector {
    pub fn new() -> Result<Self> {
        Ok(Self {
            cpu: cpu::CpuCollector::new()?,
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
}
