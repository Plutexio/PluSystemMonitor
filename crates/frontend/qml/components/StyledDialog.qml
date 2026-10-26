import QtQuick
import QtQuick.Templates as T
import PluSystemMonitor

// A Material 3 dialog over a scrim: a shape with a symbol and a headline, a
// line of text, whatever else the dialog needs, and its buttons to the
// right. It scales in on a quick spring and only fades out. Esc closes it.
T.Popup {
    id: root

    property string headline
    property string symbol
    property string iconShape: "cookie9"
    property string body
    property real preferredWidth: 440
    default property alias content: extra.data
    property alias buttons: buttonRow.data
    readonly property real maximumHeight: parent ? parent.height - 48 : 600

    parent: T.Overlay.overlay
    anchors.centerIn: parent
    modal: true
    focus: true
    closePolicy: T.Popup.CloseOnEscape | T.Popup.CloseOnPressOutside
    padding: 24
    width: Math.min(preferredWidth, parent ? parent.width - 48 : preferredWidth)
    implicitHeight: Math.min(maximumHeight, column.implicitHeight + topPadding + bottomPadding)

    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Appearance.duration(150) }
        NumberAnimation { property: "scale"; from: 0.9; to: 1; duration: Appearance.motion(350); easing.type: Easing.BezierSpline; easing.bezierCurve: Appearance.curves.expressiveFastSpatial }
    }
    exit: Transition {
        NumberAnimation { property: "opacity"; to: 0; duration: Appearance.duration(120) }
    }

    T.Overlay.modal: Rectangle {
        color: Appearance.colors.scrimOverlay
    }

    background: Rectangle {
        radius: 28
        color: Appearance.colors.surfaceContainerHigh
    }

    contentItem: Column {
        id: column
        spacing: 16

        Row {
            width: column.width
            visible: root.headline.length > 0
            spacing: 12

            ShapeIcon {
                id: headlineIcon
                anchors.verticalCenter: parent.verticalCenter
                visible: root.symbol.length > 0
                size: 40
                iconSize: 21
                shape: root.iconShape
                symbol: root.symbol
            }
            TitleText {
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - (headlineIcon.visible ? headlineIcon.width + parent.spacing : 0)
                text: root.headline
                wrapMode: Text.Wrap
                font.pixelSize: Appearance.fontSize.huge
            }
        }

        StyledText {
            width: column.width
            visible: root.body.length > 0
            text: root.body
            wrapMode: Text.Wrap
            color: Appearance.colors.onSurfaceVariant
        }

        Column {
            id: extra
            width: column.width
            spacing: 12
        }

        Row {
            id: buttonRow
            anchors.right: parent.right
            visible: children.length > 0
            topPadding: 8
            spacing: 8
            layoutDirection: Qt.RightToLeft
        }
    }
}
