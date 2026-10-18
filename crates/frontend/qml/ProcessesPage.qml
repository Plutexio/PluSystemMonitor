import QtQuick
import QtQuick.Layouts
import PluSystemMonitor

// Every process on the connected machine: search, sort, inspect and signal.
Item {
    id: root

    required property Monitor monitor
    /// `prefs.processes`: columns, sort, descending, only_mine, show_kernel_threads.
    property var settings: ({})
    property int selectedPid: -1

    signal settingsEdited(var settings)

    function update(patch) {
        settings = Object.assign({}, settings, patch)
        settingsEdited(settings)
    }

    ProcessModel {
        id: processes

        monitor: root.monitor
        sortKey: root.settings.sort ?? "cpu"
        descending: root.settings.descending ?? true
        onlyMine: root.settings.only_mine ?? false
        showKernelThreads: root.settings.show_kernel_threads ?? false
        filter: search.text

        // Clicking a header changes these directly; remember the choice.
        onSortKeyChanged: if (sortKey !== root.settings.sort) root.update({ sort: sortKey })
        onDescendingChanged: if (descending !== root.settings.descending) root.update({ descending: descending })
    }

    Connections {
        target: root.monitor
        function onProcessesUpdated() { processes.refresh() }
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
                placeholderText: "Search name, command, user or PID"
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
                model: ["All", "Mine"]
                currentIndex: processes.onlyMine ? 1 : 0
                onActivated: index => root.update({ only_mine: index === 1 })
            }

            Item {
                Layout.fillWidth: true
            }

            Text {
                text: processes.count === root.monitor.processCount
                      ? processes.count + " processes"
                      : processes.count + " of " + root.monitor.processCount + " processes"
                color: Theme.textDim
                font.pixelSize: 13
                font.features: { "tnum": 1 }
            }

            GlassButton {
                iconName: "columns"
                tip: "Columns and filters"
                onClicked: viewPopup.open()

                ProcessViewPopup {
                    id: viewPopup
                    y: parent.height + 10
                    x: parent.width - width
                    allColumns: list.allColumns
                    columnDefs: list.columnDefs
                    columns: root.settings.columns ?? []
                    showKernelThreads: processes.showKernelThreads
                    onColumnsEdited: columns => root.update({ columns: columns })
                    onKernelThreadsToggled: root.update({ show_kernel_threads: !processes.showKernelThreads })
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: Theme.spacing

            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                radius: Theme.radius
                color: Theme.card
                border.color: Theme.cardBorder

                ProcessList {
                    id: list

                    anchors.fill: parent
                    anchors.margins: 8
                    processes: processes
                    columns: (root.settings.columns ?? []).filter(key => list.columnDefs[key] !== undefined)
                    selectedPid: root.selectedPid
                    onSelected: pid => root.selectedPid = pid
                    onSignalRequested: (pid, signal) => root.monitor.sendSignal(pid, signal)
                }
            }

            // The panel keeps its full width and this wrapper reveals it; shrinking
            // the panel itself would squeeze its layouts to negative sizes, which
            // makes Qt Quick Layouts re-polish forever.
            Item {
                readonly property bool shown: root.selectedPid >= 0

                Layout.fillHeight: true
                Layout.preferredWidth: shown ? details.width : 0
                visible: Layout.preferredWidth > 1
                clip: true

                Behavior on Layout.preferredWidth {
                    NumberAnimation { duration: 260; easing.type: Easing.OutCubic }
                }

                ProcessDetails {
                    id: details

                    width: 340
                    height: parent.height
                    opacity: parent.shown ? 1 : 0
                    monitor: root.monitor
                    pid: root.selectedPid
                    onCloseRequested: root.selectedPid = -1
                    onPidRequested: pid => root.selectedPid = pid

                    Behavior on opacity {
                        NumberAnimation { duration: 200 }
                    }
                }
            }
        }
    }
}
