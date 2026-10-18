import QtQuick
import QtQuick.Controls.Basic
import PluSystemMonitor

// Translucent button with an optional icon; `primary` fills it with the accent.
Button {
    id: root

    property string iconName
    property bool primary: false
    property bool danger: false
    property string tip

    readonly property color tint: danger ? Theme.bad : Theme.primary
    readonly property color foreground: primary ? "white" : danger ? Theme.bad : Theme.text

    implicitHeight: 38
    implicitWidth: text ? contentItem.implicitWidth + leftPadding + rightPadding : implicitHeight
    leftPadding: text ? (iconName ? 13 : 18) : 0
    rightPadding: text ? 18 : 0
    hoverEnabled: true
    font.pixelSize: 13
    font.weight: Font.DemiBold

    ToolTip.visible: tip.length > 0 && hovered
    ToolTip.text: tip
    ToolTip.delay: 600

    contentItem: Item {
        implicitWidth: row.implicitWidth
        implicitHeight: row.implicitHeight

        Row {
            id: row
            anchors.centerIn: parent
            spacing: 8

            Icon {
                anchors.verticalCenter: parent.verticalCenter
                visible: root.iconName.length > 0
                name: root.iconName
                color: root.foreground
                size: Math.round(root.implicitHeight * 0.45)
            }
            Text {
                anchors.verticalCenter: parent.verticalCenter
                visible: root.text.length > 0
                text: root.text
                font: root.font
                color: root.foreground
            }
        }
    }

    background: Rectangle {
        radius: Math.min(12, height / 2)
        color: root.primary ? root.tint : root.hovered ? Qt.alpha(root.tint, 0.16) : Theme.card
        border.color: root.primary ? "transparent" : root.hovered ? Qt.alpha(root.tint, 0.5) : Theme.cardBorder
        border.width: 1
        opacity: root.down ? 0.75 : 1
        scale: root.down ? 0.96 : 1

        Behavior on color {
            ColorAnimation { duration: 140 }
        }
        Behavior on border.color {
            ColorAnimation { duration: 140 }
        }
        Behavior on scale {
            NumberAnimation { duration: 120 }
        }

        // Sheen that makes the primary button read as lit from above.
        Rectangle {
            anchors.fill: parent
            radius: parent.radius
            visible: root.primary
            gradient: Gradient {
                GradientStop { position: 0.0; color: Qt.rgba(1, 1, 1, 0.22) }
                GradientStop { position: 1.0; color: Qt.rgba(1, 1, 1, 0.0) }
            }
        }
    }
}
