//! The `Monitor` QML element owns the daemon connection and exposes its data as
//! properties; `ProcessModel` presents its process table as a list model.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qlist.h");
        type QList_f64 = cxx_qt_lib::QList<f64>;
        include!("cxx-qt-lib/qhash.h");
        type QHash_i32_QByteArray = cxx_qt_lib::QHash<cxx_qt_lib::QHashPair_i32_QByteArray>;
        include!("cxx-qt-lib/qvariant.h");
        type QVariant = cxx_qt_lib::QVariant;
        include!("cxx-qt-lib/qmodelindex.h");
        type QModelIndex = cxx_qt_lib::QModelIndex;
        include!("cxx-qt-lib/qvector.h");
        type QVector_i32 = cxx_qt_lib::QVector<i32>;
    }

    unsafe extern "C++Qt" {
        include!(<QtCore/QAbstractListModel>);
        #[qobject]
        type QAbstractListModel;
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, connected)]
        #[qproperty(QString, status)]
        #[qproperty(QString, hostname)]
        #[qproperty(i32, interval_ms, cxx_name = "intervalMs")]
        #[qproperty(i32, uid)]
        #[qproperty(f64, cpu_total, cxx_name = "cpuTotal")]
        #[qproperty(QList_f64, cpu_cores, cxx_name = "cpuCores")]
        #[qproperty(QList_f64, cpu_history, cxx_name = "cpuHistory")]
        #[qproperty(f64, mem_total, cxx_name = "memTotal")]
        #[qproperty(f64, mem_used, cxx_name = "memUsed")]
        #[qproperty(f64, swap_total, cxx_name = "swapTotal")]
        #[qproperty(f64, swap_used, cxx_name = "swapUsed")]
        #[qproperty(QList_f64, mem_history, cxx_name = "memHistory")]
        /// Set while something shows processes; the daemon only sends them then.
        #[qproperty(bool, processes_enabled, cxx_name = "processesEnabled")]
        #[qproperty(i32, process_count, cxx_name = "processCount")]
        type Monitor = super::MonitorRust;

        /// An empty token for a local daemon is read from the daemon's token file.
        #[qinvokable]
        fn connect_to(self: Pin<&mut Self>, address: &QString, token: &QString);

        #[qinvokable]
        fn disconnect_from(self: Pin<&mut Self>);

        /// `signal` is one of terminate, kill, stop, continue.
        #[qinvokable]
        fn send_signal(self: Pin<&mut Self>, pid: i32, signal: &QString);

        /// JSON with everything known about the process, or "" if it is gone.
        #[qinvokable]
        fn process_details(&self, pid: i32) -> QString;

        /// Like `send_signal`, for every process of an app.
        #[qinvokable]
        fn signal_app(self: Pin<&mut Self>, id: &QString, signal: &QString);

        /// JSON describing an app and its processes, or "" if it has none left.
        #[qinvokable]
        fn app_details(&self, id: &QString) -> QString;

        /// Emitted after the properties were updated with a fresh sample.
        #[qsignal]
        fn sample_received(self: Pin<&mut Self>);

        #[qsignal]
        fn processes_updated(self: Pin<&mut Self>);

        #[qsignal]
        fn action_failed(self: Pin<&mut Self>, message: &QString);

        /// An icon or artwork finished downloading.
        #[qsignal]
        fn assets_changed(self: Pin<&mut Self>);
    }

    impl cxx_qt::Threading for Monitor {}
    impl cxx_qt::Initialize for Monitor {}

    extern "RustQt" {
        #[qobject]
        #[base = QAbstractListModel]
        #[qml_element]
        /// Must outlive the model; in QML both live in the same window.
        #[qproperty(*mut Monitor, monitor)]
        #[qproperty(QString, sort_key, cxx_name = "sortKey")]
        #[qproperty(bool, descending)]
        #[qproperty(QString, filter)]
        #[qproperty(bool, only_mine, cxx_name = "onlyMine")]
        #[qproperty(bool, show_kernel_threads, cxx_name = "showKernelThreads")]
        #[qproperty(i32, count)]
        /// Largest values among the shown rows, for scaling in-cell bars.
        #[qproperty(f64, max_cpu, cxx_name = "maxCpu")]
        #[qproperty(f64, max_memory, cxx_name = "maxMemory")]
        type ProcessModel = super::ProcessModelRust;

        /// Rebuilds the rows from the monitor's table; call on `processesUpdated`.
        #[qinvokable]
        fn refresh(self: Pin<&mut ProcessModel>);

        #[qinvokable]
        #[cxx_name = "pidAt"]
        fn pid_at(self: &ProcessModel, row: i32) -> i32;

        /// -1 when the process isn't shown.
        #[qinvokable]
        #[cxx_name = "rowOf"]
        fn row_of(self: &ProcessModel, pid: i32) -> i32;

        #[qinvokable]
        #[cxx_override]
        fn data(self: &ProcessModel, index: &QModelIndex, role: i32) -> QVariant;

        #[qinvokable]
        #[cxx_override]
        #[cxx_name = "roleNames"]
        fn role_names(self: &ProcessModel) -> QHash_i32_QByteArray;

        #[qinvokable]
        #[cxx_override]
        #[cxx_name = "rowCount"]
        fn row_count(self: &ProcessModel, parent: &QModelIndex) -> i32;
    }

    impl cxx_qt::Initialize for ProcessModel {}

    unsafe extern "RustQt" {
        #[inherit]
        #[qsignal]
        #[cxx_name = "dataChanged"]
        fn data_changed(
            self: Pin<&mut ProcessModel>,
            top_left: &QModelIndex,
            bottom_right: &QModelIndex,
            roles: &QVector_i32,
        );

        #[inherit]
        #[cxx_name = "beginInsertRows"]
        unsafe fn begin_insert_rows(
            self: Pin<&mut ProcessModel>,
            parent: &QModelIndex,
            first: i32,
            last: i32,
        );

        #[inherit]
        #[cxx_name = "endInsertRows"]
        unsafe fn end_insert_rows(self: Pin<&mut ProcessModel>);

        #[inherit]
        #[cxx_name = "beginRemoveRows"]
        unsafe fn begin_remove_rows(
            self: Pin<&mut ProcessModel>,
            parent: &QModelIndex,
            first: i32,
            last: i32,
        );

        #[inherit]
        #[cxx_name = "endRemoveRows"]
        unsafe fn end_remove_rows(self: Pin<&mut ProcessModel>);

        #[inherit]
        fn index(self: &ProcessModel, row: i32, column: i32, parent: &QModelIndex) -> QModelIndex;
    }

    extern "RustQt" {
        #[qobject]
        #[base = QAbstractListModel]
        #[qml_element]
        /// Must outlive the model; in QML both live in the same window.
        #[qproperty(*mut Monitor, monitor)]
        /// name, cpu or memory.
        #[qproperty(QString, sort_key, cxx_name = "sortKey")]
        #[qproperty(QString, filter)]
        #[qproperty(i32, count)]
        /// Processes that belong to no app (services, shells, ...).
        #[qproperty(i32, background_count, cxx_name = "backgroundCount")]
        type AppModel = super::AppModelRust;

        /// Rebuilds the rows; call on `processesUpdated` and `assetsChanged`.
        #[qinvokable]
        fn refresh(self: Pin<&mut AppModel>);

        #[qinvokable]
        #[cxx_name = "idAt"]
        fn id_at(self: &AppModel, row: i32) -> QString;

        #[qinvokable]
        #[cxx_override]
        fn data(self: &AppModel, index: &QModelIndex, role: i32) -> QVariant;

        #[qinvokable]
        #[cxx_override]
        #[cxx_name = "roleNames"]
        fn role_names(self: &AppModel) -> QHash_i32_QByteArray;

        #[qinvokable]
        #[cxx_override]
        #[cxx_name = "rowCount"]
        fn row_count(self: &AppModel, parent: &QModelIndex) -> i32;
    }

    impl cxx_qt::Initialize for AppModel {}

    unsafe extern "RustQt" {
        #[inherit]
        #[qsignal]
        #[cxx_name = "dataChanged"]
        fn data_changed(
            self: Pin<&mut AppModel>,
            top_left: &QModelIndex,
            bottom_right: &QModelIndex,
            roles: &QVector_i32,
        );

        #[inherit]
        #[cxx_name = "beginInsertRows"]
        unsafe fn begin_insert_rows(
            self: Pin<&mut AppModel>,
            parent: &QModelIndex,
            first: i32,
            last: i32,
        );

        #[inherit]
        #[cxx_name = "endInsertRows"]
        unsafe fn end_insert_rows(self: Pin<&mut AppModel>);

        #[inherit]
        #[cxx_name = "beginRemoveRows"]
        unsafe fn begin_remove_rows(
            self: Pin<&mut AppModel>,
            parent: &QModelIndex,
            first: i32,
            last: i32,
        );

        #[inherit]
        #[cxx_name = "endRemoveRows"]
        unsafe fn end_remove_rows(self: Pin<&mut AppModel>);

        #[inherit]
        fn index(self: &AppModel, row: i32, column: i32, parent: &QModelIndex) -> QModelIndex;
    }
}

