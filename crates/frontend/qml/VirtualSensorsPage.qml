import QtQuick
import QtQuick.Templates as T
import PluSystemMonitor

// Virtual sensors: sources of the user's own (`virtual.<n>`) that each machine
// fills from the real sources linked to it there, so a dashboard widget about
// "CPU voltage" works on machines whose boards call it something else.
// Every edit is handed up right away and saved.
Column {
    id: root

    required property Monitor monitor
    /// `{ sensors: [{ id, name, unit, mode, bindings: [{ machine, hostname, sources }] }] }`
    property var sensors: ({ sensors: [] })
    readonly property var list: sensors.sensors ?? []
    readonly property string machine: monitor.fingerprint
    readonly property string hostname: monitor.hostname || qsTr("this machine")
    /// How tall the list of sensors may get.
    property real listHeight: 400

    signal edited(var sensors)

    readonly property var modes: [
        { id: "first", name: qsTr("First available") },
        { id: "average", name: qsTr("Average") },
        { id: "highest", name: qsTr("Highest") },
        { id: "lowest", name: qsTr("Lowest") },
        { id: "sum", name: qsTr("Sum") }
    ]

    function unitName(unit) {
        switch (unit) {
        case "percent": return qsTr("Percent")
        case "bytes": return qsTr("Bytes")
        case "celsius": return qsTr("Temperature")
        case "watts": return qsTr("Power")
        case "megahertz": return qsTr("Clock")
        case "volts": return qsTr("Voltage")
        case "rpm": return qsTr("Fan speed")
        case "amps": return qsTr("Current")
        }
        return unit
    }

    function sourceName(id) {
        const source = Sources.catalog(monitor).find(s => s.id === id)
        return source ? source.name : id
    }

    // Edits: copy, change, hand up.
    function change(index, update) {
        const sensors = list.map(s => Object.assign({}, s, { bindings: s.bindings.map(b => Object.assign({}, b, { sources: b.sources.slice() })) }))
        update(sensors[index])
        edited({ sensors: sensors })
    }

    function bindingOf(sensor, machine) {
        let binding = sensor.bindings.find(b => b.machine === machine)
        if (!binding) {
            binding = { machine: machine, hostname: machine ? root.hostname : "", sources: [] }
            sensor.bindings.push(binding)
        }
        return binding
    }

    function link(index, machine, id) {
        change(index, sensor => {
            const binding = bindingOf(sensor, machine)
            if (machine)
                binding.hostname = root.monitor.hostname
            if (!binding.sources.includes(id))
                binding.sources.push(id)
        })
    }

    function unlink(index, machine, id) {
        change(index, sensor => {
            const binding = bindingOf(sensor, machine)
            binding.sources = id === undefined ? [] : binding.sources.filter(s => s !== id)
        })
    }

    function create(sourceId) {
        const source = Sources.catalog(monitor).find(s => s.id === sourceId)
        if (!source)
            return
        const used = list.map(s => parseInt(s.id.split(".")[1]) || 0)
        const id = "virtual." + (Math.max(0, ...used) + 1)
        const sensor = {
            id: id,
            name: source.name,
            unit: source.unit,
            mode: "first",
            bindings: [{ machine: root.machine, hostname: root.monitor.hostname, sources: [sourceId] }]
        }
        edited({ sensors: list.concat([sensor]) })
    }

    width: parent ? parent.width : 400
    spacing: 12

    // A linked source: its name, its value now, a way to unlink it.
    component SourceChip: Rectangle {
        id: chip

        property string sourceId
        property bool present: true

        signal removed()

        width: Math.min(chipRow.implicitWidth + 8, 320)
        height: 32
        radius: 16
        color: present ? Appearance.colors.secondaryContainer : Appearance.colors.layer3

        Row {
            id: chipRow
            x: 12
            anchors.verticalCenter: parent.verticalCenter
            spacing: 4

            StyledText {
                anchors.verticalCenter: parent.verticalCenter
                width: Math.min(implicitWidth, 260)
                text: root.sourceName(chip.sourceId)
                color: chip.present ? Appearance.colors.onSecondaryContainer : Appearance.colors.onSurfaceVariant
                font.pixelSize: Appearance.fontSize.smaller
                elide: Text.ElideRight
            }
            IconButton {
                anchors.verticalCenter: parent.verticalCenter
                size: 26
                iconSize: 16
                symbol: "close"
                tooltip: qsTr("Unlink")
                onClicked: chip.removed()
            }
        }
    }

    // One machine's links: a heading, the chips, a button for more.
    component Links: Column {
        id: links

        property int index
        property var sensor
        /// Fingerprint, or "" for every other machine.
        property string machine
        property string title
        property string note
        readonly property var binding: sensor.bindings.find(b => b.machine === machine)
        readonly property var sources: binding ? binding.sources : []

        width: parent.width
        spacing: 6

        StyledText {
            width: parent.width
            text: links.title
            font.pixelSize: Appearance.fontSize.smaller
            font.weight: Font.DemiBold
            color: Appearance.colors.onSurfaceVariant
        }
        StyledText {
            width: parent.width
            visible: links.note.length > 0
            text: links.note
            font.pixelSize: Appearance.fontSize.smaller
            color: Appearance.colors.onSurfaceVariant
            wrapMode: Text.Wrap
        }
        Flow {
            width: parent.width
            spacing: 6

            Repeater {
                model: links.sources

                SourceChip {
                    required property string modelData
                    sourceId: modelData
                    present: Sources.catalog(root.monitor).some(s => s.id === modelData)
                    onRemoved: root.unlink(links.index, links.machine, modelData)
                }
            }
            TextButton {
                height: 32
                variant: TextButton.Outlined
                symbol: "add_link"
                text: qsTr("Link a Source")
                enabled: root.monitor.connected
                onClicked: picker.open()

                SourcePicker {
                    id: picker
                    monitor: root.monitor
                    headline: qsTr("Link a Source")
                    unit: links.sensor.unit
                    machineOnly: true
                    taken: links.sources
                    onPicked: id => root.link(links.index, links.machine, id)
                }
            }
        }
    }

    StyledText {
        width: parent.width
        text: qsTr("Sources of your own for the dashboard. On each machine, link the sensors that measure the thing; a widget showing the virtual sensor then works on all of them. When several are linked, the mode says how they combine. Kept in this app, with the layout.")
        color: Appearance.colors.onSurfaceVariant
        wrapMode: Text.Wrap
    }

    Flickable {
        width: parent.width
        height: Math.min(cards.implicitHeight, root.listHeight)
        visible: root.list.length > 0
        contentHeight: cards.implicitHeight
        boundsBehavior: Flickable.StopAtBounds
        clip: true

        T.ScrollBar.vertical: StyledScrollBar {}

        Column {
            id: cards

            width: parent.width - 10
            spacing: 8

            Repeater {
                model: root.list

                Rectangle {
                    id: card

                    required property var modelData
                    required property int index
                    readonly property var reading: {
                        root.monitor.revision
                        return Sources.read(root.monitor, [modelData.id])[0]
                    }
                    readonly property var others: modelData.bindings.filter(b => b.machine && b.machine !== root.machine)

                    width: cards.width
                    height: body.implicitHeight + 24
                    radius: Appearance.rounding.normal
                    color: Appearance.colors.layer2

                    Column {
                        id: body

                        x: 14
                        y: 12
                        width: parent.width - 28
                        spacing: 10

                        Item {
                            width: parent.width
                            height: 48

                            StyledTextField {
                                id: nameField
                                anchors.left: parent.left
                                anchors.right: valueText.left
                                anchors.rightMargin: 12
                                anchors.verticalCenter: parent.verticalCenter
                                height: 48
                                label: root.unitName(card.modelData.unit) + " · " + card.modelData.id
                                // Set once: a binding would fight the typing.
                                Component.onCompleted: text = card.modelData.name
                                onEditingFinished: {
                                    if (text.trim() && text.trim() !== card.modelData.name)
                                        root.change(card.index, sensor => sensor.name = text.trim())
                                }
                            }
                            StyledText {
                                id: valueText
                                anchors.right: modeButton.left
                                anchors.rightMargin: 8
                                anchors.verticalCenter: parent.verticalCenter
                                text: card.reading.text
                                color: card.reading.available ? Appearance.colors.primary : Appearance.colors.onSurfaceVariant
                                font.family: Appearance.monoFontFamily
                                font.pixelSize: Appearance.fontSize.smallie
                            }
                            TextButton {
                                id: modeButton
                                anchors.right: removeButton.left
                                anchors.rightMargin: 4
                                anchors.verticalCenter: parent.verticalCenter
                                variant: TextButton.Tonal
                                symbol: "function"
                                text: (root.modes.find(m => m.id === card.modelData.mode) ?? root.modes[0]).name
                                onClicked: modeMenu.open()

                                StyledPopover {
                                    id: modeMenu
                                    y: modeButton.height + 4
                                    padding: 6

                                    contentItem: Column {
                                        Repeater {
                                            model: root.modes

                                            RippleButton {
                                                required property var modelData
                                                width: 190
                                                height: 38
                                                buttonRadius: Appearance.rounding.small
                                                bounce: false
                                                onClicked: {
                                                    root.change(card.index, sensor => sensor.mode = modelData.id)
                                                    modeMenu.close()
                                                }
                                                contentItem: Row {
                                                    spacing: 8
                                                    leftPadding: 8
                                                    MaterialSymbol {
                                                        anchors.verticalCenter: parent.verticalCenter
                                                        text: "check"
                                                        iconSize: 18
                                                        color: Appearance.colors.primary
                                                        opacity: modelData.id === card.modelData.mode ? 1 : 0
                                                    }
                                                    StyledText {
                                                        anchors.verticalCenter: parent.verticalCenter
                                                        text: modelData.name
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            IconButton {
                                id: removeButton
                                anchors.right: parent.right
                                anchors.verticalCenter: parent.verticalCenter
                                size: 36
                                iconSize: 20
                                symbol: "delete"
                                tooltip: qsTr("Delete; widgets showing it go empty")
                                onClicked: root.edited({ sensors: root.list.filter((_, i) => i !== card.index) })
                            }
                        }

                        Links {
                            index: card.index
                            sensor: card.modelData
                            machine: root.machine
                            visible: root.machine.length > 0
                            title: qsTr("On %1 (connected)").arg(root.hostname)
                            note: sources.length === 0 && card.modelData.bindings.some(b => b.machine === "")
                                  ? qsTr("nothing of its own: it uses the links for every other machine below")
                                  : ""
                        }

                        Links {
                            index: card.index
                            sensor: card.modelData
                            machine: ""
                            title: qsTr("On every other machine")
                            note: qsTr("for machines without links of their own: the first of these each one has. handy for many alike")
                        }

                        Column {
                            width: parent.width
                            visible: card.others.length > 0
                            spacing: 6

                            StyledText {
                                text: qsTr("Linked on other machines too (connect to one to change it)")
                                font.pixelSize: Appearance.fontSize.smaller
                                font.weight: Font.DemiBold
                                color: Appearance.colors.onSurfaceVariant
                            }
                            Flow {
                                width: parent.width
                                spacing: 6

                                Repeater {
                                    model: card.others

                                    Rectangle {
                                        id: other
                                        required property var modelData
                                        width: otherRow.implicitWidth + 8
                                        height: 32
                                        radius: 16
                                        color: Appearance.colors.layer3

                                        Row {
                                            id: otherRow
                                            x: 12
                                            anchors.verticalCenter: parent.verticalCenter
                                            spacing: 4

                                            StyledText {
                                                anchors.verticalCenter: parent.verticalCenter
                                                text: qsTr("%1 · %2").arg(other.modelData.hostname || other.modelData.machine.slice(0, 11))
                                                                    .arg(other.modelData.sources.length === 1 ? qsTr("1 source") : qsTr("%1 sources").arg(other.modelData.sources.length))
                                                font.pixelSize: Appearance.fontSize.smaller
                                                color: Appearance.colors.onSurfaceVariant
                                            }
                                            IconButton {
                                                anchors.verticalCenter: parent.verticalCenter
                                                size: 26
                                                iconSize: 16
                                                symbol: "close"
                                                tooltip: qsTr("Forget these links")
                                                onClicked: root.unlink(card.index, other.modelData.machine)
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    TextButton {
        variant: TextButton.Tonal
        symbol: "add"
        text: qsTr("New Virtual Sensor")
        enabled: root.monitor.connected
        onClicked: newPicker.open()

        SourcePicker {
            id: newPicker
            monitor: root.monitor
            headline: qsTr("Start From a Source")
            machineOnly: true
            onPicked: id => root.create(id)
        }
    }
}
