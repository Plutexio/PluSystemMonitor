import QtQuick
import PluSystemMonitor

// Transient error message at the bottom of the window. Click to dismiss.
Rectangle {
    id: root

    function show(message) {
        label.text = message
        opacity = 1
        hide.restart()
    }

    width: Math.min(parent.width - 48, content.implicitWidth + 36)
    height: content.implicitHeight + 24
    radius: 14
    color: Theme.background
    border.color: Qt.alpha(Theme.bad, 0.6)
    border.width: 1
    opacity: 0
    visible: opacity > 0

    Behavior on opacity {
        NumberAnimation { duration: 220 }
    }

    Rectangle {
        anchors.fill: parent
        radius: parent.radius
        color: Qt.alpha(Theme.bad, 0.08)
    }

    Row {
        id: content

        anchors.centerIn: parent
        spacing: 10

        Icon {
            name: "alert"
            color: Theme.bad
        }
        Text {
            id: label
            width: Math.min(implicitWidth, root.parent.width - 48 - 36 - 28)
            color: Theme.text
            font.pixelSize: 13
            wrapMode: Text.Wrap
        }
    }

    Timer {
        id: hide
        interval: 8000
        onTriggered: root.opacity = 0
    }

    TapHandler {
        onTapped: root.opacity = 0
    }
}
