import QtQuick
import PluSystemMonitor

// A pill with a scalloped edge (wavy_pill.frag). Flat at rest; hovered, the
// scallops rise and start running round the edge, and slow to a stop where
// they are when the pointer leaves. With no shapes it stays a plain pill.
ShaderEffect {
    id: root

    property color color: Appearance.colors.secondaryContainer
    property bool hovered: false
    property bool pressed: false
    property real restAmplitude: 0

    property size itemSize: Qt.size(width, height)
    property real cornerRadius: Math.min(width, height) / 2
    property real amplitude: Appearance.noShapes ? 0 : (pressed ? 3 : (hovered ? 2.2 : restAmplitude))
    property real wavelength: 14
    readonly property real phase: drift.value

    Behavior on amplitude {
        enabled: !Appearance.performanceMode
        Bounce {}
    }

    Drift {
        id: drift
        active: root.hovered && root.visible && !Appearance.noShapes
        speed: 2 * Math.PI / 1.6
    }

    fragmentShader: "qrc:/shaders/wavy_pill.frag.qsb"
}
