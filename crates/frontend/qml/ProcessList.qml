import QtQuick
import QtQuick.Controls.Basic
import PluSystemMonitor

// The process table: sortable header, one row per process, keyboard navigation.
Item {
    id: root

    clip: true
    required property ProcessModel processes
    /// Visible columns besides the always-present name.
    property var columns: []
    property int selectedPid: -1

    signal selected(int pid)
    signal signalRequested(int pid, string signal)

    readonly property var columnDefs: ({
        pid: { title: "PID", width: 76, align: Text.AlignRight, sort: "pid" },
        user: { title: "User", width: 112, align: Text.AlignLeft, sort: "user" },
        cpu: { title: "CPU", width: 92, align: Text.AlignRight, sort: "cpu" },
        memory: { title: "Memory", width: 108, align: Text.AlignRight, sort: "memory" },
        gpu: { title: "GPU", width: 84, align: Text.AlignRight, sort: "gpu" },
        gpuMemory: { title: "GPU memory", width: 116, align: Text.AlignRight, sort: "gpuMemory" },
        read: { title: "Disk read", width: 108, align: Text.AlignRight, sort: "read" },
        write: { title: "Disk write", width: 108, align: Text.AlignRight, sort: "write" },
        threads: { title: "Threads", width: 82, align: Text.AlignRight, sort: "threads" },
        state: { title: "State", width: 116, align: Text.AlignLeft, sort: "state" },
        nice: { title: "Nice", width: 64, align: Text.AlignRight, sort: "nice" }
    })
    readonly property var allColumns: ["pid", "user", "cpu", "memory", "gpu", "gpuMemory", "read", "write", "threads", "state", "nice"]

    readonly property real sidePadding: 14
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

            Text {
                width: Math.min(implicitWidth, cell.width - 28)
                text: cell.title.toUpperCase()
                elide: Text.ElideRight
                color: cell.active ? Theme.text : Theme.textDim
                font.pixelSize: 11
                font.weight: Font.DemiBold
                font.letterSpacing: 1.2
            }
            Icon {
                anchors.verticalCenter: parent.verticalCenter
                visible: cell.active
                name: root.processes.descending ? "chevronDown" : "chevronUp"
                color: Theme.primary
                size: 14
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
        width: parent.width
        height: 1
        color: Theme.cardBorder
    }

    ListView {
        id: list

        anchors.top: header.bottom
        anchors.topMargin: 6
        anchors.bottom: parent.bottom
        width: parent.width
        clip: true
        model: root.processes
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

        ScrollBar.vertical: ScrollBar {
            id: scrollBar
            width: 10
            contentItem: Rectangle {
                // A custom handle skips the style's "only when needed" logic.
                visible: scrollBar.size < 1.0
                implicitWidth: 6
                radius: 3
                color: Theme.textDim
                opacity: scrollBar.active ? 0.6 : 0.25

                Behavior on opacity {
                    NumberAnimation { duration: 200 }
                }
            }
        }

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
            height: 36

            Rectangle {
                anchors.fill: parent
                anchors.leftMargin: 6
                radius: 10
                color: row.selected ? Qt.alpha(Theme.primary, 0.2) : hover.hovered ? Theme.card : "transparent"
                border.color: row.selected ? Qt.alpha(Theme.primary, 0.5) : "transparent"
            }

            Row {
                x: root.sidePadding
                height: parent.height

                Item {
                    width: root.nameWidth
                    height: parent.height

                    Text {
                        id: nameText
                        anchors.verticalCenter: parent.verticalCenter
                        width: Math.min(implicitWidth, parent.width - 10)
                        text: row.name
                        color: Theme.text
                        font.pixelSize: 13
                        font.weight: Font.Medium
                        elide: Text.ElideRight
                    }
                    Text {
                        anchors.left: nameText.right
                        anchors.leftMargin: 10
                        anchors.right: parent.right
                        anchors.rightMargin: 12
                        anchors.verticalCenter: parent.verticalCenter
                        text: row.command
                        color: Theme.textDim
                        opacity: 0.7
                        font.pixelSize: 12
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
                            height: 24
                            width: (parent.width - 12) * cell.fraction
                            radius: Math.min(6, width / 2)
                            color: Qt.alpha(cell.modelData === "cpu" ? Theme.cpu
                                            : cell.modelData === "gpu" ? Theme.gpu : Theme.memory, 0.22)

                            Behavior on width {
                                NumberAnimation { duration: 400; easing.type: Easing.OutCubic }
                            }
                        }
                        Text {
                            anchors.fill: parent
                            anchors.leftMargin: 4
                            anchors.rightMargin: 10
                            verticalAlignment: Text.AlignVCenter
                            horizontalAlignment: cell.def.align
                            text: root.cellText(row, cell.modelData)
                            color: text === "—" ? Theme.textDim : Theme.text
                            font.pixelSize: 13
                            font.features: { "tnum": 1 }
                            elide: Text.ElideRight
                        }
                    }
                }
            }

            HoverHandler {
                id: hover
            }
            TapHandler {
                onTapped: {
                    list.forceActiveFocus()
                    root.selected(row.selected ? -1 : row.pid)
                }
            }
        }
    }

    Text {
        anchors.centerIn: list
        visible: list.count === 0
        text: root.processes.filter ? "No process matches “" + root.processes.filter + "”" : "Waiting for data…"
        color: Theme.textDim
        font.pixelSize: 14
    }
}
