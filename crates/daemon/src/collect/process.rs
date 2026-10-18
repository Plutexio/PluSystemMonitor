//! Per-process statistics from /proc.

use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::time::Instant;

use plusysmon_protocol::{ProcessInfo, ProcessStats};

use super::users::Users;

/// Static info is only re-read when a process is new or has exec'd (its comm changed).
struct Tracked {
    comm: String,
    info: ProcessInfo,
    cpu_ticks: u64,
    io: Option<(u64, u64)>,
}

pub struct ProcessCollector {
    tracked: HashMap<u32, Tracked>,
    users: Users,
    page_size: u64,
    ticks_per_second: u64,
    boot_time_ms: u64,
    last_sample: Option<Instant>,
}

impl ProcessCollector {
    pub fn new() -> Self {
        // SAFETY: sysconf has no preconditions.
        let (page_size, ticks) = unsafe {
            (
                libc::sysconf(libc::_SC_PAGESIZE),
                libc::sysconf(libc::_SC_CLK_TCK),
            )
        };
        Self {
            tracked: HashMap::new(),
            users: Users::default(),
            page_size: u64::try_from(page_size).unwrap_or(4096),
            ticks_per_second: u64::try_from(ticks).unwrap_or(100).max(1),
            boot_time_ms: boot_time_ms().unwrap_or(0),
            last_sample: None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.last_sample.is_some()
    }

    /// Forgets all history, so the next sample starts fresh.
    pub fn reset(&mut self) {
        self.tracked.clear();
        self.last_sample = None;
    }

    /// `total_ticks` is the machine-wide CPU time that passed since the previous
    /// call, as measured by the CPU collector, so per-process shares add up to the
    /// total gauge.
    pub fn sample(&mut self, total_ticks: u64) -> Vec<(ProcessInfo, ProcessStats)> {
        let now = Instant::now();
        let elapsed = self
            .last_sample
            .map(|last| (now - last).as_secs_f64())
            .unwrap_or(0.0);
        self.last_sample = Some(now);

        let Ok(entries) = fs::read_dir("/proc") else {
            return Vec::new();
        };
        let mut previous = std::mem::take(&mut self.tracked);
        let mut processes = Vec::with_capacity(previous.len());
        for entry in entries.flatten() {
            let Some(pid) = entry.file_name().to_str().and_then(|n| n.parse().ok()) else {
                continue;
            };
            // A process can exit between listing and reading; just skip it.
            if let Some(process) = self.read(pid, previous.remove(&pid), total_ticks, elapsed) {
                processes.push(process);
            }
        }
        processes
    }

    fn read(
        &mut self,
        pid: u32,
        prev: Option<Tracked>,
        total_ticks: u64,
        elapsed: f64,
    ) -> Option<(ProcessInfo, ProcessStats)> {
        let stat = parse_stat(&fs::read_to_string(format!("/proc/{pid}/stat")).ok()?)?;
        let started_ms = self.boot_time_ms + stat.start_ticks * 1000 / self.ticks_per_second;
        let prev = prev.filter(|p| p.info.started_ms == started_ms && p.comm == stat.comm);

        let info = match &prev {
            Some(prev) => prev.info.clone(),
            None => self.read_info(pid, &stat, started_ms)?,
        };

        let cpu_ticks = stat.utime + stat.stime;
        let cpu = match &prev {
            Some(prev) if total_ticks > 0 => {
                let share = cpu_ticks.saturating_sub(prev.cpu_ticks) as f64 / total_ticks as f64;
                // One decimal is all the UI shows; rounding keeps idle noise out of updates.
                ((share * 1000.0).round() / 10.0).min(100.0) as f32
            }
            _ => 0.0,
        };

        let io = read_io(pid);
        let rate = |now: u64, before: u64| {
            if elapsed > 0.0 {
                (now.saturating_sub(before) as f64 / elapsed) as u64
            } else {
                0
            }
        };
        let (read_rate, write_rate) = match (io, prev.as_ref().and_then(|p| p.io)) {
            (Some((r, w)), Some((pr, pw))) => (Some(rate(r, pr)), Some(rate(w, pw))),
            (Some(_), None) => (Some(0), Some(0)),
            (None, _) => (None, None),
        };

        let stats = ProcessStats {
            pid,
            cpu,
            memory: read_private_memory(pid).unwrap_or(0) * self.page_size,
            read_rate,
            write_rate,
            threads: stat.threads,
            state: stat.state,
            nice: stat.nice,
        };
        self.tracked.insert(
            pid,
            Tracked {
                comm: stat.comm,
                info: info.clone(),
                cpu_ticks,
                io,
            },
        );
        Some((info, stats))
    }

    fn read_info(&mut self, pid: u32, stat: &Stat, started_ms: u64) -> Option<ProcessInfo> {
        let uid = fs::metadata(format!("/proc/{pid}")).ok()?.uid();
        let cmdline = fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
        let args: Vec<String> = cmdline
            .split(|&b| b == 0)
            .filter(|arg| !arg.is_empty())
            .map(|arg| String::from_utf8_lossy(arg).into_owned())
            .collect();
        Some(ProcessInfo {
            pid,
            ppid: stat.ppid,
            name: display_name(&stat.comm, args.first().map(String::as_str)),
            // Zombies also lose their command line, but they aren't kernel threads.
            kernel_thread: args.is_empty() && (pid == 2 || stat.ppid == 2),
            command: args.join(" "),
            uid,
            user: self.users.name(uid),
            started_ms,
        })
    }
}

struct Stat {
    comm: String,
    state: char,
    ppid: u32,
    utime: u64,
    stime: u64,
    nice: i32,
    threads: u32,
    start_ticks: u64,
}

/// `/proc/<pid>/stat`. The name sits in parentheses and may itself contain
/// spaces and parentheses, so fields are counted from the last `)`.
fn parse_stat(stat: &str) -> Option<Stat> {
    let open = stat.find('(')?;
    let close = stat.rfind(')')?;
    let fields: Vec<&str> = stat.get(close + 2..)?.split(' ').collect();
    // Field numbers from proc(5), minus the three before this slice.
    let field = |n: usize| fields.get(n - 3).copied();
    Some(Stat {
        comm: stat[open + 1..close].to_owned(),
        state: field(3)?.chars().next()?,
        ppid: field(4)?.parse().ok()?,
        utime: field(14)?.parse().ok()?,
        stime: field(15)?.parse().ok()?,
        nice: field(19)?.parse().ok()?,
        threads: field(20)?.parse().ok()?,
        start_ticks: field(22)?.parse().ok()?,
    })
}

/// The kernel cuts process names to 15 bytes; prefer the executable's name when
/// it is the untruncated version of it.
fn display_name(comm: &str, argv0: Option<&str>) -> String {
    const COMM_MAX: usize = 15;
    if comm.len() >= COMM_MAX
        && let Some(base) = argv0.and_then(|arg| arg.rsplit('/').next())
        && base.starts_with(comm)
    {
        return base.to_owned();
    }
    comm.to_owned()
}

/// Resident pages not shared with other processes (statm: resident - shared).
fn read_private_memory(pid: u32) -> Option<u64> {
    let statm = fs::read_to_string(format!("/proc/{pid}/statm")).ok()?;
    let mut fields = statm.split_whitespace().skip(1);
    let resident: u64 = fields.next()?.parse().ok()?;
    let shared: u64 = fields.next()?.parse().ok()?;
    Some(resident.saturating_sub(shared))
}

/// Bytes actually read from / written to storage. Other users' processes are
/// unreadable without privileges, which is reported as `None`.
fn read_io(pid: u32) -> Option<(u64, u64)> {
    let io = fs::read_to_string(format!("/proc/{pid}/io")).ok()?;
    let value = |key: &str| {
        io.lines()
            .find_map(|line| line.strip_prefix(key))
            .and_then(|v| v.trim().parse().ok())
    };
    Some((value("read_bytes:")?, value("write_bytes:")?))
}

fn boot_time_ms() -> Option<u64> {
    let stat = fs::read_to_string("/proc/stat").ok()?;
    let secs: u64 = stat
        .lines()
        .find_map(|line| line.strip_prefix("btime "))?
        .trim()
        .parse()
        .ok()?;
    Some(secs * 1000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stat_with_awkward_names() {
        let line = "1234 (Web (Content) x) S 1000 1234 1234 0 -1 4194304 147 0 0 0 \
                    250 50 0 0 20 -5 31 0 987654 9502720 1581 18446744073709551615";
        let stat = parse_stat(line).unwrap();
        assert_eq!(stat.comm, "Web (Content) x");
        assert_eq!(stat.state, 'S');
        assert_eq!(stat.ppid, 1000);
        assert_eq!((stat.utime, stat.stime), (250, 50));
        assert_eq!(stat.nice, -5);
        assert_eq!(stat.threads, 31);
        assert_eq!(stat.start_ticks, 987654);
    }

    #[test]
    fn restores_truncated_names() {
        assert_eq!(
            display_name(
                "gnome-shell-cal",
                Some("/usr/libexec/gnome-shell-calendar-server")
            ),
            "gnome-shell-calendar-server"
        );
        assert_eq!(
            display_name("Web Content", Some("/usr/lib/firefox/firefox")),
            "Web Content"
        );
        assert_eq!(display_name("kworker/0:1-eve", None), "kworker/0:1-eve");
    }

    #[test]
    fn samples_this_process() {
        let mut collector = ProcessCollector::new();
        collector.sample(0);
        let processes = collector.sample(100);
        let me = std::process::id();
        let (info, stats) = processes.iter().find(|(info, _)| info.pid == me).unwrap();
        assert!(!info.kernel_thread);
        assert!(info.command.contains("plusysmond"));
        assert!(stats.memory > 0);
        assert!(stats.threads >= 1);
        // Our own I/O counters are always readable.
        assert!(stats.read_rate.is_some());
    }
}
