import QtQuick
import PluSystemMonitor

// A group's heading in a list or a drawer: its name small and DemiBold, and
// a squiggle running on to whatever sits at the end of the row.
Item {
    id: root

    property string text
    property real inset: 16
    property alias trailing: trailingSlot.data

    implicitHeight: 32
    implicitWidth: label.implicitWidth + 2 * inset

    StyledText {
        id: label
        anchors.left: parent.left
        anchors.leftMargin: root.inset
        anchors.verticalCenter: parent.verticalCenter
        text: root.text
        font.pixelSize: Appearance.fontSize.smaller
        font.weight: Font.DemiBold
        color: Appearance.colors.onSurfaceVariant
    }

    WavyLine {
        anchors.left: label.right
        anchors.leftMargin: 10
        anchors.right: trailingSlot.left
        anchors.rightMargin: trailingSlot.width > 0 ? 6 : 0
        anchors.verticalCenter: parent.verticalCenter
        visible: width > 16
    }

    Row {
        id: trailingSlot
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
    }
}
