use cxx_qt_build::{CxxQtBuilder, QmlFile, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("PluSystemMonitor")
            .qml_file(QmlFile::from("qml/Theme.qml").singleton(true))
            .qml_files([
                "qml/Main.qml",
                "qml/Background.qml",
                "qml/Card.qml",
                "qml/RingGauge.qml",
                "qml/LiveGraph.qml",
                "qml/UsageBar.qml",
                "qml/ConnectionBar.qml",
            ]),
    )
    .qt_module("Quick")
    .files(["src/monitor.rs"])
    .build();
}
