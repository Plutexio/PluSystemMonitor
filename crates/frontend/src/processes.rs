//! The frontend's copy of the daemon's process table, and the sorted, filtered
//! view of it that the process list shows.

use std::cmp::Ordering;
use std::collections::HashMap;

use plusysmon_protocol::{AppInfo, ProcessInfo, ProcessStats, ProcessUpdate};

#[derive(Debug, Clone)]
pub struct Process {
    pub info: ProcessInfo,
    pub stats: ProcessStats,
}

#[derive(Default)]
pub struct ProcessTable {
    processes: HashMap<u32, Process>,
    apps: HashMap<String, AppInfo>,
}

impl ProcessTable {
    pub fn apply(&mut self, update: ProcessUpdate) {
        if update.full {
            self.processes.clear();
            self.apps.clear();
        }
        for app in update.apps {
            self.apps.insert(app.id.clone(), app);
        }
        for pid in update.removed {
            self.processes.remove(&pid);
        }
        for info in update.added {
            // Real stats follow in `update.stats`; this only fills the slot.
            let stats = ProcessStats {
                pid: info.pid,
                cpu: 0.0,
                memory: 0,
                read_rate: None,
                write_rate: None,
                threads: 0,
                state: '?',
                nice: 0,
                gpu: None,
                gpu_memory: None,
            };
            self.processes.insert(info.pid, Process { info, stats });
        }
        for stats in update.stats {
            if let Some(process) = self.processes.get_mut(&stats.pid) {
                process.stats = stats;
            }
        }
    }

    pub fn clear(&mut self) {
        self.processes.clear();
        self.apps.clear();
    }

    pub fn len(&self) -> usize {
        self.processes.len()
    }

    pub fn get(&self, pid: u32) -> Option<&Process> {
        self.processes.get(&pid)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Process> {
        self.processes.values()
    }

    pub fn app(&self, id: &str) -> Option<&AppInfo> {
        self.apps.get(id)
    }
}

pub struct Query<'a> {
    /// One of name, pid, user, cpu, memory, read, write, threads, state, nice,
    /// gpu, gpuMemory.
    pub sort: &'a str,
    pub descending: bool,
    /// Matched case-insensitively against name, command and user, or as a PID prefix.
    pub filter: &'a str,
    /// Only processes of this user.
    pub uid: Option<u32>,
    pub kernel_threads: bool,
}

pub fn select(table: &ProcessTable, query: &Query) -> Vec<Process> {
    let filter = query.filter.trim().to_lowercase();
    let mut rows: Vec<Process> = table
        .processes
        .values()
        .filter(|p| query.kernel_threads || !p.info.kernel_thread)
        .filter(|p| query.uid.is_none_or(|uid| p.info.uid == uid))
        .filter(|p| filter.is_empty() || matches(p, &filter))
        .cloned()
        .collect();
    rows.sort_by(|a, b| {
        let by_key = compare(a, b, query.sort);
        let by_key = if query.descending {
            by_key.reverse()
        } else {
            by_key
        };
        // PID as tie-breaker keeps equal rows from swapping places every update.
        by_key.then(a.info.pid.cmp(&b.info.pid))
    });
    rows
}

fn matches(process: &Process, filter: &str) -> bool {
    process.info.pid.to_string().starts_with(filter)
        || process.info.name.to_lowercase().contains(filter)
        || process.info.user.to_lowercase().contains(filter)
        || process.info.command.to_lowercase().contains(filter)
}

fn compare(a: &Process, b: &Process, key: &str) -> Ordering {
    let rate = |r: Option<u64>| r.map_or(-1, |r| r as i64);
    match key {
        "name" => a.info.name.to_lowercase().cmp(&b.info.name.to_lowercase()),
        "user" => a.info.user.cmp(&b.info.user),
        "cpu" => a.stats.cpu.total_cmp(&b.stats.cpu),
        "memory" => a.stats.memory.cmp(&b.stats.memory),
        "read" => rate(a.stats.read_rate).cmp(&rate(b.stats.read_rate)),
        "write" => rate(a.stats.write_rate).cmp(&rate(b.stats.write_rate)),
        "threads" => a.stats.threads.cmp(&b.stats.threads),
        "state" => a.stats.state.cmp(&b.stats.state),
        "nice" => a.stats.nice.cmp(&b.stats.nice),
        "gpu" => a
            .stats
            .gpu
            .unwrap_or(-1.0)
            .total_cmp(&b.stats.gpu.unwrap_or(-1.0)),
        "gpuMemory" => a.stats.gpu_memory.cmp(&b.stats.gpu_memory),
        _ => Ordering::Equal,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process(pid: u32, name: &str, uid: u32, cpu: f32) -> (ProcessInfo, ProcessStats) {
        let info = ProcessInfo {
            pid,
            ppid: 1,
            name: name.to_owned(),
            command: format!("/usr/bin/{name} --flag"),
            uid,
            user: if uid == 0 { "root" } else { "me" }.to_owned(),
            started_ms: 0,
            kernel_thread: name.starts_with("kworker"),
            app: None,
        };
        let stats = ProcessStats {
            pid,
            cpu,
            memory: pid as u64 * 1000,
            read_rate: None,
            write_rate: None,
            threads: 1,
            state: 'S',
            nice: 0,
            gpu: None,
            gpu_memory: None,
        };
        (info, stats)
    }

    fn table() -> ProcessTable {
        let all = vec![
            process(10, "firefox", 1000, 12.0),
            process(11, "Bash", 1000, 0.0),
            process(12, "sshd", 0, 0.5),
            process(13, "kworker/0:1", 0, 0.0),
            process(14, "zsh", 1000, 0.0),
        ];
        let (added, stats) = all.into_iter().unzip();
        let mut table = ProcessTable::default();
        table.apply(ProcessUpdate {
            full: true,
            removed: vec![],
            added,
            stats,
            apps: vec![],
        });
        table
    }

    fn pids(rows: &[Process]) -> Vec<u32> {
        rows.iter().map(|p| p.info.pid).collect()
    }

    fn query(sort: &str, descending: bool) -> Query<'_> {
        Query {
            sort,
            descending,
            filter: "",
            uid: None,
            kernel_threads: false,
        }
    }

    #[test]
    fn sorts_with_stable_ties() {
        let table = table();
        assert_eq!(pids(&select(&table, &query("cpu", true))), [10, 12, 11, 14]);
        assert_eq!(
            pids(&select(&table, &query("name", false))),
            [11, 10, 12, 14]
        );
    }

    #[test]
    fn filters() {
        let table = table();
        let mut q = query("pid", false);
        q.kernel_threads = true;
        assert_eq!(select(&table, &q).len(), 5);

        q.filter = "BASH";
        assert_eq!(pids(&select(&table, &q)), [11]);
        q.filter = "--flag";
        assert_eq!(select(&table, &q).len(), 5);
        q.filter = "1";
        assert_eq!(select(&table, &q).len(), 5);

        q.filter = "";
        q.uid = Some(1000);
        assert_eq!(pids(&select(&table, &q)), [10, 11, 14]);
    }

    #[test]
    fn applies_deltas() {
        let mut table = table();
        let (info, stats) = process(20, "new", 1000, 1.0);
        table.apply(ProcessUpdate {
            full: false,
            removed: vec![10],
            added: vec![info],
            stats: vec![stats],
            apps: vec![],
        });
        assert!(table.get(10).is_none());
        assert_eq!(table.get(20).unwrap().stats.cpu, 1.0);
        assert_eq!(table.len(), 5);
    }
}
