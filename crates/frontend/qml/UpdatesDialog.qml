import QtQuick
import QtQuick.Templates as T
import PluSystemMonitor

// Every machine connected to before: what it runs, whether there's an update
// and whether it can still talk to this app. Each machine updates from the git
// clone it was installed from: it pulls, builds, installs and restarts. Nothing
// updates without being asked here.
StyledDialog {
    id: root

    required property Monitor monitor
    required property Fleet fleet
    /// `[{ address, token, hostname }]`
    property var machines: []
    property var machineStatus: ({})
    property var updateProgress: ({})
    property bool checkUpdates: true
    /// This app's binary was replaced by an update of this machine.
    property bool appReplaced: false
    readonly property int appProtocol: fleet.protocol()
    /// Addresses whose update log is shown.
    property var expanded: ({})

    readonly property var updatable: machines.filter(m => Updates.updatable(machineStatus[m.address], updateProgress[m.address]))

    signal checkUpdatesEdited(bool value)
    /// Look at the machines and have them check upstream.
    signal checkRequested()
    /// Look at the machines (checking only if that's on).
    signal lookRequested()
    signal updateRequested(string address, string token)

    headline: qsTr("Updates")
    symbol: "system_update"
    iconShape: "cookie9"
    preferredWidth: 640
    body: qsTr("A machine updates from the git clone it was installed from: it pulls, builds, installs and restarts, which takes a while on small boards. Nothing updates unless you ask here.")

    onAboutToShow: {
        appReplaced = fleet.appReplaced()
        lookRequested()
    }

    Connections {
        target: root.fleet
        function onUpdateEvent(address, json) {
            if (JSON.parse(json).state === "done")
                root.appReplaced = root.fleet.appReplaced()
        }
    }

    // The switch, as in Appearance.
    RippleButton {
        id: autoRow

        width: parent.width
        height: 56
        buttonRadius: Appearance.rounding.small
        colBackgroundHover: Appearance.colors.layer3Hover
        colRipple: Appearance.colors.layer3Active
        bounce: false
        onClicked: root.checkUpdatesEdited(!root.checkUpdates)

        contentItem: Item {
            Column {
                anchors.left: parent.left
                anchors.leftMargin: 4
                anchors.right: autoSwitch.left
                anchors.rightMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                spacing: 1

                StyledText {
                    text: qsTr("Look for Updates Automatically")
                }
                StyledText {
                    width: parent.width
                    text: qsTr("every few hours, each machine asks GitHub. off, they only look when you press Check")
                    color: Appearance.colors.onSurfaceVariant
                    font.pixelSize: Appearance.fontSize.smaller
                    elide: Text.ElideRight
                }
            }
            StyledSwitch {
                id: autoSwitch
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                checked: root.checkUpdates
                onClicked: autoRow.clicked()
            }
        }
    }

    // After this machine updated itself, the running app is the old one.
    Rectangle {
        width: parent.width
        height: replacedRow.implicitHeight + 20
        visible: root.appReplaced
        radius: Appearance.rounding.normal
        color: Appearance.colors.primaryContainer

        Row {
            id: replacedRow
            anchors.verticalCenter: parent.verticalCenter
            x: 14
            width: parent.width - 28
            spacing: 12

            MaterialSymbol {
                anchors.verticalCenter: parent.verticalCenter
                text: "restart_alt"
                iconSize: 22
                color: Appearance.colors.onPrimaryContainer
            }
            StyledText {
                anchors.verticalCenter: parent.verticalCenter
                width: parent.width - 22 - restartButton.width - 2 * parent.spacing
                text: qsTr("This app was updated. Restart it to use the new version.")
                color: Appearance.colors.onPrimaryContainer
                wrapMode: Text.Wrap
            }
            TextButton {
                id: restartButton
                anchors.verticalCenter: parent.verticalCenter
                variant: TextButton.Filled
                text: qsTr("Restart")
                onClicked: {
                    if (root.fleet.relaunch())
                        Qt.quit()
                }
            }
        }
    }

    SectionLabel {
        width: parent.width
        inset: 4
        text: qsTr("Machines")
    }

    Flickable {
        width: parent.width
        height: Math.min(list.implicitHeight, Math.max(160, root.maximumHeight - 420))
        contentHeight: list.implicitHeight
        boundsBehavior: Flickable.StopAtBounds
        clip: true

        T.ScrollBar.vertical: StyledScrollBar {}

        Column {
            id: list

            width: parent.width - 10
            spacing: 2

            Repeater {
                model: root.machines

                Rectangle {
                    id: machine

                    required property var modelData
                    readonly property var status: root.machineStatus[modelData.address]
                    readonly property var progress: root.updateProgress[modelData.address]
                    readonly property var description: Updates.describe(status, progress, root.appProtocol)
                    readonly property bool busy: Updates.busy(progress) || (status?.checking ?? false)
                    readonly property bool canUpdate: Updates.updatable(status, progress)
                    readonly property bool showLog: (root.expanded[modelData.address] ?? false) && (progress?.lines.length ?? 0) > 0
                    readonly property string name: status?.hostname || modelData.hostname || modelData.address

                    width: list.width
                    height: content.implicitHeight + 16
                    radius: Appearance.rounding.small
                    color: Appearance.colors.layer2

                    Column {
                        id: content

                        x: 12
                        y: 8
                        width: parent.width - 24
                        spacing: 6

                        Item {
                            width: parent.width
                            height: Math.max(44, texts.implicitHeight)

                            Rectangle {
                                id: dot
                                anchors.verticalCenter: parent.verticalCenter
                                width: 10
                                height: 10
                                radius: 5
                                color: !machine.status ? Appearance.colors.outline
                                     : machine.status.online ? Appearance.colors.good : Appearance.colors.error

                                Behavior on color {
                                    ElementMoveFastColor {}
                                }
                            }

                            Column {
                                id: texts

                                anchors.left: dot.right
                                anchors.leftMargin: 12
                                anchors.right: actions.left
                                anchors.rightMargin: 8
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 1

                                StyledText {
                                    width: parent.width
                                    text: machine.name
                                    font.pixelSize: Appearance.fontSize.smallie
                                    font.weight: Font.Medium
                                    elide: Text.ElideRight
                                }
                                StyledText {
                                    width: parent.width
                                    text: {
                                        const s = machine.status
                                        const parts = [machine.modelData.address]
                                        if (s && s.build)
                                            parts.push(qsTr("build %1").arg(s.build))
                                        if (s && s.version)
                                            parts.push(qsTr("protocol %1").arg(s.version))
                                        if (s && s.arch)
                                            parts.push(s.arch)
                                        return parts.join(" · ")
                                    }
                                    color: Appearance.colors.onSurfaceVariant
                                    font.pixelSize: Appearance.fontSize.smaller
                                    elide: Text.ElideRight
                                }
                                StyledText {
                                    width: parent.width
                                    text: machine.description.text
                                    color: Updates.toneColor(machine.description.tone)
                                    font.pixelSize: Appearance.fontSize.smaller
                                    wrapMode: Text.Wrap
                                    maximumLineCount: 3
                                    elide: Text.ElideRight
                                }
                            }

                            Row {
                                id: actions

                                anchors.right: parent.right
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 4

                                LoadingSpinner {
                                    anchors.verticalCenter: parent.verticalCenter
                                    visible: machine.busy
                                    running: visible
                                    width: 28
                                    height: 28
                                }
                                IconButton {
                                    anchors.verticalCenter: parent.verticalCenter
                                    visible: (machine.progress?.lines.length ?? 0) > 0
                                    size: 36
                                    iconSize: 20
                                    symbol: machine.showLog ? "expand_less" : "terminal"
                                    tooltip: machine.showLog ? qsTr("Hide Output") : qsTr("Show Output")
                                    onClicked: {
                                        const address = machine.modelData.address
                                        root.expanded = Object.assign({}, root.expanded, { [address]: !machine.showLog })
                                    }
                                }
                                TextButton {
                                    anchors.verticalCenter: parent.verticalCenter
                                    visible: machine.canUpdate
                                    variant: TextButton.Tonal
                                    symbol: "download"
                                    text: qsTr("Update")
                                    onClicked: root.updateRequested(machine.modelData.address, machine.modelData.token)
                                }
                            }
                        }

                        // The update's output, latest at the bottom.
                        Rectangle {
                            width: parent.width
                            height: Math.min(logText.implicitHeight + 16, 200)
                            visible: machine.showLog
                            radius: Appearance.rounding.verySmall
                            color: Appearance.colors.layer0
                            clip: true

                            StyledText {
                                id: logText
                                x: 8
                                width: parent.width - 16
                                // Pinned to the bottom, so the newest line shows.
                                y: Math.min(8, parent.height - implicitHeight - 8)
                                text: (machine.progress?.lines ?? []).slice(-40).join("\n")
                                font.family: Appearance.monoFontFamily
                                font.pixelSize: Appearance.fontSize.smaller
                                color: Appearance.colors.onSurfaceVariant
                                wrapMode: Text.WrapAnywhere
                            }
                        }
                    }
                }
            }
        }
    }

    StyledText {
        width: parent.width
        text: qsTr("this app: build %1, protocol %2. a machine installed without updates (install.sh --no-updates) is left alone")
              .arg(root.fleet.build()).arg(root.appProtocol)
        color: Appearance.colors.onSurfaceVariant
        font.pixelSize: Appearance.fontSize.smaller
        wrapMode: Text.Wrap
    }

    buttons: [
        TextButton {
            variant: TextButton.Filled
            symbol: "download"
            visible: root.updatable.length > 0
            text: root.updatable.length > 1 ? qsTr("Update All (%1)").arg(root.updatable.length) : qsTr("Update")
            onClicked: {
                for (const machine of root.updatable)
                    root.updateRequested(machine.address, machine.token)
            }
        },
        TextButton {
            variant: TextButton.Tonal
            symbol: "refresh"
            text: qsTr("Check Now")
            onClicked: root.checkRequested()
        },
        TextButton {
            variant: TextButton.Text
            text: qsTr("Close")
            onClicked: root.close()
        }
    ]
}
