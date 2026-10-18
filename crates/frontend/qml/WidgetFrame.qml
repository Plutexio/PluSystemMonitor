import QtQuick
import PluSystemMonitor

// Hosts one widget and, in edit mode, the chrome to move, resize, configure
// and remove it. Positions snap to the dashboard grid on release.
Item {
    id: frame

    required property Dashboard dashboard
    required property int index
    required property string kind
    required property int gx
    required property int gy
    required property int gw
    required property int gh
    required property string options

    readonly property var descriptor: Widgets.find(kind)
    readonly property var values: Object.assign(Widgets.defaults(kind), JSON.parse(options))
    readonly property rect cell: dashboard.cellRect(gx, gy, gw, gh)
    readonly property bool editing: dashboard.editing

    property bool dragging: false
    property bool resizing: false
    property point dragOffset: Qt.point(0, 0)
    property size resizeDelta: Qt.size(0, 0)

    function target() {
        const columns = dashboard.columns
        if (resizing) {
            const min = descriptor ? descriptor.minSize : [1, 1]
            const w = Math.round((cell.width + resizeDelta.width + dashboard.gap) / dashboard.colUnit)
            const h = Math.round((cell.height + resizeDelta.height + dashboard.gap) / dashboard.rowUnit)
            return {
                x: gx, y: gy,
                w: Math.max(min[0], Math.min(columns - gx, w)),
                h: Math.max(min[1], h)
            }
        }
        const x = Math.round((cell.x + dragOffset.x) / dashboard.colUnit)
        const y = Math.round((cell.y + dragOffset.y) / dashboard.rowUnit)
        return { x: Math.max(0, Math.min(columns - gw, x)), y: Math.max(0, y), w: gw, h: gh }
    }

    function updateGhost() {
        const t = target()
        dashboard.ghost = dashboard.cellRect(t.x, t.y, t.w, t.h)
        dashboard.ghostValid = dashboard.fits(index, t.x, t.y, t.w, t.h)
        dashboard.ghostVisible = true
    }

    function finish() {
        if (!dragging && !resizing)
            return
        const t = target()
        // Clear the flags first so the Behaviors animate into the final cell
        // (or back to the old one if the spot was taken).
        dragging = false
        resizing = false
        dashboard.ghostVisible = false
        dashboard.place(index, t.x, t.y, t.w, t.h)
        dragOffset = Qt.point(0, 0)
        resizeDelta = Qt.size(0, 0)
    }

    x: cell.x + dragOffset.x
    y: cell.y + dragOffset.y
    width: Math.max(dashboard.colUnit - dashboard.gap, cell.width + resizeDelta.width)
    height: Math.max(dashboard.rowUnit - dashboard.gap, cell.height + resizeDelta.height)
    z: dragging || resizing ? 10 : 0
    scale: dragging ? 1.02 : 1

    Behavior on x {
        enabled: frame.editing && !frame.dragging
        NumberAnimation { duration: 240; easing.type: Easing.OutCubic }
    }
    Behavior on y {
        enabled: frame.editing && !frame.dragging
        NumberAnimation { duration: 240; easing.type: Easing.OutCubic }
    }
    Behavior on width {
        enabled: frame.editing && !frame.resizing
        NumberAnimation { duration: 240; easing.type: Easing.OutCubic }
    }
    Behavior on height {
        enabled: frame.editing && !frame.resizing
        NumberAnimation { duration: 240; easing.type: Easing.OutCubic }
    }
    Behavior on scale {
        NumberAnimation { duration: 150 }
    }

    onValuesChanged: {
        if (loader.item && descriptor)
            loader.item.options = values
    }

    Loader {
        id: loader

        anchors.fill: parent
        Component.onCompleted: {
            if (frame.descriptor)
                setSource(frame.descriptor.source, { monitor: frame.dashboard.monitor, options: frame.values })
            else
                sourceComponent = unknownWidget
        }
    }

    // Kept (not dropped) so layouts from newer versions survive a round trip.
    Component {
        id: unknownWidget

        Card {
            title: "Unknown widget"
            accent: Theme.warn

            Text {
                width: parent.width
                text: "“" + frame.kind + "” is not available in this version."
                color: Theme.textDim
                font.pixelSize: 13
                wrapMode: Text.Wrap
            }
        }
    }

    Item {
        id: chrome

        anchors.fill: parent
        visible: opacity > 0
        opacity: frame.editing ? 1 : 0

        Behavior on opacity {
            NumberAnimation { duration: 180 }
        }

        Rectangle {
            anchors.fill: parent
            radius: Theme.radius
            color: Qt.alpha(Theme.background, 0.78)
            border.color: frame.dragging || moveArea.containsMouse ? Theme.primary : Theme.cardBorder
            border.width: 2

            Behavior on border.color {
                ColorAnimation { duration: 140 }
            }
        }

        Column {
            anchors.centerIn: parent
            spacing: 4

            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: frame.descriptor ? frame.descriptor.name : frame.kind
                color: Theme.text
                font.pixelSize: 15
                font.weight: Font.Bold
            }
            Text {
                anchors.horizontalCenter: parent.horizontalCenter
                text: frame.gw + " × " + frame.gh
                color: Theme.textDim
                font.pixelSize: 12
                font.features: { "tnum": 1 }
            }
        }

        MouseArea {
            id: moveArea

            property point start

            anchors.fill: parent
            hoverEnabled: true
            preventStealing: true
            cursorShape: frame.dragging ? Qt.ClosedHandCursor : Qt.OpenHandCursor

            onPressed: mouse => {
                start = mapToItem(frame.dashboard.contentItem, mouse.x, mouse.y)
                frame.dragging = true
                frame.updateGhost()
            }
            onPositionChanged: mouse => {
                if (!frame.dragging)
                    return
                const p = mapToItem(frame.dashboard.contentItem, mouse.x, mouse.y)
                frame.dragOffset = Qt.point(p.x - start.x, p.y - start.y)
                frame.updateGhost()
            }
            onReleased: frame.finish()
            onCanceled: frame.finish()
        }

        Row {
            anchors.top: parent.top
            anchors.right: parent.right
            anchors.margins: 10
            spacing: 6

            GlassButton {
                implicitHeight: 32
                visible: frame.descriptor !== null && frame.descriptor.options.length > 0
                iconName: "tune"
                tip: "Widget settings"
                onClicked: frame.dashboard.settingsRequested(frame.index, frame.descriptor, frame.values)
            }
            GlassButton {
                implicitHeight: 32
                danger: true
                iconName: "close"
                tip: "Remove widget"
                onClicked: frame.dashboard.remove(frame.index)
            }
        }

        Item {
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            width: 34
            height: 34

            Icon {
                anchors.centerIn: parent
                name: "resize"
                color: resizeArea.containsMouse || frame.resizing ? Theme.primary : Theme.textDim
            }

            MouseArea {
                id: resizeArea

                property point start

                anchors.fill: parent
                hoverEnabled: true
                preventStealing: true
                cursorShape: Qt.SizeFDiagCursor

                onPressed: mouse => {
                    start = mapToItem(frame.dashboard.contentItem, mouse.x, mouse.y)
                    frame.resizing = true
                    frame.updateGhost()
                }
                onPositionChanged: mouse => {
                    if (!frame.resizing)
                        return
                    const p = mapToItem(frame.dashboard.contentItem, mouse.x, mouse.y)
                    frame.resizeDelta = Qt.size(p.x - start.x, p.y - start.y)
                    frame.updateGhost()
                }
                onReleased: frame.finish()
                onCanceled: frame.finish()
            }
        }
    }
}
