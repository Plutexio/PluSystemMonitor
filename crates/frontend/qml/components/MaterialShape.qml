import QtQuick
import QtQuick.Shapes
import PluSystemMonitor
import "../design/ShapePaths.js" as ShapePaths

// One of Material 3 Expressive's shapes, by name (Shapes in the style guide):
// never a hand-drawn path. It can morph into `morphTo` and turn. With no
// shapes (Appearance.noShapes) a decorative one is a circle; one that means
// something keeps its shape (`keepsShape`).
Item {
    id: root

    property string shape: "circle"
    property string morphTo: shape
    /// 0 is `shape`, 1 is `morphTo`; a spring may carry it a little past
    property real morph: 0
    property color color: Appearance.colors.secondaryContainer
    property real spinAngle: 0
    property bool keepsShape: false
    /// How much of the item the shape fills, leaving room for an overshoot.
    property real extent: 1

    readonly property bool plain: Appearance.noShapes && !keepsShape
    readonly property string path: plain ? ShapePaths.path("circle")
                                         : ShapePaths.morph(shape, morphTo, morph)

    Shape {
        anchors.centerIn: parent
        width: root.width * root.extent
        height: root.height * root.extent
        preferredRendererType: Shape.CurveRenderer
        rotation: root.plain ? 0 : root.spinAngle

        ShapePath {
            scale: Qt.size(root.width * root.extent, root.height * root.extent)
            fillColor: root.color
            strokeColor: "transparent"
            strokeWidth: 0

            PathSvg { path: root.path }
        }
    }
}
