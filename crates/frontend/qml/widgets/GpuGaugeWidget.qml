import QtQuick
import PluSystemMonitor

Card {
    id: root

    required property Monitor monitor
    property var options: ({})

    readonly property var gpu: JSON.parse(monitor.gpus)[0] ?? null
    readonly property color tint: Theme.accent(options.accent)

    title: "Graphics"
    subtitle: gpu ? Format.gpuName(gpu.name) : ""
    accent: tint

    Text {
        anchors.centerIn: parent
        visible: root.gpu === null
        width: parent.width
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.Wrap
        text: root.monitor.connected ? "No supported GPU found" : "Waiting for data…"
        color: Theme.textDim
        font.pixelSize: 13
    }

    RingGauge {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: details.top
        anchors.bottomMargin: 10
        visible: root.gpu !== null
        value: root.gpu ? root.gpu.usage : 0
        color: root.tint
        label: "usage"
    }

    Column {
        id: details

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        visible: root.gpu !== null
        spacing: 12

        UsageBar {
            width: parent.width
            label: "VRAM"
            used: root.gpu ? root.gpu.memoryUsed : 0
            total: root.gpu ? root.gpu.memoryTotal : 0
            color: root.tint
        }

        Row {
            width: parent.width
            // The gauge needs the room more when the card is short.
            visible: root.height > 330
            spacing: 8

            StatChip {
                width: (parent.width - parent.spacing) / 2
                visible: root.gpu !== null && root.gpu.temperature !== null
                label: "Temp"
                value: root.gpu && root.gpu.temperature !== null ? Format.temperature(root.gpu.temperature) : ""
                valueColor: root.gpu && root.gpu.temperature >= 80 ? Theme.warn : Theme.text
            }
            StatChip {
                width: (parent.width - parent.spacing) / 2
                visible: root.gpu !== null && root.gpu.power !== null
                label: "Power"
                value: root.gpu && root.gpu.power !== null ? Format.watts(root.gpu.power) : ""
            }
        }
    }
}
