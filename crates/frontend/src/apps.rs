//! Processes grouped into the applications the daemon identified.

use std::collections::{HashMap, VecDeque};

use plusysmon_protocol::AppInfo;

use crate::processes::ProcessTable;

/// Points kept for each app's CPU sparkline.
pub const HISTORY_POINTS: usize = 60;

#[derive(Debug, Clone)]
pub struct App {
    pub info: AppInfo,
    pub cpu: f64,
    pub memory: u64,
    pub read_rate: u64,
    pub write_rate: u64,
    /// Sorted, so the details list is stable.
    pub pids: Vec<u32>,
}

/// Every app with at least one process, and how many (non-kernel) processes
/// belong to no app.
pub fn group(table: &ProcessTable) -> (Vec<App>, usize) {
    let mut apps: HashMap<&str, App> = HashMap::new();
    let mut background = 0;
    for process in table.iter() {
        let Some(info) = process.info.app.as_deref().and_then(|id| table.app(id)) else {
            if !process.info.kernel_thread {
                background += 1;
            }
            continue;
        };
        let app = apps.entry(&info.id).or_insert_with(|| App {
            info: info.clone(),
            cpu: 0.0,
            memory: 0,
            read_rate: 0,
            write_rate: 0,
            pids: Vec::new(),
        });
        let stats = &process.stats;
        app.cpu += stats.cpu as f64;
        app.memory += stats.memory;
        app.read_rate += stats.read_rate.unwrap_or(0);
        app.write_rate += stats.write_rate.unwrap_or(0);
        app.pids.push(process.info.pid);
    }
    let apps = apps
        .into_values()
        .map(|mut app| {
            app.pids.sort_unstable();
            app
        })
        .collect();
    (apps, background)
}

/// `sort` is name, cpu or memory; numbers sort biggest first.
pub fn select(mut apps: Vec<App>, sort: &str, filter: &str) -> Vec<App> {
    let filter = filter.trim().to_lowercase();
    if !filter.is_empty() {
        apps.retain(|app| app.info.name.to_lowercase().contains(&filter));
    }
    apps.sort_by(|a, b| {
        let by_name = || a.info.name.to_lowercase().cmp(&b.info.name.to_lowercase());
        match sort {
            "cpu" => b.cpu.total_cmp(&a.cpu).then_with(by_name),
            "memory" => b.memory.cmp(&a.memory).then_with(by_name),
            _ => by_name(),
        }
        .then_with(|| a.info.id.cmp(&b.info.id))
    });
    apps
}

/// Recent CPU use of each app, for sparklines.
#[derive(Default)]
pub struct History {
    points: HashMap<String, VecDeque<f64>>,
}

impl History {
    pub fn record(&mut self, apps: &[App]) {
        self.points
            .retain(|id, _| apps.iter().any(|app| &app.info.id == id));
        for app in apps {
            let points = self.points.entry(app.info.id.clone()).or_default();
            if points.len() == HISTORY_POINTS {
                points.pop_front();
            }
            points.push_back(app.cpu);
        }
    }

    pub fn get(&self, id: &str) -> Vec<f64> {
        self.points
            .get(id)
            .map(|p| p.iter().copied().collect())
            .unwrap_or_default()
    }

    pub fn clear(&mut self) {
        self.points.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plusysmon_protocol::{AppKind, ProcessInfo, ProcessStats, ProcessUpdate};

    fn app(id: &str, name: &str) -> AppInfo {
        AppInfo {
            id: id.into(),
            name: name.into(),
            kind: AppKind::App,
            icon: None,
            art: None,
            logo: None,
        }
    }

    fn process(pid: u32, app: Option<&str>, cpu: f32, kernel: bool) -> (ProcessInfo, ProcessStats) {
        (
            ProcessInfo {
                pid,
                ppid: 1,
                name: format!("p{pid}"),
                command: String::new(),
                uid: 1000,
                user: "me".into(),
                started_ms: 0,
                kernel_thread: kernel,
                app: app.map(str::to_owned),
            },
            ProcessStats {
                pid,
                cpu,
                memory: 100,
                read_rate: Some(10),
                write_rate: None,
                threads: 1,
                state: 'S',
                nice: 0,
            },
        )
    }

    fn table() -> ProcessTable {
        let (added, stats) = vec![
            process(1, Some("desktop:zen"), 2.0, false),
            process(2, Some("desktop:zen"), 3.0, false),
            process(3, Some("steam:42"), 40.0, false),
            process(4, None, 1.0, false),
            process(5, None, 0.0, true),
        ]
        .into_iter()
        .unzip();
        let mut table = ProcessTable::default();
        table.apply(ProcessUpdate {
            full: true,
            removed: vec![],
            added,
            stats,
            apps: vec![app("desktop:zen", "Zen Browser"), app("steam:42", "A Game")],
        });
        table
    }

    #[test]
    fn groups_and_sums() {
        let (apps, background) = group(&table());
        assert_eq!(background, 1);
        let zen = apps.iter().find(|a| a.info.id == "desktop:zen").unwrap();
        assert_eq!(zen.cpu, 5.0);
        assert_eq!(zen.memory, 200);
        assert_eq!(zen.read_rate, 20);
        assert_eq!(zen.pids, [1, 2]);
    }

    #[test]
    fn sorts_and_filters() {
        let (apps, _) = group(&table());
        let names = |apps: Vec<App>| apps.into_iter().map(|a| a.info.name).collect::<Vec<_>>();
        assert_eq!(
            names(select(apps.clone(), "name", "")),
            ["A Game", "Zen Browser"]
        );
        assert_eq!(
            names(select(apps.clone(), "cpu", "")),
            ["A Game", "Zen Browser"]
        );
        assert_eq!(names(select(apps, "name", "zen")), ["Zen Browser"]);
    }

    #[test]
    fn keeps_history_for_live_apps() {
        let (apps, _) = group(&table());
        let mut history = History::default();
        history.record(&apps);
        history.record(&apps);
        assert_eq!(history.get("steam:42"), [40.0, 40.0]);
        history.record(&apps[..0]);
        assert!(history.get("steam:42").is_empty());
    }
}
