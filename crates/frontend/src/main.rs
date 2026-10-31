mod app_model;
mod apps;
mod assets;
mod client;
mod config;
mod layout;
mod metrics;
mod monitor;
mod paths;
mod prefs;
mod processes;
mod theme;
mod tls;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

/// Name of the installed `.desktop` file, so Wayland compositors show its icon.
const DESKTOP_FILE: &str = "plusysmon";

fn main() {
    let mut app = QGuiApplication::new();
    if let Some(app) = app.as_mut() {
        app.set_application_name(&QString::from("PluSystemMonitor"));
    }
    QGuiApplication::set_desktop_file_name(&QString::from(DESKTOP_FILE));

    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/PluSystemMonitor/qml/Main.qml"));
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