pub use crate::app_model::AppModelRust;

use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Arc;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{
    QByteArray, QHash, QHashPair_i32_QByteArray, QList, QModelIndex, QString, QVariant, QVector,
};
use plusysmon_protocol::{AppKind, Sample, Signal};
use serde_json::json;
use tokio::sync::Notify;
use tokio::sync::mpsc::{self, UnboundedSender};

use crate::apps::{self, History};
use crate::assets::AssetCache;
use crate::client::{self, Command, Event};
use crate::processes::{self, Process, ProcessTable, Query};

/// Points kept for the graphs.
const HISTORY_POINTS: usize = 120;

pub struct MonitorRust {
    connected: bool,
    status: QString,
    hostname: QString,
    interval_ms: i32,
    uid: i32,
    cpu_total: f64,
    cpu_cores: QList<f64>,
    cpu_history: QList<f64>,
    mem_total: f64,
    mem_used: f64,
    swap_total: f64,
    swap_used: f64,
    mem_history: QList<f64>,
    processes_enabled: bool,
    process_count: i32,

    /// Bumped per connection so events queued by a previous one are ignored.
    generation: u64,
    stop: Option<Arc<Notify>>,
    commands: Option<UnboundedSender<Command>>,
    cpu_points: VecDeque<f64>,
    mem_points: VecDeque<f64>,
    processes: ProcessTable,
    app_history: History,
    assets: AssetCache,
}

