import QtQuick
import PluSystemMonitor

// One source as a big number, with its recent history underneath.
Card {
    id: root

    required property Monitor monitor
    property var options: ({})

    readonly property var source: {
        monitor.revision
        return Sources.read(monitor, (options.sources ?? []).slice(0, 1))[0] ?? null
    }
    readonly property color tint: Sources.colors(source ? [source] : [], options.accent)[0] ?? Theme.cpu

    title: options.title || (source ? source.name : "Value")
    accent: tint

    LiveGraph {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        height: parent.height * 0.55
        visible: root.source !== null && root.source.available
        values: root.source ? root.source.history : []
        // The number says how much; the sparkline only needs to show the shape.
        maxValue: Sources.scale(root.source, true)
        interval: root.monitor.intervalMs
        color: root.tint
        showGrid: false
        fillOpacity: 0.6
    }

    Text {
        anchors.left: parent.left
        anchors.bottom: parent.bottom
        visible: !root.source || !root.source.available
        text: Sources.absence(root.source, root.monitor)
        color: Theme.textDim
        font.pixelSize: 12
    }

    Text {
        anchors.left: parent.left
        anchors.top: parent.top
        text: root.source && root.source.available ? root.source.text : "—"
        color: Theme.text
        font.pixelSize: Math.max(22, Math.min(parent.height * 0.42, 52))
        font.weight: Font.Bold
        font.features: { "tnum": 1 }
    }
}
