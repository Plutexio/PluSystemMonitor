import QtQuick
import QtQuick.Templates as T
import PluSystemMonitor

// Settings generated from the widget's option descriptors (see Widgets.qml).
// Every change applies at once.
StyledDialog {
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

    function source(id) {
        return Sources.catalog(monitor).find(s => s.id === id) ?? null
    }

    headline: descriptor ? descriptor.name : ""
    symbol: "tune"
    iconShape: "cookie7"
    preferredWidth: 440

    Flickable {
        width: parent.width
        height: Math.min(form.implicitHeight, root.maximumHeight - 180)
        contentHeight: form.implicitHeight
        boundsBehavior: Flickable.StopAtBounds
        clip: true

        T.ScrollBar.vertical: StyledScrollBar {}

        Column {
            id: form

            width: parent.width
            spacing: 14

            Repeater {
                model: root.descriptor ? root.descriptor.options : []

                delegate: Column {
                    id: option

                    required property var modelData
                    readonly property var value: root.values[modelData.key]

                    width: form.width
                    spacing: 8

                    // bool: the whole row flips the switch.
                    RippleButton {
                        visible: option.modelData.type === "bool"
                        width: parent.width
                        height: 48
                        buttonRadius: Appearance.rounding.small
                        colBackgroundHover: Appearance.colors.layer3Hover
                        colRipple: Appearance.colors.layer3Active
                        bounce: false
                        onClicked: root.set(option.modelData.key, !option.value)

                        contentItem: Item {
                            StyledText {
                                x: 4
                                anchors.verticalCenter: parent.verticalCenter
                                text: option.modelData.label
                            }
                            StyledSwitch {
                                anchors.right: parent.right
                                anchors.verticalCenter: parent.verticalCenter
                                checked: option.value === true
                                onClicked: root.set(option.modelData.key, !option.value)
                            }
                        }
                    }

                    SectionLabel {
                        visible: option.modelData.type === "accent" || option.modelData.type === "sources"
                        width: parent.width
                        inset: 0
                        text: option.modelData.label
                    }

                    // text: a free-form field; empty means the default.
                    StyledTextField {
                        visible: option.modelData.type === "text"
                        width: parent.width
                        label: option.modelData.label
                        text: option.modelData.type === "text" ? (option.value ?? "") : ""
                        placeholderText: option.modelData.placeholder ?? ""
                        onTextEdited: root.set(option.modelData.key, text)
                    }

                    // accent: "auto" and the data colours, the chosen one a
                    // cookie with a tick.
                    Flow {
                        visible: option.modelData.type === "accent"
                        width: parent.width
                        spacing: 8

                        Repeater {
                            // Only widgets showing data sources can follow them.
                            model: option.modelData.default === "auto" ? ["auto"].concat(Sources.accentKeys) : Sources.accentKeys

                            delegate: RippleButton {
                                id: dot

                                required property string modelData
                                readonly property bool chosen: option.value === modelData
                                readonly property bool automatic: modelData === "auto"

                                width: 44
                                height: 44
                                padding: 0
                                colBackgroundHover: "transparent"
                                rippleEnabled: false
                                tooltip: automatic ? "Follow the Source" : modelData.charAt(0).toUpperCase() + modelData.slice(1)
                                onClicked: root.set(option.modelData.key, modelData)

                                contentItem: ShapeIcon {
                                    size: 44
                                    iconSize: 20
                                    shape: dot.chosen ? "cookie9" : "circle"
                                    keepsShape: true
                                    symbol: dot.automatic ? "auto_awesome" : (dot.chosen ? "check" : "")
                                    color: dot.automatic ? Appearance.colors.secondaryContainer : Appearance.colors.accent(dot.modelData)
                                    iconColor: dot.automatic ? Appearance.colors.onSecondaryContainer : Appearance.colors.surfaceContainerHigh
                                    hovered: dot.hovered
                                    scale: dot.chosen ? 1 : 0.8

                                    Behavior on scale {
                                        enabled: !Appearance.performanceMode
                                        Bounce {}
                                    }
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
                        spacing: 4

                        Repeater {
                            model: sourceList.list

                            delegate: Rectangle {
                                id: sourceRow

                                required property int index
                                required property string modelData
                                readonly property var source: root.source(modelData)

                                width: sourceList.width
                                height: 48
                                radius: Appearance.rounding.small
                                color: Appearance.colors.layer4

                                ShapeIcon {
                                    id: badge
                                    x: 8
                                    anchors.verticalCenter: parent.verticalCenter
                                    size: 30
                                    iconSize: 16
                                    shape: Sources.look(sourceRow.source ?? { id: sourceRow.modelData }).shape
                                    symbol: Sources.look(sourceRow.source ?? { id: sourceRow.modelData }).symbol
                                    color: Appearance.colors.tonal(Appearance.colors.accent(sourceRow.source ? sourceRow.source.accent : "cpu"))
                                    iconColor: Appearance.colors.accent(sourceRow.source ? sourceRow.source.accent : "cpu")
                                }

                                StyledText {
                                    anchors.left: badge.right
                                    anchors.leftMargin: 10
                                    anchors.right: buttons.left
                                    anchors.rightMargin: 8
                                    anchors.verticalCenter: parent.verticalCenter
                                    text: sourceRow.source ? sourceRow.source.name : sourceRow.modelData + " (not available)"
                                    color: sourceRow.source ? Appearance.colors.onSurface : Appearance.colors.onSurfaceVariant
                                    font.pixelSize: Appearance.fontSize.smallie
                                    elide: Text.ElideRight
                                }

                                Row {
                                    id: buttons

                                    anchors.right: parent.right
                                    anchors.rightMargin: 6
                                    anchors.verticalCenter: parent.verticalCenter
                                    spacing: 2

                                    IconButton {
                                        visible: sourceRow.index > 0
                                        size: 34
                                        iconSize: 20
                                        symbol: "arrow_upward"
                                        tooltip: "Move Up"
                                        onClicked: {
                                            const next = sourceList.list.slice()
                                            next.splice(sourceRow.index - 1, 0, next.splice(sourceRow.index, 1)[0])
                                            root.set(option.modelData.key, next)
                                        }
                                    }
                                    IconButton {
                                        visible: sourceList.list.length > sourceList.min
                                        size: 34
                                        iconSize: 20
                                        symbol: "close"
                                        iconColor: Appearance.colors.error
                                        tooltip: "Remove"
                                        onClicked: root.set(option.modelData.key,
                                            sourceList.list.filter((_, i) => i !== sourceRow.index))
                                    }
                                }
                            }
                        }

                        Item {
                            width: 1
                            height: 4
                        }

                        TextButton {
                            // A single-source widget swaps its source instead of adding one.
                            readonly property bool swap: sourceList.max === 1 && sourceList.list.length === 1

                            visible: swap || sourceList.list.length < sourceList.max
                            variant: TextButton.Tonal
                            symbol: swap ? "swap_horiz" : "add"
                            text: swap ? "Change Source" : "Add Source"
                            onClicked: picker.open()

                            SourcePicker {
                                id: picker
                                monitor: root.monitor
                                taken: sourceList.list
                                onPicked: id => {
                                    // A widget about one app is best titled after it.
                                    const picked = Sources.catalog(root.monitor).find(s => s.id === id)
                                    if (picked && picked.app && "title" in root.values && !root.values.title)
                                        root.set("title", picked.app)
                                    root.set(option.modelData.key, parent.swap ? [id] : sourceList.list.concat([id]))
                                }
                            }
                        }

                        StyledText {
                            visible: (option.modelData.hint ?? "").length > 0
                            width: parent.width
                            topPadding: 4
                            text: (option.modelData.hint ?? "").toLowerCase().replace(/\.$/, "")
                            color: Appearance.colors.onSurfaceVariant
                            font.pixelSize: Appearance.fontSize.smaller
                            wrapMode: Text.Wrap
                        }
                    }
                }
            }
        }
    }

    buttons: [
        TextButton {
            variant: TextButton.Filled
            text: "Done"
            onClicked: root.close()
        }
    ]
}
