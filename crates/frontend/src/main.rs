mod client;
mod monitor;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QString, QUrl};

fn main() {
    let mut app = QGuiApplication::new();
    if let Some(mut app) = app.as_mut() {
        // Used by QtCore.Settings to locate ~/.config/PluSystemMonitor/PluSystemMonitor.conf
        app.as_mut().set_organization_name(&QString::from("PluSystemMonitor"));
        app.as_mut().set_application_name(&QString::from("PluSystemMonitor"));
    }

    let mut engine = QQmlApplicationEngine::new();
    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from("qrc:/qt/qml/PluSystemMonitor/qml/Main.qml"));
    }

    if let Some(app) = app.as_mut() {
        app.exec();
    }
}
