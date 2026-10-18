import QtQuick
import QtQuick.Controls.Basic
import QtQuick.Layouts
import PluSystemMonitor

// The selected app: totals, its processes, and ending it as a whole.
Rectangle {
    id: root

    required property Monitor monitor
    property string appId
    property var details: null

    signal closeRequested()
    signal processRequested(int pid)

    function reload() {
        const json = appId ? monitor.appDetails(appId) : ""
        details = json ? JSON.parse(json) : null
        if (!details)
            killButton.armed = false
    }

    onAppIdChanged: reload()

    radius: Theme.radius
    color: Theme.card
    border.color: Theme.cardBorder

    Connections {
        target: root.monitor
        function onProcessesUpdated() { root.reload() }
        function onAssetsChanged() { root.reload() }
    }

    // Banner: artwork for games, a large icon otherwise.
    Item {
        id: banner

        width: parent.width
        height: 150

        RoundedImage {
            id: bannerArt
            anchors.fill: parent
            radius: Theme.radius
            topOnly: true
            source: root.details ? root.details.art : ""
            opacity: 0.85
        }
        Rectangle {
            anchors.fill: parent
            visible: bannerArt.visible
            gradient: Gradient {
                GradientStop { position: 0.4; color: Qt.alpha(Theme.background, 0.0) }
                GradientStop { position: 1.0; color: Qt.alpha(Theme.background, 0.85) }
            }
        }
        Image {
            anchors.centerIn: parent
            visible: bannerArt.visible && status === Image.Ready
            width: parent.width * 0.65
            height: parent.height * 0.55
            source: root.details ? root.details.logo : ""
            fillMode: Image.PreserveAspectFit
            mipmap: true
        }
        AppIcon {
            anchors.centerIn: parent
            visible: !bannerArt.visible
            source: root.details ? root.details.icon : ""
            name: root.details ? root.details.name : ""
            size: 84
        }

        GlassButton {
            anchors.top: parent.top
            anchors.right: parent.right
            anchors.margins: 14
            implicitHeight: 32
            iconName: "close"
            tip: "Close"
            onClicked: root.closeRequested()
        }
    }

    ColumnLayout {
        anchors.top: banner.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: Theme.padding
        anchors.topMargin: 8
        spacing: 16

        Column {
            Layout.fillWidth: true
            Layout.preferredWidth: 1
            spacing: 4

            Text {
                width: parent.width
                text: root.details ? root.details.name : "App closed"
                color: Theme.text
                font.pixelSize: 20
                font.weight: Font.Bold
                elide: Text.ElideRight
            }
            Text {
                width: parent.width
                text: !root.details ? "None of its processes are running anymore."
                      : (root.details.game ? "Game" : "Application") + " · "
                        + root.details.processes.length + (root.details.processes.length === 1 ? " process" : " processes")
                color: Theme.textDim
                font.pixelSize: 12
                elide: Text.ElideRight
            }
        }

        GridLayout {
            Layout.fillWidth: true
            visible: root.details !== null
            columns: 2
            columnSpacing: 16
            rowSpacing: 12

            Repeater {
                model: [
                    { label: "CPU", key: "cpu", color: Theme.cpu },
                    { label: "Memory", key: "memory", color: Theme.memory },
                    { label: "Disk read", key: "readRate", color: Theme.text },
                    { label: "Disk write", key: "writeRate", color: Theme.text }
                ]

                Column {
                    required property var modelData

                    // See ProcessDetails.Stat for why the preferred width is pinned.
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    spacing: 3

                    Text {
                        text: modelData.label.toUpperCase()
                        color: Theme.textDim
                        font.pixelSize: 10
                        font.weight: Font.DemiBold
                        font.letterSpacing: 1.2
                    }
                    Text {
                        text: {
                            if (!root.details)
                                return ""
                            const value = root.details[modelData.key]
                            if (modelData.key === "cpu")
                                return value.toFixed(1) + "%"
                            if (modelData.key === "memory")
                                return Format.bytes(value)
                            return Format.rate(value) || "0 B/s"
                        }
                        color: modelData.color
                        font.pixelSize: 15
                        font.weight: Font.DemiBold
                        font.features: { "tnum": 1 }
                    }
                }
            }
        }

        Text {
            visible: root.details !== null
            text: "PROCESSES"
            color: Theme.textDim
            font.pixelSize: 10
            font.weight: Font.DemiBold
            font.letterSpacing: 1.2
        }

        ListView {
            id: processList

            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: root.details !== null
            clip: true
            spacing: 2
            boundsBehavior: Flickable.StopAtBounds
            model: root.details ? root.details.processes : []

            ScrollBar.vertical: ScrollBar {
                width: 8
                contentItem: Rectangle {
                    // A custom handle skips the style's "only when needed" logic.
                    visible: parent.size < 1.0
                    implicitWidth: 4
                    radius: 2
                    color: Theme.textDim
                    opacity: 0.4
                }
            }

            delegate: Rectangle {
                id: processRow

                required property var modelData

                width: processList.width - 8
                height: 30
                radius: 8
                color: rowHover.hovered ? Theme.track : "transparent"

                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: 8
                    anchors.right: rowCpu.left
                    anchors.rightMargin: 8
                    anchors.verticalCenter: parent.verticalCenter
                    text: processRow.modelData.name + "  <font color='" + Theme.textDim + "'>" + processRow.modelData.pid + "</font>"
                    textFormat: Text.StyledText
                    color: Theme.text
                    font.pixelSize: 13
                    elide: Text.ElideRight
                }
                Text {
                    id: rowCpu
                    anchors.right: rowMemory.left
                    anchors.rightMargin: 10
                    anchors.verticalCenter: parent.verticalCenter
                    width: 52
                    horizontalAlignment: Text.AlignRight
                    text: processRow.modelData.cpu.toFixed(1) + "%"
                    color: Theme.textDim
                    font.pixelSize: 12
                    font.features: { "tnum": 1 }
                }
                Text {
                    id: rowMemory
                    anchors.right: parent.right
                    anchors.rightMargin: 8
                    anchors.verticalCenter: parent.verticalCenter
                    width: 70
                    horizontalAlignment: Text.AlignRight
                    text: Format.bytes(processRow.modelData.memory)
                    color: Theme.textDim
                    font.pixelSize: 12
                    font.features: { "tnum": 1 }
                }

                HoverHandler {
                    id: rowHover
                    cursorShape: Qt.PointingHandCursor
                }
                TapHandler {
                    onTapped: root.processRequested(processRow.modelData.pid)
                }
                ToolTip.visible: rowHover.hovered
                ToolTip.delay: 700
                ToolTip.text: "Show in Processes"
            }
        }

        RowLayout {
            Layout.fillWidth: true
            visible: root.details !== null
            spacing: 8

            GlassButton {
                Layout.fillWidth: true
                iconName: "power"
                text: "End app"
                tip: "Ask every process of the app to exit (SIGTERM)"
                onClicked: root.monitor.signalApp(root.appId, "terminate")
            }
            GlassButton {
                id: killButton

                property bool armed: false

                Layout.fillWidth: true
                danger: true
                primary: armed
                iconName: "close"
                text: armed ? "Sure?" : "Kill app"
                tip: "End every process immediately (SIGKILL); unsaved work is lost"
                onClicked: {
                    if (armed) {
                        armed = false
                        root.monitor.signalApp(root.appId, "kill")
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
