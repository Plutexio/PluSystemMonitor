import QtQuick
import PluSystemMonitor

// Translucent panel with a title and an accent hairline along the top edge.
Item {
    id: root

    property string title
    property string subtitle
    property color accent: Theme.cpu
    default property alias content: body.data

    Rectangle {
        anchors.fill: parent
        radius: Theme.radius
        color: Theme.card
        border.color: Theme.cardBorder
        border.width: 1

        // Soft sheen from the top, like light hitting glass.
        Rectangle {
            anchors.fill: parent
            anchors.margins: 1
            radius: parent.radius - 1
            gradient: Gradient {
                GradientStop { position: 0.0; color: Theme.cardHighlight }
                GradientStop { position: 0.35; color: "transparent" }
            }
        }

        Rectangle {
            anchors.top: parent.top
            anchors.horizontalCenter: parent.horizontalCenter
            width: parent.width * 0.6
            height: 1
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0.0; color: "transparent" }
                GradientStop { position: 0.5; color: root.accent }
                GradientStop { position: 1.0; color: "transparent" }
            }
        }
    }

    Row {
        id: header

        x: Theme.padding
        y: Theme.padding - 4
        spacing: 10

        Text {
            text: root.title.toUpperCase()
            color: Theme.textDim
            font.pixelSize: 12
            font.weight: Font.DemiBold
            font.letterSpacing: 1.6
        }
        Text {
            text: root.subtitle
            color: root.accent
            font.pixelSize: 12
            font.weight: Font.DemiBold
            font.features: { "tnum": 1 }
            visible: text.length > 0
        }
    }

    Item {
        id: body

        anchors.fill: parent
        anchors.margins: Theme.padding
        anchors.topMargin: header.y + header.height + 14
    }
}
