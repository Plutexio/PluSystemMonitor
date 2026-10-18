use std::fs;

use anyhow::{Context, Result};
use plusysmon_protocol::MemSample;

pub fn sample() -> Result<MemSample> {
    let meminfo = fs::read_to_string("/proc/meminfo").context("reading /proc/meminfo")?;
    Ok(parse(&meminfo))
}

fn parse(meminfo: &str) -> MemSample {
    let (mut total, mut available, mut swap_total, mut swap_free) = (0, 0, 0, 0);
    for line in meminfo.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let slot = match key {
            "MemTotal" => &mut total,
            "MemAvailable" => &mut available,
            "SwapTotal" => &mut swap_total,
            "SwapFree" => &mut swap_free,
            _ => continue,
        };
        // Values are reported in KiB.
        *slot = value
            .trim()
            .trim_end_matches(" kB")
            .parse::<u64>()
            .unwrap_or(0)
            * 1024;
    }
    MemSample {
        total,
        used: total.saturating_sub(available),
        available,
        swap_total,
        swap_used: swap_total.saturating_sub(swap_free),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_meminfo() {
        let m = parse(
            "MemTotal:       16000 kB\nMemFree:  1000 kB\nMemAvailable:   6000 kB\n\
             SwapTotal:       4000 kB\nSwapFree:        3000 kB\n",
        );
        assert_eq!(m.total, 16000 * 1024);
        assert_eq!(m.used, 10000 * 1024);
        assert_eq!(m.swap_used, 1000 * 1024);
    }
}
