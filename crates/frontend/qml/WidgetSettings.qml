import QtQuick
import PluSystemMonitor

// Settings form generated from the widget's option descriptors (see Widgets.qml).
GlassPopup {
    id: root

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

    title: descriptor ? descriptor.name : ""
    width: 320

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

                    Text {
                        anchors.verticalCenter: parent.verticalCenter
                        text: option.modelData.label
                        color: Theme.text
                        font.pixelSize: 14
                    }

                    Toggle {
                        anchors.right: parent.right
                        anchors.verticalCenter: parent.verticalCenter
                        checked: option.value === true
                        onToggled: root.set(option.modelData.key, !option.value)
                    }
                }

                // accent: label above a row of colour dots.
                Text {
                    visible: option.modelData.type === "accent"
                    text: option.modelData.label
                    color: Theme.text
                    font.pixelSize: 14
                }
                Row {
                    visible: option.modelData.type === "accent"
                    spacing: 10

                    Repeater {
                        model: Theme.accentKeys

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
                                color: Theme.accent(dot.modelData)

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
            }
        }
    }
}
