import QtQuick
import PluSystemMonitor

// On/off switch. Doesn't flip itself: the owner updates `checked` in `toggled`.
Rectangle {
    id: root

    property bool checked: false

    signal toggled()

    implicitWidth: 44
    implicitHeight: 24
    radius: height / 2
    color: checked ? Theme.primary : Theme.track

    Behavior on color {
        ColorAnimation { duration: 160 }
    }

    Rectangle {
        width: parent.height - 6
        height: width
        radius: width / 2
        y: 3
        x: root.checked ? parent.width - width - 3 : 3
        color: "white"

        Behavior on x {
            NumberAnimation { duration: 180; easing.type: Easing.OutCubic }
        }
    }

    TapHandler {
        onTapped: root.toggled()
    }
    HoverHandler {
        cursorShape: Qt.PointingHandCursor
    }
}
