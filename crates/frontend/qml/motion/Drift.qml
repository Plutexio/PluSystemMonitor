import QtQuick
import PluSystemMonitor

// A phase that runs while `active`: it eases up to speed and back down to
// rest rather than starting and stopping dead, and carries on from wherever
// it stopped (a looping animation from 0 jumped back each time it was set
// going again). It asks for frames only while it moves.
FrameAnimation {
    id: root

    property bool active: false
    /// Units a second at full speed; negative runs it backwards
    property real speed: 1
    /// Where the value wraps round to 0
    property real period: 2 * Math.PI
    property real value: 0
    property real rate: active && !Appearance.reduceMotion ? speed : 0

    Behavior on rate {
        enabled: !Appearance.performanceMode
        NumberAnimation { duration: 650; easing.type: Easing.InOutSine }
    }

    running: Math.abs(rate) > 0.0001
    onTriggered: value = (value + rate * frameTime) % period
}
