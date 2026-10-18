import QtQuick
import QtQuick.Effects
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
    property color color: Theme.cpu
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
            color: Theme.grid
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
            duration: root.interval
        }

        Shape {
            anchors.fill: parent
            preferredRendererType: Shape.CurveRenderer

            ShapePath {
                strokeColor: "transparent"
                fillGradient: LinearGradient {
                    y1: 0
                    y2: root.height
                    GradientStop { position: 0.0; color: Qt.alpha(root.color, 0.38 * root.fillOpacity) }
                    GradientStop { position: 1.0; color: Qt.alpha(root.color, 0.0) }
                }
                PathPolyline { path: root.fillPoints }
            }

            ShapePath {
                strokeColor: root.color
                strokeWidth: 2
                fillColor: "transparent"
                joinStyle: ShapePath.RoundJoin
                capStyle: ShapePath.RoundCap
                PathPolyline { path: root.linePoints }
            }
        }

        // Glowing head on the newest point.
        Rectangle {
            visible: root.values.length > 0
            width: 9
            height: 9
            radius: 4.5
            x: root.width - width / 2
            y: (root.linePoints.length ? root.linePoints[root.linePoints.length - 1].y : root.height) - height / 2
            color: root.color
            layer.enabled: Theme.glow > 0
            layer.effect: MultiEffect {
                shadowEnabled: true
                shadowColor: root.color
                shadowBlur: 1.0
                shadowOpacity: Theme.glow
                shadowHorizontalOffset: 0
                shadowVerticalOffset: 0
            }

            Behavior on y {
                NumberAnimation { duration: root.interval / 2; easing.type: Easing.OutCubic }
            }
        }
    }
}
