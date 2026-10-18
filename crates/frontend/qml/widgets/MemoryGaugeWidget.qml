import QtQuick
import PluSystemMonitor

Card {
    id: root

    required property Monitor monitor
    property var options: ({})

    readonly property color tint: Theme.accent(options.accent)

    title: "Memory"
    accent: tint

    RingGauge {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: bars.top
        anchors.bottomMargin: 12
        value: root.monitor.memTotal > 0 ? root.monitor.memUsed / root.monitor.memTotal * 100 : 0
        color: root.tint
        label: "in use"
    }

    Column {
        id: bars

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        spacing: 14

        UsageBar {
            width: parent.width
            label: "RAM"
            used: root.monitor.memUsed
            total: root.monitor.memTotal
            color: root.tint
        }
        UsageBar {
            width: parent.width
            visible: root.options.showSwap
            label: "Swap"
            used: root.monitor.swapUsed
            total: root.monitor.swapTotal
            color: Theme.swap
        }
    }
}
