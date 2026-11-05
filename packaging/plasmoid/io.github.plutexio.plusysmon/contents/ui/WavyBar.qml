import QtQuick
import QtQuick.Shapes

// Material 3 Expressive's wavy linear progress: the value rides a sine wave,
// then a gap, then a flat track to a stop dot.
Item {
    id: root

    /// 0..1
    property real value: 0
    property color color: "steelblue"
    property color trackColor: Qt.alpha(color, 0.25)
    property real thickness: 4
    property bool wavy: true
    property real wavelength: thickness * 6.5

    property real shown: Math.max(0, Math.min(1, value))
    Behavior on shown {
        NumberAnimation { duration: 650; easing.type: Easing.OutCubic }
    }

    readonly property real amplitude: wavy ? thickness * 0.75 : 0
    readonly property real startX: thickness / 2
    readonly property real endX: width - thickness / 2
    readonly property real end: startX + (endX - startX) * shown
    readonly property real mid: height / 2

    implicitHeight: thickness + 2 * amplitude + 2

    function line(from, to, amplitude) {
        const points = []
        if (to <= from)
            return points
        const steps = Math.max(2, Math.ceil((to - from) / 2))
        for (let i = 0; i <= steps; i++) {
            const x = from + (to - from) * i / steps
            points.push(Qt.point(x, mid + amplitude * Math.sin(2 * Math.PI * (x - startX) / wavelength)))
        }
        return points
    }

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer

        ShapePath {
            strokeColor: root.trackColor
            strokeWidth: root.thickness
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            PathPolyline {
                path: root.line(Math.min(root.end + (root.shown > 0.005 ? root.thickness + 4 : 0), root.endX),
                                root.endX, 0)
            }
        }
        ShapePath {
            strokeColor: root.shown > 0.005 ? root.color : "transparent"
            strokeWidth: root.thickness
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathPolyline {
                path: root.line(root.startX, Math.max(root.startX + 0.5, root.end), root.amplitude)
            }
        }
    }

    Rectangle {
        visible: root.shown < 0.97
        width: root.thickness * 0.55
        height: width
        radius: width / 2
        color: root.color
        x: root.endX - width / 2
        y: root.mid - height / 2
    }
}
