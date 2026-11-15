import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.plasma.components as PlasmaComponents

import "Sources.js" as Sources

// The widget on the desktop, and in the popup a panel opens: the sources as
// a gauge, a graph, big values or a list, like the app's widgets.
Item {
    id: root

    property var sources: []
    property var colors: []
    property var reply: null
    property bool serviceUp: false
    property string style: "gauge"
    property string title
    property bool showTitle: true
    property bool fit: false
    property int interval: 1000
    /// Shown beside the title for a machine other than this one.
    property string machineLabel

    readonly property var main: sources[0] ?? null
    readonly property bool offline: serviceUp && Sources.isOffline(reply)
    /// No daemon to talk to, or it went away: shown as a big indicator.
    readonly property var trouble: Sources.trouble(reply, serviceUp)
    // Offline, the last values stay up, faded, if there are any.
    readonly property bool live: serviceUp && reply !== null
                                 && (reply.connected || offline && sources.some(s => s.available))
    readonly property color ground: Kirigami.Theme.backgroundColor
    readonly property bool dark: Kirigami.ColorUtils.brightnessForColor(ground) === Kirigami.ColorUtils.Dark
    readonly property color dim: Kirigami.Theme.disabledTextColor

    function tonal(tint) {
        return Sources.tonal(tint, ground, dark)
    }

    implicitWidth: Kirigami.Units.gridUnit * 14
    implicitHeight: Kirigami.Units.gridUnit * 12

    ColumnLayout {
        anchors.fill: parent
        spacing: Kirigami.Units.smallSpacing * 2

        RowLayout {
            Layout.fillWidth: true
            visible: root.showTitle
            spacing: Kirigami.Units.smallSpacing * 2

            Kirigami.Icon {
                Layout.preferredWidth: Kirigami.Units.iconSizes.small
                Layout.preferredHeight: Kirigami.Units.iconSizes.small
                source: Sources.icon(root.main)
                isMask: true
                color: root.colors[0] ?? Kirigami.Theme.highlightColor
            }
            Text {
                Layout.fillWidth: true
                text: root.title || (root.main ? root.main.name : "PluSystemMonitor")
                elide: Text.ElideRight
                color: Kirigami.Theme.textColor
                font.weight: Font.DemiBold
                font.pointSize: Kirigami.Theme.defaultFont.pointSize * 1.05
            }
            Rectangle {
                visible: root.trouble !== null
                width: Kirigami.Units.smallSpacing * 2
                height: width
                radius: width / 2
                color: Kirigami.Theme.negativeTextColor
            }
            Text {
                visible: root.machineLabel.length > 0
                text: root.machineLabel
                elide: Text.ElideRight
                Layout.maximumWidth: root.width * 0.4
                color: root.dim
                font.pointSize: Kirigami.Theme.smallFont.pointSize
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            Loader {
                anchors.fill: parent
                active: root.live
                visible: root.live
                opacity: root.offline ? 0.25 : 1
                Behavior on opacity { NumberAnimation { duration: 400 } }
                sourceComponent: {
                    switch (root.style) {
                    case "graph": return graphView
                    case "value": return valueView
                    case "list": return listView
                    default: return gaugeView
                    }
                }
            }

            Absence {
                anchors.fill: parent
                visible: !root.live && root.trouble === null
                text: Sources.absence(null, root.reply, root.serviceUp)
            }

            // Over the faded last values, if there are any.
            Disconnected {
                anchors.fill: parent
                trouble: root.trouble
            }
        }
    }

    // No daemon to talk to: a big "Offline", the why on hover or click.
    component Disconnected: Item {
        id: box
        property var trouble: null
        /// The detail shown under the badge, after a click.
        property bool open: false
        readonly property bool small: width < Kirigami.Units.gridUnit * 9
                                      || height < Kirigami.Units.gridUnit * 5
        readonly property color negative: Kirigami.Theme.negativeTextColor

        visible: opacity > 0
        opacity: trouble !== null ? 1 : 0
        Behavior on opacity { NumberAnimation { duration: 250 } }
        onTroubleChanged: if (trouble === null) open = false

        // Taps beside the badge close the detail again.
        MouseArea {
            anchors.fill: parent
            enabled: box.open
            onClicked: box.open = false
        }

        ColumnLayout {
            anchors.centerIn: parent
            width: parent.width
            spacing: Kirigami.Units.smallSpacing * 2

            Rectangle {
                id: badge
                Layout.alignment: Qt.AlignHCenter
                Layout.maximumWidth: box.width
                implicitWidth: badgeRow.implicitWidth + Kirigami.Units.largeSpacing * (box.small ? 2 : 3)
                implicitHeight: badgeRow.implicitHeight + Kirigami.Units.largeSpacing * (box.small ? 1 : 1.6)
                radius: height / 2
                color: Sources.tonal(box.negative, root.ground, root.dark)
                border.width: 1
                border.color: Qt.alpha(box.negative, hover.containsMouse || box.open ? 0.7 : 0.35)
                Behavior on border.color { ColorAnimation { duration: 150 } }
                scale: hover.pressed ? 0.96 : hover.containsMouse ? 1.03 : 1
                Behavior on scale { NumberAnimation { duration: 150; easing.type: Easing.OutCubic } }

                RowLayout {
                    id: badgeRow
                    anchors.centerIn: parent
                    spacing: Kirigami.Units.smallSpacing * (box.small ? 1.5 : 2.5)

                    Kirigami.Icon {
                        Layout.preferredWidth: box.small ? Kirigami.Units.iconSizes.smallMedium : Kirigami.Units.iconSizes.medium
                        Layout.preferredHeight: Layout.preferredWidth
                        source: box.trouble ? box.trouble.icon : ""
                        isMask: true
                        color: box.negative
                    }
                    ColumnLayout {
                        spacing: 0
                        Text {
                            text: box.trouble ? box.trouble.title : ""
                            color: box.negative
                            font.weight: Font.DemiBold
                            font.pointSize: Kirigami.Theme.defaultFont.pointSize * (box.small ? 1.2 : 1.6)
                        }
                        Text {
                            visible: !box.small && text.length > 0
                            text: box.trouble && box.trouble.brief ? "for " + box.trouble.brief : ""
                            color: Kirigami.Theme.textColor
                            opacity: 0.65
                            font.pointSize: Kirigami.Theme.smallFont.pointSize
                            font.features: { "tnum": 1 }
                        }
                    }
                }

                MouseArea {
                    id: hover
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: box.open = !box.open
                }

                PlasmaComponents.ToolTip {
                    visible: hover.containsMouse && !box.open
                    text: box.trouble ? box.trouble.detail : ""
                }
            }

            // The why, on a card so it reads over the faded values.
            Rectangle {
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Math.min(box.width, Kirigami.Units.gridUnit * 18)
                Layout.preferredHeight: detail.implicitHeight + Kirigami.Units.largeSpacing * 2
                Layout.maximumHeight: Math.max(0, box.height - badge.height - parent.spacing)
                visible: opacity > 0
                opacity: box.open ? 1 : 0
                Behavior on opacity { NumberAnimation { duration: 200 } }
                radius: Kirigami.Units.cornerRadius * 2
                color: Qt.alpha(root.ground, 0.92)
                border.width: 1
                border.color: Qt.alpha(Kirigami.Theme.textColor, 0.1)
                clip: true

                Text {
                    id: detail
                    anchors.fill: parent
                    anchors.margins: Kirigami.Units.largeSpacing
                    text: box.trouble ? box.trouble.detail : ""
                    horizontalAlignment: Text.AlignHCenter
                    verticalAlignment: Text.AlignVCenter
                    wrapMode: Text.Wrap
                    elide: Text.ElideRight
                    color: Kirigami.Theme.textColor
                    font.pointSize: Kirigami.Theme.smallFont.pointSize
                }
            }
        }
    }

    component Absence: Item {
        property alias text: label.text
        ColumnLayout {
            anchors.centerIn: parent
            width: parent.width
            spacing: Kirigami.Units.smallSpacing
            Kirigami.Icon {
                Layout.alignment: Qt.AlignHCenter
                Layout.preferredWidth: Kirigami.Units.iconSizes.medium
                Layout.preferredHeight: Kirigami.Units.iconSizes.medium
                visible: parent.parent.height > Kirigami.Units.iconSizes.medium * 3
                source: "plusysmon"
                opacity: 0.6
            }
            Text {
                id: label
                Layout.fillWidth: true
                horizontalAlignment: Text.AlignHCenter
                wrapMode: Text.Wrap
                color: root.dim
                font.pointSize: Kirigami.Theme.smallFont.pointSize
            }
        }
    }

    // A source's name and value over its bar.
    component SourceRow: ColumnLayout {
        id: row
        required property var source
        required property color tint
        property bool bar: true
        spacing: Kirigami.Units.smallSpacing

        RowLayout {
            Layout.fillWidth: true
            Text {
                Layout.fillWidth: true
                text: row.source.name
                elide: Text.ElideRight
                color: Kirigami.Theme.textColor
                opacity: row.source.available ? 1 : 0.6
            }
            Text {
                text: row.source.available ? row.source.text : Sources.absence(row.source, root.reply, root.serviceUp)
                color: row.source.available ? Kirigami.Theme.textColor : root.dim
                font.features: { "tnum": 1 }
                font.weight: Font.Medium
            }
        }
        WavyBar {
            Layout.fillWidth: true
            visible: row.bar
            value: row.source.fraction
            color: row.tint
            trackColor: root.tonal(row.tint)
            thickness: Math.max(3, Kirigami.Units.smallSpacing * 1.2)
        }
    }

    Component {
        id: gaugeView

        ColumnLayout {
            spacing: Kirigami.Units.largeSpacing
            // Rows below that still leave the ring room.
            readonly property int shownDetails: Math.min(root.sources.length - 1,
                Math.max(0, Math.floor((height - Kirigami.Units.gridUnit * 5) / (Kirigami.Units.gridUnit * 2.2))))

            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                WavyRing {
                    id: ring
                    anchors.centerIn: parent
                    width: Math.min(parent.width, parent.height)
                    height: width
                    visible: root.main && root.main.available
                    value: root.main ? root.main.fraction : 0
                    color: root.colors[0] ?? Kirigami.Theme.highlightColor
                    trackColor: root.tonal(color)
                }
                ColumnLayout {
                    anchors.centerIn: ring
                    width: ring.width * 0.7
                    spacing: 0
                    visible: ring.visible
                    Text {
                        Layout.fillWidth: true
                        horizontalAlignment: Text.AlignHCenter
                        text: root.main ? root.main.text : ""
                        color: Kirigami.Theme.textColor
                        fontSizeMode: Text.HorizontalFit
                        minimumPixelSize: 10
                        font.pixelSize: Math.max(12, ring.width * (root.main && root.main.unit === "percent" ? 0.22 : 0.16))
                        font.weight: Font.DemiBold
                        font.features: { "tnum": 1 }
                    }
                    Text {
                        Layout.fillWidth: true
                        horizontalAlignment: Text.AlignHCenter
                        visible: ring.width >= Kirigami.Units.gridUnit * 6 && text.length > 0
                        text: root.main && root.main.unit !== "percent" && root.main.max > 0
                              ? "of " + Sources.metric(root.main.max, root.main.unit) : ""
                        color: root.dim
                        elide: Text.ElideRight
                        font.pointSize: Kirigami.Theme.smallFont.pointSize
                    }
                }
                Absence {
                    anchors.fill: parent
                    visible: !ring.visible
                    text: Sources.absence(root.main, root.reply, root.serviceUp)
                }
            }

            Repeater {
                model: parent.shownDetails
                SourceRow {
                    required property int index
                    Layout.fillWidth: true
                    source: root.sources[index + 1]
                    tint: root.colors[index + 1]
                }
            }
        }
    }

    Component {
        id: graphView

        ColumnLayout {
            spacing: Kirigami.Units.smallSpacing * 2

            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                // Quarter lines, faint.
                Repeater {
                    model: 3
                    Rectangle {
                        required property int index
                        width: parent.width
                        height: 1
                        y: Math.round(parent.height * (index + 1) / 4)
                        color: Qt.alpha(Kirigami.Theme.textColor, 0.07)
                    }
                }
                Repeater {
                    model: root.sources.length
                    Sparkline {
                        required property int index
                        anchors.fill: parent
                        readonly property var source: root.sources[index]
                        values: source.history
                        maxValue: Sources.scale(source, root.fit)
                        color: root.colors[index]
                        fill: index === 0 || root.sources.length <= 2
                        points: 120
                        interval: root.interval
                        lineWidth: 2
                    }
                }
            }

            Flow {
                Layout.fillWidth: true
                spacing: Kirigami.Units.largeSpacing
                Repeater {
                    model: root.sources.length
                    RowLayout {
                        required property int index
                        readonly property var source: root.sources[index]
                        spacing: Kirigami.Units.smallSpacing
                        Rectangle {
                            width: Kirigami.Units.smallSpacing * 2
                            height: width
                            radius: width / 2
                            color: root.colors[parent.index]
                        }
                        Text {
                            text: parent.source.name
                            color: root.dim
                            font.pointSize: Kirigami.Theme.smallFont.pointSize
                        }
                        Text {
                            text: parent.source.text
                            color: Kirigami.Theme.textColor
                            font.pointSize: Kirigami.Theme.smallFont.pointSize
                            font.weight: Font.Medium
                            font.features: { "tnum": 1 }
                        }
                    }
                }
            }
        }
    }

    Component {
        id: valueView

        GridLayout {
            id: tiles
            readonly property int count: root.sources.length
            columns: count <= 1 ? 1 : (width > height * 1.6 ? Math.min(count, 3) : (count > 2 ? 2 : 1))
            rowSpacing: Kirigami.Units.smallSpacing * 2
            columnSpacing: Kirigami.Units.smallSpacing * 2

            Repeater {
                model: tiles.count
                Rectangle {
                    id: tile
                    required property int index
                    readonly property var source: root.sources[index]
                    readonly property color tint: root.colors[index]
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    radius: Kirigami.Units.cornerRadius * 2
                    color: tiles.count > 1 ? Qt.alpha(root.tonal(tint), 0.6) : "transparent"
                    clip: true

                    Sparkline {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.bottom: parent.bottom
                        height: parent.height * 0.55
                        visible: tile.source.available
                        values: tile.source.history
                        // The number says how much; this only shows the shape.
                        maxValue: Sources.scale(tile.source, true)
                        color: tile.tint
                        interval: root.interval
                    }
                    ColumnLayout {
                        anchors.left: parent.left
                        anchors.right: parent.right
                        anchors.top: parent.top
                        anchors.margins: tiles.count > 1 ? Kirigami.Units.smallSpacing * 2 : 0
                        spacing: 0
                        Text {
                            Layout.fillWidth: true
                            // A single value is named by the title already.
                            visible: tiles.count > 1 || !root.showTitle
                            text: tile.source.name
                            elide: Text.ElideRight
                            color: root.dim
                            font.pointSize: Kirigami.Theme.smallFont.pointSize
                        }
                        Text {
                            Layout.fillWidth: true
                            text: tile.source.available ? tile.source.text : "—"
                            color: Kirigami.Theme.textColor
                            fontSizeMode: Text.HorizontalFit
                            minimumPixelSize: 10
                            font.pixelSize: Math.max(14, Math.min(tile.height * 0.36, Kirigami.Units.gridUnit * 3))
                            font.weight: Font.DemiBold
                            font.features: { "tnum": 1 }
                        }
                    }
                }
            }
        }
    }

    Component {
        id: listView

        ColumnLayout {
            spacing: Kirigami.Units.largeSpacing
            Repeater {
                model: root.sources.length
                SourceRow {
                    required property int index
                    Layout.fillWidth: true
                    source: root.sources[index]
                    tint: root.colors[index]
                }
            }
            Item { Layout.fillHeight: true }
        }
    }
}
