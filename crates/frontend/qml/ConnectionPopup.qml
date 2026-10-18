import QtQuick
import PluSystemMonitor

GlassPopup {
    id: root

    required property Monitor monitor
    property alias address: addressField.text
    property alias token: tokenField.text

    signal connectRequested(string address, string token)

    function submit() {
        connectRequested(addressField.text, tokenField.text)
        close()
    }

    title: "Connection"
    width: 340

    contentItem: Column {
        spacing: 12

        Text {
            width: parent.width
            text: root.monitor.status
            color: root.monitor.connected ? Theme.good : Theme.textDim
            font.pixelSize: 12
            wrapMode: Text.Wrap
        }

        GlassField {
            id: addressField
            width: parent.width
            placeholderText: "host[:port]"
            onAccepted: root.submit()
        }

        GlassField {
            id: tokenField
            width: parent.width
            placeholderText: "token"
            echoMode: TextInput.Password
            onAccepted: root.submit()
        }

        Text {
            width: parent.width
            text: "Leave the token empty for this computer. For another machine, copy it from "
                + "~/.config/plusysmon/token there."
            color: Theme.textDim
            font.pixelSize: 12
            wrapMode: Text.Wrap
            lineHeight: 1.2
        }

        GlassButton {
            width: parent.width
            primary: true
            iconName: "plug"
            text: "Connect"
            onClicked: root.submit()
        }
    }
}
