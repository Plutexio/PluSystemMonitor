import QtQuick
import PluSystemMonitor

// The first source on a ring, the rest as bars below it.
Card {
    id: root

    required property Monitor monitor
    property var options: ({})

    readonly property var sources: {
        monitor.revision
        return Sources.read(monitor, options.sources)
    }
    readonly property var colors: Sources.colors(sources, options.accent)
    readonly property var main: sources[0] ?? null
    readonly property var details: sources.slice(1)

    // Bars that fit below the header while leaving the ring ~90 px.
    readonly property int shownDetails: Math.min(details.length, Math.max(0, Math.floor((height - 176) / 44)))

    title: options.title || (main ? main.name : "Gauge")
    accent: colors[0] ?? Theme.cpu

    Text {
        anchors.centerIn: parent
        width: parent.width
        visible: !root.main || !root.main.available
        horizontalAlignment: Text.AlignHCenter
        wrapMode: Text.Wrap
        text: Sources.absence(root.main, root.monitor)
        color: Theme.textDim
        font.pixelSize: 13
    }

    RingGauge {
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: bars.top
        anchors.bottomMargin: root.shownDetails > 0 ? 12 : 0
        visible: root.main !== null && root.main.available
        value: root.main ? root.main.fraction * 100 : 0
        color: root.accent
        // Percentages animate as numbers; other units print their value.
        valueText: root.main && root.main.unit !== "percent" ? root.main.text : ""
        label: root.main && root.main.unit !== "percent" && root.main.max > 0
               ? "of " + Format.metric(root.main.max, root.main.unit) : ""
    }

    Column {
        id: bars

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        spacing: 12

        Repeater {
            model: root.shownDetails

            MetricBar {
                required property int index
                width: parent.width
                source: root.details[index]
                color: root.colors[index + 1]
            }
        }
    }
}
