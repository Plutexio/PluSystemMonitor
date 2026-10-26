import QtQuick
import PluSystemMonitor

// Which daemon to watch: this computer, or another one by address and token.
StyledDialog {
    id: root

    required property Monitor monitor
    property alias address: addressField.text
    property alias token: tokenField.text

    signal connectRequested(string address, string token)

    function submit() {
        connectRequested(addressField.text, tokenField.text)
        close()
    }

    headline: qsTr("Connection")
    symbol: "lan"
    iconShape: "clover8"
    preferredWidth: 420

    Row {
        width: parent.width
        spacing: 10

        MaterialShape {
            id: dot
            anchors.verticalCenter: parent.verticalCenter
            width: 12
            height: 12
            shape: root.monitor.connected ? "cookie9" : "pixelCircle"
            keepsShape: true
            color: root.monitor.connected ? Appearance.colors.good : Appearance.colors.error
        }
        StyledText {
            width: parent.width - dot.width - parent.spacing
            text: root.monitor.status
            color: Appearance.colors.onSurfaceVariant
            wrapMode: Text.Wrap
        }
    }

    StyledTextField {
        id: addressField
        width: parent.width
        label: qsTr("Address")
        placeholderText: "host[:port]"
        onAccepted: root.submit()
    }

    StyledTextField {
        id: tokenField
        width: parent.width
        label: qsTr("Token")
        placeholderText: qsTr("empty for this computer")
        echoMode: TextInput.Password
        onAccepted: root.submit()
    }

    StyledText {
        width: parent.width
        text: qsTr("leave the token empty for this computer. for another machine, it's in ~/.config/plusysmon/token over there")
        color: Appearance.colors.onSurfaceVariant
        font.pixelSize: Appearance.fontSize.smaller
        wrapMode: Text.Wrap
    }

    buttons: [
        TextButton {
            variant: TextButton.Filled
            symbol: "link"
            text: qsTr("Connect")
            onClicked: root.submit()
        },
        TextButton {
            variant: TextButton.Text
            text: qsTr("Cancel")
            onClicked: root.close()
        }
    ]
}
