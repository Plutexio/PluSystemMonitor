import QtQuick
import QtQuick.Shapes

// Material 3 Expressive's wavy circular indicator opened at the bottom like a
// gauge, as the app draws it: the value rides a wave round the ring, the rest
// is a flat track to a stop dot.
Item {
    id: root

    /// 0..1
    property real value: 0
    property color color: "steelblue"
    property color trackColor: Qt.alpha(color, 0.25)
    property bool wavy: true
    readonly property real size: Math.min(width, height)
    property real thickness: Math.max(2.5, size * 0.065)

    property real shown: Math.max(0, Math.min(1, value))
    Behavior on shown {
        NumberAnimation { duration: 650; easing.type: Easing.OutCubic }
    }

    readonly property real amplitude: wavy && size >= 40 ? thickness * 0.28 : 0
    readonly property real radius: Math.max(1, (size - thickness) / 2 - amplitude)
    // Small, close waves: one every 2.8 strokes' width.
    readonly property real waves: Math.max(8, Math.round(2 * Math.PI * radius / (thickness * 2.8)))
    readonly property real startAngle: 0.75 * Math.PI
    readonly property real sweep: 1.5 * Math.PI
    // The gap between value and track, as an angle; caps stick out by half a stroke.
    readonly property real gap: (thickness + 3) / radius
    readonly property real endAngle: startAngle + sweep * shown

    function arc(from, to, amplitude) {
        const points = []
        if (to <= from)
            return points
        const cx = width / 2, cy = height / 2
        const steps = Math.max(2, Math.ceil((to - from) / (2 * Math.PI) * waves * 14))
        for (let i = 0; i <= steps; i++) {
            const a = from + (to - from) * i / steps
            const r = radius + amplitude * Math.sin(waves * (a - startAngle))
            points.push(Qt.point(cx + r * Math.cos(a), cy + r * Math.sin(a)))
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
            joinStyle: ShapePath.RoundJoin
            PathPolyline {
                path: root.arc(Math.min(root.endAngle + (root.shown > 0.005 ? root.gap : 0),
                                        root.startAngle + root.sweep),
                               root.startAngle + root.sweep, 0)
            }
        }
        ShapePath {
            strokeColor: root.shown > 0.005 ? root.color : "transparent"
            strokeWidth: root.thickness
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathPolyline {
                path: root.arc(root.startAngle, Math.max(root.startAngle + 0.001, root.endAngle), root.amplitude)
            }
        }
    }

    // The stop dot at the end of the track.
    Rectangle {
        readonly property real angle: root.startAngle + root.sweep
        visible: root.shown < 0.97
        width: root.thickness * 0.55
        height: width
        radius: width / 2
        color: root.color
        x: root.width / 2 + root.radius * Math.cos(angle) - width / 2
        y: root.height / 2 + root.radius * Math.sin(angle) - height / 2
    }
}