impl Default for MonitorRust {
    fn default() -> Self {
        Self {
            connected: false,
            status: QString::from("Not connected"),
            hostname: QString::default(),
            interval_ms: 1000,
            uid: -1,
            cpu_total: 0.0,
            cpu_cores: QList::default(),
            cpu_history: QList::default(),
            mem_total: 0.0,
            mem_used: 0.0,
            swap_total: 0.0,
            swap_used: 0.0,
            mem_history: QList::default(),
            processes_enabled: false,
            process_count: 0,
            generation: 0,
            stop: None,
            commands: None,
            cpu_points: VecDeque::with_capacity(HISTORY_POINTS),
            mem_points: VecDeque::with_capacity(HISTORY_POINTS),
            processes: ProcessTable::default(),
            app_history: History::default(),
            assets: AssetCache::default(),
        }
    }
}

impl Drop for MonitorRust {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            stop.notify_one();
        }
    }
}

impl cxx_qt::Initialize for qobject::Monitor {
    fn initialize(self: Pin<&mut Self>) {
        self.on_processes_enabled_changed(|monitor| {
            let processes = *monitor.processes_enabled();
            monitor.command(Command::Subscribe { processes });
        })
        .release();
    }
}

impl qobject::Monitor {
    pub fn connect_to(mut self: Pin<&mut Self>, address: &QString, token: &QString) {
        self.as_mut().disconnect_from();

        let address = address.to_string().trim().to_owned();
        let mut token = token.to_string().trim().to_owned();
        if token.is_empty() && client::is_local(&address) {
            token = plusysmon_protocol::token_path()
                .and_then(|path| std::fs::read_to_string(path).ok())
                .map(|t| t.trim().to_owned())
                .unwrap_or_default();
        }

        let stop = Arc::new(Notify::new());
        let (commands, command_rx) = mpsc::unbounded_channel();
        if *self.processes_enabled() {
            let _ = commands.send(Command::Subscribe { processes: true });
        }
        let generation = {
            let mut rust = self.as_mut().rust_mut();
            rust.stop = Some(stop.clone());
            rust.commands = Some(commands);
            rust.generation
        };
        let qt_thread = self.qt_thread();
        client::spawn(address, token, stop, command_rx, move |event| {
            qt_thread
                .queue(move |mut monitor| {
                    if monitor.rust().generation == generation {
                        monitor.as_mut().apply(event);
                    }
                })
                .is_ok()
        });
    }

