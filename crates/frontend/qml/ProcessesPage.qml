import QtQuick
import QtQuick.Layouts
import PluSystemMonitor

// Every process on the connected machine: search, sort, pick one for the
// inspector, and signal it.
Item {
    id: root

    required property Monitor monitor
    /// `prefs.processes`: columns, sort, descending, only_mine, show_kernel_threads.
    property var settings: ({})
    property int selectedPid: -1
    readonly property alias columnDefs: list.columnDefs
    readonly property alias allColumns: list.allColumns

    signal settingsEdited(var settings)

    function update(patch) {
        settingsEdited(Object.assign({}, settings, patch))
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
        anchors.leftMargin: Appearance.sizes.pageMargin - 8
        anchors.rightMargin: Appearance.sizes.pageMargin - 8
        anchors.bottomMargin: 8
        spacing: 12

        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 8
            Layout.rightMargin: 8
            spacing: 12

            SearchField {
                id: search
                Layout.preferredWidth: 320
                placeholder: qsTr("Search name, command, user or PID")
            }

            SegmentedControl {
                currentIndex: processes.onlyMine ? 1 : 0
                segments: [
                    Segment { text: qsTr("All"); symbol: "group" },
                    Segment { text: qsTr("Mine"); symbol: "person" }
                ]
                onActivated: index => root.update({ only_mine: index === 1 })
            }

            Item {
                Layout.fillWidth: true
            }

            StyledText {
                text: processes.count === root.monitor.processCount
                      ? processes.count + " processes"
                      : processes.count + " of " + root.monitor.processCount + " processes"
                color: Appearance.colors.onSurfaceVariant
                font.pixelSize: Appearance.fontSize.smallie
                tabularNumbers: true
            }
        }

        ProcessList {
            id: list

            Layout.fillWidth: true
            Layout.fillHeight: true
            processes: processes
            columns: (root.settings.columns ?? []).filter(key => list.columnDefs[key] !== undefined)
            selectedPid: root.selectedPid
            waiting: !root.monitor.connected || root.monitor.processCount === 0
            onSelected: pid => root.selectedPid = pid
            onSignalRequested: (pid, signal) => root.monitor.sendSignal(pid, signal)
        }
    }
}
