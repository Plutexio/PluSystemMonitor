import QtQuick
import QtQuick.Layouts
import PluSystemMonitor

// Everything about the selected process, plus the actions on it.
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

    radius: Theme.radius
    color: Theme.card
    border.color: Theme.cardBorder
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
        property color valueColor: Theme.text

        Layout.fillWidth: true
        Layout.preferredWidth: 1
        spacing: 3

        Text {
            text: parent.label.toUpperCase()
            color: Theme.textDim
            font.pixelSize: 10
            font.weight: Font.DemiBold
            font.letterSpacing: 1.2
        }
        Text {
            width: parent.width
            text: parent.value
            color: parent.valueColor
            font.pixelSize: 15
            font.weight: Font.DemiBold
            font.features: { "tnum": 1 }
            elide: Text.ElideRight
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: Theme.padding
        spacing: 18

        RowLayout {
            Layout.fillWidth: true
            spacing: 8

            Column {
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                spacing: 4

                Text {
                    width: parent.width
                    text: root.info ? root.info.name : "Process ended"
                    color: Theme.text
                    font.pixelSize: 20
                    font.weight: Font.Bold
                    elide: Text.ElideRight
                }
                Text {
                    width: parent.width
                    text: root.info ? "PID " + root.info.pid + " · " + root.info.user
                                    : "PID " + root.pid + " is no longer running."
                    color: Theme.textDim
                    font.pixelSize: 12
                    elide: Text.ElideRight
                }
            }

            GlassButton {
                Layout.alignment: Qt.AlignTop
                implicitHeight: 32
                iconName: "close"
                tip: "Close (Esc)"
                onClicked: root.closeRequested()
            }
        }

        GridLayout {
            Layout.fillWidth: true
            visible: root.stats !== null
            columns: 2
            columnSpacing: 16
            rowSpacing: 14

            Stat {
                label: "CPU"
                value: root.stats ? root.stats.cpu.toFixed(1) + "%" : ""
                valueColor: Theme.cpu
            }
            Stat {
                label: "Memory"
                value: root.stats ? Format.bytes(root.stats.memory) : ""
                valueColor: Theme.memory
            }
            Stat {
                visible: root.stats !== null && root.stats.gpu !== null
                label: "GPU"
                value: root.stats && root.stats.gpu !== null ? root.stats.gpu.toFixed(0) + "%" : ""
                valueColor: Theme.gpu
            }
            Stat {
                visible: root.stats !== null && root.stats.gpu_memory !== null
                label: "GPU memory"
                value: root.stats && root.stats.gpu_memory !== null ? Format.bytes(root.stats.gpu_memory) : ""
                valueColor: Theme.gpu
            }
            Stat {
                label: "Disk read"
                value: root.stats ? (root.stats.read_rate === null ? "Not permitted" : Format.rate(root.stats.read_rate) || "0 B/s") : ""
            }
            Stat {
                label: "Disk write"
                value: root.stats ? (root.stats.write_rate === null ? "Not permitted" : Format.rate(root.stats.write_rate) || "0 B/s") : ""
            }
            Stat {
                label: "State"
                value: root.stats ? Format.processState(root.stats.state) : ""
                valueColor: root.paused ? Theme.warn : Theme.text
            }
            Stat {
                label: "Threads"
                value: root.stats ? root.stats.threads : ""
            }
            Stat {
                label: "Running for"
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
            spacing: 3

            Text {
                text: "PARENT"
                color: Theme.textDim
                font.pixelSize: 10
                font.weight: Font.DemiBold
                font.letterSpacing: 1.2
            }
            Text {
                width: parent.width
                text: root.details ? (root.details.parentName ?? "process") + " (" + root.info.ppid + ")" : ""
                color: parentHover.hovered ? Theme.primary : Theme.text
                font.pixelSize: 14
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

            Text {
                id: commandLabel
                text: "COMMAND"
                color: Theme.textDim
                font.pixelSize: 10
                font.weight: Font.DemiBold
                font.letterSpacing: 1.2
            }
            Rectangle {
                anchors.top: commandLabel.bottom
                anchors.topMargin: 6
                width: parent.width
                height: Math.min(command.implicitHeight + 20, parent.height - commandLabel.height - 6)
                radius: 10
                color: Theme.track
                clip: true

                TextEdit {
                    id: command
                    anchors.fill: parent
                    anchors.margins: 10
                    readOnly: true
                    selectByMouse: true
                    wrapMode: TextEdit.WrapAnywhere
                    text: root.info ? (root.info.command || "[" + root.info.name + "] (kernel thread)") : ""
                    color: Theme.text
                    selectionColor: Theme.primary
                    selectedTextColor: "white"
                    font.family: "monospace"
                    font.pixelSize: 12
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            visible: root.info !== null && !root.info.kernel_thread
            spacing: 8

            GlassButton {
                Layout.fillWidth: true
                iconName: root.paused ? "play" : "pause"
                text: root.paused ? "Resume" : "Pause"
                onClicked: root.monitor.sendSignal(root.pid, root.paused ? "continue" : "stop")
            }
            GlassButton {
                Layout.fillWidth: true
                iconName: "power"
                text: "End"
                tip: "Ask the process to exit (SIGTERM). Shortcut: Delete"
                onClicked: root.monitor.sendSignal(root.pid, "terminate")
            }
            GlassButton {
                id: killButton

                // Two-step: killing loses unsaved work.
                property bool armed: false

                Layout.fillWidth: true
                danger: true
                primary: armed
                iconName: "close"
                text: armed ? "Sure?" : "Kill"
                tip: "End immediately (SIGKILL); unsaved work is lost"
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
