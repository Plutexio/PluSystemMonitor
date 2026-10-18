import QtQuick
import QtQuick.Shapes
import PluSystemMonitor

// Stroke icons drawn on a 24×24 grid, so they scale cleanly and follow the theme.
Shape {
    id: root

    property string name
    property color color: Theme.text
    property real size: 18

    readonly property var paths: ({
        plus: "M12 5 L12 19 M5 12 L19 12",
        close: "M6 6 L18 18 M18 6 L6 18",
        check: "M5 12.5 L10 17.5 L19 7",
        edit: "M4 20 L4 16 L15 5 L19 9 L8 20 Z M13 7 L17 11",
        theme: "M12 3 C12 3 5 10.5 5 15 A7 7 0 0 0 19 15 C19 10.5 12 3 12 3 Z",
        plug: "M9 3 L9 8 M15 3 L15 8 M6 8 L18 8 L18 11 A6 6 0 0 1 6 11 Z M12 17 L12 21",
        tune: "M4 7 L7 7 M11 7 L20 7 M4 17 L13 17 M17 17 L20 17 M11 7 A2 2 0 1 1 7 7 A2 2 0 1 1 11 7 M17 17 A2 2 0 1 1 13 17 A2 2 0 1 1 17 17",
        reset: "M5 12 A7 7 0 1 0 7.05 7.05 M7 3 L7 7 L11 7",
        folder: "M3 6 L10 6 L12 8 L21 8 L21 19 L3 19 Z",
        grid: "M4 4 L10 4 L10 10 L4 10 Z M14 4 L20 4 L20 10 L14 10 Z M4 14 L10 14 L10 20 L4 20 Z M14 14 L20 14 L20 20 L14 20 Z",
        resize: "M20 13 L13 20 M20 7 L7 20",
        alert: "M12 4 L21 20 L3 20 Z M12 10 L12 14 M12 17 L12 17.5",
        search: "M10.5 17 A6.5 6.5 0 1 1 10.5 4 A6.5 6.5 0 1 1 10.5 17 Z M15.5 15.5 L20 20",
        columns: "M4 5 L20 5 L20 19 L4 19 Z M10 5 L10 19 M15 5 L15 19",
        pause: "M9 6 L9 18 M15 6 L15 18",
        play: "M8 5 L19 12 L8 19 Z",
        power: "M12 3 L12 11 M7.5 6 A7.5 7.5 0 1 0 16.5 6",
        chevronUp: "M6 15 L12 9 L18 15",
        chevronDown: "M6 9 L12 15 L18 9"
    })

    width: size
    height: size
    preferredRendererType: Shape.CurveRenderer

    ShapePath {
        scale: Qt.size(root.size / 24, root.size / 24)
        strokeColor: root.color
        strokeWidth: 2
        fillColor: "transparent"
        capStyle: ShapePath.RoundCap
        joinStyle: ShapePath.RoundJoin

        PathSvg { path: root.paths[root.name] ?? "" }
    }
}
