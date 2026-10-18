import QtQuick
import PluSystemMonitor

// Widgets on a grid. Rows stretch so the layout fills the view when it fits,
// and the view scrolls when it doesn't.
Flickable {
    id: root

    required property Monitor monitor
    property bool editing: false
    property int columns: 12
    property int usedRows: 1
    // Bumped on every change so bindings that scan the model re-evaluate.
    property int revision: 0

    readonly property real gap: Theme.spacing
    readonly property real minRowUnit: 56
    readonly property real colUnit: (width + gap) / columns
    readonly property real rowUnit: Math.max(minRowUnit, (height + gap) / usedRows)
    // Spare rows in edit mode give room to drop widgets below the layout.
    readonly property int shownRows: usedRows + (editing ? 3 : 0)

    // Drop preview while a widget is dragged or resized.
    property rect ghost
    property bool ghostVisible: false
    property bool ghostValid: true

    signal edited()
    signal settingsRequested(int index, var descriptor, var values)

    function cellRect(x, y, w, h) {
        return Qt.rect(x * colUnit, y * rowUnit, w * colUnit - gap, h * rowUnit - gap)
    }

    function fits(skip, x, y, w, h) {
        if (x < 0 || y < 0 || w < 1 || h < 1 || x + w > columns)
            return false
        for (let i = 0; i < widgets.count; i++) {
            if (i === skip)
                continue
            const o = widgets.get(i)
            if (x < o.gx + o.gw && o.gx < x + w && y < o.gy + o.gh && o.gy < y + h)
                return false
        }
        return true
    }

    function updateRows() {
        let rows = 1
        for (let i = 0; i < widgets.count; i++) {
            const o = widgets.get(i)
            rows = Math.max(rows, o.gy + o.gh)
        }
        usedRows = rows
        revision++
    }

    function load(json) {
        const layout = JSON.parse(json)
        columns = layout.columns
        widgets.clear()
        for (const w of layout.widgets) {
            widgets.append({
                kind: w.type, gx: w.x, gy: w.y, gw: w.w, gh: w.h,
                options: JSON.stringify(w.options ?? {})
            })
        }
        updateRows()
    }

    function serialize() {
        const list = []
        for (let i = 0; i < widgets.count; i++) {
            const o = widgets.get(i)
            list.push({ type: o.kind, x: o.gx, y: o.gy, w: o.gw, h: o.gh, options: JSON.parse(o.options) })
        }
        return JSON.stringify({ columns: columns, widgets: list })
    }

    function place(index, x, y, w, h) {
        if (!fits(index, x, y, w, h))
            return false
        widgets.set(index, { gx: x, gy: y, gw: w, gh: h })
        updateRows()
        edited()
        return true
    }

    function add(type) {
        const descriptor = Widgets.find(type)
        const w = Math.min(descriptor.size[0], columns)
        const h = descriptor.size[1]
        // First free spot in reading order; below everything at the latest.
        for (let y = 0; y <= usedRows; y++) {
            for (let x = 0; x + w <= columns; x++) {
                if (!fits(-1, x, y, w, h))
                    continue
                widgets.append({ kind: type, gx: x, gy: y, gw: w, gh: h, options: "{}" })
                updateRows()
                edited()
                const target = cellRect(x, y, w, h)
                contentY = Math.max(0, Math.min(target.y, contentHeight - height))
                return
            }
        }
    }

    function remove(index) {
        widgets.remove(index)
        updateRows()
        edited()
    }

    function setOptions(index, values) {
        widgets.setProperty(index, "options", JSON.stringify(values))
        revision++
        edited()
    }

    contentWidth: width
    contentHeight: Math.max(height, shownRows * rowUnit - gap)
    clip: true
    boundsBehavior: Flickable.StopAtBounds
    interactive: contentHeight > height

    ListModel {
        id: widgets
    }

    // Outlines of the free cells in edit mode.
    Repeater {
        model: root.editing ? root.columns * root.shownRows : 0

        Rectangle {
            required property int index
            readonly property int column: index % root.columns
            readonly property int row: Math.floor(index / root.columns)
            readonly property rect cell: root.cellRect(column, row, 1, 1)

            visible: root.revision >= 0 && root.fits(-1, column, row, 1, 1)
            x: cell.x
            y: cell.y
            width: cell.width
            height: cell.height
            radius: Math.min(10, Theme.radius / 2)
            color: "transparent"
            border.color: Theme.grid
            border.width: 1
        }
    }

    Rectangle {
        x: root.ghost.x
        y: root.ghost.y
        width: root.ghost.width
        height: root.ghost.height
        visible: root.ghostVisible
        radius: Theme.radius
        color: Qt.alpha(root.ghostValid ? Theme.good : Theme.bad, 0.12)
        border.color: root.ghostValid ? Theme.good : Theme.bad
        border.width: 2

        Behavior on x {
            NumberAnimation { duration: 120; easing.type: Easing.OutCubic }
        }
        Behavior on y {
            NumberAnimation { duration: 120; easing.type: Easing.OutCubic }
        }
        Behavior on width {
            NumberAnimation { duration: 120; easing.type: Easing.OutCubic }
        }
        Behavior on height {
            NumberAnimation { duration: 120; easing.type: Easing.OutCubic }
        }
    }

    Repeater {
        model: widgets

        delegate: WidgetFrame {
            dashboard: root
        }
    }
}
