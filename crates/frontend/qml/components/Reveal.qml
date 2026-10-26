import QtQuick
import PluSystemMonitor

// Shows and hides `target` instead of a sudden `visible`: it grows in from
// 0.85 and brightens with an overshoot, and shrinks and fades away, hidden
// only once it has gone. The first state applies at once, so a view being
// built doesn't animate. Put it beside the target, not in a positioner.
Item {
    id: root

    required property Item target
    property bool shown: true
    property real overshoot: 2.2
    property bool ready: false

    function settle() {
        show.stop()
        hide.stop()
        target.visible = shown
        target.opacity = shown ? 1 : 0
        target.scale = shown ? 1 : 0.85
    }

    width: 0
    height: 0
    visible: false

    Component.onCompleted: {
        settle()
        ready = true
    }
    onShownChanged: {
        if (!ready || Appearance.performanceMode) {
            settle()
        } else if (shown) {
            hide.stop()
            target.visible = true
            show.restart()
        } else {
            show.stop()
            hide.restart()
        }
    }

    ParallelAnimation {
        id: show
        NumberAnimation { target: root.target; property: "scale"; to: 1; duration: 440; easing.type: Easing.OutBack; easing.overshoot: root.overshoot }
        NumberAnimation { target: root.target; property: "opacity"; to: 1; duration: 200; easing.type: Easing.OutCubic }
    }

    SequentialAnimation {
        id: hide
        ParallelAnimation {
            NumberAnimation { target: root.target; property: "scale"; to: 0.85; duration: 200; easing.type: Easing.InCubic }
            NumberAnimation { target: root.target; property: "opacity"; to: 0; duration: 150 }
        }
        PropertyAction { target: root.target; property: "visible"; value: false }
    }
}