    pub fn disconnect_from(mut self: Pin<&mut Self>) {
        {
            let mut rust = self.as_mut().rust_mut();
            if let Some(stop) = rust.stop.take() {
                stop.notify_one();
            }
            rust.commands = None;
            rust.generation += 1;
        }
        self.as_mut().clear_processes();
        self.as_mut().set_connected(false);
        self.set_status(QString::from("Not connected"));
    }

    pub fn send_signal(mut self: Pin<&mut Self>, pid: i32, signal: &QString) {
        let Ok(pid) = u32::try_from(pid) else { return };
        let Some(signal) = self.as_mut().signal_checked(signal) else {
            return;
        };
        self.command(Command::Signal { pid, signal });
    }

    pub fn signal_app(mut self: Pin<&mut Self>, id: &QString, signal: &QString) {
        let Some(signal) = self.as_mut().signal_checked(signal) else {
            return;
        };
        let id = id.to_string();
        let pids: Vec<u32> = self
            .rust()
            .processes
            .iter()
            .filter(|p| p.info.app.as_deref() == Some(id.as_str()))
            .map(|p| p.info.pid)
            .collect();
        for pid in pids {
            self.command(Command::Signal { pid, signal });
        }
    }

    /// Parses a signal name, reporting why nothing can be sent.
    fn signal_checked(self: Pin<&mut Self>, signal: &QString) -> Option<Signal> {
        let signal = match signal.to_string().as_str() {
            "terminate" => Signal::Terminate,
            "kill" => Signal::Kill,
            "stop" => Signal::Stop,
            "continue" => Signal::Continue,
            other => {
                let message = format!("Unknown signal {other:?}");
                self.action_failed(&QString::from(&message));
                return None;
            }
        };
        if !*self.connected() {
            self.action_failed(&QString::from("Not connected to a daemon"));
            return None;
        }
        Some(signal)
    }

    pub fn app_details(&self, id: &QString) -> QString {
        let id = id.to_string();
        let table = &self.rust().processes;
        let (apps, _) = apps::group(table);
        let Some(app) = apps.into_iter().find(|app| app.info.id == id) else {
            return QString::default();
        };
        let mut processes: Vec<&Process> =
            app.pids.iter().filter_map(|&pid| table.get(pid)).collect();
        processes.sort_by(|a, b| {
            b.stats
                .cpu
                .total_cmp(&a.stats.cpu)
                .then(a.info.pid.cmp(&b.info.pid))
        });
        let details = json!({
            "id": app.info.id,
            "name": app.info.name,
            "game": app.info.kind == AppKind::Game,
            "icon": self.asset_url(app.info.icon.as_deref()),
            "art": self.asset_url(app.info.art.as_deref()),
            "logo": self.asset_url(app.info.logo.as_deref()),
            "cpu": app.cpu,
            "memory": app.memory,
            "readRate": app.read_rate,
            "writeRate": app.write_rate,
            "processes": processes.iter().map(|p| json!({
                "pid": p.info.pid,
                "name": p.info.name,
                "cpu": p.stats.cpu,
                "memory": p.stats.memory,
            })).collect::<Vec<_>>(),
        });
        QString::from(&details.to_string())
    }

    /// file:// URL of a downloaded asset, or "" (not fetched yet, or none).
    pub(crate) fn asset_url(&self, key: Option<&str>) -> String {
        key.and_then(|key| self.rust().assets.url(key))
            .unwrap_or_default()
            .to_owned()
    }

