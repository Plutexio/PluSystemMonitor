import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import PluSystemMonitor

// The inspector for the picked app: its totals, its processes, and ending it
// as a whole.
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

    radius: Appearance.rounding.large
    color: Appearance.colors.layer1

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
            radius: Appearance.rounding.large
            topOnly: true
            source: root.details ? root.details.art : ""
            opacity: 0.85
        }
        Rectangle {
            anchors.fill: parent
            visible: bannerArt.visible
            gradient: Gradient {
                GradientStop { position: 0.4; color: Qt.alpha(Appearance.colors.layer1, 0.0) }
                GradientStop { position: 1.0; color: Appearance.colors.layer1 }
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
        // Without artwork, the icon sits on a big pale flower among stickers.
        Stickers {
            anchors.fill: parent
            visible: !bannerArt.visible
            key: root.appId
            count: 3
            minimumSize: 40
            maximumSize: 110
        }
        MaterialShape {
            anchors.centerIn: parent
            visible: !bannerArt.visible
            width: 116
            height: 116
            shape: root.details && root.details.game ? "puffy" : "flower"
            color: Appearance.colors.secondaryContainer
        }
        AppIcon {
            anchors.centerIn: parent
            visible: !bannerArt.visible
            source: root.details ? root.details.icon : ""
            name: root.details ? root.details.name : ""
            size: 72
        }

        IconButton {
            anchors.top: parent.top
            anchors.right: parent.right
            anchors.margins: 12
            variant: IconButton.Tonal
            symbol: "close"
            size: 36
            iconSize: 20
            tooltip: "Close"
            onClicked: root.closeRequested()
        }
    }

    ColumnLayout {
        anchors.top: banner.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        anchors.bottom: parent.bottom
        anchors.margins: 20
        anchors.topMargin: 4
        spacing: 14

        Column {
            Layout.fillWidth: true
            Layout.preferredWidth: 1
            spacing: 4

            TitleText {
                width: parent.width
                text: root.details ? root.details.name : "App Closed"
                font.pixelSize: Appearance.fontSize.huge
                elide: Text.ElideRight
            }
            StyledText {
                width: parent.width
                text: !root.details ? "None of its processes are running anymore."
                      : (root.details.game ? "Game" : "Application") + " · "
                        + root.details.processes.length + (root.details.processes.length === 1 ? " process" : " processes")
                color: Appearance.colors.onSurfaceVariant
                font.pixelSize: Appearance.fontSize.smaller
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
                    { label: "CPU", key: "cpu", color: Appearance.colors.cpu },
                    { label: "Memory", key: "memory", color: Appearance.colors.memory },
                    { label: "GPU", key: "gpu", color: Appearance.colors.gpu },
                    { label: "GPU Memory", key: "gpuMemory", color: Appearance.colors.gpu },
                    { label: "Disk Read", key: "readRate", color: Appearance.colors.onSurface },
                    { label: "Disk Write", key: "writeRate", color: Appearance.colors.onSurface }
                ]

                Column {
                    required property var modelData

                    // See ProcessDetails.Stat for why the preferred width is pinned.
                    Layout.fillWidth: true
                    Layout.preferredWidth: 1
                    spacing: 2

                    StyledText {
                        text: modelData.label
                        color: Appearance.colors.onSurfaceVariant
                        font.pixelSize: Appearance.fontSize.smaller
                    }
                    StyledText {
                        text: {
                            if (!root.details)
                                return ""
                            const value = root.details[modelData.key]
                            if (modelData.key === "cpu")
                                return value.toFixed(1) + "%"
                            if (modelData.key === "memory" || modelData.key === "gpuMemory")
                                return Format.bytes(value)
                            if (modelData.key === "gpu")
                                return value.toFixed(0) + "%"
                            return Format.rate(value) || "0 B/s"
                        }
                        color: modelData.color
                        font.weight: Font.DemiBold
                        tabularNumbers: true
                    }
                }
            }
        }

        SectionLabel {
            Layout.fillWidth: true
            visible: root.details !== null
            inset: 0
            text: "Processes"
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

            T.ScrollBar.vertical: StyledScrollBar {}

            delegate: Rectangle {
                id: processRow

                required property var modelData

                width: processList.width - 8
                height: 34
                radius: Appearance.rounding.small
                color: Qt.alpha(Appearance.colors.onSurface, rowHover.hovered ? 0.05 : 0)

                Behavior on color {
                    ElementMoveFastColor {}
                }

                StyledText {
                    anchors.left: parent.left
                    anchors.leftMargin: 8
                    anchors.right: rowCpu.left
                    anchors.rightMargin: 8
                    anchors.verticalCenter: parent.verticalCenter
                    text: processRow.modelData.name + "  <font color='" + Appearance.colors.onSurfaceVariant + "'>" + processRow.modelData.pid + "</font>"
                    textFormat: Text.StyledText
                    font.pixelSize: Appearance.fontSize.smallie
                    elide: Text.ElideRight
                }
                StyledText {
                    id: rowCpu
                    anchors.right: rowMemory.left
                    anchors.rightMargin: 10
                    anchors.verticalCenter: parent.verticalCenter
                    width: 52
                    horizontalAlignment: Text.AlignRight
                    text: processRow.modelData.cpu.toFixed(1) + "%"
                    color: Appearance.colors.onSurfaceVariant
                    font.pixelSize: Appearance.fontSize.smaller
                    tabularNumbers: true
                }
                StyledText {
                    id: rowMemory
                    anchors.right: parent.right
                    anchors.rightMargin: 8
                    anchors.verticalCenter: parent.verticalCenter
                    width: 70
                    horizontalAlignment: Text.AlignRight
                    text: Format.bytes(processRow.modelData.memory)
                    color: Appearance.colors.onSurfaceVariant
                    font.pixelSize: Appearance.fontSize.smaller
                    tabularNumbers: true
                }

                HoverHandler {
                    id: rowHover
                    cursorShape: Qt.PointingHandCursor
                }
                TapHandler {
                    onTapped: root.processRequested(processRow.modelData.pid)
                }
                Loader {
                    anchors.fill: parent
                    active: rowHover.hovered
                    sourceComponent: StyledToolTip {
                        visible: true
                        text: "Show in Processes"
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            visible: root.details !== null
            spacing: 8

            TextButton {
                Layout.fillWidth: true
                Layout.preferredWidth: 1
                variant: TextButton.Tonal
                text: "End App"
                leftPadding: 12
                rightPadding: 12
                tooltip: "Asks every one of its processes to exit (SIGTERM)."
                onClicked: root.monitor.signalApp(root.appId, "terminate")
            }
            TextButton {
                id: killButton

                property bool armed: false

                Layout.fillWidth: true
                Layout.preferredWidth: 1
                variant: armed ? TextButton.Danger : TextButton.DangerTonal
                text: armed ? "Sure?" : "Kill App"
                leftPadding: 12
                rightPadding: 12
                tooltip: "Ends every process at once (SIGKILL); unsaved work is lost."
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
