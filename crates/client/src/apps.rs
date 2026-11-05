//! Processes grouped into the applications the daemon identified.

use std::collections::HashMap;

use plusysmon_protocol::AppInfo;

use crate::processes::ProcessTable;

#[derive(Debug, Clone)]
pub struct App {
    pub info: AppInfo,
    pub cpu: f64,
    pub memory: u64,
    pub read_rate: u64,
    pub write_rate: u64,
    pub gpu: f64,
    pub gpu_memory: u64,
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
            gpu: 0.0,
            gpu_memory: 0,
            pids: Vec::new(),
        });
        let stats = &process.stats;
        app.cpu += stats.cpu as f64;
        app.memory += stats.memory;
        app.read_rate += stats.read_rate.unwrap_or(0);
        app.write_rate += stats.write_rate.unwrap_or(0);
        app.gpu = (app.gpu + stats.gpu.unwrap_or(0.0) as f64).min(100.0);
        app.gpu_memory += stats.gpu_memory.unwrap_or(0);
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

/// `sort` is name, cpu, memory or gpu; numbers sort biggest first.
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
            "gpu" => b.gpu.total_cmp(&a.gpu).then_with(by_name),
            _ => by_name(),
        }
        .then_with(|| a.info.id.cmp(&b.info.id))
    });
    apps
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
                gpu: (cpu > 10.0).then_some(cpu * 2.0),
                gpu_memory: (cpu > 10.0).then_some(1 << 30),
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
        assert_eq!(zen.gpu, 0.0);
        let game = apps.iter().find(|a| a.info.id == "steam:42").unwrap();
        assert_eq!(game.gpu, 80.0);
        assert_eq!(game.gpu_memory, 1 << 30);
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
        assert_eq!(
            names(select(apps.clone(), "gpu", "")),
            ["A Game", "Zen Browser"]
        );
        assert_eq!(names(select(apps, "name", "zen")), ["Zen Browser"]);
    }
}
