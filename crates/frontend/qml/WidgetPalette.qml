import QtQuick
import PluSystemMonitor

// The widgets that can be added, each on the shape of what it shows.
StyledPopover {
    id: root

    signal picked(string type)

    readonly property var looks: ({
        gauge: { symbol: "speed", shape: "cookie9" },
        graph: { symbol: "show_chart", shape: "clover8" },
        value: { symbol: "pin", shape: "pentagon" },
        "cpu.cores": { symbol: "memory", shape: "cookie12" }
    })

    title: qsTr("Add Widget")
    symbol: "add"
    iconShape: "clover4"
    width: 360

    contentItem: Column {
        spacing: 2

        Repeater {
            model: Widgets.types

            delegate: RippleButton {
                id: tile

                required property var modelData
                readonly property color tint: Appearance.colors.accent(modelData.options.find(o => o.type === "accent")?.default)
                readonly property var look: root.looks[modelData.type] ?? { symbol: "widgets", shape: "cookie9" }

                width: parent.width
                height: 64
                buttonRadius: Appearance.rounding.small
                colBackgroundHover: Appearance.colors.layer2Hover
                colRipple: Appearance.colors.layer2Active
                onClicked: {
                    root.picked(modelData.type)
                    root.close()
                }

                contentItem: Item {
                    ShapeIcon {
                        id: badge
                        x: 4
                        anchors.verticalCenter: parent.verticalCenter
                        size: 40
                        iconSize: 20
                        shape: tile.look.shape
                        symbol: tile.look.symbol
                        color: Appearance.colors.tonal(tile.tint)
                        iconColor: tile.tint
                        hovered: tile.hovered
                    }

                    Column {
                        anchors.left: badge.right
                        anchors.leftMargin: 14
                        anchors.right: plus.left
                        anchors.rightMargin: 8
                        anchors.verticalCenter: parent.verticalCenter
                        spacing: 2

                        StyledText {
                            text: tile.modelData.name
                            font.weight: Font.DemiBold
                        }
                        StyledText {
                            width: parent.width
                            text: tile.modelData.description
                            color: Appearance.colors.onSurfaceVariant
                            font.pixelSize: Appearance.fontSize.smaller
                            elide: Text.ElideRight
                        }
                    }

                    MaterialSymbol {
                        id: plus
                        anchors.right: parent.right
                        anchors.rightMargin: 4
                        anchors.verticalCenter: parent.verticalCenter
                        text: "add"
                        iconSize: 22
                        weight: tile.hovered ? 600 : 400
                        color: tile.hovered ? tile.tint : Appearance.colors.onSurfaceVariant
                    }
                }
            }
        }
    }
}
