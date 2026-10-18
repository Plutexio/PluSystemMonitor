import QtQuick
import PluSystemMonitor

Card {
    id: root

    required property Monitor monitor
    property var options: ({})

    readonly property color tint: Theme.accent(options.accent)

    title: "Memory history"
    subtitle: monitor.connected ? Theme.formatBytes(monitor.memUsed) : ""
    accent: tint

    LiveGraph {
        anchors.fill: parent
        values: root.monitor.memHistory
        interval: root.monitor.intervalMs
        color: root.tint
    }
}
