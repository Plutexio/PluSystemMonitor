import QtQuick
import PluSystemMonitor

GlassPopup {
    id: root

    signal picked(string type)

    title: "Add widget"
    width: 360

    contentItem: Column {
        spacing: 6

        Repeater {
            model: Widgets.types

            delegate: Rectangle {
                id: tile

                required property var modelData
                readonly property color tint: Theme.accent(modelData.options.find(o => o.type === "accent")?.default)

                width: parent.width
                height: 62
                radius: 14
                color: hover.hovered ? Qt.alpha(tint, 0.12) : "transparent"
                border.color: hover.hovered ? Qt.alpha(tint, 0.45) : "transparent"
                border.width: 1

                Behavior on color {
                    ColorAnimation { duration: 140 }
                }

                HoverHandler {
                    id: hover
                    cursorShape: Qt.PointingHandCursor
                }
                TapHandler {
                    onTapped: {
                        root.picked(tile.modelData.type)
                        root.close()
                    }
                }

                Rectangle {
                    id: marker
                    x: 12
                    anchors.verticalCenter: parent.verticalCenter
                    width: 4
                    height: 30
                    radius: 2
                    color: tile.tint
                }

                Column {
                    anchors.left: marker.right
                    anchors.leftMargin: 14
                    anchors.right: plus.left
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 3

                    Text {
                        text: tile.modelData.name
                        color: Theme.text
                        font.pixelSize: 14
                        font.weight: Font.DemiBold
                    }
                    Text {
                        width: parent.width
                        text: tile.modelData.description
                        color: Theme.textDim
                        font.pixelSize: 12
                        elide: Text.ElideRight
                    }
                }

                Icon {
                    id: plus
                    anchors.right: parent.right
                    anchors.rightMargin: 14
                    anchors.verticalCenter: parent.verticalCenter
                    name: "plus"
                    color: hover.hovered ? tile.tint : Theme.textDim
                }
            }
        }
    }
}
