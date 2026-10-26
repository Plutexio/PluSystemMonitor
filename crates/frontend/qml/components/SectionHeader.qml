import QtQuick
import PluSystemMonitor

// A section's heading: a symbol on a shape, the title drawn round, and a
// squiggle running on to whatever sits at the end of the row. Hovering the
// heading turns the shape.
Item {
    id: root

    property string title
    property string symbol
    property string iconShape: "cookie9"
    property real titleSize: width < 320 ? Appearance.fontSize.larger : Appearance.fontSize.title
    property alias trailing: trailingSlot.data

    implicitHeight: 40
    implicitWidth: titleText.x + titleText.implicitWidth

    HoverHandler {
        id: hover
    }

    ShapeIcon {
        id: badge
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        visible: root.symbol.length > 0
        size: 34
        iconSize: 18
        shape: root.iconShape
        symbol: root.symbol
        hovered: hover.hovered
    }

    TitleText {
        id: titleText
        x: badge.visible ? badge.width + 12 : 0
        anchors.verticalCenter: parent.verticalCenter
        width: Math.min(implicitWidth, Math.max(0, trailingSlot.x - 12 - x))
        text: root.title
        elide: Text.ElideRight
        font.pixelSize: root.titleSize
    }

    WavyLine {
        anchors.left: titleText.right
        anchors.leftMargin: 16
        anchors.right: trailingSlot.left
        anchors.rightMargin: trailingSlot.width > 0 ? 14 : 0
        anchors.verticalCenter: parent.verticalCenter
        visible: width > 24
    }

    Row {
        id: trailingSlot
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        spacing: 4
    }
}
