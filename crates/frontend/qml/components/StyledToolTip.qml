import QtQuick
import QtQuick.Templates as T
import PluSystemMonitor

// A Material plain tooltip above what it describes, clear of the pointer (one
// under the pointer took the hover from the button and swallowed its
// clicks). It springs in from the control's edge.
T.ToolTip {
    id: root

    x: parent ? Math.round((parent.width - width) / 2) : 0
    y: -height - 8
    delay: 600
    timeout: 6000
    margins: 6
    closePolicy: T.Popup.CloseOnPressOutsideParent | T.Popup.CloseOnReleaseOutsideParent
    transformOrigin: T.Popup.Bottom
    padding: 6
    leftPadding: 10
    rightPadding: 10
    implicitWidth: Math.min(implicitContentWidth + leftPadding + rightPadding, 320)
    implicitHeight: implicitContentHeight + topPadding + bottomPadding

    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Appearance.duration(150); easing.type: Easing.OutCubic }
        NumberAnimation { property: "scale"; from: 0.6; to: 1; duration: Appearance.motion(380); easing.type: Easing.OutBack; easing.overshoot: 2.2 }
    }
    exit: Transition {
        NumberAnimation { property: "opacity"; to: 0; duration: Appearance.duration(120) }
        NumberAnimation { property: "scale"; to: 0.8; duration: Appearance.motion(120); easing.type: Easing.InCubic }
    }

    contentItem: StyledText {
        text: root.text
        color: Appearance.colors.onTooltip
        font.pixelSize: Appearance.fontSize.smaller
        wrapMode: Text.Wrap
    }

    background: Rectangle {
        radius: Appearance.rounding.verySmall
        color: Appearance.colors.tooltip
    }
}
