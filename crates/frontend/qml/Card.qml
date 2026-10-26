import QtQuick
import PluSystemMonitor

// A widget's card on the dashboard sheet (layer2): the source's symbol on its
// shape, tinted in the data's colour, the title, and an optional reading at
// the end of the row. Hovering the card turns the shape.
Item {
    id: root

    property string title
    property string subtitle
    property color accent: Appearance.colors.cpu
    property string symbol: "monitoring"
    property string shape: "cookie9"
    default property alias content: body.data

    readonly property real padding: 18

    Rectangle {
        anchors.fill: parent
        radius: Appearance.rounding.large
        color: Appearance.colors.layer2
    }

    HoverHandler {
        id: hover
    }

    ShapeIcon {
        id: badge
        x: root.padding - 2
        y: root.padding - 4
        size: 32
        iconSize: 17
        shape: root.shape
        symbol: root.symbol
        color: Appearance.colors.tonal(root.accent)
        iconColor: root.accent
        hovered: hover.hovered
    }

    StyledText {
        anchors.left: badge.right
        anchors.leftMargin: 10
        anchors.right: reading.visible ? reading.left : parent.right
        anchors.rightMargin: reading.visible ? 8 : root.padding
        anchors.verticalCenter: badge.verticalCenter
        text: root.title
        elide: Text.ElideRight
        font.weight: Font.DemiBold
    }

    StyledText {
        id: reading
        anchors.right: parent.right
        anchors.rightMargin: root.padding
        anchors.verticalCenter: badge.verticalCenter
        visible: root.subtitle.length > 0
        text: root.subtitle
        color: Appearance.colors.onSurfaceVariant
        font.pixelSize: Appearance.fontSize.smallie
        font.weight: Font.DemiBold
        tabularNumbers: true
    }

    Item {
        id: body

        anchors.fill: parent
        anchors.margins: root.padding
        anchors.topMargin: badge.y + badge.height + 12
    }
}
