import QtQuick
import QtQuick.Templates as T
import PluSystemMonitor

// Every fan the machine lets us drive, grouped by what it cools, or why
// there are none to show.
Item {
    id: root

    required property FanStore store
    required property Monitor monitor

    signal newCurveRequested(string fanId)

    readonly property var devices: {
        const seen = []
        for (const output of store.outputs) {
            if (!seen.includes(output.device))
                seen.push(output.device)
        }
        return seen
    }
    // How many cards fit across.
    readonly property int columns: Math.max(1, Math.floor((width - 2 * Appearance.sizes.pageMargin + 12) / (340 + 12)))
    readonly property real cardWidth: (width - 2 * Appearance.sizes.pageMargin - (columns - 1) * 12) / columns

    // Why there's nothing here: not connected, an old or fanless daemon, no helper, no fans.
    readonly property var absence: {
        if (!monitor.connected)
            return { symbol: "lan", title: qsTr("Not connected"), text: monitor.status }
        if (!store.loaded)
            return { symbol: "hourglass_empty", title: qsTr("Asking the daemon…"), text: "" }
        if (!store.supported)
            return { symbol: "system_update", title: qsTr("This daemon is too old"),
                     text: qsTr("%1 runs a daemon from before fan control. Update it under Updates.").arg(monitor.hostname) }
        if (!store.status.helper)
            return { symbol: "mode_fan_off", title: qsTr("Fan control isn't set up on %1").arg(monitor.hostname),
                     text: store.status.problem }
        if (store.outputs.length === 0)
            return { symbol: "mode_fan_off", title: qsTr("No fans to control"),
                     text: qsTr("Nothing on %1 takes a fan speed from software: the board's fan chip may need its driver (for many boards it87 or nct6775), or the BIOS keeps the fans to itself.").arg(monitor.hostname) }
        return null
    }

    EmptyState {
        anchors.centerIn: parent
        width: Math.min(parent.width - 64, 520)
        visible: root.absence !== null
        symbol: root.absence ? root.absence.symbol : ""
        title: root.absence ? root.absence.title : ""
        description: root.absence ? root.absence.text : ""
    }

    Flickable {
        id: flick

        anchors.fill: parent
        visible: root.absence === null
        contentHeight: content.implicitHeight + Appearance.sizes.pageMargin
        boundsBehavior: Flickable.StopAtBounds
        clip: true

        T.ScrollBar.vertical: StyledScrollBar {}

        Column {
            id: content

            x: Appearance.sizes.pageMargin
            width: flick.width - 2 * Appearance.sizes.pageMargin
            spacing: Appearance.sizes.sectionGap

            Repeater {
                model: root.devices

                Column {
                    id: group

                    required property string modelData
                    readonly property var outputs: root.store.outputs.filter(o => o.device === modelData)

                    width: content.width
                    spacing: 14

                    SectionHeader {
                        width: parent.width
                        title: group.modelData
                        symbol: group.outputs[0].id.startsWith("nvidia.") ? "developer_board" : "memory"
                        iconShape: group.outputs[0].id.startsWith("nvidia.") ? "gem" : "cookie12"
                    }

                    Flow {
                        width: parent.width
                        spacing: 12

                        Repeater {
                            model: group.outputs

                            FanCard {
                                required property var modelData
                                width: root.cardWidth
                                store: root.store
                                monitor: root.monitor
                                output: modelData
                                onNewCurveRequested: fanId => root.newCurveRequested(fanId)
                            }
                        }
                    }
                }
            }

            StyledText {
                width: parent.width
                text: qsTr("Fans on Auto stay with the BIOS or the driver. Whatever this app sets, the machine's daemon keeps doing without the app open; should the daemon stop, its helper gives every fan back to the firmware within seconds.")
                color: Appearance.colors.onSurfaceVariant
                font.pixelSize: Appearance.fontSize.smaller
                wrapMode: Text.Wrap
            }
        }
    }
}
