import QtQuick
import PluSystemMonitor

// Labelled horizontal bar: "RAM   9.4 GiB / 31.2 GiB".
Column {
    id: root

    property string label
    property real used: 0
    property real total: 0
    property color color: Theme.memory

    readonly property real fraction: total > 0 ? Math.min(1, used / total) : 0

    spacing: 8

    Item {
        width: root.width
        height: valueText.height

        Text {
            text: root.label
            color: Theme.textDim
            font.pixelSize: 13
        }
        Text {
            id: valueText
            anchors.right: parent.right
            text: root.total > 0 ? Theme.formatBytes(root.used) + " / " + Theme.formatBytes(root.total) : "—"
            color: Theme.text
            font.pixelSize: 13
            font.weight: Font.DemiBold
            font.features: { "tnum": 1 }
        }
    }

    Rectangle {
        width: root.width
        height: 8
        radius: 4
        color: Theme.track

        Rectangle {
            width: Math.max(height, parent.width * root.fraction)
            height: parent.height
            radius: parent.radius
            visible: root.fraction > 0
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0.0; color: Qt.darker(root.color, 1.4) }
                GradientStop { position: 1.0; color: root.color }
            }

            Behavior on width {
                NumberAnimation { duration: 650; easing.type: Easing.OutCubic }
            }
        }
    }
}
