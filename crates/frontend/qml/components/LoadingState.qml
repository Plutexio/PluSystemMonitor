import QtQuick
import PluSystemMonitor

// Waiting for something: the loading indicator and one sentence ending in an
// ellipsis.
Column {
    id: root

    property string text: qsTr("Waiting for data…")

    spacing: 14

    LoadingSpinner {
        anchors.horizontalCenter: parent.horizontalCenter
        width: 48
        height: 48
        running: root.visible
    }
    StyledText {
        anchors.horizontalCenter: parent.horizontalCenter
        text: root.text
        color: Appearance.colors.onSurfaceVariant
    }
}
