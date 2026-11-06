import QtQuick
import QtQuick.Shapes
import PluSystemMonitor

// A fan curve to shape by hand: speed (up) by input (across). Drag a point
// to move it, double-click the empty graph to add one and a point to remove
// it. Where the input is now shows as a line through the curve. Moves stay
// here while dragging and go out (`edited`) on release.
Item {
    id: root

    /// `[[input, percent], ...]`, ascending by input.
    property var points: []
    property real xMin: 20
    property real xMax: 100
    property string unit: "celsius"
    /// The input now, or null.
    property var input: null
    property real hysteresis: 0
    property color color: Appearance.colors.primary

    signal edited(var points)

    // What is drawn: `points`, or the drag in progress.
    property var draft: points
    property int dragging: -1
    onPointsChanged: if (dragging < 0) draft = points

    readonly property real padLeft: 52
    readonly property real padRight: 16
    readonly property real padTop: 16
    readonly property real padBottom: 30
    readonly property real plotWidth: Math.max(1, width - padLeft - padRight)
    readonly property real plotHeight: Math.max(1, height - padTop - padBottom)
    readonly property real xStep: {
        const span = xMax - xMin
        return span <= 40 ? 5 : span <= 100 ? 10 : span <= 250 ? 25 : Math.pow(10, Math.floor(Math.log10(span / 2)))
    }

    function toX(value) { return padLeft + (value - xMin) / (xMax - xMin) * plotWidth }
    function toY(percent) { return padTop + (1 - percent / 100) * plotHeight }
    function fromX(px) { return xMin + (px - padLeft) / plotWidth * (xMax - xMin) }
    function fromY(py) { return (1 - (py - padTop) / plotHeight) * 100 }
    function clamp(v, lo, hi) { return Math.max(lo, Math.min(hi, v)) }

    function speedAt(input) {
        const p = draft
        if (!p.length)
            return 100
        if (input <= p[0][0])
            return p[0][1]
        for (let i = 1; i < p.length; i++) {
            if (input <= p[i][0])
                return p[i][0] <= p[i - 1][0] ? p[i][1]
                     : p[i - 1][1] + (p[i][1] - p[i - 1][1]) * (input - p[i - 1][0]) / (p[i][0] - p[i - 1][0])
        }
        return p[p.length - 1][1]
    }

    function label(value) {
        return Format.metric(value, unit)
    }

    // The curve, flat beyond its ends as the daemon has it.
    readonly property var linePoints: {
        const p = draft
        if (!p.length)
            return []
        const out = [Qt.point(padLeft, toY(p[0][1]))]
        for (const [x, y] of p)
            out.push(Qt.point(clamp(toX(x), padLeft, padLeft + plotWidth), toY(y)))
        out.push(Qt.point(padLeft + plotWidth, toY(p[p.length - 1][1])))
        return out
    }

    // Grid: speeds across, inputs up.
    Repeater {
        model: [0, 25, 50, 75, 100]

        Item {
            required property int modelData
            y: root.toY(modelData)
            width: root.width
            height: 1

            Rectangle {
                x: root.padLeft
                width: root.plotWidth
                height: 1
                color: Appearance.colors.outlineVariant
                opacity: modelData === 0 ? 0.9 : 0.45
            }
            StyledText {
                anchors.right: parent.left
                anchors.rightMargin: -root.padLeft + 10
                anchors.verticalCenter: parent.verticalCenter
                text: modelData + " %"
                color: Appearance.colors.onSurfaceVariant
                font.pixelSize: Appearance.fontSize.smallest
                tabularNumbers: true
            }
        }
    }
    Repeater {
        model: {
            const ticks = []
            for (let v = Math.ceil(root.xMin / root.xStep) * root.xStep; v <= root.xMax + 1e-6; v += root.xStep)
                ticks.push(v)
            return ticks
        }

        Item {
            required property real modelData
            x: root.toX(modelData)
            y: root.padTop
            width: 1
            height: root.plotHeight

            Rectangle {
                width: 1
                height: parent.height
                color: Appearance.colors.outlineVariant
                opacity: 0.25
            }
            StyledText {
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.top: parent.bottom
                anchors.topMargin: 8
                text: root.label(modelData)
                color: Appearance.colors.onSurfaceVariant
                font.pixelSize: Appearance.fontSize.smallest
                tabularNumbers: true
            }
        }
    }

    // The area under the curve, fading down, then the curve itself.
    Shape {
        anchors.fill: parent
        preferredRendererType: Shape.CurveRenderer
        visible: root.linePoints.length > 1

        ShapePath {
            strokeColor: "transparent"
            fillGradient: LinearGradient {
                x1: 0; y1: root.padTop
                x2: 0; y2: root.padTop + root.plotHeight
                GradientStop { position: 0; color: Qt.alpha(root.color, 0.32) }
                GradientStop { position: 1; color: Qt.alpha(root.color, 0.02) }
            }
            startX: root.padLeft
            startY: root.padTop + root.plotHeight
            PathPolyline { path: root.linePoints }
            PathLine { x: root.padLeft + root.plotWidth; y: root.padTop + root.plotHeight }
            PathLine { x: root.padLeft; y: root.padTop + root.plotHeight }
        }
        ShapePath {
            strokeColor: root.color
            strokeWidth: 3
            fillColor: "transparent"
            capStyle: ShapePath.RoundCap
            joinStyle: ShapePath.RoundJoin
            PathPolyline { path: root.linePoints }
        }
    }

    // Double-clicking the graph adds a point there.
    MouseArea {
        x: root.padLeft
        y: root.padTop
        width: root.plotWidth
        height: root.plotHeight
        onDoubleClicked: mouse => {
            const input = Math.round(root.fromX(mouse.x + x))
            const speed = Math.round(root.clamp(root.fromY(mouse.y + y), 0, 100))
            if (root.draft.some(p => p[0] === input) || root.draft.length >= 32)
                return
            const next = root.draft.concat([[input, speed]]).sort((a, b) => a[0] - b[0])
            root.edited(next)
        }
    }

    // Where the input is now.
    Item {
        id: now

        readonly property bool shown: root.input !== null && root.input !== undefined && !isNaN(root.input)
        readonly property real at: shown ? root.clamp(root.input, root.xMin, root.xMax) : root.xMin
        readonly property real speed: root.speedAt(shown ? root.input : 0)

        visible: shown
        x: root.toX(at)
        y: root.padTop
        width: 1
        height: root.plotHeight

        Behavior on x {
            ElementMoveFast {}
        }

        // The band the input may fall through before the speed drops.
        Rectangle {
            x: -width
            width: Math.min(root.hysteresis / (root.xMax - root.xMin) * root.plotWidth, now.x - root.padLeft)
            height: parent.height
            color: Qt.alpha(root.color, 0.08)
            visible: root.hysteresis > 0
        }
        Rectangle {
            width: 2
            x: -1
            height: parent.height
            color: Appearance.colors.tertiary
            opacity: 0.8
        }
        Rectangle {
            x: -7
            y: root.toY(now.speed) - root.padTop - 7
            width: 14
            height: 14
            radius: 7
            color: Appearance.colors.tertiary
            border.width: 3
            border.color: Appearance.colors.layer2
        }
        Rectangle {
            readonly property bool flip: now.x + width + 12 > root.width - root.padRight
            x: flip ? -width - 10 : 10
            y: Math.max(0, root.toY(now.speed) - root.padTop - height - 10)
            width: nowText.implicitWidth + 16
            height: 24
            radius: 12
            color: Appearance.colors.tertiaryContainer

            StyledText {
                id: nowText
                anchors.centerIn: parent
                text: qsTr("%1 → %2 %").arg(root.label(root.input ?? 0)).arg(Math.round(now.speed))
                color: Appearance.colors.onTertiaryContainer
                font.pixelSize: Appearance.fontSize.smaller
                font.weight: Font.DemiBold
                tabularNumbers: true
            }
        }
    }

    // The points: drag to move, double-click to remove.
    Repeater {
        model: root.draft.length

        Item {
            id: handle

            required property int index
            readonly property var point: root.draft[index] ?? [0, 0]
            readonly property bool active: root.dragging === index || area.containsMouse

            x: root.toX(point[0]) - width / 2
            y: root.toY(point[1]) - height / 2
            width: 28
            height: 28
            z: active ? 2 : 1

            MaterialShape {
                anchors.centerIn: parent
                width: handle.active ? 26 : 18
                height: width
                shape: "cookie9"
                color: root.color

                Behavior on width {
                    ElementMoveFast {}
                }
            }
            Rectangle {
                anchors.centerIn: parent
                width: 6
                height: 6
                radius: 3
                color: Appearance.colors.layer2
            }

            // Its values, while it's pointed at or moved.
            Rectangle {
                anchors.horizontalCenter: parent.horizontalCenter
                anchors.bottom: parent.top
                anchors.bottomMargin: 4
                width: pointText.implicitWidth + 16
                height: 24
                radius: 12
                color: Appearance.colors.inverseSurface
                visible: handle.active

                StyledText {
                    id: pointText
                    anchors.centerIn: parent
                    text: qsTr("%1 → %2 %").arg(root.label(handle.point[0])).arg(Math.round(handle.point[1]))
                    color: Appearance.colors.inverseOnSurface
                    font.pixelSize: Appearance.fontSize.smaller
                    font.weight: Font.DemiBold
                    tabularNumbers: true
                }
            }

            MouseArea {
                id: area

                anchors.fill: parent
                hoverEnabled: true
                cursorShape: pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor
                preventStealing: true

                onPressed: root.dragging = handle.index
                onPositionChanged: mouse => {
                    if (!pressed)
                        return
                    const p = mapToItem(root, mouse.x, mouse.y)
                    const i = handle.index
                    const draft = root.draft.map(q => q.slice())
                    // Between its neighbours, so the curve stays a curve.
                    const lo = i > 0 ? draft[i - 1][0] : root.xMin
                    const hi = i < draft.length - 1 ? draft[i + 1][0] : root.xMax
                    draft[i] = [Math.round(root.clamp(root.fromX(p.x), lo, hi)),
                                Math.round(root.clamp(root.fromY(p.y), 0, 100))]
                    root.draft = draft
                }
                onReleased: {
                    root.dragging = -1
                    root.edited(root.draft)
                }
                onDoubleClicked: {
                    if (root.draft.length > 1)
                        root.edited(root.draft.filter((_, i) => i !== handle.index))
                }
            }
        }
    }
}
