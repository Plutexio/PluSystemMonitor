import QtQuick
import PluSystemMonitor

// An app's icon, or its initial on a tinted tile while the icon is missing or loading.
Item {
    id: root

    property string source
    property string name
    property real size: 40

    implicitWidth: size
    implicitHeight: size

    Rectangle {
        anchors.fill: parent
        visible: icon.status !== Image.Ready
        radius: width * 0.28
        color: Qt.alpha(Theme.primary, 0.2)

        Text {
            anchors.centerIn: parent
            text: root.name.length > 0 ? root.name[0].toUpperCase() : "?"
            color: Theme.primary
            font.pixelSize: root.size * 0.45
            font.weight: Font.Bold
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
