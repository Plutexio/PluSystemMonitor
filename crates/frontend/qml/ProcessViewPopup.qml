import QtQuick
import QtQuick.Templates as T
import PluSystemMonitor

// Which columns the process list shows, and whether kernel threads are listed.
StyledPopover {
    id: root

    property var allColumns: []
    property var columnDefs: ({})
    property var columns: []
    property bool showKernelThreads: false

    signal columnsEdited(var columns)
    signal kernelThreadsToggled()

    title: qsTr("Columns")
    symbol: "view_column"
    iconShape: "cookie7"
    width: 300

    component OptionRow: RippleButton {
        id: option

        property string label
        property bool value

        width: parent.width
        height: 40
        buttonRadius: Appearance.rounding.small
        colBackgroundHover: Appearance.colors.layer2Hover
        colRipple: Appearance.colors.layer2Active
        bounce: false

        contentItem: Item {
            StyledText {
                x: 4
                anchors.verticalCenter: parent.verticalCenter
                text: option.label
            }
            StyledSwitch {
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                checked: option.value
                onClicked: option.clicked()
            }
        }
    }

    // It scrolls once it would run past the window.
    contentItem: Flickable {
        implicitWidth: options.implicitWidth
        implicitHeight: Math.min(options.implicitHeight, 440)
        contentHeight: options.implicitHeight
        boundsBehavior: Flickable.StopAtBounds
        clip: true

        T.ScrollBar.vertical: StyledScrollBar {}

        Column {
            id: options

            width: parent.width
            spacing: 2

            Repeater {
                model: root.allColumns

                OptionRow {
                    required property string modelData
                    label: root.columnDefs[modelData].title
                    value: root.columns.includes(modelData)
                    // Keep the canonical column order no matter the click order.
                    onClicked: root.columnsEdited(root.allColumns.filter(key =>
                        key === modelData ? !value : root.columns.includes(key)))
                }
            }

            SectionLabel {
                width: parent.width
                inset: 4
                text: qsTr("Filters")
            }

            OptionRow {
                label: qsTr("Kernel Threads")
                value: root.showKernelThreads
                onClicked: root.kernelThreadsToggled()
            }
        }
    }
}
