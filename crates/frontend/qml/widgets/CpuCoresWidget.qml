import QtQuick
import PluSystemMonitor

Card {
    id: root

    required property Monitor monitor
    property var options: ({})

    readonly property color tint: Theme.accent(options.accent)

    title: "Cores"
    subtitle: monitor.cpuCores.length > 0 ? monitor.cpuCores.length + " threads" : ""
    accent: tint

    CoreBars {
        anchors.fill: parent
        values: root.monitor.cpuCores
        color: root.tint
    }
}
