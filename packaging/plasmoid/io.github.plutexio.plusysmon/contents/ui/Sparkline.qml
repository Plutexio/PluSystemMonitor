import QtQuick
import QtQuick.Shapes

// Recent history as a line over a fading fill. Each new point slides in
// over one interval rather than jumping.
Item {
    id: root

    property var values: []
    property real maxValue: 1
    property color color: "steelblue"
    property real lineWidth: 2
    property bool fill: true
    /// How many points fit across.
    property int points: 60
    property int interval: 1000

    readonly property var shownValues: (values ?? []).slice(-points - 1)
    readonly property real step: width / Math.max(1, points - 1)
    property real slide: 0

    clip: true

    // The newest points, to tell a new sample from the same history again.
    property string lastKey
    onValuesChanged: {
        const key = (values ?? []).slice(-3).join(",") + "/" + (values ?? []).length
        if (key === lastKey)
            return
        const first = lastKey === ""
        lastKey = key
        if (first)
            return
        slideIn.stop()
        slide = step
        slideIn.start()
    }

    NumberAnimation {
        id: slideIn
        target: root
        property: "slide"
        to: 0
        duration: Math.max(100, root.interval - 50)
    }

    function yOf(v) {
        const top = lineWidth
        const bottom = height - lineWidth / 2
        return bottom - (bottom - top) * Math.max(0, Math.min(1, v / Math.max(1e-9, maxValue)))
    }

    readonly property var line: {
        const n = shownValues.length
        const out = []
        for (let i = 0; i < n; i++)
            out.push(Qt.point(width - (n - 1 - i) * step + slide, yOf(shownValues[i])))
        return out
    }

    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer
        visible: root.line.length > 1

        ShapePath {
            strokeColor: "transparent"
            fillGradient: LinearGradient {
                x1: 0; y1: 0
                x2: 0; y2: root.height
                GradientStop { position: 0; color: Qt.alpha(root.color, root.fill ? 0.38 : 0) }
                GradientStop { position: 1; color: Qt.alpha(root.color, 0) }
            }
            PathPolyline {
                path: root.line.length > 1
                      ? [Qt.point(root.line[0].x, root.height)].concat(root.line,
                            [Qt.point(root.line[root.line.length - 1].x, root.height)])
                      : []
            }
        }
        ShapePath {
            strokeColor: root.color
            strokeWidth: root.lineWidth
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathPolyline { path: root.line }
        }
    }
}
