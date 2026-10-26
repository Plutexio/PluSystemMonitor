import QtQuick
import PluSystemMonitor

// The content sheet's top bar. It has no background: the page scrolls up
// under it, and every button stands on a shape of its own. The page's title
// on the left, its actions and the window's controls on the right. Its empty
// stretches move the window, and a double click maximises it.
Item {
    id: root

    required property Window window
    required property Monitor monitor
    property string title
    property int page: 0
    property bool editing: false
    property var columnDefs: ({})
    property var allColumns: []
    property var processSettings: ({})

    signal editRequested()
    signal doneRequested()
    signal addWidgetRequested(string type)
    signal resetRequested()
    signal processSettingsEdited(var settings)

    onEditingChanged: resetButton.armed = false

    DragHandler {
        target: null
        onActiveChanged: if (active) root.window.startSystemMove()
    }
    TapHandler {
        onDoubleTapped: root.window.visibility === Window.Maximized ? root.window.showNormal() : root.window.showMaximized()
    }

    Row {
        id: heading

        x: Appearance.sizes.pageMargin
        anchors.verticalCenter: parent.verticalCenter
        spacing: 14

        TitleText {
            id: pageTitle
            anchors.verticalCenter: parent.verticalCenter
            level: "headline"
            text: root.title
            animateChange: true
        }
        StyledText {
            anchors.baseline: pageTitle.baseline
            width: Math.min(implicitWidth, Math.max(0, actions.x - heading.x - pageTitle.width - 40))
            visible: root.editing
            text: qsTr("Drag widgets to move them, and the corner to resize. Esc to finish.")
            color: Appearance.colors.onSurfaceVariant
            font.pixelSize: Appearance.fontSize.smallie
            elide: Text.ElideRight
        }
    }

    Row {
        id: actions

        anchors.right: controls.left
        anchors.rightMargin: 16
        anchors.verticalCenter: parent.verticalCenter
        spacing: 8

        // Overview, at rest.
        ToolbarButton {
            anchors.verticalCenter: parent.verticalCenter
            visible: root.page === 0 && !root.editing
            symbol: "edit"
            shape: "clover4"
            effect: ToolbarButton.Spin
            tooltip: qsTr("Edit Layout")
            onClicked: root.editRequested()
        }

        // Overview, while the layout is edited.
        TextButton {
            id: addButton
            anchors.verticalCenter: parent.verticalCenter
            visible: root.editing
            variant: TextButton.Tonal
            symbol: "add"
            text: qsTr("Add Widget")
            onClicked: palette.open()

            WidgetPalette {
                id: palette
                y: parent.height + 8
                x: parent.width - width
                onPicked: type => root.addWidgetRequested(type)
            }
        }
        TextButton {
            id: resetButton

            // Two steps, so a stray click can't wipe a carefully built layout.
            property bool armed: false

            anchors.verticalCenter: parent.verticalCenter
            visible: root.editing
            variant: armed ? TextButton.Danger : TextButton.DangerTonal
            symbol: "restart_alt"
            text: armed ? qsTr("Click Again to Reset") : qsTr("Reset")
            onClicked: {
                if (armed) {
                    armed = false
                    root.resetRequested()
                } else {
                    armed = true
                    disarm.restart()
                }
            }

            Timer {
                id: disarm
                interval: 3000
                onTriggered: resetButton.armed = false
            }
        }
        TextButton {
            anchors.verticalCenter: parent.verticalCenter
            visible: root.editing
            variant: TextButton.Filled
            symbol: "check"
            text: qsTr("Done")
            onClicked: root.doneRequested()
        }

        // Processes.
        ToolbarButton {
            anchors.verticalCenter: parent.verticalCenter
            visible: root.page === 2
            symbol: "view_column"
            shape: "cookie7"
            tooltip: qsTr("Columns and Filters")
            onClicked: columnsPopup.open()

            ProcessViewPopup {
                id: columnsPopup
                y: parent.height + 8
                x: parent.width - width
                allColumns: root.allColumns
                columnDefs: root.columnDefs
                columns: root.processSettings.columns ?? []
                showKernelThreads: root.processSettings.show_kernel_threads ?? false
                onColumnsEdited: columns => root.processSettingsEdited(Object.assign({}, root.processSettings, { columns: columns }))
                onKernelThreadsToggled: root.processSettingsEdited(Object.assign({}, root.processSettings, {
                    show_kernel_threads: !(root.processSettings.show_kernel_threads ?? false)
                }))
            }
        }
    }

    WindowControls {
        id: controls
        anchors.right: parent.right
        anchors.rightMargin: 14
        anchors.verticalCenter: parent.verticalCenter
        window: root.window
    }
}
