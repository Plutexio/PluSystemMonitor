import QtQuick
import PluSystemMonitor

// Colours easing between states. A state is a mix, never a new hue.
ColorAnimation {
    duration: Appearance.duration(200)
    easing.type: Easing.BezierSpline
    easing.bezierCurve: Appearance.curves.expressiveEffects
}
