use std::fs;

use anyhow::{Context, Result};
use plusysmon_protocol::CpuSample;

#[derive(Clone, Copy, Default)]
struct Times {
    busy: u64,
    total: u64,
}

/// Computes utilisation from the deltas between successive `/proc/stat` reads.
pub struct CpuCollector {
    prev_total: Times,
    prev_cores: Vec<Times>,
}

impl CpuCollector {
    pub fn new() -> Result<Self> {
        let (prev_total, prev_cores) = read_stat()?;
        Ok(Self {
            prev_total,
            prev_cores,
        })
    }

    pub fn sample(&mut self) -> Result<CpuSample> {
        let (total, cores) = read_stat()?;
        let sample = CpuSample {
            total: usage(self.prev_total, total),
            per_core: cores
                .iter()
                .enumerate()
                .map(|(i, &now)| usage(self.prev_cores.get(i).copied().unwrap_or_default(), now))
                .collect(),
        };
        self.prev_total = total;
        self.prev_cores = cores;
        Ok(sample)
    }
}

fn usage(prev: Times, now: Times) -> f32 {
    let total = now.total.saturating_sub(prev.total);
    if total == 0 {
        return 0.0;
    }
    let busy = now.busy.saturating_sub(prev.busy);
    (busy as f64 / total as f64 * 100.0).clamp(0.0, 100.0) as f32
}

fn read_stat() -> Result<(Times, Vec<Times>)> {
    let stat = fs::read_to_string("/proc/stat").context("reading /proc/stat")?;
    let mut total = Times::default();
    let mut cores = Vec::new();
    for line in stat.lines() {
        let Some(rest) = line.strip_prefix("cpu") else {
            // The cpu lines come first; nothing of interest follows them.
            break;
        };
        // "cpu  <times>" is the aggregate, "cpuN <times>" a single core.
        match rest.strip_prefix(' ') {
            Some(times) => total = parse_times(times),
            None => cores.push(parse_times(rest.split_once(' ').map_or("", |(_, t)| t))),
        }
    }
    Ok((total, cores))
}

/// Parses `user nice system idle iowait irq softirq steal ...`.
/// guest/guest_nice are already accounted for in user/nice, so they are skipped.
fn parse_times(times: &str) -> Times {
    let fields: Vec<u64> = times
        .split_whitespace()
        .take(8)
        .map(|f| f.parse().unwrap_or(0))
        .collect();
    let total: u64 = fields.iter().sum();
    let idle = fields.get(3).copied().unwrap_or(0) + fields.get(4).copied().unwrap_or(0);
    Times {
        busy: total - idle,
        total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stat_line() {
        let t = parse_times("  100 0 50 800 50 0 0 0 0 0");
        assert_eq!(t.total, 1000);
        assert_eq!(t.busy, 150);
    }

    #[test]
    fn usage_from_delta() {
        let prev = Times {
            busy: 100,
            total: 1000,
        };
        let now = Times {
            busy: 150,
            total: 1100,
        };
        assert_eq!(usage(prev, now), 50.0);
        assert_eq!(usage(now, now), 0.0);
    }
}
