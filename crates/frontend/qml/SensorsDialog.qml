import QtQuick
import QtQuick.Templates as T
import PluSystemMonitor

// How the connected machine presents its sensors: which one is the CPU's
// temperature (and power), and per sensor a name, a scale and whether it's
// listed. The daemon keeps this, so it holds for every app connected to that
// machine, and one dashboard fits machines whose sensors are named differently.
StyledDialog {
    id: root

    required property Monitor monitor

    /// What the daemon reported: `{ settings, sensors, error }`.
    property bool loaded: false
    property bool supported: true
    property bool saving: false
    property string error
    property var sensors: []
    /// The settings being edited.
    property var draft: ({ sensors: {} })

    readonly property var temperatures: sensors.filter(s => s.kind === "Temperature")
    readonly property var powers: sensors.filter(s => s.kind === "Power")

    headline: qsTr("Sensors")
    symbol: "sensors"
    iconShape: "clover4"
    preferredWidth: 760
    body: qsTr("How %1 shows its sensors. This is kept on that machine, so every app connected to it sees the same, and one dashboard works on machines whose sensors are named differently.")
          .arg(monitor.hostname || qsTr("this machine"))

    onAboutToShow: {
        loaded = false
        saving = false
        error = ""
        sensors = []
        supported = monitor.requestSensorSettings()
    }

    Connections {
        target: root.monitor
        function onSensorSettingsReceived(json) {
            const reply = JSON.parse(json)
            if (root.saving) {
                root.saving = false
                if (reply.error) {
                    root.error = reply.error
                    return
                }
                root.close()
                return
            }
            const settings = reply.settings
            root.draft = {
                cpu_temperature: settings.cpu_temperature ?? null,
                cpu_power: settings.cpu_power ?? null,
                sensors: settings.sensors ?? {}
            }
            root.sensors = reply.sensors
            root.error = reply.error ?? ""
            root.loaded = true
        }
    }

    function unitOf(kind) {
        switch (kind) {
        case "Temperature": return "celsius"
        case "Voltage": return "volts"
        case "Fan": return "rpm"
        case "Power": return "watts"
        case "Current": return "amps"
        }
        return ""
    }

    function overrideOf(id) {
        return Object.assign({ label: null, multiplier: 1, offset: 0, hidden: false }, draft.sensors[id] ?? {})
    }

    function setOverride(id, key, value) {
        const sensors = Object.assign({}, draft.sensors)
        sensors[id] = Object.assign(overrideOf(id), { [key]: value })
        draft = Object.assign({}, draft, { sensors: sensors })
    }

    function setChoice(key, id) {
        draft = Object.assign({}, draft, { [key]: id || null })
    }

    function sensorName(id) {
        const sensor = sensors.find(s => s.id === id)
        if (!sensor)
            return id
        const label = overrideOf(id).label
        return sensor.device + " " + (label || sensor.label)
    }

    // A setting picked from a list: its label, the choice, a menu.
    component Choice: RippleButton {
        id: choice

        property string label
        property string note
        /// `[{ id, name }]`; "" is Automatic.
        property var options: []
        property string current

        signal chosen(string id)

        width: parent.width
        height: 56
        buttonRadius: Appearance.rounding.small
        colBackgroundHover: Appearance.colors.layer3Hover
        colRipple: Appearance.colors.layer3Active
        bounce: false
        onClicked: menu.open()

        contentItem: Item {
            Column {
                anchors.left: parent.left
                anchors.leftMargin: 4
                anchors.right: value.left
                anchors.rightMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                spacing: 1

                StyledText {
                    text: choice.label
                }
                StyledText {
                    width: parent.width
                    text: choice.note
                    color: Appearance.colors.onSurfaceVariant
                    font.pixelSize: Appearance.fontSize.smaller
                    elide: Text.ElideRight
                }
            }
            Row {
                id: value
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                spacing: 4

                StyledText {
                    anchors.verticalCenter: parent.verticalCenter
                    width: Math.min(implicitWidth, choice.width * 0.5)
                    text: (choice.options.find(o => o.id === choice.current) ?? { name: choice.current }).name
                    color: Appearance.colors.primary
                    font.weight: Font.Medium
                    elide: Text.ElideRight
                }
                MaterialSymbol {
                    anchors.verticalCenter: parent.verticalCenter
                    text: "expand_more"
                    iconSize: 20
                    color: Appearance.colors.primary
                }
            }
        }

        StyledPopover {
            id: menu

            x: choice.width - width
            y: choice.height
            width: Math.min(420, choice.width)
            padding: 6

            contentItem: ListView {
                implicitHeight: Math.min(contentHeight, 300)
                clip: true
                boundsBehavior: Flickable.StopAtBounds
                model: choice.options

                T.ScrollBar.vertical: StyledScrollBar {}

                delegate: RippleButton {
                    required property var modelData

                    width: ListView.view.width
                    height: 40
                    buttonRadius: Appearance.rounding.small
                    bounce: false
                    onClicked: {
                        choice.chosen(modelData.id)
                        menu.close()
                    }

                    contentItem: Row {
                        spacing: 8
                        leftPadding: 8

                        MaterialSymbol {
                            anchors.verticalCenter: parent.verticalCenter
                            text: "check"
                            iconSize: 18
                            color: Appearance.colors.primary
                            opacity: modelData.id === choice.current ? 1 : 0
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

    StyledText {
        width: parent.width
        visible: !root.supported
        text: root.monitor.connected
              ? qsTr("The daemon on this machine is too old for sensor settings. Update it under Updates.")
              : qsTr("Not connected.")
        color: Appearance.colors.error
        wrapMode: Text.Wrap
    }

    LoadingSpinner {
        anchors.horizontalCenter: parent.horizontalCenter
        visible: root.supported && !root.loaded
        running: visible
    }

    StyledText {
        width: parent.width
        visible: root.error.length > 0
        text: qsTr("Not saved: %1").arg(root.error)
        color: Appearance.colors.error
        wrapMode: Text.Wrap
    }

    Choice {
        visible: root.loaded
        label: qsTr("CPU Temperature")
        note: qsTr("what widgets showing the CPU temperature use on this machine")
        options: [{ id: "", name: qsTr("Automatic") }].concat(root.temperatures.map(s => ({ id: s.id, name: root.sensorName(s.id) })))
        current: root.draft.cpu_temperature ?? ""
        onChosen: id => root.setChoice("cpu_temperature", id)
    }

    Choice {
        visible: root.loaded && root.powers.length > 0
        label: qsTr("CPU Power")
        note: qsTr("for boards that report the CPU's power as a sensor of their own")
        options: [{ id: "", name: qsTr("Automatic") }].concat(root.powers.map(s => ({ id: s.id, name: root.sensorName(s.id) })))
        current: root.draft.cpu_power ?? ""
        onChosen: id => root.setChoice("cpu_power", id)
    }

    SectionLabel {
        width: parent.width
        visible: root.loaded
        inset: 4
        text: qsTr("Every Sensor")
    }

    StyledText {
        width: parent.width
        visible: root.loaded
        text: qsTr("a name of your own, and value × scale + offset: motherboards measure most voltages through dividers, so e.g. a 12 V rail may read 2 V until scaled")
        color: Appearance.colors.onSurfaceVariant
        font.pixelSize: Appearance.fontSize.smaller
        wrapMode: Text.Wrap
    }

    ListView {
        id: list

        width: parent.width
        height: Math.min(contentHeight, Math.max(160, root.maximumHeight - 520))
        visible: root.loaded
        clip: true
        spacing: 2
        boundsBehavior: Flickable.StopAtBounds
        model: root.sensors
        section.property: "device"
        section.delegate: SectionLabel {
            required property string section
            width: list.width - 10
            inset: 4
            text: section
        }

        T.ScrollBar.vertical: StyledScrollBar {}

        delegate: Item {
            id: row

            required property var modelData
            readonly property var setting: root.overrideOf(modelData.id)
            readonly property real shown: modelData.value * setting.multiplier + setting.offset

            width: list.width - 10
            height: 52
            opacity: setting.hidden ? 0.55 : 1

            Column {
                anchors.left: parent.left
                anchors.leftMargin: 4
                anchors.right: nameField.left
                anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                spacing: 1

                StyledText {
                    width: parent.width
                    text: row.modelData.label
                    font.pixelSize: Appearance.fontSize.smallie
                    elide: Text.ElideRight
                }
                StyledText {
                    width: parent.width
                    text: row.modelData.id
                    color: Appearance.colors.onSurfaceVariant
                    font.pixelSize: Appearance.fontSize.smaller
                    elide: Text.ElideRight
                }
            }

            StyledTextField {
                id: nameField
                anchors.right: scaleField.left
                anchors.rightMargin: 6
                anchors.verticalCenter: parent.verticalCenter
                width: 180
                height: 44
                topPadding: 12
                bottomPadding: 12
                placeholderText: qsTr("Name")
                // Set once: a binding would fight the typing.
                Component.onCompleted: text = row.setting.label ?? ""
                onTextEdited: root.setOverride(row.modelData.id, "label", text.trim() || null)
            }
            StyledTextField {
                id: scaleField
                anchors.right: offsetField.left
                anchors.rightMargin: 6
                anchors.verticalCenter: parent.verticalCenter
                width: 76
                height: 44
                topPadding: 12
                bottomPadding: 12
                placeholderText: "× 1"
                inputMethodHints: Qt.ImhFormattedNumbersOnly
                validator: DoubleValidator { notation: DoubleValidator.StandardNotation; locale: "C" }
                Component.onCompleted: text = row.setting.multiplier === 1 ? "" : String(row.setting.multiplier)
                onTextEdited: {
                    const value = parseFloat(text)
                    root.setOverride(row.modelData.id, "multiplier", isFinite(value) ? value : 1)
                }
            }
            StyledTextField {
                id: offsetField
                anchors.right: valueText.left
                anchors.rightMargin: 8
                anchors.verticalCenter: parent.verticalCenter
                width: 76
                height: 44
                topPadding: 12
                bottomPadding: 12
                placeholderText: "+ 0"
                inputMethodHints: Qt.ImhFormattedNumbersOnly
                validator: DoubleValidator { notation: DoubleValidator.StandardNotation; locale: "C" }
                Component.onCompleted: text = row.setting.offset === 0 ? "" : String(row.setting.offset)
                onTextEdited: {
                    const value = parseFloat(text)
                    root.setOverride(row.modelData.id, "offset", isFinite(value) ? value : 0)
                }
            }
            StyledText {
                id: valueText
                anchors.right: hide.left
                anchors.rightMargin: 4
                anchors.verticalCenter: parent.verticalCenter
                width: 84
                horizontalAlignment: Text.AlignRight
                text: Format.metric(row.shown, root.unitOf(row.modelData.kind))
                font.family: Appearance.monoFontFamily
                font.pixelSize: Appearance.fontSize.smaller
            }
            IconButton {
                id: hide
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                size: 36
                iconSize: 20
                symbol: row.setting.hidden ? "visibility_off" : "visibility"
                tooltip: row.setting.hidden ? qsTr("Hidden from the source list; click to show") : qsTr("Hide from the source list")
                onClicked: root.setOverride(row.modelData.id, "hidden", !row.setting.hidden)
            }
        }
    }

    buttons: [
        TextButton {
            variant: TextButton.Filled
            symbol: "save"
            text: qsTr("Save")
            enabled: root.loaded && !root.saving
            onClicked: {
                root.saving = true
                root.error = ""
                root.monitor.saveSensorSettings(JSON.stringify(root.draft))
            }
        },
        TextButton {
            variant: TextButton.Text
            text: qsTr("Cancel")
            onClicked: root.close()
        }
    ]
}
