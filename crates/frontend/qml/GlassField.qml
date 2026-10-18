import QtQuick
import QtQuick.Controls.Basic
import PluSystemMonitor

TextField {
    id: root

    implicitHeight: 38
    color: Theme.text
    placeholderTextColor: Theme.textDim
    selectionColor: Theme.primary
    selectedTextColor: "white"
    font.pixelSize: 13
    leftPadding: 14
    rightPadding: 14

    background: Rectangle {
        radius: 12
        color: Theme.card
        border.color: root.activeFocus ? Theme.primary : Theme.cardBorder
        border.width: 1

        Behavior on border.color {
            ColorAnimation { duration: 150 }
        }
    }
}
