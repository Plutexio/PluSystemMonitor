import QtQuick
import QtQuick.Layouts
import PluSystemMonitor

// The inspector for the picked process: everything about it, and the
// signals to send it.
Rectangle {
    id: root

    required property Monitor monitor
    property int pid: -1
    property var details: null

    signal closeRequested()
    signal pidRequested(int pid)

    function reload() {
        const json = pid >= 0 ? monitor.processDetails(pid) : ""
        details = json ? JSON.parse(json) : null
        killButton.armed = false
    }

    readonly property var info: details ? details.info : null
    readonly property var stats: details ? details.stats : null
    readonly property bool paused: stats !== null && stats.state === "T"

    onPidChanged: reload()

    radius: Appearance.rounding.large
    color: Appearance.colors.layer1
    clip: true

    Connections {
        target: root.monitor
        function onProcessesUpdated() { root.reload() }
    }

    // Keeps "running for" ticking between process updates.
    Timer {
        id: clock
        property real now: Date.now()
        interval: 1000
        running: root.visible && root.info !== null
        repeat: true
        onTriggered: now = Date.now()
    }

    // Layout.preferredWidth breaks the loop between this Column's implicit width
    // (taken from its children's widths) and the children following its width.
    component Stat: Column {
        property string label
        property string value
        property color valueColor: Appearance.colors.onSurface

        Layout.fillWidth: true
        Layout.preferredWidth: 1
        spacing: 2

        StyledText {
            text: parent.label
            color: Appearance.colors.onSurfaceVariant
            font.pixelSize: Appearance.fontSize.smaller
        }
        StyledText {
            width: parent.width
            text: parent.value
            color: parent.valueColor
            font.weight: Font.DemiBold
            tabularNumbers: true
            elide: Text.ElideRight
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 16

        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            ShapeIcon {
                Layout.alignment: Qt.AlignTop
                size: 40
                iconSize: 21
                shape: root.info ? (root.paused ? "pentagon" : "cookie9") : "ghostish"
                symbol: root.info ? (root.info.kernel_thread ? "settings" : "terminal") : "close"
                hovered: headerHover.hovered

                HoverHandler {
                    id: headerHover
                }
            }

            Column {
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                spacing: 2

                TitleText {
                    width: parent.width
                    text: root.info ? root.info.name : "Process Ended"
                    font.pixelSize: Appearance.fontSize.larger
                    elide: Text.ElideRight
                }
                StyledText {
                    width: parent.width
                    text: root.info ? "PID " + root.info.pid + " · " + root.info.user
                                    : "PID " + root.pid + " isn't running anymore."
                    color: Appearance.colors.onSurfaceVariant
                    font.pixelSize: Appearance.fontSize.smaller
                    elide: Text.ElideRight
                }
            }

            IconButton {
                Layout.alignment: Qt.AlignTop
                symbol: "close"
                size: 36
                iconSize: 20
                tooltip: "Close (Esc)"
                onClicked: root.closeRequested()
            }
        }

        GridLayout {
            Layout.fillWidth: true
            visible: root.stats !== null
            columns: 2
            columnSpacing: 16
            rowSpacing: 12

            Stat {
                label: "CPU"
                value: root.stats ? root.stats.cpu.toFixed(1) + "%" : ""
                valueColor: Appearance.colors.cpu
            }
            Stat {
                label: "Memory"
                value: root.stats ? Format.bytes(root.stats.memory) : ""
                valueColor: Appearance.colors.memory
            }
            Stat {
                visible: root.stats !== null && root.stats.gpu !== null
                label: "GPU"
                value: root.stats && root.stats.gpu !== null ? root.stats.gpu.toFixed(0) + "%" : ""
                valueColor: Appearance.colors.gpu
            }
            Stat {
                visible: root.stats !== null && root.stats.gpu_memory !== null
                label: "GPU Memory"
                value: root.stats && root.stats.gpu_memory !== null ? Format.bytes(root.stats.gpu_memory) : ""
                valueColor: Appearance.colors.gpu
            }
            Stat {
                label: "Disk Read"
                value: root.stats ? (root.stats.read_rate === null ? "Not permitted" : Format.rate(root.stats.read_rate) || "0 B/s") : ""
            }
            Stat {
                label: "Disk Write"
                value: root.stats ? (root.stats.write_rate === null ? "Not permitted" : Format.rate(root.stats.write_rate) || "0 B/s") : ""
            }
            Stat {
                label: "State"
                value: root.stats ? Format.processState(root.stats.state) : ""
                valueColor: root.paused ? Appearance.colors.warn : Appearance.colors.onSurface
            }
            Stat {
                label: "Threads"
                value: root.stats ? root.stats.threads : ""
            }
            Stat {
                label: "Running For"
                value: root.info ? Format.duration(clock.now - root.info.started_ms) : ""
            }
            Stat {
                label: "Nice"
                value: root.stats ? root.stats.nice : ""
            }
        }

        // Parent, clickable to jump up the tree.
        Column {
            Layout.fillWidth: true
            Layout.preferredWidth: 1
            visible: root.info !== null && root.info.ppid > 0
            spacing: 2

            SectionLabel {
                width: parent.width
                inset: 0
                text: "Parent"
            }
            StyledText {
                width: parent.width
                text: root.details ? (root.details.parentName ?? "process") + " (" + root.info.ppid + ")" : ""
                color: parentHover.hovered ? Appearance.colors.primary : Appearance.colors.onSurface
                font.underline: parentHover.hovered
                elide: Text.ElideRight

                HoverHandler {
                    id: parentHover
                    cursorShape: Qt.PointingHandCursor
                }
                TapHandler {
                    onTapped: root.pidRequested(root.info.ppid)
                }
            }
        }

        // An Item, not a Column: the box fills the remaining height, and a Column
        // would report that back as its implicit height and loop the layout.
        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: root.info !== null

            SectionLabel {
                id: commandLabel
                width: parent.width
                inset: 0
                text: "Command"
            }
            Rectangle {
                anchors.top: commandLabel.bottom
                anchors.topMargin: 4
                width: parent.width
                height: Math.min(command.implicitHeight + 24, parent.height - commandLabel.height - 4)
                radius: Appearance.rounding.small
                color: Appearance.colors.layer2
                clip: true

                TextEdit {
                    id: command
                    anchors.fill: parent
                    anchors.margins: 12
                    readOnly: true
                    selectByMouse: true
                    wrapMode: TextEdit.WrapAnywhere
                    text: root.info ? (root.info.command || "[" + root.info.name + "] (kernel thread)") : ""
                    color: Appearance.colors.onSurface
                    selectionColor: Appearance.colors.primary
                    selectedTextColor: Appearance.colors.onPrimary
                    font.family: Appearance.monoFontFamily
                    font.pixelSize: Appearance.fontSize.smaller
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            visible: root.info !== null && !root.info.kernel_thread
            spacing: 8

            TextButton {
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                variant: TextButton.Tonal
                text: root.paused ? "Resume" : "Pause"
                leftPadding: 12
                rightPadding: 12
                onClicked: root.monitor.sendSignal(root.pid, root.paused ? "continue" : "stop")
            }
            TextButton {
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                variant: TextButton.Tonal
                text: "End"
                leftPadding: 12
                rightPadding: 12
                tooltip: "Asks it to exit (SIGTERM). Shortcut: Delete."
                onClicked: root.monitor.sendSignal(root.pid, "terminate")
            }
            TextButton {
                id: killButton

                // Two steps: killing loses unsaved work.
                property bool armed: false

                Layout.fillWidth: true
                Layout.preferredWidth: 1
                variant: armed ? TextButton.Danger : TextButton.DangerTonal
                text: armed ? "Sure?" : "Kill"
                leftPadding: 12
                rightPadding: 12
                tooltip: "Ends it at once (SIGKILL); unsaved work is lost."
                onClicked: {
                    if (armed) {
                        armed = false
                        root.monitor.sendSignal(root.pid, "kill")
                    } else {
                        armed = true
                        disarm.restart()
                    }
                }

                Timer {
                    id: disarm
                    interval: 3000
                    onTriggered: killButton.armed = false
                }
            }
        }
    }
}
