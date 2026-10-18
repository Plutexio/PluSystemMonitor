import QtQuick
import PluSystemMonitor

// Up to four sources over time, each scaled to its own full range.
Card {
    id: root

    required property Monitor monitor
    property var options: ({})

    readonly property var ids: options.sources ?? []
    readonly property var sources: {
        monitor.revision
        return Sources.read(monitor, ids)
    }
    readonly property var colors: Sources.colors(sources, options.accent)
    readonly property var main: sources[0] ?? null
    readonly property bool legend: sources.length > 1

    title: options.title || (main ? main.name : "Graph")
    subtitle: !legend && main && main.available ? main.text : ""
    accent: colors[0] ?? Theme.cpu

    Item {
        id: plot

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.top: parent.top
        anchors.bottom: legendRow.visible ? legendRow.top : parent.bottom
        anchors.bottomMargin: legendRow.visible ? 14 : 0

        // Iterates the stable ID list, not the per-sample objects, so each
        // series keeps its delegate (and its glide animation) across samples.
        Repeater {
            model: root.ids

            LiveGraph {
                required property int index

                readonly property var source: root.sources[index] ?? null

                anchors.fill: parent
                visible: source !== null && source.available
                values: source ? source.history : []
                maxValue: source && source.max > 0 ? source.max : 1
                interval: root.monitor.intervalMs
                color: root.colors[index] ?? Theme.cpu
                showGrid: index === 0
                fillOpacity: index === 0 ? 1 : 0.35
            }
        }
    }

    Flow {
        id: legendRow

        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        visible: root.legend && parent.height > 120
        spacing: 16

        Repeater {
            model: root.ids

            Row {
                required property int index

                readonly property var source: root.sources[index] ?? null

                spacing: 7

                Rectangle {
                    anchors.verticalCenter: parent.verticalCenter
                    width: 8
                    height: 8
                    radius: 4
                    color: root.colors[parent.index] ?? Theme.cpu
                }
                Text {
                    text: parent.source ? parent.source.name : ""
                    color: Theme.textDim
                    font.pixelSize: 12
                }
                Text {
                    text: parent.source ? parent.source.text : ""
                    color: Theme.text
                    font.pixelSize: 12
                    font.weight: Font.DemiBold
                    font.features: { "tnum": 1 }
                }
            }
        }
    }

    Text {
        anchors.centerIn: parent
        visible: !root.main || !root.main.available
        text: !root.main ? "Pick sources in the widget's settings"
              : root.monitor.connected ? "Not available on this machine" : "Waiting for data…"
        color: Theme.textDim
        font.pixelSize: 13
    }
}
