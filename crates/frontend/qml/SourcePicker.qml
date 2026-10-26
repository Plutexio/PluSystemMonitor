import QtQuick
import QtQuick.Templates as T
import PluSystemMonitor

// Searchable list of the data sources the connected machine offers, grouped
// by what they belong to.
StyledDialog {
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

    headline: qsTr("Add a Source")
    symbol: "add_chart"
    iconShape: "clover8"
    preferredWidth: 420
    onAboutToShow: {
        search.text = ""
        search.focusField()
    }

    SearchField {
        id: search
        width: parent.width
        placeholder: qsTr("Search sources")
    }

    ListView {
        id: list

        width: parent.width
        height: Math.min(380, root.maximumHeight - 220)
        clip: true
        spacing: 2
        boundsBehavior: Flickable.StopAtBounds
        model: root.matches
        section.property: "category"
        section.delegate: SectionLabel {
            required property string section
            width: list.width - 10
            inset: 4
            text: section
        }

        T.ScrollBar.vertical: StyledScrollBar {}

        delegate: RippleButton {
            id: row

            required property var modelData
            readonly property bool taken: root.taken.includes(modelData.id)
            readonly property color tint: Appearance.colors.accent(modelData.accent)

            width: list.width - 10
            height: 40
            enabled: !taken
            opacity: 1
            buttonRadius: Appearance.rounding.small
            colBackgroundHover: Appearance.colors.layer3Hover
            colRipple: Appearance.colors.layer3Active
            bounce: false
            onClicked: {
                root.picked(modelData.id)
                root.close()
            }

            contentItem: Item {
                Rectangle {
                    id: marker
                    x: 6
                    anchors.verticalCenter: parent.verticalCenter
                    width: 10
                    height: 10
                    radius: 5
                    color: row.tint
                }
                StyledText {
                    anchors.left: marker.right
                    anchors.leftMargin: 12
                    anchors.right: trailing.left
                    anchors.rightMargin: 8
                    anchors.verticalCenter: parent.verticalCenter
                    text: row.modelData.name
                    color: row.taken ? Appearance.colors.onSurfaceVariant : Appearance.colors.onSurface
                    font.pixelSize: Appearance.fontSize.smallie
                    elide: Text.ElideRight
                }
                MaterialSymbol {
                    id: trailing
                    anchors.right: parent.right
                    anchors.rightMargin: 6
                    anchors.verticalCenter: parent.verticalCenter
                    text: row.taken ? "check" : "add"
                    iconSize: 20
                    color: row.taken ? Appearance.colors.onSurfaceVariant : row.tint
                    opacity: row.taken || row.hovered ? 1 : 0

                    Behavior on opacity {
                        NumberAnimation { duration: Appearance.duration(150) }
                    }
                }
            }
        }
    }

    EmptyState {
        anchors.horizontalCenter: parent.horizontalCenter
        visible: list.count === 0
        symbol: "search_off"
        title: qsTr("Nothing Matches")
        description: root.monitor.connected ? qsTr("Check the spelling or try a new search.")
                                            : qsTr("Connect to a machine to see what it offers.")
    }
}