    pub(crate) fn processes(&self) -> &ProcessTable {
        &self.rust().processes
    }

    pub(crate) fn app_history(&self, id: &str) -> Vec<f64> {
        self.rust().app_history.get(id)
    }

    pub fn process_details(&self, pid: i32) -> QString {
        let table = &self.rust().processes;
        let Some(Process { info, stats }) = u32::try_from(pid).ok().and_then(|pid| table.get(pid))
        else {
            return QString::default();
        };
        let parent = table.get(info.ppid).map(|p| p.info.name.as_str());
        let details = json!({
            "info": info,
            "stats": stats,
            "parentName": parent,
        });
        QString::from(&details.to_string())
    }

    fn command(&self, command: Command) {
        if let Some(commands) = &self.rust().commands {
            // Err only means the connection thread is gone; the UI shows that already.
            let _ = commands.send(command);
        }
    }

    fn clear_processes(mut self: Pin<&mut Self>) {
        self.as_mut().rust_mut().processes.clear();
        self.as_mut().rust_mut().app_history.clear();
        self.as_mut().set_process_count(0);
        self.processes_updated();
    }

    fn apply(mut self: Pin<&mut Self>, event: Event) {
        match event {
            Event::Connecting(url) => {
                self.as_mut()
                    .set_status(QString::from(&format!("Connecting to {url}…")));
            }
            Event::Connected {
                hostname,
                interval_ms,
                uid,
                history,
            } => {
                {
                    let mut rust = self.as_mut().rust_mut();
                    rust.cpu_points.clear();
                    rust.mem_points.clear();
                }
                for sample in &history {
                    self.as_mut().record(sample);
                }
                if let Some(last) = history.last() {
                    self.as_mut().show(last);
                }
                self.as_mut().rust_mut().assets = AssetCache::for_host(&hostname);
                self.as_mut().set_hostname(QString::from(&hostname));
                self.as_mut().set_interval_ms(interval_ms as i32);
                self.as_mut().set_uid(uid as i32);
                self.as_mut().set_status(QString::from("Connected"));
                self.set_connected(true);
            }
            Event::Sample(sample) => {
                self.as_mut().record(&sample);
                self.as_mut().show(&sample);
                self.sample_received();
            }
            Event::Processes(update) => {
                let (count, fetch) = {
                    let mut pinned = self.as_mut().rust_mut();
                    let rust: &mut MonitorRust = &mut pinned;
                    let keys: Vec<String> = update
                        .apps
                        .iter()
                        .flat_map(|app| [&app.icon, &app.art, &app.logo])
                        .flatten()
                        .cloned()
                        .collect();
                    rust.processes.apply(update);
                    let (apps, _) = apps::group(&rust.processes);
                    rust.app_history.record(&apps);
                    let fetch: Vec<String> = keys
                        .into_iter()
                        .filter(|key| rust.assets.should_fetch(key))
                        .collect();
                    (rust.processes.len() as i32, fetch)
                };
                for key in fetch {
                    self.command(Command::Asset { key });
                }
                self.as_mut().set_process_count(count);
                self.processes_updated();
            }
            Event::Asset { key, format, data } => {
                if let Err(e) = self.as_mut().rust_mut().assets.store(&key, &format, &data) {
                    eprintln!("plusysmon: dropping asset: {e:#}");
                }
                self.assets_changed();
            }
            Event::SignalResult {
                pid,
                signal,
                error: Some(error),
            } => {
                let verb = match signal {
                    Signal::Terminate => "end",
                    Signal::Kill => "kill",
                    Signal::Stop => "pause",
                    Signal::Continue => "resume",
                };
                let name = self.rust().processes.get(pid).map_or_else(
                    || format!("process {pid}"),
                    |p| format!("{} ({pid})", p.info.name),
                );
                let message = format!("Could not {verb} {name}: {error}");
                self.action_failed(&QString::from(&message));
            }
            // Success shows up in the next process update.
            Event::SignalResult { error: None, .. } => {}
            Event::Lost(reason) => {
                self.as_mut().clear_processes();
                self.as_mut().set_connected(false);
                self.set_status(QString::from(&format!(
                    "Connection lost ({reason}), retrying…"
                )));
            }
            Event::Rejected(reason) => {
                self.as_mut().clear_processes();
                self.as_mut().set_connected(false);
                self.set_status(QString::from(&format!("Rejected by daemon: {reason}")));
            }
        }
    }

