import QtQuick
import PluSystemMonitor

// Material 3 Expressive's wavy circular indicator opened at the bottom like a
// gauge (wavy_arc.frag): the value rides a wave round the ring, the rest is a
// flat track to a stop dot. The number in the middle is set round.
Item {
    id: root

    property real value: 0
    property real maxValue: 100
    property color color: Appearance.colors.cpu
    property string label
    /// Shown instead of the animated percentage, for values that aren't one.
    property string valueText
    property string caption

    readonly property real size: Math.min(width, height)
    property real shown: value

    Behavior on shown {
        enabled: !Appearance.performanceMode
        NumberAnimation { duration: 650; easing.type: Easing.OutCubic }
    }

    ShaderEffect {
        id: ring

        readonly property size itemSize: Qt.size(width, height)
        readonly property color color: root.color
        readonly property color trackColor: Appearance.colors.tonal(root.color)
        readonly property real progress: Math.max(0, Math.min(1, root.shown / root.maxValue))
        readonly property real thickness: Math.max(7, root.size * 0.065)
        readonly property real amplitude: Appearance.noShapes ? 0 : thickness * 0.28
        readonly property real radius: (root.size - thickness) / 2 - amplitude - 2
        // Small, close waves, as Material's are: one every 2.8 strokes' width.
        readonly property real waves: Math.max(10, Math.round(2 * Math.PI * radius / (thickness * 2.8)))
        readonly property real startAngle: 0.75 * Math.PI
        readonly property real sweep: 1.5 * Math.PI
        readonly property real gap: 4

        anchors.centerIn: parent
        width: root.size
        height: root.size
        fragmentShader: "qrc:/shaders/wavy_arc.frag.qsb"
    }

    Column {
        anchors.centerIn: parent
        spacing: 0

        TitleText {
            anchors.horizontalCenter: parent.horizontalCenter
            text: root.valueText || Math.round(root.shown) + "%"
            font.pixelSize: Math.max(14, root.size * (root.valueText ? 0.15 : 0.22))
            tabularNumbers: true
        }
        StyledText {
            anchors.horizontalCenter: parent.horizontalCenter
            // Small gauges keep just the number readable.
            visible: root.size >= 100 && root.label.length > 0
            text: root.label
            color: Appearance.colors.onSurfaceVariant
            font.pixelSize: Math.max(Appearance.fontSize.smaller, root.size * 0.065)
        }
    }

    StyledText {
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: root.size * 0.04
        // It sits in the ring's open bottom, which only has room on larger gauges.
        visible: root.size >= 150
        text: root.caption
        color: Appearance.colors.onSurfaceVariant
        font.pixelSize: Appearance.fontSize.smaller
        tabularNumbers: true
    }
}
