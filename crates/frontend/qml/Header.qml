import QtQuick
import QtQuick.Layouts
import PluSystemMonitor

// Host and status on the left; connection, theme and layout controls on the right.
RowLayout {
    id: root

    required property Monitor monitor
    /// 0: overview dashboard, 1: apps, 2: processes.
    property int page: 0
    property bool editing: false
    property string address
    property string token

    signal connectRequested(string address, string token)
    signal themeSelected(string id)
    signal addWidgetRequested(string type)
    signal resetRequested()

    spacing: 10
    onPageChanged: editing = false

    Item {
        implicitWidth: 14
        implicitHeight: 14
        Layout.rightMargin: 4

        // Expanding ring that pulses with every sample while data is flowing.
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
        // See ProcessDetails.Stat: keeps the width from feeding back into itself.
        Layout.preferredWidth: 1
        spacing: 2

        Text {
            text: root.monitor.hostname || "PluSystemMonitor"
            color: Theme.text
            font.pixelSize: 22
            font.weight: Font.Bold
        }
        Text {
            width: parent.width
            text: root.editing ? "Drag widgets to move them, use the corner handle to resize. Esc to finish."
                               : root.monitor.status
            color: Theme.textDim
            font.pixelSize: 12
            elide: Text.ElideRight
        }
    }

    Segmented {
        visible: !root.editing
        Layout.rightMargin: 6
        model: ["Overview", "Apps", "Processes"]
        currentIndex: root.page
        onActivated: index => root.page = index
    }

    // Normal mode
    GlassButton {
        visible: !root.editing
        iconName: "plug"
        tip: "Connection"
        onClicked: connectionPopup.open()

        ConnectionPopup {
            id: connectionPopup
            y: parent.height + 10
            x: parent.width - width
            monitor: root.monitor
            address: root.address
            token: root.token
            onConnectRequested: (address, token) => root.connectRequested(address, token)
        }
    }

    GlassButton {
        visible: !root.editing
        iconName: "theme"
        tip: "Theme"
        onClicked: themePopup.open()

        ThemePopup {
            id: themePopup
            y: parent.height + 10
            x: parent.width - width
            onThemeSelected: id => root.themeSelected(id)
        }
    }

    GlassButton {
        visible: !root.editing && root.page === 0
        iconName: "edit"
        text: "Edit layout"
        onClicked: root.editing = true
    }

    // Edit mode
    GlassButton {
        visible: root.editing
        iconName: "plus"
        text: "Add widget"
        onClicked: palette.open()

        WidgetPalette {
            id: palette
            y: parent.height + 10
            x: parent.width - width
            onPicked: type => root.addWidgetRequested(type)
        }
    }

    GlassButton {
        id: resetButton

        // Two-step so a stray click can't wipe a carefully built layout.
        property bool armed: false

        visible: root.editing
        iconName: "reset"
        danger: armed
        text: armed ? "Click again to reset" : "Reset"
        onClicked: {
            if (armed) {
                armed = false
                root.resetRequested()
            } else {
                armed = true
                disarm.restart()
            }
        }
        onVisibleChanged: armed = false

        Timer {
            id: disarm
            interval: 3000
            onTriggered: resetButton.armed = false
        }
    }

    GlassButton {
        visible: root.editing
        primary: true
        iconName: "check"
        text: "Done"
        onClicked: root.editing = false
    }
}
