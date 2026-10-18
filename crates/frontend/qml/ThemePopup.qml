import QtQuick
import QtQuick.Controls.Basic
import PluSystemMonitor

// Theme list with a miniature preview of each one.
GlassPopup {
    id: root

    property var themes: []

    signal themeSelected(string id)

    title: "Theme"
    width: 340
    onAboutToShow: themes = JSON.parse(Config.themes())

    contentItem: Column {
        spacing: 6

        Repeater {
            model: root.themes

            delegate: Rectangle {
                id: row

                required property var modelData
                readonly property bool selected: modelData.id === Theme.current
                readonly property bool broken: modelData.error !== undefined

                width: parent.width
                height: 60
                radius: 14
                color: selected ? Qt.alpha(Theme.primary, 0.14) : hover.hovered && !broken ? Theme.card : "transparent"
                border.color: selected ? Qt.alpha(Theme.primary, 0.6) : "transparent"
                border.width: 1

                Behavior on color {
                    ColorAnimation { duration: 140 }
                }

                HoverHandler {
                    id: hover
                    cursorShape: row.broken ? Qt.ArrowCursor : Qt.PointingHandCursor
                }
                TapHandler {
                    enabled: !row.broken
                    onTapped: root.themeSelected(row.modelData.id)
                }

                // Miniature: background, a card and the three main accents.
                Rectangle {
                    id: swatch

                    x: 10
                    anchors.verticalCenter: parent.verticalCenter
                    width: 54
                    height: 40
                    radius: 9
                    color: row.broken ? Theme.track : row.modelData.background
                    border.color: Theme.cardBorder

                    Rectangle {
                        anchors.fill: parent
                        anchors.margins: 6
                        radius: 5
                        visible: !row.broken
                        color: row.modelData.card ?? "transparent"

                        Row {
                            anchors.centerIn: parent
                            spacing: 4

                            Repeater {
                                model: row.modelData.accents ?? []

                                Rectangle {
                                    required property string modelData
                                    width: 8
                                    height: 8
                                    radius: 4
                                    color: modelData
                                }
                            }
                        }
                    }

                    Icon {
                        anchors.centerIn: parent
                        visible: row.broken
                        name: "alert"
                        color: Theme.bad
                    }
                }

                Column {
                    anchors.left: swatch.right
                    anchors.leftMargin: 14
                    anchors.right: check.left
                    anchors.rightMargin: 8
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 3

                    Text {
                        width: parent.width
                        text: row.modelData.name
                        color: Theme.text
                        font.pixelSize: 14
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                    }
                    Text {
                        width: parent.width
                        text: row.broken ? row.modelData.error : row.modelData.custom ? "Custom" : "Built-in"
                        color: row.broken ? Theme.bad : Theme.textDim
                        font.pixelSize: 12
                        elide: Text.ElideRight

                        HoverHandler { id: errorHover }
                        ToolTip.visible: row.broken && errorHover.hovered
                        ToolTip.text: row.modelData.error ?? ""
                    }
                }

                Icon {
                    id: check
                    anchors.right: parent.right
                    anchors.rightMargin: 14
                    anchors.verticalCenter: parent.verticalCenter
                    name: "check"
                    color: Theme.primary
                    opacity: row.selected ? 1 : 0

                    Behavior on opacity {
                        NumberAnimation { duration: 160 }
                    }
                }
            }
        }

        Item {
            width: 1
            height: 8
        }

        GlassButton {
            width: parent.width
            iconName: "folder"
            text: "Open themes folder"
            onClicked: Qt.openUrlExternally(Config.themesFolder())
        }

        Text {
            width: parent.width
            text: "Drop a .toml file there to add a theme; it only needs the values it changes. "
                + "Edits to the active theme apply instantly."
            color: Theme.textDim
            font.pixelSize: 12
            wrapMode: Text.Wrap
            lineHeight: 1.2
        }
    }
}
