import QtQuick
import PluSystemMonitor

// A small labelled value: "TEMP / 43 °C".
Rectangle {
    id: root

    property string label
    property string value
    property color valueColor: Theme.text

    implicitWidth: Math.max(labelText.implicitWidth, valueText.implicitWidth) + 24
    implicitHeight: 48
    radius: 12
    color: Theme.track

    Column {
        anchors.centerIn: parent
        spacing: 2

        Text {
            id: labelText
            anchors.horizontalCenter: parent.horizontalCenter
            text: root.label.toUpperCase()
            color: Theme.textDim
            font.pixelSize: 9
            font.weight: Font.DemiBold
            font.letterSpacing: 1.1
        }
        Text {
            id: valueText
            anchors.horizontalCenter: parent.horizontalCenter
            text: root.value
            color: root.valueColor
            font.pixelSize: 13
            font.weight: Font.DemiBold
            font.features: { "tnum": 1 }
        }
    }
}
