import QtQuick
import QtQuick.Templates as T
import PluSystemMonitor

// A navigation drawer row: Material 3's pill behind the chosen one, opening
// from the middle and going wavy under the pointer, its icon popping onto a
// shape of its own; a state layer on hover, and room for a trailing control.
T.AbstractButton {
    id: root

    property string symbol
    property bool selected: false
    /// The shape the icon sits on while chosen.
    property string iconShape: "cookie9"
    property alias trailing: trailingSlot.data

    implicitHeight: 40
    hoverEnabled: true
    focusPolicy: Qt.TabFocus

    background: Item {
        // The state layer of a row that isn't chosen.
        Rectangle {
            anchors.fill: parent
            radius: Appearance.rounding.full
            color: !root.selected && root.down ? Appearance.colors.layer0Active
                 : Qt.alpha(Appearance.colors.layer0Hover, !root.selected && root.hovered ? 1 : 0)

            Behavior on color {
                ElementMoveFastColor {}
            }
        }

        // The chosen pill opens from the middle on a quick ease that stops at
        // the row's edges; a spring past them carried it out of the drawer.
        WavyPill {
            property real openness: root.selected ? 1 : 0

            anchors.centerIn: parent
            width: parent.width * (0.35 + 0.65 * Math.min(openness, 1))
            height: parent.height
            opacity: root.selected ? 1 : 0
            visible: opacity > 0
            hovered: root.selected && root.hovered
            pressed: root.selected && root.down
            color: root.down ? Appearance.colors.secondaryContainerActive
                 : root.hovered ? Appearance.colors.secondaryContainerHover : Appearance.colors.secondaryContainer

            Behavior on openness {
                enabled: !Appearance.performanceMode
                ElementMove {}
            }
            Behavior on opacity {
                NumberAnimation { duration: Appearance.duration(160); easing.type: Easing.OutCubic }
            }
            Behavior on color {
                ElementMoveFastColor {}
            }
        }

        Rectangle {
            anchors.fill: parent
            visible: root.visualFocus
            radius: Appearance.rounding.full
            color: "transparent"
            border.width: 2
            border.color: Appearance.colors.secondary
        }
    }

    contentItem: Item {
        Item {
            id: iconSlot
            anchors.left: parent.left
            anchors.leftMargin: 16
            anchors.verticalCenter: parent.verticalCenter
            width: 24
            height: 24

            MaterialSymbol {
                id: plainIcon
                anchors.centerIn: parent
                text: root.symbol
                iconSize: 22
                weight: root.hovered ? 600 : 400
                color: Appearance.colors.onSurfaceVariant
            }
            Reveal {
                target: plainIcon
                shown: !root.selected
            }

            ShapeIcon {
                id: chosenIcon
                anchors.centerIn: parent
                size: 30
                iconSize: 17
                shape: root.iconShape
                symbol: root.symbol
                color: Appearance.colors.primary
                iconColor: Appearance.colors.onPrimary
                hovered: root.hovered
            }
            Reveal {
                target: chosenIcon
                shown: root.selected
                overshoot: 2.4
            }
        }

        StyledText {
            anchors.left: iconSlot.right
            anchors.leftMargin: 12
            anchors.right: trailingSlot.left
            anchors.rightMargin: 8
            anchors.verticalCenter: parent.verticalCenter
            text: root.text
            elide: Text.ElideRight
            font.pixelSize: Appearance.fontSize.small
            font.weight: root.selected ? Font.DemiBold : Font.Medium
            color: root.selected ? Appearance.colors.onSecondaryContainer : Appearance.colors.onSurface
        }

        Row {
            id: trailingSlot
            anchors.right: parent.right
            anchors.rightMargin: 8
            anchors.verticalCenter: parent.verticalCenter
            spacing: 2
        }
    }

    HoverHandler {
        cursorShape: Qt.PointingHandCursor
    }
}
