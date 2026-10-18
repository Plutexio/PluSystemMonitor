use cxx_qt_build::{CxxQtBuilder, QmlFile, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(
        QmlModule::new("PluSystemMonitor")
            .qml_file(QmlFile::from("qml/Theme.qml").singleton(true))
            .qml_file(QmlFile::from("qml/Widgets.qml").singleton(true))
            .qml_files([
                "qml/Main.qml",
                "qml/Background.qml",
                "qml/Header.qml",
                "qml/Dashboard.qml",
                "qml/WidgetFrame.qml",
                "qml/Card.qml",
                "qml/RingGauge.qml",
                "qml/LiveGraph.qml",
                "qml/UsageBar.qml",
                "qml/CoreBars.qml",
                "qml/Icon.qml",
                "qml/GlassButton.qml",
                "qml/GlassField.qml",
                "qml/GlassPopup.qml",
                "qml/ConnectionPopup.qml",
                "qml/ThemePopup.qml",
                "qml/WidgetPalette.qml",
                "qml/WidgetSettings.qml",
                "qml/Toast.qml",
                "qml/widgets/CpuGaugeWidget.qml",
                "qml/widgets/CpuGraphWidget.qml",
                "qml/widgets/CpuCoresWidget.qml",
                "qml/widgets/MemoryGaugeWidget.qml",
                "qml/widgets/MemoryGraphWidget.qml",
            ]),
    )
    .qt_module("Quick")
    .files(["src/monitor.rs", "src/config.rs"])
    .build();
}
