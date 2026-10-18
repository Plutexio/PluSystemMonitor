import QtQuick
import QtQuick.Shapes
import PluSystemMonitor

// Slowly drifting colour glows behind the cards. Radial gradients fading to
// transparent give the soft look of a heavy blur without any blur pass.
Rectangle {
    id: root

    property bool running: true

    color: Theme.background

    Repeater {
        model: [
            { color: Theme.cpu, size: 1.10, x: 0.15, y: 0.20, dx: 0.25, dy: 0.15, period: 31000 },
            { color: Theme.mem, size: 0.95, x: 0.70, y: 0.75, dx: -0.30, dy: -0.20, period: 37000 },
            { color: Theme.swap, size: 0.70, x: 0.80, y: 0.05, dx: -0.15, dy: 0.30, period: 43000 }
        ]

        delegate: Shape {
            id: blob

            required property var modelData
            property real phase: 0

            readonly property real diameter: Math.max(root.width, root.height) * modelData.size

            width: diameter
            height: diameter
            x: root.width * (modelData.x + modelData.dx * phase) - diameter / 2
            y: root.height * (modelData.y + modelData.dy * phase) - diameter / 2

            ShapePath {
                strokeColor: "transparent"
                fillGradient: RadialGradient {
                    centerX: blob.diameter / 2
                    centerY: blob.diameter / 2
                    centerRadius: blob.diameter / 2
                    focalX: centerX
                    focalY: centerY
                    GradientStop { position: 0.0; color: Qt.alpha(blob.modelData.color, 0.32) }
                    GradientStop { position: 0.45; color: Qt.alpha(blob.modelData.color, 0.12) }
                    GradientStop { position: 1.0; color: Qt.alpha(blob.modelData.color, 0.0) }
                }

                PathLine { x: blob.diameter; y: 0 }
                PathLine { x: blob.diameter; y: blob.diameter }
                PathLine { x: 0; y: blob.diameter }
                PathLine { x: 0; y: 0 }
            }

            SequentialAnimation on phase {
                running: root.running && Theme.animatedBackground
                loops: Animation.Infinite

                NumberAnimation { to: 1; duration: blob.modelData.period; easing.type: Easing.InOutSine }
                NumberAnimation { to: 0; duration: blob.modelData.period; easing.type: Easing.InOutSine }
            }
        }
    }

    // Vignette keeps the edges calm so the cards stay the focus.
    Rectangle {
        anchors.fill: parent
        gradient: Gradient {
            GradientStop { position: 0.0; color: Qt.rgba(0, 0, 0, 0.0) }
            GradientStop { position: 1.0; color: Qt.rgba(0, 0, 0, 0.45) }
        }
    }
}
