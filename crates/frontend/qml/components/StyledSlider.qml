import QtQuick
import QtQuick.Templates as T
import PluSystemMonitor

// The Material 3 Expressive slider: a thick track, filled up to a tall thin
// handle that stands in a gap of its own and narrows while held. The value
// shows in a bubble above the handle while it moves. Like the switch, it
// reports moves (`moved`) and leaves `value` to its owner, so bindings hold.
T.Slider {
    id: root

    property var scheme: Appearance.colors
    /// What the bubble says, from the value.
    property var format: value => Math.round(value)
    property color activeColor: scheme.primary
    property color inactiveColor: scheme.secondaryContainer
    readonly property real trackHeight: 16

    implicitWidth: 200
    implicitHeight: 44
    hoverEnabled: true
    focusPolicy: Qt.TabFocus
    opacity: enabled ? 1 : 0.38
    live: true

    background: Item {
        x: root.leftPadding
        y: root.topPadding + (root.availableHeight - height) / 2
        width: root.availableWidth
        height: root.trackHeight

        readonly property real gap: 6
        readonly property real handleX: root.visualPosition * (width - root.handle.width) + root.handle.width / 2

        // Filled part, up to the handle's gap.
        Rectangle {
            width: Math.max(0, parent.handleX - parent.gap - root.handle.width / 2)
            height: parent.height
            visible: width > 2
            radius: Math.min(height / 2, width / 2)
            topRightRadius: 3
            bottomRightRadius: 3
            color: root.activeColor
        }
        // The rest, after it.
        Rectangle {
            x: parent.handleX + parent.gap + root.handle.width / 2
            width: Math.max(0, parent.width - x)
            height: parent.height
            visible: width > 2
            radius: Math.min(height / 2, width / 2)
            topLeftRadius: 3
            bottomLeftRadius: 3
            color: root.inactiveColor

            // The stop at the end.
            Rectangle {
                anchors.right: parent.right
                anchors.rightMargin: 5
                anchors.verticalCenter: parent.verticalCenter
                width: 4
                height: 4
                radius: 2
                color: root.activeColor
                visible: parent.width > 14
            }
        }
    }

    handle: Rectangle {
        x: root.leftPadding + root.visualPosition * (root.availableWidth - width)
        y: root.topPadding + (root.availableHeight - height) / 2
        implicitWidth: root.pressed ? 2 : 4
        implicitHeight: 44
        radius: width / 2
        color: root.activeColor

        Behavior on implicitWidth {
            ElementMoveFast {}
        }

        // The value, above the handle while it is held.
        Rectangle {
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.top
            anchors.bottomMargin: 6
            width: Math.max(40, bubble.implicitWidth + 20)
            height: 30
            radius: 15
            color: root.scheme.inverseSurface
            opacity: root.pressed ? 1 : 0
            scale: root.pressed ? 1 : 0.6
            transformOrigin: Item.Bottom
            visible: opacity > 0

            Behavior on opacity {
                NumberAnimation { duration: Appearance.duration(120) }
            }
            Behavior on scale {
                ElementMoveFast {}
            }

            StyledText {
                id: bubble
                anchors.centerIn: parent
                text: root.format(root.value)
                color: root.scheme.inverseOnSurface
                font.pixelSize: Appearance.fontSize.smaller
                font.weight: Font.DemiBold
                tabularNumbers: true
            }
        }
    }

    HoverHandler {
        cursorShape: root.enabled ? Qt.PointingHandCursor : Qt.ArrowCursor
    }
}
