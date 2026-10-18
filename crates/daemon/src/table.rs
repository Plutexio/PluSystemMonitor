//! The process table as clients know it, and the deltas that keep them in sync.

use std::collections::HashMap;

use plusysmon_protocol::{AppInfo, ProcessInfo, ProcessStats, ProcessUpdate};

#[derive(Default)]
pub struct ProcessTable {
    processes: HashMap<u32, (ProcessInfo, ProcessStats)>,
    apps: HashMap<String, AppInfo>,
}

impl ProcessTable {
    /// Replaces the table with `current` and returns what changed. `new_apps`
    /// describes apps that `current` references for the first time.
    pub fn update(
        &mut self,
        current: Vec<(ProcessInfo, ProcessStats)>,
        new_apps: Vec<AppInfo>,
    ) -> ProcessUpdate {
        let mut update = ProcessUpdate::default();
        for app in &new_apps {
            self.apps.insert(app.id.clone(), app.clone());
        }
        update.apps = new_apps;
        let mut next = HashMap::with_capacity(current.len());
        for (info, stats) in current {
            match self.processes.remove(&info.pid) {
                Some((old_info, old_stats)) if old_info == info => {
                    if old_stats != stats {
                        update.stats.push(stats.clone());
                    }
                }
                // Same PID but a different process (reused PID or exec): replace it.
                Some(_) => {
                    update.removed.push(info.pid);
                    update.added.push(info.clone());
                    update.stats.push(stats.clone());
                }
                None => {
                    update.added.push(info.clone());
                    update.stats.push(stats.clone());
                }
            }
            next.insert(info.pid, (info, stats));
        }
        update.removed.extend(self.processes.keys());
        self.processes = next;
        update
    }

    /// Everything, for a client that just subscribed.
    pub fn snapshot(&self) -> ProcessUpdate {
        let (added, stats) = self.processes.values().cloned().unzip();
        ProcessUpdate {
            full: true,
            removed: Vec::new(),
            added,
            stats,
            apps: self.apps.values().cloned().collect(),
        }
    }

    pub fn clear(&mut self) {
        self.processes.clear();
        self.apps.clear();
    }
}

pub fn is_empty(update: &ProcessUpdate) -> bool {
    !update.full
        && update.removed.is_empty()
        && update.added.is_empty()
        && update.stats.is_empty()
        && update.apps.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process(pid: u32, name: &str, cpu: f32) -> (ProcessInfo, ProcessStats) {
        let info = ProcessInfo {
            pid,
            ppid: 1,
            name: name.to_owned(),
            command: String::new(),
            uid: 1000,
            user: "me".to_owned(),
            started_ms: 1,
            kernel_thread: false,
            app: None,
        };
        let stats = ProcessStats {
            pid,
            cpu,
            memory: 0,
            read_rate: None,
            write_rate: None,
            threads: 1,
            state: 'S',
            nice: 0,
        };
        (info, stats)
    }

    #[test]
    fn sends_only_differences() {
        let mut table = ProcessTable::default();
        let first = table.update(
            vec![process(10, "a", 0.0), process(11, "b", 0.0)],
            Vec::new(),
        );
        assert_eq!(first.added.len(), 2);
        assert_eq!(first.stats.len(), 2);

        let second = table.update(
            vec![process(10, "a", 0.0), process(11, "b", 5.0)],
            Vec::new(),
        );
        assert!(second.added.is_empty() && second.removed.is_empty());
        assert_eq!(second.stats.len(), 1);
        assert_eq!(second.stats[0].pid, 11);

        let third = table.update(vec![process(11, "b", 5.0)], Vec::new());
        assert_eq!(third.removed, [10]);
        assert!(third.stats.is_empty());

        assert!(is_empty(
            &table.update(vec![process(11, "b", 5.0)], Vec::new())
        ));
    }

    #[test]
    fn replaces_reused_pids() {
        let mut table = ProcessTable::default();
        table.update(vec![process(10, "old", 0.0)], Vec::new());
        let update = table.update(vec![process(10, "new", 0.0)], Vec::new());
        assert_eq!(update.removed, [10]);
        assert_eq!(update.added[0].name, "new");
    }

    #[test]
    fn snapshot_is_full() {
        let mut table = ProcessTable::default();
        table.update(vec![process(10, "a", 1.0)], Vec::new());
        let snapshot = table.snapshot();
        assert!(snapshot.full);
        assert_eq!(snapshot.added.len(), 1);
        assert_eq!(snapshot.stats.len(), 1);
    }
}
