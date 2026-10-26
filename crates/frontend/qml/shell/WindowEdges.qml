import QtQuick
import PluSystemMonitor

// The frameless window's borders: grabbing an edge or a corner hands the
// resize to the compositor.
Item {
    id: root

    required property Window window
    property real grip: 6

    component Edge: Item {
        id: edge

        required property int edges
        property int cursor: Qt.ArrowCursor

        HoverHandler {
            cursorShape: edge.cursor
        }
        DragHandler {
            target: null
            onActiveChanged: if (active) root.window.startSystemResize(edge.edges)
        }
    }

    Edge { edges: Qt.TopEdge; cursor: Qt.SizeVerCursor; x: root.grip; width: root.width - 2 * root.grip; height: root.grip }
    Edge { edges: Qt.BottomEdge; cursor: Qt.SizeVerCursor; x: root.grip; y: root.height - root.grip; width: root.width - 2 * root.grip; height: root.grip }
    Edge { edges: Qt.LeftEdge; cursor: Qt.SizeHorCursor; y: root.grip; width: root.grip; height: root.height - 2 * root.grip }
    Edge { edges: Qt.RightEdge; cursor: Qt.SizeHorCursor; x: root.width - root.grip; y: root.grip; width: root.grip; height: root.height - 2 * root.grip }
    Edge { edges: Qt.TopEdge | Qt.LeftEdge; cursor: Qt.SizeFDiagCursor; width: root.grip * 2; height: root.grip * 2 }
    Edge { edges: Qt.TopEdge | Qt.RightEdge; cursor: Qt.SizeBDiagCursor; x: root.width - width; width: root.grip * 2; height: root.grip * 2 }
    Edge { edges: Qt.BottomEdge | Qt.LeftEdge; cursor: Qt.SizeBDiagCursor; y: root.height - height; width: root.grip * 2; height: root.grip * 2 }
    Edge { edges: Qt.BottomEdge | Qt.RightEdge; cursor: Qt.SizeFDiagCursor; x: root.width - width; y: root.height - height; width: root.grip * 2; height: root.grip * 2 }
}
