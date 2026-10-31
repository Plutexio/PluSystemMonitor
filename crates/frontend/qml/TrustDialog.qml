import QtQuick
import PluSystemMonitor

// Asks whether a daemon's certificate is the right one: the first time we see
// a machine, or, with a warning, when it no longer has the one we remember.
StyledDialog {
    id: root

    property string authority
    property string fingerprint
    property string previous
    readonly property bool changed: previous.length > 0

    signal trusted(string authority, string fingerprint)

    function ask(authority, fingerprint, previous) {
        root.authority = authority
        root.fingerprint = fingerprint
        root.previous = previous
        open()
    }

    headline: changed ? qsTr("Certificate Changed") : qsTr("New Machine")
    symbol: changed ? "gpp_maybe" : "encrypted"
    iconShape: "clover8"
    preferredWidth: 480
    body: changed
          ? qsTr("%1 has a different certificate than last time. Reinstalling the daemon or deleting its config does that, but so would someone intercepting the connection. Nothing was sent to it.").arg(authority)
          : qsTr("This is the first connection to %1. To be sure it's the right machine, run plusysmond --fingerprint over there and compare. Nothing was sent to it yet.").arg(authority)

    StyledText {
        width: parent.width
        visible: root.changed
        text: qsTr("Remembered")
        color: Appearance.colors.onSurfaceVariant
        font.pixelSize: Appearance.fontSize.smaller
    }
    StyledText {
        width: parent.width
        visible: root.changed
        text: root.previous
        color: Appearance.colors.onSurfaceVariant
        font.family: Appearance.monoFontFamily
        font.pixelSize: Appearance.fontSize.smaller
        wrapMode: Text.WrapAnywhere
    }
    StyledText {
        width: parent.width
        text: root.changed ? qsTr("Now") : qsTr("Fingerprint")
        color: Appearance.colors.onSurfaceVariant
        font.pixelSize: Appearance.fontSize.smaller
    }
    StyledText {
        width: parent.width
        text: root.fingerprint
        font.family: Appearance.monoFontFamily
        font.pixelSize: Appearance.fontSize.smaller
        wrapMode: Text.WrapAnywhere
    }

    buttons: [
        TextButton {
            variant: root.changed ? TextButton.DangerTonal : TextButton.Filled
            symbol: root.changed ? "warning" : "verified_user"
            text: root.changed ? qsTr("Trust the New One") : qsTr("Trust and Connect")
            onClicked: {
                root.trusted(root.authority, root.fingerprint)
                root.close()
            }
        },
        TextButton {
            variant: root.changed ? TextButton.Tonal : TextButton.Text
            text: qsTr("Cancel")
            onClicked: root.close()
        }
    ]
}
