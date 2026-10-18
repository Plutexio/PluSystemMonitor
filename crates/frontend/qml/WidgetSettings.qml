import QtQuick
import PluSystemMonitor

// Settings form generated from the widget's option descriptors (see Widgets.qml).
GlassPopup {
    id: root

    required property Monitor monitor
    property int index: -1
    property var descriptor: null
    property var values: ({})

    signal valuesEdited(var values)

    function edit(index, descriptor, values) {
        root.index = index
        root.descriptor = descriptor
        root.values = values
        open()
    }

    function set(key, value) {
        const next = Object.assign({}, values)
        next[key] = value
        values = next
        valuesEdited(next)
    }

    function sourceName(id) {
        const source = Sources.catalog(monitor).find(s => s.id === id)
        return source ? source.name : id + " (not available)"
    }

    title: descriptor ? descriptor.name : ""
    width: 340

    component Label: Text {
        color: Theme.text
        font.pixelSize: 14
    }

    contentItem: Column {
        spacing: 18

        Repeater {
            model: root.descriptor ? root.descriptor.options : []

            delegate: Column {
                id: option

                required property var modelData
                readonly property var value: root.values[modelData.key]

                width: parent.width
                spacing: 10

                // bool: label with a toggle on the right.
                Item {
                    visible: option.modelData.type === "bool"
                    width: parent.width
                    height: 26

                    Label {
                        anchors.verticalCenter: parent.verticalCenter
                        text: option.modelData.label
                    }
                    Toggle {
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        checked: option.value === true
                        onToggled: root.set(option.modelData.key, !option.value)
                    }
                }

                Label {
                    visible: option.modelData.type !== "bool"
                    text: option.modelData.label
                }

                // text: a free-form field; empty means the default.
                GlassField {
                    visible: option.modelData.type === "text"
                    width: parent.width
                    text: option.modelData.type === "text" ? (option.value ?? "") : ""
                    placeholderText: option.modelData.placeholder ?? ""
                    onTextEdited: root.set(option.modelData.key, text)
                }

                // accent: "auto" plus a row of colour dots.
                Row {
                    visible: option.modelData.type === "accent"
                    spacing: 10

                    Repeater {
                        // Only widgets showing data sources can follow them.
                        model: option.modelData.default === "auto" ? ["auto"].concat(Theme.accentKeys) : Theme.accentKeys

                        delegate: Rectangle {
                            id: dot

                            required property string modelData
                            readonly property bool chosen: option.value === modelData

                            width: 28
                            height: 28
                            radius: 14
                            color: "transparent"
                            border.color: chosen ? Theme.text : "transparent"
                            border.width: 2

                            Rectangle {
                                anchors.centerIn: parent
                                width: dot.chosen ? 18 : 22
                                height: width
                                radius: width / 2
                                color: dot.modelData === "auto" ? "transparent" : Theme.accent(dot.modelData)
                                border.color: Theme.textDim
                                border.width: dot.modelData === "auto" ? 2 : 0

                                Text {
                                    anchors.centerIn: parent
                                    visible: dot.modelData === "auto"
                                    text: "A"
                                    color: Theme.textDim
                                    font.pixelSize: 11
                                    font.weight: Font.Bold
                                }

                                Behavior on width {
                                    NumberAnimation { duration: 140 }
                                }
                            }

                            TapHandler {
                                onTapped: root.set(option.modelData.key, dot.modelData)
                            }
                            HoverHandler {
                                cursorShape: Qt.PointingHandCursor
                            }
                        }
                    }
                }

                // sources: the chosen IDs in order, reorderable and removable.
                Column {
                    id: sourceList

                    readonly property var list: option.modelData.type === "sources" ? (option.value ?? []) : []
                    readonly property int min: option.modelData.min ?? 0
                    readonly property int max: option.modelData.max ?? 8

                    visible: option.modelData.type === "sources"
                    width: parent.width
                    spacing: 6

                    Repeater {
                        model: sourceList.list

                        delegate: Rectangle {
                            id: sourceRow

                            required property int index
                            required property string modelData

                            width: sourceList.width
                            height: 38
                            radius: 10
                            color: Theme.track

                            Text {
                                x: 12
                                anchors.verticalCenter: parent.verticalCenter
                                width: parent.width - buttons.width - 24
                                text: root.sourceName(sourceRow.modelData)
                                color: Theme.text
                                font.pixelSize: 13
                                elide: Text.ElideRight
                            }

                            Row {
                                id: buttons

                                anchors.right: parent.right
                                anchors.rightMargin: 4
                                anchors.verticalCenter: parent.verticalCenter
                                spacing: 2

                                GlassButton {
                                    implicitHeight: 30
                                    visible: sourceRow.index > 0
                                    iconName: "chevronUp"
                                    tip: "Move up"
                                    onClicked: {
                                        const next = sourceList.list.slice()
                                        next.splice(sourceRow.index - 1, 0, next.splice(sourceRow.index, 1)[0])
                                        root.set(option.modelData.key, next)
                                    }
                                }
                                GlassButton {
                                    implicitHeight: 30
                                    visible: sourceList.list.length > sourceList.min
                                    danger: true
                                    iconName: "close"
                                    tip: "Remove"
                                    onClicked: root.set(option.modelData.key,
                                        sourceList.list.filter((_, i) => i !== sourceRow.index))
                                }
                            }
                        }
                    }

                    GlassButton {
                        // A single-source widget swaps its source instead of adding one.
                        readonly property bool swap: sourceList.max === 1 && sourceList.list.length === 1

                        visible: swap || sourceList.list.length < sourceList.max
                        iconName: swap ? "reset" : "plus"
                        text: swap ? "Change source" : "Add source"
                        onClicked: picker.open()

                        SourcePicker {
                            id: picker
                            monitor: root.monitor
                            taken: sourceList.list
                            onPicked: id => root.set(option.modelData.key,
                                parent.swap ? [id] : sourceList.list.concat([id]))
                        }
                    }

                    Text {
                        visible: (option.modelData.hint ?? "").length > 0
                        width: parent.width
                        text: option.modelData.hint ?? ""
                        color: Theme.textDim
                        font.pixelSize: 12
                        wrapMode: Text.Wrap
                    }
                }
            }
        }
    }
}
