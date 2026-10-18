import QtQuick
import PluSystemMonitor

Card {
    id: root

    required property Monitor monitor
    property var options: ({})

    readonly property color tint: Theme.accent(options.accent)

    title: "Processor"
    accent: tint

    RingGauge {
        anchors.fill: parent
        value: root.monitor.cpuTotal
        color: root.tint
        label: "usage"
        caption: root.monitor.cpuCores.length + " threads"
    }
}
