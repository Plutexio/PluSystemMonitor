import QtQuick
import QtQuick.Templates as T
import PluSystemMonitor

// Fan curves: the list on the left, the picked one on the right to shape,
// with what it listens to and how calmly it follows.
Item {
    id: root

    required property FanStore store
    required property Monitor monitor
    property var virtualSensors: ({ sensors: [] })
    property string selectedId: ""

    signal failed(string message)

    readonly property var curves: store.settings.curves
    readonly property var curve: curves.find(c => c.id === selectedId) ?? curves[0] ?? null
    readonly property var catalog: Sources.catalog(monitor)
    readonly property var input: {
        monitor.revision
        return store.inputOf(curve)
    }
    readonly property string unit: {
        if (!curve)
            return "celsius"
        const first = catalog.find(s => curve.sources.includes(s.id))
        return first ? first.unit : "celsius"
    }

    function sourceName(id) {
        return catalog.find(s => s.id === id)?.name ?? id
    }

    function addSource(id) {
        if (!curve)
            return
        if (id.startsWith("virtual.")) {
            const linked = store.expandVirtual(virtualSensors, id)
            if (!linked || !linked.sources.length) {
                failed(qsTr("Nothing is linked to that virtual sensor on %1.").arg(monitor.hostname))
                return
            }
            store.editCurve(curve.id, c => {
                c.sources = linked.sources
                c.combine = ["first", "average", "highest", "lowest", "sum"].includes(linked.mode) ? linked.mode : "highest"
                c.virtual_id = id
            })
            return
        }
        store.editCurve(curve.id, c => {
            if (!c.sources.includes(id))
                c.sources.push(id)
            delete c.virtual_id
        })
    }

    // The list.
    Column {
        id: list

        x: Appearance.sizes.pageMargin
        width: 270
        height: parent.height - Appearance.sizes.pageMargin
        spacing: 8

        Flickable {
            width: parent.width
            height: Math.min(listColumn.implicitHeight, parent.height - newButton.height - 8)
            contentHeight: listColumn.implicitHeight
            boundsBehavior: Flickable.StopAtBounds
            clip: true

            T.ScrollBar.vertical: StyledScrollBar {}

            Column {
                id: listColumn
                width: parent.width
                spacing: 6

                Repeater {
                    model: root.curves

                    RippleButton {
                        id: item

                        required property var modelData
                        readonly property bool chosen: root.curve && root.curve.id === modelData.id
                        readonly property var following: root.store.fansFollowing(modelData.id)
                        readonly property var now: {
                            root.monitor.revision
                            return root.store.inputOf(modelData)
                        }

                        width: listColumn.width
                        height: 72
                        buttonRadius: Appearance.rounding.normal
                        toggled: chosen
                        colBackground: Appearance.colors.layer2
                        colBackgroundToggled: Appearance.colors.secondaryContainer
                        colBackgroundToggledHover: Appearance.colors.secondaryContainerHover
                        onClicked: root.selectedId = modelData.id

                        contentItem: Item {
                            ShapeIcon {
                                id: icon
                                x: 12
                                anchors.verticalCenter: parent.verticalCenter
                                size: 40
                                iconSize: 20
                                shape: "sunny"
                                symbol: "show_chart"
                                color: item.chosen ? Appearance.colors.primary : Appearance.colors.layer3
                                iconColor: item.chosen ? Appearance.colors.onPrimary : Appearance.colors.onSurfaceVariant
                                hovered: item.hovered
                            }
                            Column {
                                anchors.left: icon.right
                                anchors.leftMargin: 12
                                anchors.right: parent.right
                                anchors.rightMargin: 12
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 2

                                StyledText {
                                    width: parent.width
                                    text: item.modelData.name
                                    font.weight: Font.DemiBold
                                    elide: Text.ElideRight
                                    color: item.chosen ? Appearance.colors.onSecondaryContainer : Appearance.colors.onSurface
                                }
                                StyledText {
                                    width: parent.width
                                    text: {
                                        const fans = item.following.length === 1 ? qsTr("1 fan") : qsTr("%1 fans").arg(item.following.length)
                                        if (item.now === null)
                                            return fans
                                        const speed = Math.round(root.store.speedAt(item.modelData.points, item.now))
                                        return qsTr("%1 · %2 → %3 %").arg(fans).arg(Format.metric(item.now, "celsius")).arg(speed)
                                    }
                                    color: Appearance.colors.onSurfaceVariant
                                    font.pixelSize: Appearance.fontSize.smaller
                                    elide: Text.ElideRight
                                    tabularNumbers: true
                                }
                            }
                        }
                    }
                }
            }
        }

        TextButton {
            id: newButton
            variant: TextButton.Tonal
            symbol: "add"
            text: qsTr("New Curve")
            enabled: root.store.usable
            onClicked: newPicker.open()

            SourcePicker {
                id: newPicker
                monitor: root.monitor
                headline: qsTr("What Should It Follow?")
                machineOnly: true
                withVirtual: true
                onPicked: id => {
                    const created = root.store.addCurve(root.sourceName(id), [])
                    root.selectedId = created
                    Qt.callLater(() => root.addSource(id))
                }
            }
        }
    }

    EmptyState {
        anchors.left: list.right
        anchors.right: parent.right
        anchors.verticalCenter: parent.verticalCenter
        visible: !root.curve
        symbol: "show_chart"
        title: qsTr("No curves yet")
        description: qsTr("A curve sets a fan's speed by a temperature. Make one, then pick it on the fans that should follow it.")
    }

    // The editor.
    Flickable {
        id: editor

        anchors.left: list.right
        anchors.leftMargin: 16
        anchors.right: parent.right
        anchors.rightMargin: Appearance.sizes.pageMargin - 10
        height: parent.height
        visible: root.curve !== null
        contentHeight: editorColumn.implicitHeight + Appearance.sizes.pageMargin
        boundsBehavior: Flickable.StopAtBounds
        clip: true

        T.ScrollBar.vertical: StyledScrollBar {}

        Column {
            id: editorColumn

            width: editor.width - 10
            spacing: 12

            Rectangle {
                width: parent.width
                height: graphColumn.implicitHeight + 36
                radius: Appearance.rounding.large
                color: Appearance.colors.layer2

                Column {
                    id: graphColumn

                    x: 18
                    y: 18
                    width: parent.width - 36
                    spacing: 12

                    Item {
                        width: parent.width
                        height: 52

                        StyledTextField {
                            id: nameField

                            readonly property string curveId: root.curve ? root.curve.id : ""

                            anchors.left: parent.left
                            anchors.right: deleteButton.left
                            anchors.rightMargin: 12
                            anchors.verticalCenter: parent.verticalCenter
                            label: qsTr("Name")
                            // Set when another curve is picked: a binding would fight the typing.
                            onCurveIdChanged: text = root.curve ? root.curve.name : ""
                            Component.onCompleted: text = root.curve ? root.curve.name : ""
                            onEditingFinished: {
                                const name = text.trim()
                                if (root.curve && name && name !== root.curve.name)
                                    root.store.editCurve(root.curve.id, c => c.name = name)
                            }
                        }
                        TextButton {
                            id: deleteButton

                            property bool armed: false

                            anchors.right: parent.right
                            anchors.verticalCenter: parent.verticalCenter
                            variant: armed ? TextButton.Danger : TextButton.DangerTonal
                            symbol: "delete"
                            text: armed ? qsTr("Click Again") : qsTr("Delete")
                            onClicked: {
                                if (!armed) {
                                    armed = true
                                    disarm.restart()
                                    return
                                }
                                armed = false
                                root.store.removeCurve(root.curve.id)
                                root.selectedId = ""
                            }

                            Timer {
                                id: disarm
                                interval: 3000
                                onTriggered: deleteButton.armed = false
                            }
                        }
                    }

                    CurveGraph {
                        width: parent.width
                        height: 300
                        points: root.curve ? root.curve.points : []
                        unit: root.unit
                        input: root.input
                        hysteresis: root.curve ? root.curve.hysteresis : 0
                        xMin: {
                            const lowest = Math.min(...points.map(p => p[0]))
                            const base = unit === "celsius" ? 20 : 0
                            return Math.min(base, isFinite(lowest) ? lowest : base)
                        }
                        xMax: {
                            const highest = Math.max(...points.map(p => p[0]))
                            const source = Sources.read(root.monitor, root.curve ? root.curve.sources.slice(0, 1) : [])[0]
                            const base = source && source.max > 0 && unit !== "celsius" ? source.max : 100
                            return Math.max(base, isFinite(highest) ? highest : base)
                        }
                        onEdited: points => root.store.editCurve(root.curve.id, c => c.points = points)
                    }

                    StyledText {
                        width: parent.width
                        text: qsTr("Drag a point to move it. Double-click the graph to add a point, or a point to remove it.")
                        color: Appearance.colors.onSurfaceVariant
                        font.pixelSize: Appearance.fontSize.smaller
                        wrapMode: Text.Wrap
                    }
                }
            }

            // What it listens to.
            Rectangle {
                width: parent.width
                height: inputColumn.implicitHeight + 36
                radius: Appearance.rounding.large
                color: Appearance.colors.layer2

                Column {
                    id: inputColumn

                    x: 18
                    y: 18
                    width: parent.width - 36
                    spacing: 12

                    SectionHeader {
                        width: parent.width
                        title: qsTr("Follows")
                        symbol: "thermostat"
                        iconShape: "sunny"
                    }

                    StyledText {
                        width: parent.width
                        visible: !!(root.curve && root.curve.virtual_id)
                        text: qsTr("From the virtual sensor %1, as linked on %2. Changing the sources here unlinks it.")
                              .arg(root.curve && root.curve.virtual_id ? root.sourceName(root.curve.virtual_id) : "")
                              .arg(root.monitor.hostname)
                        color: Appearance.colors.onSurfaceVariant
                        font.pixelSize: Appearance.fontSize.smaller
                        wrapMode: Text.Wrap
                    }

                    Flow {
                        width: parent.width
                        spacing: 6

                        Repeater {
                            model: root.curve ? root.curve.sources : []

                            Rectangle {
                                id: chip

                                required property string modelData
                                readonly property var reading: {
                                    root.monitor.revision
                                    return Sources.read(root.monitor, [modelData])[0]
                                }

                                width: chipRow.implicitWidth + 8
                                height: 32
                                radius: 16
                                color: reading.available ? Appearance.colors.secondaryContainer : Appearance.colors.layer3

                                Row {
                                    id: chipRow
                                    x: 12
                                    anchors.verticalCenter: parent.verticalCenter
                                    spacing: 6

                                    StyledText {
                                        anchors.verticalCenter: parent.verticalCenter
                                        width: Math.min(implicitWidth, 260)
                                        text: root.sourceName(chip.modelData)
                                        color: chip.reading.available ? Appearance.colors.onSecondaryContainer : Appearance.colors.onSurfaceVariant
                                        font.pixelSize: Appearance.fontSize.smaller
                                        elide: Text.ElideRight
                                    }
                                    StyledText {
                                        anchors.verticalCenter: parent.verticalCenter
                                        text: chip.reading.text
                                        color: Appearance.colors.primary
                                        font.pixelSize: Appearance.fontSize.smaller
                                        font.weight: Font.DemiBold
                                        tabularNumbers: true
                                    }
                                    IconButton {
                                        anchors.verticalCenter: parent.verticalCenter
                                        size: 26
                                        iconSize: 16
                                        symbol: "close"
                                        tooltip: qsTr("Remove")
                                        onClicked: root.store.editCurve(root.curve.id, c => {
                                            c.sources = c.sources.filter(s => s !== chip.modelData)
                                            delete c.virtual_id
                                        })
                                    }
                                }
                            }
                        }
                        TextButton {
                            height: 32
                            variant: TextButton.Outlined
                            symbol: "add"
                            text: qsTr("Add a Source")
                            enabled: root.monitor.connected
                            onClicked: sourcePicker.open()

                            SourcePicker {
                                id: sourcePicker
                                monitor: root.monitor
                                headline: qsTr("Follow a Source")
                                machineOnly: true
                                withVirtual: true
                                taken: root.curve ? root.curve.sources : []
                                onPicked: id => root.addSource(id)
                            }
                        }
                    }

                    Column {
                        width: parent.width
                        visible: root.curve && root.curve.sources.length > 1
                        spacing: 6

                        StyledText {
                            text: qsTr("With several, follow the")
                            color: Appearance.colors.onSurfaceVariant
                            font.pixelSize: Appearance.fontSize.smaller
                        }
                        SegmentedControl {
                            currentIndex: root.curve ? root.store.modes.findIndex(m => m.id === root.curve.combine) : -1
                            segments: [
                                Segment { text: root.store.modes[0].name; symbol: "local_fire_department" },
                                Segment { text: root.store.modes[1].name; symbol: "functions" },
                                Segment { text: root.store.modes[2].name; symbol: "ac_unit" },
                                Segment { text: root.store.modes[3].name; symbol: "looks_one" }
                            ]
                            onActivated: index => root.store.editCurve(root.curve.id, c => {
                                c.combine = root.store.modes[index].id
                                delete c.virtual_id
                            })
                        }
                    }
                }
            }

            // How calmly it follows.
            Rectangle {
                width: parent.width
                height: tuneColumn.implicitHeight + 36
                radius: Appearance.rounding.large
                color: Appearance.colors.layer2

                Column {
                    id: tuneColumn

                    x: 18
                    y: 18
                    width: parent.width - 36
                    spacing: 6

                    SectionHeader {
                        width: parent.width
                        title: qsTr("Calm")
                        symbol: "waves"
                        iconShape: "cookie9"
                    }

                    component Tuning: Column {
                        id: tuning

                        property string title
                        property string note
                        property real from: 0
                        property real to: 10
                        property real stepSize: 1
                        property real value: 0
                        property var format: v => v
                        property string key

                        width: tuneColumn.width
                        spacing: 0

                        Item {
                            width: parent.width
                            height: 22
                            StyledText {
                                text: tuning.title
                                font.pixelSize: Appearance.fontSize.smallie
                                font.weight: Font.DemiBold
                            }
                            StyledText {
                                anchors.right: parent.right
                                text: tuning.format(tuning.value)
                                color: Appearance.colors.primary
                                font.pixelSize: Appearance.fontSize.smallie
                                font.weight: Font.DemiBold
                                tabularNumbers: true
                            }
                        }
                        StyledText {
                            width: parent.width
                            text: tuning.note
                            color: Appearance.colors.onSurfaceVariant
                            font.pixelSize: Appearance.fontSize.smaller
                            wrapMode: Text.Wrap
                        }
                        StyledSlider {
                            id: slider
                            width: parent.width
                            from: tuning.from
                            to: tuning.to
                            stepSize: tuning.stepSize
                            value: tuning.value
                            format: tuning.format
                            onMoved: save.restart()
                        }
                        Timer {
                            id: save
                            interval: 200
                            // Seconds are whole; the rest keeps the slider's steps.
                            onTriggered: root.store.editCurve(root.curve.id, c => c[tuning.key] = tuning.key === "average_secs" ? Math.round(slider.value) : slider.value)
                        }
                    }

                    Tuning {
                        title: qsTr("Hysteresis")
                        note: qsTr("Speeds rise at once but only drop once the input fell this far: no hunting up and down around a point.")
                        key: "hysteresis"
                        to: 10
                        stepSize: 0.5
                        value: root.curve ? root.curve.hysteresis : 0
                        format: v => v === 0 ? qsTr("off") : v.toFixed(v % 1 ? 1 : 0) + " °C"
                    }
                    Tuning {
                        title: qsTr("Smoothing")
                        note: qsTr("Follows the input's average over this long, so a short spike doesn't spin the fans up.")
                        key: "average_secs"
                        to: 60
                        value: root.curve ? root.curve.average_secs : 0
                        format: v => v === 0 ? qsTr("off") : qsTr("%1 s").arg(v)
                    }
                    Tuning {
                        title: qsTr("Rise at most")
                        note: qsTr("How fast speeds may go up.")
                        key: "step_up"
                        to: 20
                        value: root.curve ? root.curve.step_up : 0
                        format: v => v === 0 ? qsTr("at once") : qsTr("%1 % a second").arg(v)
                    }
                    Tuning {
                        title: qsTr("Fall at most")
                        note: qsTr("How fast speeds may come down; slow is quieter on the ear.")
                        key: "step_down"
                        to: 20
                        value: root.curve ? root.curve.step_down : 0
                        format: v => v === 0 ? qsTr("at once") : qsTr("%1 % a second").arg(v)
                    }
                }
            }
        }
    }
}