    /// Appends a sample to the graph buffers.
    fn record(self: Pin<&mut Self>, sample: &Sample) {
        let mut pinned = self.rust_mut();
        // A plain &mut lets us borrow both buffers at once.
        let rust: &mut MonitorRust = &mut pinned;
        let mem_percent = if sample.mem.total > 0 {
            sample.mem.used as f64 / sample.mem.total as f64 * 100.0
        } else {
            0.0
        };
        for (points, value) in [
            (&mut rust.cpu_points, sample.cpu.total as f64),
            (&mut rust.mem_points, mem_percent),
        ] {
            if points.len() == HISTORY_POINTS {
                points.pop_front();
            }
            points.push_back(value);
        }
    }

    /// Publishes the latest sample and graph buffers to QML.
    fn show(mut self: Pin<&mut Self>, sample: &Sample) {
        let cpu_history: QList<f64> = self.rust().cpu_points.iter().collect();
        let mem_history: QList<f64> = self.rust().mem_points.iter().collect();
        let cores: Vec<f64> = sample.cpu.per_core.iter().map(|&c| c as f64).collect();

        self.as_mut().set_cpu_total(sample.cpu.total as f64);
        self.as_mut().set_cpu_cores(QList::from(cores));
        self.as_mut().set_cpu_history(cpu_history);
        self.as_mut().set_mem_total(sample.mem.total as f64);
        self.as_mut().set_mem_used(sample.mem.used as f64);
        self.as_mut().set_swap_total(sample.mem.swap_total as f64);
        self.as_mut().set_swap_used(sample.mem.swap_used as f64);
        self.set_mem_history(mem_history);
    }
}

// Roles exposed to QML delegates, starting at Qt::UserRole.
const ROLES: &[&str] = &[
    "pid",
    "name",
    "command",
    "user",
    "cpu",
    "memory",
    "readRate",
    "writeRate",
    "threads",
    "state",
    "nice",
    "mine",
];
const USER_ROLE: i32 = 0x0100;

/// Rows are positional slots: a refresh rewrites their contents in place and
/// only adds or removes slots at the end. Views keep their scroll position and
/// delegates, even though sorting by a live value reorders processes constantly.
pub struct ProcessModelRust {
    monitor: *mut qobject::Monitor,
    sort_key: QString,
    descending: bool,
    filter: QString,
    only_mine: bool,
    show_kernel_threads: bool,
    count: i32,
    max_cpu: f64,
    max_memory: f64,
    rows: Vec<Process>,
}

impl Default for ProcessModelRust {
    fn default() -> Self {
        Self {
            monitor: std::ptr::null_mut(),
            sort_key: QString::from("cpu"),
            descending: true,
            filter: QString::default(),
            only_mine: false,
            show_kernel_threads: false,
            count: 0,
            max_cpu: 0.0,
            max_memory: 0.0,
            rows: Vec::new(),
        }
    }
}

impl cxx_qt::Initialize for qobject::ProcessModel {
    fn initialize(mut self: Pin<&mut Self>) {
        self.as_mut()
            .on_monitor_changed(|model| model.refresh())
            .release();
        self.as_mut()
            .on_sort_key_changed(|model| model.refresh())
            .release();
        self.as_mut()
            .on_descending_changed(|model| model.refresh())
            .release();
        self.as_mut()
            .on_filter_changed(|model| model.refresh())
            .release();
        self.as_mut()
            .on_only_mine_changed(|model| model.refresh())
            .release();
        self.on_show_kernel_threads_changed(|model| model.refresh())
            .release();
    }
}

