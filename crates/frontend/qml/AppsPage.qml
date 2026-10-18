import QtQuick
import QtQuick.Layouts
import PluSystemMonitor

// Running applications, each a card with its processes rolled up.
Item {
    id: root

    required property Monitor monitor
    /// `prefs.apps`: sort.
    property var settings: ({})
    property string selectedId: ""

    signal settingsEdited(var settings)
    signal processRequested(int pid)

    readonly property var sortKeys: ["name", "cpu", "memory"]

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
        spacing: Theme.spacing

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            GlassField {
                id: search

                Layout.preferredWidth: 320
                leftPadding: 40
                placeholderText: "Search apps"
                Keys.onEscapePressed: text = ""

                Icon {
                    x: 13
                    anchors.verticalCenter: parent.verticalCenter
                    name: "search"
                    color: Theme.textDim
                    size: 16
                }
            }

            Segmented {
                model: ["Name", "CPU", "Memory"]
                currentIndex: Math.max(0, root.sortKeys.indexOf(apps.sortKey))
                onActivated: index => {
                    root.settings = Object.assign({}, root.settings, { sort: root.sortKeys[index] })
                    root.settingsEdited(root.settings)
                }
            }

            Item {
                Layout.fillWidth: true
            }

            Text {
                text: apps.count + (apps.count === 1 ? " app" : " apps") + " · "
                      + apps.backgroundCount + " background processes"
                color: Theme.textDim
                font.pixelSize: 13
                font.features: { "tnum": 1 }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: Theme.spacing

            Item {
                Layout.fillWidth: true
                Layout.fillHeight: true

                GridView {
                    id: grid

                    readonly property int columns: Math.max(1, Math.floor(width / 290))

                    // Bleed half a gap out so card edges line up with the toolbar.
                    anchors.fill: parent
                    anchors.margins: -Theme.spacing / 2
                    cellWidth: width / columns
                    cellHeight: 210
                    clip: true
                    model: apps
                    boundsBehavior: Flickable.StopAtBounds

                    delegate: AppCard {
                        width: grid.cellWidth
                        height: grid.cellHeight
                        interval: root.monitor.intervalMs
                        selected: appId === root.selectedId
                        onClicked: root.selectedId = selected ? "" : appId
                    }
                }

                Text {
                    anchors.centerIn: parent
                    visible: apps.count === 0
                    text: search.text ? "No app matches “" + search.text + "”" : "Waiting for data…"
                    color: Theme.textDim
                    font.pixelSize: 14
                }
            }

            // Same reveal-by-wrapper trick as on the Processes page.
            Item {
                readonly property bool shown: root.selectedId !== ""

                Layout.fillHeight: true
                Layout.preferredWidth: shown ? details.width : 0
                visible: Layout.preferredWidth > 1
                clip: true

                Behavior on Layout.preferredWidth {
                    NumberAnimation { duration: 260; easing.type: Easing.OutCubic }
                }

                AppDetails {
                    id: details

                    width: 340
                    height: parent.height
                    opacity: parent.shown ? 1 : 0
                    monitor: root.monitor
                    appId: root.selectedId
                    onCloseRequested: root.selectedId = ""
                    onProcessRequested: pid => root.processRequested(pid)

                    Behavior on opacity {
                        NumberAnimation { duration: 200 }
                    }
                }
            }
        }
    }
}
