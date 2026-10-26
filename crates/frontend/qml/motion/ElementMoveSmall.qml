import QtQuick
import PluSystemMonitor

// A short move: a thumb, a toast, a pill sliding to its choice.
NumberAnimation {
    duration: Appearance.motion(350)
    easing.type: Easing.BezierSpline
    easing.bezierCurve: Appearance.curves.expressiveFastSpatial
}
