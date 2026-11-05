import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.kcmutils as KCM

// Which machine, and which of its sources.
KCM.SimpleKCM {
    id: page

    property string cfg_machine
    property var cfg_sources: []
    property string cfg_title
    property string cfg_machineDefault
    property var cfg_sourcesDefault
    property string cfg_titleDefault
    // Set on every page by Plasma; the other page uses them.
    property bool cfg_showTitle
    property bool cfg_showTitleDefault
    property string cfg_style
    property string cfg_styleDefault
    property string cfg_accent
    property string cfg_accentDefault
    property bool cfg_fit
    property bool cfg_fitDefault

    readonly property int maxSources: 6

    property var machines: []
    /// The machine's sources: [{ id, name, category, unit, ... }].
    property var catalog: []
    property string status: ""

    function definition(id) {
        return catalog.find(s => s.id === id) ?? null
    }

    function loadCatalog() {
        status = "Loading…"
        const machine = cfg_machine
        daemon.call("Catalog", [machine], result => {
            if (machine !== cfg_machine)
                return
            catalog = result ? result.sources : []
            status = !result ? "" : result.connected ? "" : result.status
            // A machine only just connected to answers once it has data.
            if (result && !result.connected)
                retry.restart()
        })
    }

    function setSources(list) {
        cfg_sources = list
    }

    Daemon {
        id: daemon
        onUpChanged: if (up) page.reload()
    }

    function reload() {
        daemon.call("Machines", [], result => {
            const list = result ?? [{ address: "", name: "This computer", local: true }]
            if (!list.some(m => m.address === cfg_machine))
                list.push({ address: cfg_machine, name: cfg_machine, local: false })
            machines = list
        })
        loadCatalog()
    }

    Component.onCompleted: if (daemon.up) reload()

    Timer {
        id: retry
        interval: 1500
        onTriggered: page.loadCatalog()
    }

    Kirigami.FormLayout {
        Kirigami.InlineMessage {
            Kirigami.FormData.isSection: true
            Layout.fillWidth: true
            visible: !daemon.up
            type: Kirigami.MessageType.Warning
            text: "The PluSystemMonitor daemon isn't running, so there is nothing to pick from. "
                  + "Start it with: systemctl --user start plusysmond"
        }

        QQC2.ComboBox {
            id: machineBox
            Kirigami.FormData.label: "Machine:"
            Layout.fillWidth: true
            model: page.machines
            textRole: "name"
            valueRole: "address"
            currentIndex: page.machines.findIndex(m => m.address === page.cfg_machine)
            onActivated: {
                page.cfg_machine = currentValue
                page.loadCatalog()
            }
        }
        QQC2.Label {
            Layout.fillWidth: true
            Layout.maximumWidth: Kirigami.Units.gridUnit * 24
            visible: text.length > 0
            text: page.status
            wrapMode: Text.Wrap
            color: Kirigami.Theme.disabledTextColor
        }

        QQC2.TextField {
            Kirigami.FormData.label: "Title:"
            Layout.fillWidth: true
            text: page.cfg_title
            placeholderText: page.definition(page.cfg_sources[0] ?? "")?.name ?? "Name of the first source"
            onTextEdited: page.cfg_title = text
        }

        Item {
            Kirigami.FormData.isSection: true
            Kirigami.FormData.label: "Shown (" + page.cfg_sources.length + " of " + page.maxSources + ")"
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.minimumWidth: Kirigami.Units.gridUnit * 22
            spacing: 0

            QQC2.Label {
                visible: page.cfg_sources.length === 0
                text: "Nothing yet. Add sources below."
                color: Kirigami.Theme.disabledTextColor
            }

            Repeater {
                model: page.cfg_sources

                RowLayout {
                    id: chosen
                    required property string modelData
                    required property int index
                    readonly property var def: page.definition(modelData)
                    Layout.fillWidth: true

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 0
                        QQC2.Label {
                            Layout.fillWidth: true
                            text: chosen.def ? chosen.def.name : chosen.modelData
                            elide: Text.ElideRight
                            font.weight: chosen.index === 0 ? Font.DemiBold : Font.Normal
                        }
                        QQC2.Label {
                            Layout.fillWidth: true
                            text: (chosen.def ? chosen.def.category : "Not on this machine right now")
                                  + (chosen.index === 0 ? " · the main one" : "")
                            elide: Text.ElideRight
                            color: Kirigami.Theme.disabledTextColor
                            font: Kirigami.Theme.smallFont
                        }
                    }
                    QQC2.ToolButton {
                        icon.name: "arrow-up"
                        enabled: chosen.index > 0
                        onClicked: {
                            const list = page.cfg_sources.slice()
                            list.splice(chosen.index - 1, 0, list.splice(chosen.index, 1)[0])
                            page.setSources(list)
                        }
                        QQC2.ToolTip.text: "Move up"
                        QQC2.ToolTip.visible: hovered
                    }
                    QQC2.ToolButton {
                        icon.name: "arrow-down"
                        enabled: chosen.index < page.cfg_sources.length - 1
                        onClicked: {
                            const list = page.cfg_sources.slice()
                            list.splice(chosen.index + 1, 0, list.splice(chosen.index, 1)[0])
                            page.setSources(list)
                        }
                        QQC2.ToolTip.text: "Move down"
                        QQC2.ToolTip.visible: hovered
                    }
                    QQC2.ToolButton {
                        icon.name: "list-remove"
                        onClicked: {
                            const list = page.cfg_sources.slice()
                            list.splice(chosen.index, 1)
                            page.setSources(list)
                        }
                        QQC2.ToolTip.text: "Remove"
                        QQC2.ToolTip.visible: hovered
                    }
                }
            }
        }

        Item {
            Kirigami.FormData.isSection: true
            Kirigami.FormData.label: "Add a source"
        }

        Kirigami.SearchField {
            id: search
            Layout.fillWidth: true
            placeholderText: "Search " + page.catalog.length + " sources…"
        }

        QQC2.ScrollView {
            Layout.fillWidth: true
            Layout.preferredHeight: Kirigami.Units.gridUnit * 16
            Layout.minimumWidth: Kirigami.Units.gridUnit * 22
            Component.onCompleted: background.visible = true

            ListView {
                id: list
                clip: true
                // Grouped by category, in the order categories first come.
                model: {
                    const words = search.text.toLowerCase().split(/\s+/).filter(w => w.length > 0)
                    const found = page.catalog.filter(s => {
                        const text = (s.name + " " + s.category + " " + s.id).toLowerCase()
                        return words.every(w => text.includes(w))
                    })
                    const order = []
                    found.forEach(s => { if (!order.includes(s.category)) order.push(s.category) })
                    return order.reduce((all, c) => all.concat(found.filter(s => s.category === c)), [])
                }
                delegate: ColumnLayout {
                    id: entry
                    required property var modelData
                    required property int index
                    readonly property bool added: page.cfg_sources.includes(modelData.id)
                    width: ListView.view.width
                    spacing: 0

                    Kirigami.ListSectionHeader {
                        Layout.fillWidth: true
                        visible: entry.index === 0 || list.model[entry.index - 1].category !== entry.modelData.category
                        text: entry.modelData.category
                    }
                    QQC2.ItemDelegate {
                        Layout.fillWidth: true
                        enabled: !entry.added && page.cfg_sources.length < page.maxSources
                        text: entry.modelData.name
                        icon.name: entry.added ? "checkmark" : "list-add"
                        onClicked: page.setSources(page.cfg_sources.concat([entry.modelData.id]))
                    }
                }
            }
        }
    }
}
