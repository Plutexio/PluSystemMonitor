import QtQuick
import PluSystemMonitor

Card {
    id: root

    required property Monitor monitor
    property var options: ({})

    readonly property color tint: Theme.accent(options.accent)

    title: "Processor history"
    subtitle: monitor.connected ? monitor.cpuTotal.toFixed(1) + "%" : ""
    accent: tint

    LiveGraph {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: cores.visible ? cores.top : parent.bottom
        anchors.bottomMargin: cores.visible ? 18 : 0
        values: root.monitor.cpuHistory
        interval: root.monitor.intervalMs
        color: root.tint
    }

    CoreBars {
        id: cores

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        // Bars only where there is room for a meaningful graph above them.
        visible: root.options.showCores && parent.height > 140
        height: Math.min(44, parent.height * 0.25)
        values: root.monitor.cpuCores
        color: root.tint
    }
}
