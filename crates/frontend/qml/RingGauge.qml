import QtQuick
import QtQuick.Effects
import QtQuick.Shapes
import PluSystemMonitor

// 270° arc gauge with a neon glow; the value and the number animate together.
Item {
    id: root

    property real value: 0
    property real maxValue: 100
    property color color: Theme.cpu
    property string label
    /// Shown instead of the animated percentage, for values that aren't one.
    property string valueText
    property string caption
    property real thickness: Math.max(8, size * 0.075)

    readonly property real size: Math.min(width, height)
    // The inset keeps the glow inside the item; on small gauges it must shrink too.
    readonly property real radius: (size - thickness) / 2 - Math.min(10, size * 0.06)
    property real shown: value

    Behavior on shown {
        NumberAnimation { duration: 650; easing.type: Easing.OutCubic }
    }

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: Theme.track
            strokeWidth: root.thickness
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap

            PathAngleArc {
                centerX: root.width / 2
                centerY: root.height / 2
                radiusX: root.radius
                radiusY: root.radius
                startAngle: 135
                sweepAngle: 270
            }
        }
    }

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer
        layer.enabled: Theme.glow > 0
        layer.effect: MultiEffect {
            shadowEnabled: true
            shadowColor: root.color
            shadowBlur: 1.0
            shadowOpacity: Theme.glow
            shadowHorizontalOffset: 0
            shadowVerticalOffset: 0
        }

        ShapePath {
            strokeColor: root.color
            strokeWidth: root.thickness
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap

            PathAngleArc {
                centerX: root.width / 2
                centerY: root.height / 2
                radiusX: root.radius
                radiusY: root.radius
                startAngle: 135
                // A zero sweep would still draw the round cap; keep a hairline minimum.
                sweepAngle: Math.max(0.5, 270 * Math.min(1, root.shown / root.maxValue))
            }
        }
    }

    Column {
        anchors.centerIn: parent
        spacing: 2

        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            text: root.valueText || Math.round(root.shown) + "%"
            color: Theme.text
            font.pixelSize: root.size * (root.valueText ? 0.15 : 0.22)
            font.weight: Font.Bold
            font.features: { "tnum": 1 }
        }
        Text {
            anchors.horizontalCenter: parent.horizontalCenter
            // Small gauges keep just the number readable.
            visible: root.size >= 100 && root.label.length > 0
            text: root.label
            color: Theme.textDim
            font.pixelSize: Math.max(11, root.size * 0.065)
        }
    }

    Text {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: root.size * 0.04
        // Sits in the arc's open bottom, which only has room on larger gauges.
        visible: root.size >= 150
        text: root.caption
        color: Theme.textDim
        font.pixelSize: 12
        font.features: { "tnum": 1 }
    }
}
