import QtQuick
import PluSystemMonitor

// A row of mutually exclusive choices with a pill that slides to the active one.
// Like Toggle, it reports clicks and leaves `currentIndex` to its owner.
Item {
    id: root

    property var model: []
    property int currentIndex: 0

    signal activated(int index)

    implicitWidth: row.implicitWidth + 8
    implicitHeight: 38

    Rectangle {
        anchors.fill: parent
        radius: 12
        color: Theme.card
        border.color: Theme.cardBorder
    }

    Rectangle {
        readonly property Item target: {
            repeater.count // re-evaluate once the items exist
            return repeater.itemAt(root.currentIndex)
        }

        visible: target !== null
        x: target ? row.x + target.x : 0
        y: 4
        width: target ? target.width : 0
        height: root.height - 8
        radius: 9
        color: Theme.primary

        Behavior on x {
            NumberAnimation { duration: 240; easing.type: Easing.OutCubic }
        }
        Behavior on width {
            NumberAnimation { duration: 240; easing.type: Easing.OutCubic }
        }
    }

    Row {
        id: row

        x: 4
        anchors.verticalCenter: parent.verticalCenter

        Repeater {
            id: repeater

            model: root.model

            delegate: Item {
                id: segment

                required property int index
                required property string modelData

                width: label.implicitWidth + 30
                height: root.height - 8

                Text {
                    id: label
                    anchors.centerIn: parent
                    text: segment.modelData
                    color: segment.index === root.currentIndex ? "white" : hover.hovered ? Theme.text : Theme.textDim
                    font.pixelSize: 13
                    font.weight: Font.DemiBold

                    Behavior on color {
                        ColorAnimation { duration: 160 }
                    }
                }

                HoverHandler {
                    id: hover
                    cursorShape: Qt.PointingHandCursor
                }
                TapHandler {
                    onTapped: root.activated(segment.index)
                }
            }
        }
    }
}
