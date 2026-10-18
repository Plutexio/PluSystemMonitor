import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Effects
import PluSystemMonitor

// Floating panel used by every popup: opaque so text stays readable over the
// dashboard, with a soft shadow and a small scale-in.
Popup {
    id: root

    property string title

    padding: 20
    topPadding: title ? 54 : 20
    modal: true
    focus: true
    closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

    Overlay.modal: Rectangle {
        color: Qt.rgba(0, 0, 0, 0.25)
    }

    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: 160 }
        NumberAnimation { property: "scale"; from: 0.95; to: 1; duration: 220; easing.type: Easing.OutCubic }
    }
    exit: Transition {
        NumberAnimation { property: "opacity"; to: 0; duration: 120 }
        NumberAnimation { property: "scale"; to: 0.97; duration: 120 }
    }

    background: Item {
        Rectangle {
            id: panel

            anchors.fill: parent
            radius: Theme.radius
            color: Theme.background
            border.color: Theme.cardBorder
            border.width: 1
            layer.enabled: true
            layer.effect: MultiEffect {
                shadowEnabled: true
                shadowColor: Qt.rgba(0, 0, 0, 0.5)
                shadowBlur: 1.0
                shadowVerticalOffset: 10
            }

            Rectangle {
                anchors.fill: parent
                anchors.margins: 1
                radius: parent.radius - 1
                color: Theme.card
            }
        }

        Text {
            x: root.padding
            y: 20
            visible: root.title.length > 0
            text: root.title
            color: Theme.text
            font.pixelSize: 16
            font.weight: Font.Bold
        }
    }
}
