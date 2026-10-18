import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import PluSystemMonitor

// Host name and status on the left, address/token form on the right.
RowLayout {
    id: root

    required property Monitor monitor
    property alias address: addressField.text
    property alias token: tokenField.text

    signal connectRequested(string address, string token)

    spacing: 12

    component Field: TextField {
        implicitHeight: 38
        color: Theme.text
        placeholderTextColor: Theme.textDim
        selectionColor: Theme.cpu
        font.pixelSize: 13
        leftPadding: 14
        rightPadding: 14
        background: Rectangle {
            radius: 12
            color: Theme.card
            border.color: parent.activeFocus ? Theme.cpu : Theme.cardBorder
            border.width: 1

            Behavior on border.color {
                ColorAnimation { duration: 150 }
            }
        }
        onAccepted: root.connectRequested(addressField.text, tokenField.text)
    }

    Item {
        implicitWidth: 14
        implicitHeight: 14

        // Expanding ring that pulses while live data is flowing.
        Rectangle {
            id: pulse
            anchors.centerIn: parent
            width: 14
            height: 14
            radius: 7
            color: "transparent"
            border.color: dot.color
            border.width: 2
            opacity: 0
            visible: root.monitor.connected
        }
        Rectangle {
            id: dot
            anchors.centerIn: parent
            width: 10
            height: 10
            radius: 5
            color: root.monitor.connected ? Theme.good : Theme.bad

            Behavior on color {
                ColorAnimation { duration: 300 }
            }
        }
        Connections {
            target: root.monitor
            function onSampleReceived() { pulseAnim.restart() }
        }
        ParallelAnimation {
            id: pulseAnim
            NumberAnimation { target: pulse; property: "scale"; from: 0.8; to: 2.4; duration: 900; easing.type: Easing.OutCubic }
            NumberAnimation { target: pulse; property: "opacity"; from: 0.8; to: 0; duration: 900; easing.type: Easing.OutCubic }
        }
    }

    Column {
        Layout.fillWidth: true
        spacing: 2

        Text {
            text: root.monitor.hostname || "PluSystemMonitor"
            color: Theme.text
            font.pixelSize: 22
            font.weight: Font.Bold
        }
        Text {
            width: parent.width
            text: root.monitor.status
            color: Theme.textDim
            font.pixelSize: 12
            elide: Text.ElideRight
        }
    }

    Field {
        id: addressField
        Layout.preferredWidth: 200
        placeholderText: "host[:port]"
    }

    Field {
        id: tokenField
        Layout.preferredWidth: 200
        placeholderText: "token (auto on localhost)"
        echoMode: TextInput.Password
    }

    Button {
        id: connectButton

        implicitHeight: 38
        leftPadding: 20
        rightPadding: 20
        text: "Connect"
        font.pixelSize: 13
        font.weight: Font.DemiBold
        onClicked: root.connectRequested(addressField.text, tokenField.text)

        contentItem: Text {
            text: connectButton.text
            font: connectButton.font
            color: "white"
            horizontalAlignment: Text.AlignHCenter
            verticalAlignment: Text.AlignVCenter
        }
        background: Rectangle {
            radius: 12
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0.0; color: Theme.cpu }
                GradientStop { position: 1.0; color: Qt.lighter(Theme.cpu, 1.3) }
            }
            opacity: connectButton.down ? 0.75 : connectButton.hovered ? 1.0 : 0.9
            scale: connectButton.down ? 0.97 : 1.0

            Behavior on opacity {
                NumberAnimation { duration: 120 }
            }
            Behavior on scale {
                NumberAnimation { duration: 120 }
            }
        }
    }
}
