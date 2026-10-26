import QtQuick
import PluSystemMonitor

// An overlay fading in or out under the pointer.
NumberAnimation {
    duration: Appearance.duration(350)
    easing.type: Easing.BezierSpline
    easing.bezierCurve: Appearance.curves.standard
}
