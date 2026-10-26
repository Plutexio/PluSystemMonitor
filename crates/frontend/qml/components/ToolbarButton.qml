import QtQuick
import PluSystemMonitor

// A top-bar button: bigger than an icon button, and lively. It stands on a
// shape of its own, as the top bar has no background and the page scrolls
// up under it. Hovered, it grows a touch and its shape takes the hover
// colour and slowly turns; pressed, it squishes; clicked, it nudges, spins
// or pops, by `effect`, on transforms of their own so they never fight the
// hover and press springs.
IconButton {
    id: root

    enum Effect {
        Pop,
        NudgeLeft,
        NudgeRight,
        Spin
    }

    property int effect: ToolbarButton.Pop
    property string shape: "cookie9"
    property real shapeAngle: 0
    property bool turnsOnHover: true
    property color shapeColor: scheme.surfaceContainerHigh
    property color shapeHoverColor: scheme.secondaryContainer
    property color shapePressedColor: scheme.secondaryContainerActive

    size: 46
    iconSize: 24
    // The shape behind it is its state layer: no round one, and no round
    // ripple, under it.
    colBackgroundHover: "transparent"
    colBackgroundToggledHover: "transparent"
    rippleEnabled: false
    bounce: false
    scale: down ? 0.86 : (hovered && enabled ? 1.08 : 1)

    Behavior on scale {
        enabled: !Appearance.performanceMode
        Bounce {}
    }

    onClicked: {
        if (Appearance.performanceMode)
            return
        switch (effect) {
        case ToolbarButton.NudgeLeft:
        case ToolbarButton.NudgeRight:
            nudge.restart()
            break
        case ToolbarButton.Spin:
            spin.restart()
            break
        default:
            popIn.restart()
        }
    }

    transform: [
        Translate {
            id: shift
        },
        Rotation {
            id: turn
            origin.x: root.width / 2
            origin.y: root.height / 2
        },
        Scale {
            id: pop
            origin.x: root.width / 2
            origin.y: root.height / 2
        }
    ]

    MaterialShape {
        anchors.centerIn: parent
        width: root.width
        height: root.height
        z: -0.5
        shape: root.shape
        spinAngle: root.shapeAngle + drift.value
        color: root.down ? root.shapePressedColor : (root.hovered && root.enabled ? root.shapeHoverColor : root.shapeColor)
        scale: root.hovered && root.enabled ? 1 : 0.9

        Behavior on scale {
            enabled: !Appearance.performanceMode
            Bounce {}
        }
        Behavior on color {
            ElementMoveFastColor {}
        }

        // Eases into its turn on hover and out of it after, carrying on from
        // wherever it stopped.
        Drift {
            id: drift
            active: root.turnsOnHover && root.hovered && root.visible
            speed: 40
            period: 360
        }
    }

    SequentialAnimation {
        id: nudge
        NumberAnimation {
            target: shift
            property: "x"
            to: root.effect === ToolbarButton.NudgeLeft ? -6 : 6
            duration: 90
            easing.type: Easing.OutCubic
        }
        NumberAnimation { target: shift; property: "x"; to: 0; duration: 420; easing.type: Easing.OutBack; easing.overshoot: 2.4 }
    }

    NumberAnimation {
        id: spin
        target: turn
        property: "angle"
        from: 0
        to: 180
        duration: 520
        easing.type: Easing.OutBack
        easing.overshoot: 1.6
    }

    SequentialAnimation {
        id: popIn
        ParallelAnimation {
            NumberAnimation { target: pop; property: "xScale"; to: 1.22; duration: 100; easing.type: Easing.OutCubic }
            NumberAnimation { target: pop; property: "yScale"; to: 1.22; duration: 100; easing.type: Easing.OutCubic }
        }
        ParallelAnimation {
            NumberAnimation { target: pop; property: "xScale"; to: 1; duration: 420; easing.type: Easing.OutBack; easing.overshoot: 2.6 }
            NumberAnimation { target: pop; property: "yScale"; to: 1; duration: 420; easing.type: Easing.OutBack; easing.overshoot: 2.6 }
        }
    }
}
