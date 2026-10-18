import QtQuick
import PluSystemMonitor

Card {
    id: root

    required property Monitor monitor
    property var options: ({})

    readonly property var gpu: JSON.parse(monitor.gpus)[0] ?? null
    readonly property color tint: Theme.accent(options.accent)

    // Only what this GPU reports; drivers differ.
    readonly property var chips: {
        const g = gpu
        if (!g)
            return []
        const chips = [{ label: "VRAM", value: Format.bytes(g.memoryUsed) + " / " + Format.bytes(g.memoryTotal) }]
        if (g.temperature !== null)
            chips.push({ label: "Temp", value: Format.temperature(g.temperature), hot: g.temperature >= 80 })
        if (g.power !== null)
            chips.push({ label: "Power", value: g.powerLimit !== null
                ? Math.round(g.power) + " / " + Format.watts(g.powerLimit) : Format.watts(g.power) })
        if (g.coreClock !== null)
            chips.push({ label: "Core", value: g.coreClock + " MHz" })
        if (g.memoryClock !== null)
            chips.push({ label: "Mem clock", value: g.memoryClock + " MHz" })
        if (g.fan !== null)
            chips.push({ label: "Fan", value: Math.round(g.fan) + "%" })
        if (g.encoder > 0 || g.decoder > 0)
            chips.push({ label: "Video", value: "E " + Math.round(g.encoder) + "% · D " + Math.round(g.decoder) + "%" })
        return chips
    }

    title: "Graphics history"
    subtitle: gpu ? gpu.usage.toFixed(0) + "% · " + Format.gpuName(gpu.name) : ""
    accent: tint

    LiveGraph {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: chipRow.visible ? chipRow.top : parent.bottom
        anchors.bottomMargin: chipRow.visible ? 16 : 0
        values: root.gpu ? root.gpu.history : []
        interval: root.monitor.intervalMs
        color: root.tint
    }

    Flow {
        id: chipRow

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        // Chips only where the graph keeps enough room above them.
        visible: root.chips.length > 0 && parent.height > 150
        spacing: 8

        Repeater {
            model: root.chips

            StatChip {
                required property var modelData
                label: modelData.label
                value: modelData.value
                valueColor: modelData.hot ? Theme.warn : Theme.text
            }
        }
    }

    Text {
        anchors.centerIn: parent
        visible: root.gpu === null
        text: root.monitor.connected ? "No supported GPU found" : "Waiting for data…"
        color: Theme.textDim
        font.pixelSize: 13
    }
}
