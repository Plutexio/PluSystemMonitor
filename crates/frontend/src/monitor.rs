//! The `Monitor` QML element: owns the daemon connection and exposes its data as properties.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qlist.h");
        type QList_f64 = cxx_qt_lib::QList<f64>;
    }

    #[auto_cxx_name]
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(bool, connected)]
        #[qproperty(QString, status)]
        #[qproperty(QString, hostname)]
        #[qproperty(i32, interval_ms, cxx_name = "intervalMs")]
        #[qproperty(f64, cpu_total, cxx_name = "cpuTotal")]
        #[qproperty(QList_f64, cpu_cores, cxx_name = "cpuCores")]
        #[qproperty(QList_f64, cpu_history, cxx_name = "cpuHistory")]
        #[qproperty(f64, mem_total, cxx_name = "memTotal")]
        #[qproperty(f64, mem_used, cxx_name = "memUsed")]
        #[qproperty(f64, swap_total, cxx_name = "swapTotal")]
        #[qproperty(f64, swap_used, cxx_name = "swapUsed")]
        #[qproperty(QList_f64, mem_history, cxx_name = "memHistory")]
        type Monitor = super::MonitorRust;

        /// An empty token for a local daemon is read from the daemon's token file.
        #[qinvokable]
        fn connect_to(self: Pin<&mut Self>, address: &QString, token: &QString);

        #[qinvokable]
        fn disconnect_from(self: Pin<&mut Self>);

        /// Emitted after the properties were updated with a fresh sample.
        #[qsignal]
        fn sample_received(self: Pin<&mut Self>);
    }

    impl cxx_qt::Threading for Monitor {}
}

use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Arc;

use cxx_qt::{CxxQtType, Threading};
use cxx_qt_lib::{QList, QString};
use plusysmon_protocol::Sample;
use tokio::sync::Notify;

use crate::client::{self, Event};

/// Points kept for the graphs.
const HISTORY_POINTS: usize = 120;

pub struct MonitorRust {
    connected: bool,
    status: QString,
    hostname: QString,
    interval_ms: i32,
    cpu_total: f64,
    cpu_cores: QList<f64>,
    cpu_history: QList<f64>,
    mem_total: f64,
    mem_used: f64,
    swap_total: f64,
    swap_used: f64,
    mem_history: QList<f64>,

    /// Bumped per connection so events queued by a previous one are ignored.
    generation: u64,
    stop: Option<Arc<Notify>>,
    cpu_points: VecDeque<f64>,
    mem_points: VecDeque<f64>,
}

impl Default for MonitorRust {
    fn default() -> Self {
        Self {
            connected: false,
            status: QString::from("Not connected"),
            hostname: QString::default(),
            interval_ms: 1000,
            cpu_total: 0.0,
            cpu_cores: QList::default(),
            cpu_history: QList::default(),
            mem_total: 0.0,
            mem_used: 0.0,
            swap_total: 0.0,
            swap_used: 0.0,
            mem_history: QList::default(),
            generation: 0,
            stop: None,
            cpu_points: VecDeque::with_capacity(HISTORY_POINTS),
            mem_points: VecDeque::with_capacity(HISTORY_POINTS),
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
        let generation = {
            let mut rust = self.as_mut().rust_mut();
            rust.stop = Some(stop.clone());
            rust.generation
        };
        let qt_thread = self.qt_thread();
        client::spawn(address, token, stop, move |event| {
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
            rust.generation += 1;
        }
        self.as_mut().set_connected(false);
        self.set_status(QString::from("Not connected"));
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
                self.as_mut().set_hostname(QString::from(&hostname));
                self.as_mut().set_interval_ms(interval_ms as i32);
                self.as_mut().set_status(QString::from("Connected"));
                self.set_connected(true);
            }
            Event::Sample(sample) => {
                self.as_mut().record(&sample);
                self.as_mut().show(&sample);
                self.sample_received();
            }
            Event::Lost(reason) => {
                self.as_mut().set_connected(false);
                self.set_status(QString::from(&format!(
                    "Connection lost ({reason}), retrying…"
                )));
            }
            Event::Rejected(reason) => {
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
