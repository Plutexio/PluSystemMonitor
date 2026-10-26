import QtQuick
import QtQuick.Templates as T
import PluSystemMonitor

// The process table straight on the sheet: a sortable header, one row per
// process with a state layer on hover and the picked one on a tonal pill,
// and keyboard navigation.
Item {
    id: root

    clip: true
    required property ProcessModel processes
    /// Visible columns besides the always-present name.
    property var columns: []
    property int selectedPid: -1
    /// Nothing to show because no data has come yet.
    property bool waiting: false

    signal selected(int pid)
    signal signalRequested(int pid, string signal)

    readonly property var columnDefs: ({
        pid: { title: "PID", width: 76, align: Text.AlignRight, sort: "pid" },
        user: { title: "User", width: 112, align: Text.AlignLeft, sort: "user" },
        cpu: { title: "CPU", width: 92, align: Text.AlignRight, sort: "cpu" },
        memory: { title: "Memory", width: 108, align: Text.AlignRight, sort: "memory" },
        gpu: { title: "GPU", width: 84, align: Text.AlignRight, sort: "gpu" },
        gpuMemory: { title: "GPU Memory", width: 116, align: Text.AlignRight, sort: "gpuMemory" },
        read: { title: "Disk Read", width: 108, align: Text.AlignRight, sort: "read" },
        write: { title: "Disk Write", width: 108, align: Text.AlignRight, sort: "write" },
        threads: { title: "Threads", width: 82, align: Text.AlignRight, sort: "threads" },
        state: { title: "State", width: 116, align: Text.AlignLeft, sort: "state" },
        nice: { title: "Nice", width: 64, align: Text.AlignRight, sort: "nice" }
    })
    readonly property var allColumns: ["pid", "user", "cpu", "memory", "gpu", "gpuMemory", "read", "write", "threads", "state", "nice"]

    readonly property real sidePadding: 16
    readonly property real minNameWidth: 200
    readonly property real fixedWidth: columns.reduce((sum, key) => sum + columnDefs[key].width, 0)
    // When space runs short (e.g. the details panel is open) the other columns
    // shrink together, down to 60 %, before anything spills over the edge.
    readonly property real columnScale: fixedWidth > 0
        ? Math.max(0.6, Math.min(1, (width - 2 * sidePadding - minNameWidth) / fixedWidth))
        : 1
    readonly property real nameWidth: Math.max(120, width - fixedWidth * columnScale - 2 * sidePadding)

    function columnWidth(key) {
        return columnDefs[key].width * columnScale
    }

    function cellText(row, key) {
        switch (key) {
        case "pid": return row.pid
        case "user": return row.user
        case "cpu": return row.cpu.toFixed(1) + "%"
        case "memory": return Format.bytes(row.memory)
        case "gpu": return row.gpu < 0 ? "" : row.gpu.toFixed(0) + "%"
        case "gpuMemory": return row.gpuMemory < 0 ? "" : Format.bytes(row.gpuMemory)
        case "read": return Format.rate(row.readRate)
        case "write": return Format.rate(row.writeRate)
        case "threads": return row.threads
        case "state": return Format.processState(row.state)
        case "nice": return row.nice
        }
        return ""
    }

    // Bars are relative to the busiest shown process, with a floor so a
    // near-idle machine doesn't draw full bars for noise.
    function barFraction(row, key) {
        if (key === "cpu")
            return row.cpu / Math.max(processes.maxCpu, 5)
        if (key === "memory")
            return row.memory / Math.max(processes.maxMemory, 64 * 1024 * 1024)
        // GPU share is already relative to the whole card.
        if (key === "gpu")
            return Math.max(0, row.gpu) / 100
        return 0
    }

    function sortBy(key) {
        if (processes.sortKey === key) {
            processes.descending = !processes.descending
        } else {
            // Numbers read best biggest-first, text alphabetically.
            processes.descending = !["name", "user", "state"].includes(key)
            processes.sortKey = key
        }
    }

    function step(delta) {
        const current = processes.rowOf(selectedPid)
        const next = Math.max(0, Math.min(list.count - 1, current < 0 ? 0 : current + delta))
        if (list.count > 0) {
            selected(processes.pidAt(next))
            list.positionViewAtIndex(next, ListView.Contain)
        }
    }

    component HeaderCell: Item {
        id: cell

        property string title
        property string sortKey
        property int align: Text.AlignLeft

        readonly property bool active: root.processes.sortKey === sortKey

        height: header.height

        Row {
            anchors.verticalCenter: parent.verticalCenter
            anchors.left: cell.align === Text.AlignLeft ? parent.left : undefined
            anchors.right: cell.align === Text.AlignRight ? parent.right : undefined
            anchors.rightMargin: 10
            layoutDirection: cell.align === Text.AlignRight ? Qt.RightToLeft : Qt.LeftToRight
            spacing: 4

            StyledText {
                width: Math.min(implicitWidth, cell.width - 28)
                text: cell.title
                elide: Text.ElideRight
                color: cell.active ? Appearance.colors.primary : Appearance.colors.onSurfaceVariant
                font.pixelSize: Appearance.fontSize.smaller
                font.weight: Font.DemiBold

                Behavior on color {
                    ElementMoveFastColor {}
                }
            }
            MaterialSymbol {
                anchors.verticalCenter: parent.verticalCenter
                visible: cell.active
                text: root.processes.descending ? "arrow_downward" : "arrow_upward"
                color: Appearance.colors.primary
                iconSize: 16
                weight: 600
            }
        }

        HoverHandler {
            cursorShape: Qt.PointingHandCursor
        }
        TapHandler {
            onTapped: root.sortBy(cell.sortKey)
        }
    }

    Row {
        id: header

        x: root.sidePadding
        height: 40

        HeaderCell {
            width: root.nameWidth
            title: "Name"
            sortKey: "name"
        }
        Repeater {
            model: root.columns

            HeaderCell {
                required property string modelData
                width: root.columnWidth(modelData)
                title: root.columnDefs[modelData].title
                sortKey: root.columnDefs[modelData].sort
                align: root.columnDefs[modelData].align
            }
        }
    }

    Rectangle {
        anchors.top: header.bottom
        x: root.sidePadding / 2
        width: parent.width - root.sidePadding
        height: 1
        color: Appearance.colors.outlineVariant
    }

    ListView {
        id: list

        anchors.top: header.bottom
        anchors.topMargin: 8
        anchors.bottom: parent.bottom
        width: parent.width
        clip: true
        model: root.processes
        spacing: 2
        reuseItems: true
        boundsBehavior: Flickable.StopAtBounds
        focus: true
        keyNavigationEnabled: false

        Keys.onUpPressed: root.step(-1)
        Keys.onDownPressed: root.step(1)
        Keys.onPressed: event => {
            if (event.key === Qt.Key_PageUp)
                root.step(-10)
            else if (event.key === Qt.Key_PageDown)
                root.step(10)
            else if (event.key === Qt.Key_Escape && root.selectedPid >= 0)
                root.selected(-1)
            else if (event.key === Qt.Key_Delete && root.selectedPid >= 0)
                root.signalRequested(root.selectedPid, "terminate")
            else
                return
            event.accepted = true
        }

        T.ScrollBar.vertical: StyledScrollBar {}

        delegate: Item {
            id: row

            required property int index
            required property int pid
            required property string name
            required property string command
            required property string user
            required property real cpu
            required property real memory
            required property real readRate
            required property real writeRate
            required property int threads
            required property string state
            required property int nice
            required property bool mine
            required property real gpu
            required property real gpuMemory

            readonly property bool selected: pid === root.selectedPid

            width: list.width - 12
            height: 40

            Rectangle {
                anchors.fill: parent
                radius: Appearance.rounding.small
                color: row.selected ? Appearance.colors.secondaryContainer
                     : Qt.alpha(Appearance.colors.onSurface, hover.hovered ? 0.05 : 0)

                Behavior on color {
                    ElementMoveFastColor {}
                }
            }

            Row {
                x: root.sidePadding
                height: parent.height

                Item {
                    width: root.nameWidth
                    height: parent.height

                    StyledText {
                        id: nameText
                        anchors.verticalCenter: parent.verticalCenter
                        width: Math.min(implicitWidth, parent.width - 10)
                        text: row.name
                        color: row.selected ? Appearance.colors.onSecondaryContainer : Appearance.colors.onSurface
                        font.pixelSize: Appearance.fontSize.smallie
                        font.weight: row.selected ? Font.DemiBold : Font.Medium
                        elide: Text.ElideRight
                    }
                    StyledText {
                        anchors.left: nameText.right
                        anchors.leftMargin: 10
                        anchors.right: parent.right
                        anchors.rightMargin: 12
                        anchors.verticalCenter: parent.verticalCenter
                        text: row.command
                        color: Appearance.colors.onSurfaceVariant
                        opacity: 0.75
                        font.pixelSize: Appearance.fontSize.smaller
                        elide: Text.ElideRight
                    }
                }

                Repeater {
                    model: root.columns

                    Item {
                        id: cell

                        required property string modelData
                        readonly property var def: root.columnDefs[modelData]
                        readonly property real fraction: Math.min(1, root.barFraction(row, modelData))

                        width: root.columnWidth(modelData)
                        height: row.height

                        // Grows from the left so the right-aligned number stays readable.
                        Rectangle {
                            // Slivers read as noise; only meaningful bars are drawn.
                            visible: width >= 6
                            x: 8
                            anchors.verticalCenter: parent.verticalCenter
                            height: 26
                            width: (parent.width - 12) * cell.fraction
                            radius: Math.min(Appearance.rounding.verySmall, width / 2)
                            color: Qt.alpha(cell.modelData === "cpu" ? Appearance.colors.cpu
                                            : cell.modelData === "gpu" ? Appearance.colors.gpu : Appearance.colors.memory, 0.24)

                            Behavior on width {
                                enabled: !Appearance.performanceMode
                                NumberAnimation { duration: 400; easing.type: Easing.BezierSpline; easing.bezierCurve: Appearance.curves.expressiveEffects }
                            }
                        }
                        StyledText {
                            anchors.fill: parent
                            anchors.leftMargin: 4
                            anchors.rightMargin: 10
                            verticalAlignment: Text.AlignVCenter
                            horizontalAlignment: cell.def.align
                            text: root.cellText(row, cell.modelData)
                            color: text === "—" ? Appearance.colors.onSurfaceVariant
                                 : row.selected ? Appearance.colors.onSecondaryContainer : Appearance.colors.onSurface
                            font.pixelSize: Appearance.fontSize.smallie
                            tabularNumbers: true
                            elide: Text.ElideRight
                        }
                    }
                }
            }

            HoverHandler {
                id: hover
                cursorShape: Qt.PointingHandCursor
            }
            TapHandler {
                onTapped: {
                    list.forceActiveFocus()
                    root.selected(row.selected ? -1 : row.pid)
                }
            }
        }
    }

    LoadingState {
        anchors.centerIn: list
        visible: list.count === 0 && root.waiting
    }

    EmptyState {
        anchors.centerIn: list
        visible: list.count === 0 && !root.waiting
        symbol: "search_off"
        title: qsTr("Nothing Matches")
        description: root.processes.filter
                     ? qsTr("No process matches “%1”. Check the spelling or search for something else.").arg(root.processes.filter)
                     : qsTr("Nothing to show with these filters. Try All instead of Mine.")
    }
}
