import QtQuick
import QtQuick.Shapes
import PluSystemMonitor

// Area graph whose newest point sits on the right edge. Each new sample the
// whole curve glides one step left over one interval instead of jumping.
Item {
    id: root

    property list<real> values
    property int maxPoints: 120
    property real maxValue: 100
    property int interval: 1000
    property color color: Appearance.colors.cpu
    /// Off for series overlaid on another graph, so the grid isn't drawn twice.
    property bool showGrid: true
    property real fillOpacity: 1

    readonly property real step: width / Math.max(1, maxPoints - 1)
    property list<point> linePoints
    property list<point> fillPoints

    function yFor(v) {
        // Keep a little headroom so 100% doesn't touch the top edge.
        return height - Math.min(1, Math.max(0, v / maxValue)) * height * 0.92
    }

    function rebuild() {
        const n = values.length
        const line = []
        for (let i = 0; i < n; i++)
            line.push(Qt.point(width - (n - 1 - i) * step, yFor(values[i])))
        linePoints = line
        fillPoints = n > 0 ? line.concat([Qt.point(width, height), Qt.point(line[0].x, height)]) : []
    }

    clip: true
    onValuesChanged: {
        rebuild()
        slide.restart()
    }
    onWidthChanged: rebuild()
    onHeightChanged: rebuild()

    Repeater {
        model: root.showGrid ? 3 : 0

        Rectangle {
            required property int index
            width: root.width
            height: 1
            y: root.height - root.height * 0.92 * (index + 1) / 3
            color: Qt.alpha(Appearance.colors.outlineVariant, 0.6)
        }
    }

    Item {
        id: track

        width: root.width
        height: root.height

        NumberAnimation on x {
            id: slide
            from: root.step
            to: 0
            // With no animations the curve steps instead of gliding.
            duration: Appearance.motion(root.interval)
        }

        Shape {
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                strokeColor: "transparent"
                fillGradient: LinearGradient {
                    y1: 0
                    y2: root.height
                    GradientStop { position: 0.0; color: Qt.alpha(root.color, 0.32 * root.fillOpacity) }
                    GradientStop { position: 1.0; color: Qt.alpha(root.color, 0.0) }
                }
                PathPolyline { path: root.fillPoints }
            }

            ShapePath {
                strokeColor: root.color
                strokeWidth: 2.5
                fillColor: "transparent"
                joinStyle: ShapePath.RoundJoin
                capStyle: ShapePath.RoundCap
                PathPolyline { path: root.linePoints }
            }
        }

        // The newest point: a dot in a soft halo of its own colour.
        Rectangle {
            visible: root.values.length > 0
            width: 16
            height: 16
            radius: 8
            x: root.width - width / 2
            y: (root.linePoints.length ? root.linePoints[root.linePoints.length - 1].y : root.height) - height / 2
            color: Qt.alpha(root.color, 0.28)

            Behavior on y {
                enabled: !Appearance.performanceMode
                NumberAnimation { duration: root.interval / 2; easing.type: Easing.OutCubic }
            }

            Rectangle {
                anchors.centerIn: parent
                width: 8
                height: 8
                radius: 4
                color: root.color
            }
        }
    }
}