impl qobject::ProcessModel {
    pub fn refresh(mut self: Pin<&mut Self>) {
        let rows = {
            // SAFETY: QML keeps the monitor alive for as long as this model exists.
            let monitor = unsafe { self.monitor().as_ref() };
            match monitor {
                Some(monitor) => {
                    let sort = self.sort_key().to_string();
                    let filter = self.filter().to_string();
                    let uid = u32::try_from(*monitor.uid()).ok();
                    let query = Query {
                        sort: &sort,
                        descending: *self.descending(),
                        filter: &filter,
                        uid: uid.filter(|_| *self.only_mine()),
                        kernel_threads: *self.show_kernel_threads(),
                    };
                    processes::select(&monitor.rust().processes, &query)
                }
                None => Vec::new(),
            }
        };

        let max_cpu = rows.iter().map(|p| p.stats.cpu as f64).fold(0.0, f64::max);
        let max_memory = rows
            .iter()
            .map(|p| p.stats.memory as f64)
            .fold(0.0, f64::max);
        let old = self.rows.len() as i32;
        let new = rows.len() as i32;
        let root = QModelIndex::default();
        // SAFETY: each begin is paired with its end, with the row change in between.
        unsafe {
            if new > old {
                self.as_mut().begin_insert_rows(&root, old, new - 1);
                self.as_mut().rust_mut().rows = rows;
                self.as_mut().end_insert_rows();
            } else if new < old {
                self.as_mut().begin_remove_rows(&root, new, old - 1);
                self.as_mut().rust_mut().rows = rows;
                self.as_mut().end_remove_rows();
            } else {
                self.as_mut().rust_mut().rows = rows;
            }
        }
        let kept = old.min(new);
        if kept > 0 {
            let first = self.index(0, 0, &root);
            let last = self.index(kept - 1, 0, &root);
            self.as_mut()
                .data_changed(&first, &last, &QVector::default());
        }
        self.as_mut().set_max_cpu(max_cpu);
        self.as_mut().set_max_memory(max_memory);
        self.set_count(new);
    }

    pub fn pid_at(&self, row: i32) -> i32 {
        usize::try_from(row)
            .ok()
            .and_then(|row| self.rows.get(row))
            .map_or(-1, |p| p.info.pid as i32)
    }

    pub fn row_of(&self, pid: i32) -> i32 {
        self.rows
            .iter()
            .position(|p| p.info.pid as i32 == pid)
            .map_or(-1, |row| row as i32)
    }

    pub fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let Some(process) = usize::try_from(index.row())
            .ok()
            .and_then(|row| self.rows.get(row))
        else {
            return QVariant::default();
        };
        let Process { info, stats } = process;
        let rate = |r: Option<u64>| r.map_or(-1.0, |r| r as f64);
        match ROLES.get((role - USER_ROLE) as usize).copied() {
            Some("pid") => QVariant::from(&(info.pid as i32)),
            Some("name") => QVariant::from(&QString::from(&info.name)),
            Some("command") => QVariant::from(&QString::from(&info.command)),
            Some("user") => QVariant::from(&QString::from(&info.user)),
            Some("cpu") => QVariant::from(&(stats.cpu as f64)),
            Some("memory") => QVariant::from(&(stats.memory as f64)),
            Some("readRate") => QVariant::from(&rate(stats.read_rate)),
            Some("writeRate") => QVariant::from(&rate(stats.write_rate)),
            Some("threads") => QVariant::from(&(stats.threads as i32)),
            Some("state") => QVariant::from(&QString::from(&stats.state.to_string())),
            Some("nice") => QVariant::from(&stats.nice),
            Some("mine") => {
                // SAFETY: see `refresh`.
                let uid = unsafe { self.monitor().as_ref() }.map(|m| *m.uid());
                QVariant::from(&(uid == Some(info.uid as i32)))
            }
            _ => QVariant::default(),
        }
    }

    pub fn role_names(&self) -> QHash<QHashPair_i32_QByteArray> {
        let mut roles = QHash::<QHashPair_i32_QByteArray>::default();
        for (i, name) in ROLES.iter().enumerate() {
            roles.insert(USER_ROLE + i as i32, QByteArray::from(*name));
        }
        roles
    }

    pub fn row_count(&self, _parent: &QModelIndex) -> i32 {
        self.rows.len() as i32
    }
}
