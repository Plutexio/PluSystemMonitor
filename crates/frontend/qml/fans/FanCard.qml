import QtQuick
import PluSystemMonitor

// One fan: its name, how fast it runs, and who decides that: the firmware,
// a fixed speed, or a curve. Its badge turns as fast as the fan does.
Rectangle {
    id: root

    required property FanStore store
    required property Monitor monitor
    /// As the daemon lists it: `{ id, name, device, rpm_source, percent, min_percent }`.
    required property var output

    signal newCurveRequested(string fanId)

    readonly property var fan: store.fan(output.id)
    readonly property var fanState: store.states[output.id] ?? null
    readonly property var rpm: {
        monitor.revision
        return output.rpm_source ? Sources.read(monitor, [output.rpm_source])[0] : null
    }
    readonly property real percent: fanState ? fanState.percent : (output.percent ?? -1)
    readonly property string problem: fan.mode !== "auto" && fanState ? fanState.problem : ""
    readonly property var curve: fan.mode === "curve" ? store.curve(fan.curve) : null
    readonly property var modes: ["auto", "fixed", "curve"]

    implicitHeight: body.implicitHeight + 36
    radius: Appearance.rounding.large
    color: Appearance.colors.layer2

    HoverHandler {
        id: hover
    }

    Column {
        id: body

        x: 18
        y: 18
        width: parent.width - 36
        spacing: 14

        // Badge, name and device, rename.
        Item {
            width: parent.width
            height: 44

            Item {
                id: badge

                width: 44
                height: 44

                MaterialShape {
                    anchors.fill: parent
                    shape: "clover4"
                    color: root.fan.mode === "auto" ? Appearance.colors.layer3 : Appearance.colors.primaryContainer
                }
                MaterialSymbol {
                    id: glyph
                    anchors.centerIn: parent
                    text: "mode_fan"
                    iconSize: 24
                    fill: 1
                    color: root.fan.mode === "auto" ? Appearance.colors.onSurfaceVariant : Appearance.colors.onPrimaryContainer
                }

                // Faster fans turn faster, in a few steps so a wobbling
                // reading doesn't restart the turn every second.
                readonly property int period: {
                    const speed = root.rpm && root.rpm.available ? Math.min(1, root.rpm.value / 2000)
                                : root.percent >= 0 ? root.percent / 100 : 0
                    if (speed < 0.02)
                        return 0
                    return [2400, 1600, 1000, 650, 420][Math.min(4, Math.floor(speed * 5))]
                }

                RotationAnimator {
                    target: badge
                    from: 0
                    to: 360
                    duration: badge.period
                    loops: Animation.Infinite
                    running: badge.period > 0 && !Appearance.reduceMotion && root.visible
                    onStopped: badge.rotation = 0
                }
            }

            Column {
                anchors.left: badge.right
                anchors.leftMargin: 14
                anchors.right: renameButton.left
                anchors.rightMargin: 6
                anchors.verticalCenter: parent.verticalCenter
                spacing: 2

                StyledText {
                    width: parent.width
                    text: root.store.outputName(root.output.id)
                    font.pixelSize: Appearance.fontSize.large
                    font.weight: Font.DemiBold
                    elide: Text.ElideRight
                }
                StyledText {
                    width: parent.width
                    text: root.fan.label ? root.output.name : root.output.device
                    color: Appearance.colors.onSurfaceVariant
                    font.pixelSize: Appearance.fontSize.smaller
                    elide: Text.ElideRight
                }
            }

            IconButton {
                id: renameButton
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                size: 36
                iconSize: 20
                symbol: "edit"
                tooltip: qsTr("Rename")
                opacity: hover.hovered || renamePopover.opened ? 1 : 0
                onClicked: {
                    nameField.text = root.fan.label ?? ""
                    renamePopover.open()
                }

                Behavior on opacity {
                    NumberAnimation { duration: Appearance.duration(150) }
                }

                StyledPopover {
                    id: renamePopover
                    x: renameButton.width - width
                    y: renameButton.height + 4
                    onOpened: nameField.forceActiveFocus()

                    contentItem: Column {
                        width: 260
                        spacing: 8

                        StyledTextField {
                            id: nameField
                            width: parent.width
                            label: qsTr("Name")
                            placeholderText: root.output.name
                            onAccepted: renamePopover.rename()
                        }
                        TextButton {
                            anchors.right: parent.right
                            variant: TextButton.Filled
                            text: qsTr("Rename")
                            onClicked: renamePopover.rename()
                        }
                    }

                    function rename() {
                        const name = nameField.text.trim()
                        root.store.editFan(root.output.id, fan => {
                            if (name)
                                fan.label = name
                            else
                                delete fan.label
                        })
                        close()
                    }
                }
            }
        }

        // The speed now.
        Row {
            spacing: 18

            Row {
                spacing: 4

                StyledText {
                    id: percentText
                    text: root.percent >= 0 ? Math.round(root.percent) : "—"
                    font.pixelSize: Appearance.fontSize.display
                    font.weight: Font.Bold
                    tabularNumbers: true
                    color: root.fan.mode === "auto" ? Appearance.colors.onSurface : Appearance.colors.primary
                }
                StyledText {
                    anchors.baseline: percentText.baseline
                    text: "%"
                    color: Appearance.colors.onSurfaceVariant
                    font.pixelSize: Appearance.fontSize.large
                }
            }
            Column {
                anchors.bottom: parent.bottom
                anchors.bottomMargin: 6
                spacing: 0

                StyledText {
                    visible: root.rpm !== null
                    text: root.rpm && root.rpm.available ? qsTr("%1 RPM").arg(Math.round(root.rpm.value)) : qsTr("stopped")
                    font.pixelSize: Appearance.fontSize.small
                    font.weight: Font.DemiBold
                    tabularNumbers: true
                }
                StyledText {
                    text: {
                        if (root.fan.mode === "auto")
                            return qsTr("set by the firmware")
                        if (root.fan.mode === "fixed")
                            return qsTr("fixed")
                        if (!root.curve)
                            return qsTr("no curve")
                        const input = root.fanState ? root.fanState.input : null
                        return input === null || input === undefined
                            ? root.curve.name
                            : qsTr("%1 at %2").arg(root.curve.name).arg(Format.metric(input, "celsius"))
                    }
                    color: Appearance.colors.onSurfaceVariant
                    font.pixelSize: Appearance.fontSize.smaller
                }
            }
        }

        SegmentedControl {
            currentIndex: root.modes.indexOf(root.fan.mode)
            segments: [
                Segment { text: qsTr("Auto"); symbol: "auto_mode"; tooltip: qsTr("The BIOS or the driver decides") },
                Segment { text: qsTr("Fixed"); symbol: "horizontal_rule" },
                Segment { text: qsTr("Curve"); symbol: "show_chart" }
            ]
            onActivated: index => {
                const mode = root.modes[index]
                if (mode === "curve" && root.store.settings.curves.length === 0) {
                    root.newCurveRequested(root.output.id)
                    return
                }
                root.store.editFan(root.output.id, fan => {
                    fan.mode = mode
                    if (mode === "curve" && !root.store.curve(fan.curve))
                        fan.curve = root.store.settings.curves[0].id
                    if (mode === "fixed" && root.percent >= 0 && root.fan.mode === "auto")
                        fan.percent = Math.round(root.percent)
                })
            }
        }

        // Fixed: the speed.
        Column {
            width: parent.width
            visible: root.fan.mode === "fixed"
            spacing: 2

            StyledText {
                text: qsTr("Speed")
                color: Appearance.colors.onSurfaceVariant
                font.pixelSize: Appearance.fontSize.smaller
            }
            StyledSlider {
                width: parent.width
                from: root.output.min_percent
                to: 100
                stepSize: 1
                value: root.fan.percent
                format: v => Math.round(v) + " %"
                onMoved: fixedSave.restart()

                Timer {
                    id: fixedSave
                    interval: 150
                    onTriggered: root.store.editFan(root.output.id, fan => fan.percent = Math.round(parent.value))
                }
            }
        }

        // Curve: which one, and the limits.
        Column {
            width: parent.width
            visible: root.fan.mode === "curve"
            spacing: 10

            Flow {
                width: parent.width
                spacing: 6

                Repeater {
                    model: root.store.settings.curves

                    RippleButton {
                        id: chip
                        required property var modelData
                        readonly property bool chosen: root.fan.curve === modelData.id

                        width: chipText.implicitWidth + 28
                        height: 32
                        buttonRadius: 16
                        toggled: chosen
                        colBackground: Appearance.colors.layer3
                        onClicked: root.store.editFan(root.output.id, fan => fan.curve = chip.modelData.id)

                        contentItem: StyledText {
                            id: chipText
                            horizontalAlignment: Text.AlignHCenter
                            verticalAlignment: Text.AlignVCenter
                            text: chip.modelData.name
                            font.pixelSize: Appearance.fontSize.smaller
                            font.weight: Font.DemiBold
                            color: chip.chosen ? Appearance.colors.onPrimary : Appearance.colors.onSurface
                        }
                    }
                }
                RippleButton {
                    width: newText.implicitWidth + 40
                    height: 32
                    buttonRadius: 16
                    colBackground: "transparent"
                    onClicked: root.newCurveRequested(root.output.id)

                    contentItem: Row {
                        spacing: 4
                        leftPadding: 10
                        MaterialSymbol {
                            anchors.verticalCenter: parent.verticalCenter
                            text: "add"
                            iconSize: 18
                            color: Appearance.colors.primary
                        }
                        StyledText {
                            id: newText
                            anchors.verticalCenter: parent.verticalCenter
                            text: qsTr("New Curve")
                            font.pixelSize: Appearance.fontSize.smaller
                            font.weight: Font.DemiBold
                            color: Appearance.colors.primary
                        }
                    }
                }
            }

            Row {
                width: parent.width
                spacing: 12

                Column {
                    width: (parent.width - 12) / 2
                    StyledText {
                        text: qsTr("At least %1 %").arg(Math.round(root.fan.min))
                        color: Appearance.colors.onSurfaceVariant
                        font.pixelSize: Appearance.fontSize.smaller
                    }
                    StyledSlider {
                        id: minSlider
                        width: parent.width
                        from: 0
                        to: 100
                        stepSize: 1
                        value: root.fan.min
                        format: v => Math.round(v) + " %"
                        onMoved: limitSave.restart()
                    }
                }
                Column {
                    width: (parent.width - 12) / 2
                    StyledText {
                        text: qsTr("At most %1 %").arg(Math.round(root.fan.max))
                        color: Appearance.colors.onSurfaceVariant
                        font.pixelSize: Appearance.fontSize.smaller
                    }
                    StyledSlider {
                        id: maxSlider
                        width: parent.width
                        from: 0
                        to: 100
                        stepSize: 1
                        value: root.fan.max
                        format: v => Math.round(v) + " %"
                        onMoved: limitSave.restart()
                    }
                }
                Timer {
                    id: limitSave
                    interval: 150
                    onTriggered: root.store.editFan(root.output.id, fan => {
                        fan.min = Math.round(Math.min(minSlider.value, maxSlider.value))
                        fan.max = Math.round(Math.max(minSlider.value, maxSlider.value))
                    })
                }
            }

            Item {
                width: parent.width
                height: 32

                StyledText {
                    anchors.left: parent.left
                    anchors.right: stopSwitch.left
                    anchors.verticalCenter: parent.verticalCenter
                    text: qsTr("Stop below the minimum")
                    font.pixelSize: Appearance.fontSize.smallie
                    elide: Text.ElideRight
                }
                StyledSwitch {
                    id: stopSwitch
                    anchors.right: parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    checked: root.fan.allow_stop
                    onClicked: root.store.editFan(root.output.id, fan => fan.allow_stop = !fan.allow_stop)
                }
            }
        }

        StyledText {
            width: parent.width
            visible: root.problem.length > 0
            text: root.problem
            color: Appearance.colors.error
            font.pixelSize: Appearance.fontSize.smaller
            wrapMode: Text.Wrap
        }
    }
}
