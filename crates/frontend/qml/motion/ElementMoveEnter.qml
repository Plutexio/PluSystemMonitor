import QtQuick
import PluSystemMonitor

// Something arriving: it decelerates into place.
NumberAnimation {
    duration: Appearance.motion(400)
    easing.type: Easing.BezierSpline
    easing.bezierCurve: Appearance.curves.emphasizedDecel
}
