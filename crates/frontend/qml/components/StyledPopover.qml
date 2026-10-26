import QtQuick
import QtQuick.Templates as T
import PluSystemMonitor

// A panel that opens by the button it belongs to, drawn as end4 draws its
// menus: a surface-container card with a hairline edge, rounded 17. It fades
// in quickly and is drawn in the window it opens over.
T.Popup {
    id: root

    property string title
    property string symbol
    property string iconShape: "cookie9"

    popupType: T.Popup.Item
    padding: 16
    topPadding: title.length > 0 ? 64 : 16
    focus: true
    closePolicy: T.Popup.CloseOnEscape | T.Popup.CloseOnPressOutside
    implicitWidth: contentWidth + leftPadding + rightPadding
    implicitHeight: contentHeight + topPadding + bottomPadding

    enter: Transition {
        NumberAnimation { property: "opacity"; from: 0; to: 1; duration: Appearance.duration(150); easing.type: Easing.OutCubic }
        NumberAnimation { property: "scale"; from: 0.94; to: 1; duration: Appearance.motion(350); easing.type: Easing.BezierSpline; easing.bezierCurve: Appearance.curves.expressiveFastSpatial }
    }
    exit: Transition {
        NumberAnimation { property: "opacity"; to: 0; duration: Appearance.duration(100) }
    }

    background: Rectangle {
        radius: Appearance.rounding.normal
        color: Appearance.colors.surfaceContainer
        border.width: 1
        border.color: Appearance.colors.layer0Border

        Row {
            x: root.leftPadding
            y: 16
            visible: root.title.length > 0
            spacing: 10

            ShapeIcon {
                id: icon
                anchors.verticalCenter: parent.verticalCenter
                visible: root.symbol.length > 0
                size: 32
                iconSize: 17
                shape: root.iconShape
                symbol: root.symbol
            }
            TitleText {
                anchors.verticalCenter: parent.verticalCenter
                text: root.title
                font.pixelSize: Appearance.fontSize.larger
            }
        }
    }
}
