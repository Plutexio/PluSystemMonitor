import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "Sources.js" as Sources

// The widget on a panel: the desktop styles made small, one per source.
// Gauge is a ring with the value inside, Graph a tiny graph with the value
// over it, Values the value over its faint history. (List doesn't fit.)
Item {
    id: root

    property var sources: []
    property var colors: []
    property bool live: false
    /// Show the last values, faded, with a red dot.
    property bool offline: false
    property string style: "gauge"
    property bool vertical: false
    property int interval: 1000

    readonly property real thickness: vertical ? width : height
    /// One source's room across the panel: all of it, as Plasma's own
    /// widgets take it (the panel keeps its margins already).
    readonly property real size: Math.max(16, thickness)
    readonly property color ground: Kirigami.Theme.backgroundColor
    readonly property bool dark: Kirigami.ColorUtils.brightnessForColor(ground) === Kirigami.ColorUtils.Dark

    signal clicked()

    // Across the panel the panel decides; this is only a fallback.
    implicitWidth: vertical ? Kirigami.Units.iconSizes.medium : grid.implicitWidth
    implicitHeight: vertical ? grid.implicitHeight : Kirigami.Units.iconSizes.medium

    MouseArea {
        anchors.fill: parent
        onClicked: root.clicked()
    }

    GridLayout {
        id: grid
        anchors.centerIn: parent
        opacity: root.offline ? 0.4 : 1
        Behavior on opacity { NumberAnimation { duration: 400 } }
        flow: root.vertical ? GridLayout.TopToBottom : GridLayout.LeftToRight
        rows: root.vertical ? -1 : 1
        columns: root.vertical ? 1 : -1
        rowSpacing: Kirigami.Units.smallSpacing * 1.5
        columnSpacing: Kirigami.Units.smallSpacing * 1.5

        Repeater {
            model: Math.max(1, root.sources.length)

            Loader {
                required property int index
                readonly property var source: root.sources[index] ?? null
                readonly property color tint: root.colors[index] ?? Kirigami.Theme.highlightColor
                readonly property bool shown: root.live && source !== null && source.available
                readonly property string text: shown ? source.brief : "—"

                Layout.preferredWidth: item ? item.implicitWidth : 0
                Layout.preferredHeight: item ? item.implicitHeight : 0
                sourceComponent: root.style === "graph" ? graph : root.style === "value" ? value : gauge
            }
        }
    }

    // Offline, as the app's Recent list marks it.
    Rectangle {
        visible: root.offline
        width: Math.max(6, root.size * 0.2)
        height: width
        radius: width / 2
        x: grid.x + grid.width - width * 0.8
        y: Math.max(0, grid.y - height * 0.2)
        color: Kirigami.Theme.negativeTextColor
        border.width: 1
        border.color: root.ground
    }

    // The value, as large as fits the box it is given.
    component Reading: Text {
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        color: Kirigami.Theme.textColor
        fontSizeMode: Text.Fit
        minimumPixelSize: 6
        font.weight: Font.DemiBold
        font.features: { "tnum": 1 }
    }

    Component {
        id: gauge

        Item {
            implicitWidth: root.size
            implicitHeight: root.size

            WavyRing {
                id: ring
                anchors.fill: parent
                value: parent.parent.shown ? parent.parent.source.fraction : 0
                color: parent.parent.tint
                trackColor: Sources.tonal(color, root.ground, root.dark)
                thickness: Math.max(2, root.size * 0.085)
            }
            Reading {
                anchors.centerIn: parent
                // Inside the ring, reaching into its open bottom.
                anchors.verticalCenterOffset: ring.radius * 0.08
                width: ring.radius * 2 - ring.thickness * 1.2
                height: ring.radius * 1.1
                text: parent.parent.text
                font.pixelSize: root.size * 0.4
            }
        }
    }

    Component {
        id: graph

        Rectangle {
            id: tile
            readonly property var slot: parent
            implicitWidth: root.vertical ? root.size : root.size * 2
            implicitHeight: root.vertical ? root.size * 0.75 : root.size
            radius: Kirigami.Units.cornerRadius
            color: Qt.alpha(Sources.tonal(slot.tint, root.ground, root.dark), 0.8)
            clip: true

            Sparkline {
                anchors.fill: parent
                anchors.topMargin: parent.height * 0.48
                values: tile.slot.shown ? tile.slot.source.history : []
                maxValue: Sources.scale(tile.slot.source, false)
                color: tile.slot.tint
                lineWidth: 1.5
                points: root.vertical ? 20 : 40
                interval: root.interval
            }
            Reading {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.top: parent.top
                anchors.margins: 2
                height: parent.height * 0.5
                horizontalAlignment: root.vertical ? Text.AlignHCenter : Text.AlignLeft
                leftPadding: root.vertical ? 0 : 2
                text: tile.slot.text
                font.pixelSize: root.size * 0.42
            }
        }
    }

    Component {
        id: value

        Rectangle {
            id: tile
            readonly property var slot: parent
            // The widest value seen, so the panel doesn't twitch as numbers change.
            property real widest: 0
            implicitWidth: root.vertical ? root.size : Math.max(root.size * 1.2, widest + Kirigami.Units.smallSpacing * 3)
            implicitHeight: root.vertical ? root.size * 0.7 : root.size
            radius: Kirigami.Units.cornerRadius
            color: Qt.alpha(Sources.tonal(slot.tint, root.ground, root.dark), 0.55)
            clip: true

            Sparkline {
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                height: parent.height * 0.6
                values: tile.slot.shown ? tile.slot.source.history : []
                // The number says how much; this only shows the shape.
                maxValue: Sources.scale(tile.slot.source, true)
                color: tile.slot.tint
                lineWidth: 1
                points: root.vertical ? 20 : 30
                interval: root.interval
                opacity: 0.7
            }
            Reading {
                id: number
                anchors.fill: parent
                anchors.margins: 2
                text: tile.slot.text
                // Fixed on a horizontal panel: the tile grows to the number instead.
                fontSizeMode: root.vertical ? Text.Fit : Text.FixedSize
                font.pixelSize: root.size * (root.vertical ? 0.4 : 0.42)
                onContentWidthChanged: tile.widest = Math.max(tile.widest, number.contentWidth)
            }
        }
    }
}
