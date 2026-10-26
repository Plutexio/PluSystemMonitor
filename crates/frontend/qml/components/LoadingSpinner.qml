import QtQuick
import PluSystemMonitor

// Material 3 Expressive's loading indicator: a shape that morphs through the
// shape library with a spring, kicking round a quarter turn each time while
// it slowly turns. With reduced motion or no shapes it breathes instead.
// Nothing animates while it is hidden.
Item {
    id: root

    property bool running: true
    property color color: Appearance.colors.primary

    readonly property list<string> sequence: ["softBurst", "cookie9", "pentagon", "pill", "sunny", "cookie4", "oval"]
    property int step: 0
    property real morph: 0
    property real turn: 0
    property real kick: 0
    readonly property bool animating: running && visible && !Appearance.reduceMotion && !Appearance.noShapes
    readonly property bool pulsing: running && visible && (Appearance.reduceMotion || Appearance.noShapes)
                                    && !Appearance.performanceMode

    implicitWidth: 40
    implicitHeight: 40

    Behavior on kick {
        SpringAnimation { spring: 3; damping: 0.3; epsilon: 0.05 }
    }

    MaterialShape {
        id: body
        anchors.centerIn: parent
        width: parent.width * 0.86
        height: width
        shape: root.sequence[root.step]
        morphTo: root.sequence[(root.step + 1) % root.sequence.length]
        morph: root.morph
        color: root.color
        spinAngle: (root.turn + root.kick) % 360

        SequentialAnimation on opacity {
            running: root.pulsing
            loops: Animation.Infinite
            onRunningChanged: if (!running) body.opacity = 1
            NumberAnimation { to: 0.35; duration: 700; easing.type: Easing.InOutSine }
            NumberAnimation { to: 1; duration: 700; easing.type: Easing.InOutSine }
        }
    }

    NumberAnimation on turn {
        running: root.animating
        from: 0
        to: 360
        duration: 4800
        loops: Animation.Infinite
    }

    SequentialAnimation {
        running: root.animating
        loops: Animation.Infinite

        ScriptAction { script: root.kick += 90 }
        NumberAnimation {
            target: root
            property: "morph"
            from: 0
            to: 1
            duration: 650
            easing.type: Easing.OutBack
            easing.overshoot: 2
        }
        ScriptAction {
            script: {
                root.step = (root.step + 1) % root.sequence.length
                root.morph = 0
            }
        }
    }
}
