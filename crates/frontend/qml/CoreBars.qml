import QtQuick
import PluSystemMonitor

// One pill per logical core, filling from the bottom.
Row {
    id: root

    property list<real> values
    property color color: Appearance.colors.cpu

    readonly property int count: Math.max(1, values.length)

    spacing: count > 32 ? 2 : 4

    Repeater {
        model: root.values

        Rectangle {
            required property real modelData

            width: (root.width - root.spacing * (root.count - 1)) / root.count
            height: root.height
            radius: Math.min(Appearance.rounding.full, width / 2)
            color: Appearance.colors.tonal(root.color)

            Rectangle {
                anchors.bottom: parent.bottom
                width: parent.width
                height: Math.max(width, parent.height * modelData / 100)
                radius: parent.radius
                color: root.color
                opacity: 0.55 + 0.45 * modelData / 100

                Behavior on height {
                    enabled: !Appearance.performanceMode
                    NumberAnimation { duration: 500; easing.type: Easing.BezierSpline; easing.bezierCurve: Appearance.curves.expressiveEffects }
                }
            }
        }
    }
}
