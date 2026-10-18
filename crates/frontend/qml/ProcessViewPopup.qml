import QtQuick
import PluSystemMonitor

// Which columns the process list shows, and whether kernel threads are listed.
GlassPopup {
    id: root

    property var allColumns: []
    property var columnDefs: ({})
    property var columns: []
    property bool showKernelThreads: false

    signal columnsEdited(var columns)
    signal kernelThreadsToggled()

    title: "Columns"
    width: 280

    component OptionRow: Item {
        id: option

        property string label
        property bool checked

        signal toggled()

        width: parent.width
        height: 32

        Text {
            anchors.verticalCenter: parent.verticalCenter
            text: option.label
            color: Theme.text
            font.pixelSize: 14
        }
        Toggle {
            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter
            checked: option.checked
            onToggled: option.toggled()
        }
    }

    contentItem: Column {
        spacing: 4

        Repeater {
            model: root.allColumns

            OptionRow {
                required property string modelData
                label: root.columnDefs[modelData].title
                checked: root.columns.includes(modelData)
                // Keep the canonical column order no matter the click order.
                onToggled: root.columnsEdited(root.allColumns.filter(key =>
                    key === modelData ? !checked : root.columns.includes(key)))
            }
        }

        Rectangle {
            width: parent.width
            height: 1
            color: Theme.cardBorder
        }
        Item {
            width: 1
            height: 6
        }

        OptionRow {
            label: "Kernel threads"
            checked: root.showKernelThreads
            onToggled: root.kernelThreadsToggled()
        }
    }
}
