import QtQuick
import PluSystemMonitor

// One vertical bar per logical core.
Row {
    id: root

    property list<real> values
    property color color: Theme.cpu

    readonly property int count: Math.max(1, values.length)

    spacing: count > 32 ? 2 : 4

    Repeater {
        model: root.values

        Rectangle {
            required property real modelData

            width: (root.width - root.spacing * (root.count - 1)) / root.count
            height: root.height
            radius: Math.min(4, width / 2)
            color: Theme.track

            Rectangle {
                anchors.bottom: parent.bottom
                width: parent.width
                height: Math.max(2, parent.height * modelData / 100)
                radius: parent.radius
                color: root.color
                opacity: 0.45 + 0.55 * modelData / 100

                Behavior on height {
                    NumberAnimation { duration: 500; easing.type: Easing.OutCubic }
                }
            }
        }
    }
}
