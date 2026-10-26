import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import PluSystemMonitor

// Running applications, each a card with its processes rolled up. The one
// picked opens in the inspector.
Item {
    id: root

    required property Monitor monitor
    /// `prefs.apps`: sort.
    property var settings: ({})
    property string selectedId: ""

    signal settingsEdited(var settings)

    readonly property var sortKeys: ["name", "cpu", "memory", "gpu"]

    AppModel {
        id: apps

        monitor: root.monitor
        sortKey: root.settings.sort ?? "name"
        filter: search.text
    }

    Connections {
        target: root.monitor
        function onProcessesUpdated() { apps.refresh() }
        function onAssetsChanged() { apps.refresh() }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.leftMargin: Appearance.sizes.pageMargin
        anchors.rightMargin: Appearance.sizes.pageMargin
        spacing: 12

        RowLayout {
            Layout.fillWidth: true
            spacing: 12

            SearchField {
                id: search
                Layout.preferredWidth: 300
                placeholder: qsTr("Search apps")
            }

            SegmentedControl {
                currentIndex: Math.max(0, root.sortKeys.indexOf(apps.sortKey))
                segments: [
                    Segment { text: qsTr("Name"); symbol: "sort_by_alpha" },
                    Segment { text: qsTr("CPU"); symbol: "memory" },
                    Segment { text: qsTr("Memory"); symbol: "memory_alt" },
                    Segment { text: qsTr("GPU"); symbol: "developer_board" }
                ]
                onActivated: index => root.settingsEdited(Object.assign({}, root.settings, { sort: root.sortKeys[index] }))
            }

            Item {
                Layout.fillWidth: true
            }

            StyledText {
                text: apps.count + (apps.count === 1 ? " app" : " apps") + " · "
                      + apps.backgroundCount + " background processes"
                color: Appearance.colors.onSurfaceVariant
                font.pixelSize: Appearance.fontSize.smallie
                tabularNumbers: true
                elide: Text.ElideRight
                Layout.maximumWidth: 280
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            GridView {
                id: grid

                readonly property int columns: Math.max(1, Math.floor(width / 290))

                // Bleed half a gap out so card edges line up with the toolbar.
                anchors.fill: parent
                anchors.leftMargin: -Appearance.sizes.shelfGap / 2
                anchors.rightMargin: -Appearance.sizes.shelfGap / 2
                cellWidth: width / columns
                cellHeight: 210
                clip: true
                model: apps
                boundsBehavior: Flickable.StopAtBounds
                bottomMargin: Appearance.sizes.shelfGap

                T.ScrollBar.vertical: StyledScrollBar {}

                delegate: AppCard {
                    width: grid.cellWidth
                    height: grid.cellHeight
                    interval: root.monitor.intervalMs
                    selected: appId === root.selectedId
                    onClicked: root.selectedId = selected ? "" : appId
                }
            }

            LoadingState {
                anchors.centerIn: parent
                visible: apps.count === 0 && !search.text && (!root.monitor.connected || root.monitor.processCount === 0)
            }

            EmptyState {
                anchors.centerIn: parent
                visible: apps.count === 0 && (search.text.length > 0 || (root.monitor.connected && root.monitor.processCount > 0))
                symbol: search.text ? "search_off" : "apps"
                title: search.text ? qsTr("Nothing Matches") : qsTr("No Apps Open")
                description: search.text ? qsTr("No app matches “%1”. Check the spelling or try a new search.").arg(search.text)
                                          : qsTr("Only background processes are running. They're on the Processes page.")
            }
        }
    }
}
