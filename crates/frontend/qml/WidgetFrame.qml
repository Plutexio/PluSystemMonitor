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
        ElementMove {}
    }
    Behavior on y {
        enabled: frame.editing && !frame.dragging
        ElementMove {}
    }
    Behavior on width {
        enabled: frame.editing && !frame.resizing
        ElementMove {}
    }
    Behavior on height {
        enabled: frame.editing && !frame.resizing
        ElementMove {}
    }
    Behavior on scale {
        enabled: !Appearance.performanceMode
        Bounce {}
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
            title: "Unknown Widget"
            accent: Appearance.colors.warn
            symbol: "question_mark"
            shape: "ghostish"

            StyledText {
                width: parent.width
                text: "“" + frame.kind + "” isn't in this version. It stays in the layout for the one that has it."
                color: Appearance.colors.onSurfaceVariant
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
            NumberAnimation { duration: Appearance.duration(180) }
        }

        Rectangle {
            anchors.fill: parent
            radius: Appearance.rounding.large
            color: Qt.alpha(Appearance.colors.layer1, 0.84)
            border.color: frame.dragging || moveArea.containsMouse ? Appearance.colors.primary : Appearance.colors.outlineVariant
            border.width: 2

            Behavior on border.color {
                ElementMoveFastColor {}
            }
        }

        Column {
            anchors.centerIn: parent
            spacing: 2

            TitleText {
                anchors.horizontalCenter: parent.horizontalCenter
                text: frame.descriptor ? frame.descriptor.name : frame.kind
                font.pixelSize: Appearance.fontSize.larger
            }
            StyledText {
                anchors.horizontalCenter: parent.horizontalCenter
                text: frame.gw + " × " + frame.gh
                color: Appearance.colors.onSurfaceVariant
                font.pixelSize: Appearance.fontSize.smaller
                tabularNumbers: true
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

            IconButton {
                visible: frame.descriptor !== null && frame.descriptor.options.length > 0
                variant: IconButton.Tonal
                size: 36
                iconSize: 20
                symbol: "tune"
                tooltip: "Widget Settings"
                onClicked: frame.dashboard.settingsRequested(frame.index, frame.descriptor, frame.values)
            }
            IconButton {
                variant: IconButton.Tonal
                size: 36
                iconSize: 20
                symbol: "delete"
                colBackground: Appearance.colors.errorContainer
                colBackgroundHover: Appearance.colors.errorContainerHover
                colRipple: Appearance.colors.errorContainerActive
                iconColor: Appearance.colors.onErrorContainer
                tooltip: "Remove Widget"
                onClicked: frame.dashboard.remove(frame.index)
            }
        }

        // The corner to resize by: a turned arrow on a shape of its own.
        Item {
            anchors.right: parent.right
            anchors.bottom: parent.bottom
            anchors.margins: 6
            width: 34
            height: 34

            ShapeIcon {
                anchors.centerIn: parent
                size: 30
                iconSize: 17
                shape: "cookie6"
                symbol: "south_east"
                color: resizeArea.containsMouse || frame.resizing ? Appearance.colors.primary : Appearance.colors.secondaryContainer
                iconColor: resizeArea.containsMouse || frame.resizing ? Appearance.colors.onPrimary : Appearance.colors.onSecondaryContainer
                hovered: resizeArea.containsMouse || frame.resizing
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
