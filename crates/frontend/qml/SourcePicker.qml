import QtQuick
import QtQuick.Controls.Basic
import PluSystemMonitor

// Searchable list of the data sources the connected machine offers.
GlassPopup {
    id: root

    required property Monitor monitor
    /// IDs already chosen; shown but not pickable again.
    property var taken: []

    signal picked(string id)

    readonly property var matches: {
        const query = search.text.trim().toLowerCase()
        return Sources.catalog(root.monitor).filter(source =>
            !query || source.name.toLowerCase().includes(query) || source.category.toLowerCase().includes(query))
    }

    title: "Add a source"
    width: 360
    height: 460
    onAboutToShow: {
        search.text = ""
        search.forceActiveFocus()
    }

    contentItem: Column {
        spacing: 12

        GlassField {
            id: search
            width: parent.width
            leftPadding: 40
            placeholderText: "Search sources"

            Icon {
                x: 13
                anchors.verticalCenter: parent.verticalCenter
                name: "search"
                color: Theme.textDim
                size: 16
            }
        }

        ListView {
            id: list

            width: parent.width
            height: parent.height - search.height - parent.spacing
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            model: root.matches
            section.property: "category"
            section.delegate: Text {
                required property string section
                width: list.width
                topPadding: 10
                bottomPadding: 4
                text: section.toUpperCase()
                color: Theme.textDim
                font.pixelSize: 10
                font.weight: Font.DemiBold
                font.letterSpacing: 1.2
            }

            ScrollBar.vertical: ScrollBar {
                width: 8
                contentItem: Rectangle {
                    // A custom handle skips the style's "only when needed" logic.
                    visible: parent.size < 1.0
                    implicitWidth: 4
                    radius: 2
                    color: Theme.textDim
                    opacity: 0.4
                }
            }

            delegate: Rectangle {
                id: row

                required property var modelData
                readonly property bool taken: root.taken.includes(modelData.id)

                width: list.width - 10
                height: 34
                radius: 9
                color: hover.hovered && !taken ? Qt.alpha(Theme.accent(modelData.accent), 0.14) : "transparent"

                Rectangle {
                    x: 10
                    anchors.verticalCenter: parent.verticalCenter
                    width: 8
                    height: 8
                    radius: 4
                    color: Theme.accent(row.modelData.accent)
                }
                Text {
                    x: 28
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width - 40
                    text: row.modelData.name
                    color: row.taken ? Theme.textDim : Theme.text
                    font.pixelSize: 13
                    elide: Text.ElideRight
                }
                Icon {
                    anchors.right: parent.right
                    anchors.rightMargin: 10
                    anchors.verticalCenter: parent.verticalCenter
                    name: row.taken ? "check" : "plus"
                    color: row.taken ? Theme.textDim : Theme.accent(row.modelData.accent)
                    size: 16
                    opacity: row.taken || hover.hovered ? 1 : 0
                }

                HoverHandler {
                    id: hover
                    cursorShape: row.taken ? Qt.ArrowCursor : Qt.PointingHandCursor
                }
                TapHandler {
                    enabled: !row.taken
                    onTapped: {
                        root.picked(row.modelData.id)
                        root.close()
                    }
                }
            }
        }
    }
}
