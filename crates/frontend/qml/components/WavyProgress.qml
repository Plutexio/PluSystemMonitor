import QtQuick
import PluSystemMonitor

// Material 3 Expressive's wavy linear progress: what is done rides a sine
// wave, then a gap, then a flat track to a stop dot. The wave holds still.
// With no shapes the done part is straight too.
ShaderEffect {
    id: root

    /// 0..1
    property real value: 0
    property color color: Appearance.colors.primary
    property color trackColor: Appearance.colors.secondaryContainer
    property real thickness: 4
    property real amplitude: Appearance.noShapes ? 0 : thickness * 0.75
    property real wavelength: 26
    property real phase: 0
    property real gap: 4

    readonly property size itemSize: Qt.size(width, height)
    property real shown: value

    Behavior on shown {
        enabled: !Appearance.performanceMode
        NumberAnimation { duration: 650; easing.type: Easing.OutCubic }
    }

    implicitHeight: thickness + 2 * amplitude + 2
    // What the shader draws: the value, eased.
    readonly property real progress: Math.max(0, Math.min(1, shown))
    fragmentShader: "qrc:/shaders/wavy_progress.frag.qsb"
}
