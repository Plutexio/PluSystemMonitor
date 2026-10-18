//! `AppModel`: the monitor's processes grouped into apps, as a QML list model.
//! Declared in the `monitor` bridge (it needs the `Monitor` type).

use std::pin::Pin;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{
    QByteArray, QHash, QHashPair_i32_QByteArray, QList, QModelIndex, QString, QVariant, QVector,
};
use plusysmon_protocol::AppKind;

use crate::apps::{self, App};
use crate::monitor::qobject;

const ROLES: &[&str] = &[
    "appId",
    "name",
    "game",
    "icon",
    "art",
    "logo",
    "cpu",
    "memory",
    "readRate",
    "writeRate",
    "processCount",
    "history",
];
const USER_ROLE: i32 = 0x0100;

struct Row {
    app: App,
    icon: String,
    art: String,
    logo: String,
    history: Vec<f64>,
}

/// Like `ProcessModel`, rows are positional slots updated in place.
pub struct AppModelRust {
    pub(crate) monitor: *mut qobject::Monitor,
    pub(crate) sort_key: QString,
    pub(crate) filter: QString,
    pub(crate) count: i32,
    pub(crate) background_count: i32,
    rows: Vec<Row>,
}

impl Default for AppModelRust {
    fn default() -> Self {
        Self {
            monitor: std::ptr::null_mut(),
            sort_key: QString::from("name"),
            filter: QString::default(),
            count: 0,
            background_count: 0,
            rows: Vec::new(),
        }
    }
}

impl cxx_qt::Initialize for qobject::AppModel {
    fn initialize(mut self: Pin<&mut Self>) {
        self.as_mut()
            .on_monitor_changed(|model| model.refresh())
            .release();
        self.as_mut()
            .on_sort_key_changed(|model| model.refresh())
            .release();
        self.on_filter_changed(|model| model.refresh()).release();
    }
}

impl qobject::AppModel {
    pub fn refresh(mut self: Pin<&mut Self>) {
        let (rows, background) = {
            // SAFETY: QML keeps the monitor alive for as long as this model exists.
            match unsafe { self.monitor().as_ref() } {
                Some(monitor) => {
                    let (grouped, background) = apps::group(monitor.processes());
                    let selected = apps::select(
                        grouped,
                        &self.sort_key().to_string(),
                        &self.filter().to_string(),
                    );
                    let rows = selected
                        .into_iter()
                        .map(|app| Row {
                            icon: monitor.asset_url(app.info.icon.as_deref()),
                            art: monitor.asset_url(app.info.art.as_deref()),
                            logo: monitor.asset_url(app.info.logo.as_deref()),
                            history: monitor.app_history(&app.info.id),
                            app,
                        })
                        .collect::<Vec<_>>();
                    (rows, background)
                }
                None => (Vec::new(), 0),
            }
        };

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
        self.as_mut().set_background_count(background as i32);
        self.set_count(new);
    }

    pub fn id_at(&self, row: i32) -> QString {
        usize::try_from(row)
            .ok()
            .and_then(|row| self.rows.get(row))
            .map(|r| QString::from(&r.app.info.id))
            .unwrap_or_default()
    }

    pub fn data(&self, index: &QModelIndex, role: i32) -> QVariant {
        let Some(row) = usize::try_from(index.row())
            .ok()
            .and_then(|row| self.rows.get(row))
        else {
            return QVariant::default();
        };
        let app = &row.app;
        match ROLES.get((role - USER_ROLE) as usize).copied() {
            Some("appId") => QVariant::from(&QString::from(&app.info.id)),
            Some("name") => QVariant::from(&QString::from(&app.info.name)),
            Some("game") => QVariant::from(&(app.info.kind == AppKind::Game)),
            Some("icon") => QVariant::from(&QString::from(&row.icon)),
            Some("art") => QVariant::from(&QString::from(&row.art)),
            Some("logo") => QVariant::from(&QString::from(&row.logo)),
            Some("cpu") => QVariant::from(&app.cpu),
            Some("memory") => QVariant::from(&(app.memory as f64)),
            Some("readRate") => QVariant::from(&(app.read_rate as f64)),
            Some("writeRate") => QVariant::from(&(app.write_rate as f64)),
            Some("processCount") => QVariant::from(&(app.pids.len() as i32)),
            Some("history") => QVariant::from(&QList::from(&row.history)),
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
