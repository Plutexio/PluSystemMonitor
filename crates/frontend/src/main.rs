mod app;
mod app_model;
mod assets;
mod config;
mod fleet;
mod layout;
mod monitor;
mod prefs;
mod theme;

// Shared with the daemon's widget service.
use plusysmon_client::{apps, client, metrics, paths, processes, tls, virtual_sensors};

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

use app::App;

fn main() {
    let which = App::from_args(&std::env::args().collect::<Vec<_>>());
    which.set_current();
    let mut app = QGuiApplication::new();
    if let Some(app) = app.as_mut() {
        app.set_application_name(&QString::from(which.name()));
    }
    QGuiApplication::set_desktop_file_name(&QString::from(which.desktop_file()));

    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from(which.root_qml()));
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
