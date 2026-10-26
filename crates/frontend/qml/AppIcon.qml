import QtQuick
import PluSystemMonitor

// An app's icon, or its initial set round on a tonal cookie while the icon is
// missing or loading.
Item {
    id: root

    property string source
    property string name
    property real size: 40

    implicitWidth: size
    implicitHeight: size

    MaterialShape {
        anchors.fill: parent
        visible: icon.status !== Image.Ready
        shape: "cookie9"
        color: Appearance.colors.primaryContainer

        TitleText {
            anchors.centerIn: parent
            text: root.name.length > 0 ? root.name[0].toUpperCase() : "?"
            color: Appearance.colors.onPrimaryContainer
            font.pixelSize: root.size * 0.42
        }
    }

    Image {
        id: icon

        anchors.fill: parent
        source: root.source
        sourceSize: Qt.size(root.size * 2, root.size * 2)
        fillMode: Image.PreserveAspectFit
        asynchronous: true
        smooth: true
        mipmap: true
    }
}
