import QtQuick
import QtQuick.Templates as T
import PluSystemMonitor

// Which daemon to watch: this computer, another one by address and token, or
// one connected to before.
StyledDialog {
    id: root

    required property Monitor monitor
    property alias address: addressField.text
    property alias token: tokenField.text
    property string currentAddress
    /// `{ address, token, hostname }`, latest first.
    property var recent: []

    signal connectRequested(string address, string token)
    signal forgetRequested(string address)

    function submit() {
        connectRequested(addressField.text.trim(), tokenField.text.trim())
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

    Column {
        width: parent.width
        visible: root.monitor.fingerprint.length > 0
        spacing: 2

        StyledText {
            width: parent.width
            text: qsTr("Encrypted. Certificate fingerprint:")
            color: Appearance.colors.onSurfaceVariant
            font.pixelSize: Appearance.fontSize.smaller
        }
        StyledText {
            width: parent.width
            text: root.monitor.fingerprint
            color: Appearance.colors.onSurfaceVariant
            font.family: Appearance.monoFontFamily
            font.pixelSize: Appearance.fontSize.smaller
            wrapMode: Text.WrapAnywhere
        }
    }

    SectionLabel {
        width: parent.width
        visible: recentList.count > 0
        inset: 4
        text: qsTr("Recent")
    }

    ListView {
        id: recentList

        readonly property real rowHeight: 48

        width: parent.width
        visible: count > 0
        height: Math.min(count, 4) * (rowHeight + spacing)
        clip: true
        spacing: 2
        boundsBehavior: Flickable.StopAtBounds
        model: root.recent

        T.ScrollBar.vertical: StyledScrollBar {}

        delegate: RippleButton {
            id: row

            required property var modelData
            readonly property bool current: root.monitor.connected && modelData.address === root.currentAddress

            width: recentList.width - (recentList.count > 4 ? 10 : 0)
            height: recentList.rowHeight
            padding: 0
            buttonRadius: Appearance.rounding.small
            colBackgroundHover: Appearance.colors.layer3Hover
            colRipple: Appearance.colors.layer3Active
            bounce: false
            onClicked: {
                root.connectRequested(modelData.address, modelData.token)
                root.close()
            }

            contentItem: Item {
                MaterialSymbol {
                    id: rowIcon
                    x: 10
                    anchors.verticalCenter: parent.verticalCenter
                    text: row.current ? "check_circle" : "dns"
                    iconSize: 22
                    fill: row.current ? 1 : 0
                    color: row.current ? Appearance.colors.primary : Appearance.colors.onSurfaceVariant
                }
                Column {
                    anchors.left: rowIcon.right
                    anchors.leftMargin: 12
                    anchors.right: forget.left
                    anchors.rightMargin: 8
                    anchors.verticalCenter: parent.verticalCenter

                    StyledText {
                        width: parent.width
                        text: row.modelData.hostname || row.modelData.address
                        font.pixelSize: Appearance.fontSize.smallie
                        elide: Text.ElideRight
                    }
                    StyledText {
                        width: parent.width
                        visible: row.modelData.hostname.length > 0
                        text: row.modelData.address
                        color: Appearance.colors.onSurfaceVariant
                        font.pixelSize: Appearance.fontSize.smaller
                        elide: Text.ElideRight
                    }
                }
                IconButton {
                    id: forget
                    anchors.right: parent.right
                    anchors.rightMargin: 4
                    anchors.verticalCenter: parent.verticalCenter
                    size: 32
                    iconSize: 18
                    symbol: "close"
                    tooltip: qsTr("Forget")
                    onClicked: root.forgetRequested(row.modelData.address)
                }
            }
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
