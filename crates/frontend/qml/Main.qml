import QtQuick
import PluSystemMonitor

// The system monitor's window: the drawer on layer0, the content sheet
// beside it with a top bar that has no background, and the inspector sheet
// that opens on a spring when something is picked. 8 px of window between
// them all. What both apps share is in AppWindow.
AppWindow {
    id: window

    property bool editing: false

    readonly property var pageInfo: [
        { title: qsTr("Overview"), key: "overview" },
        { title: qsTr("Apps"), key: "apps" },
        { title: qsTr("Processes"), key: "processes" }
    ]

    appName: "PluSystemMonitor"
    // Apps are built from processes too, as are app and game widget sources.
    processesWanted: window.page !== 0 || dashboard.usesProcesses
    snackbar: messages

    onPageChanged: editing = false
    onOpenRequested: name => {
        if (name === "virtual")
            sensorsDialog.tab = 1
        if (name === "sensors" || name === "virtual")
            sensorsDialog.open()
    }

    Sidebar {
        id: sidebar

        x: window.gutter
        y: 0
        width: preferredWidth
        height: parent.height
        window: window
        monitor: window.monitor
        page: window.page
        preferredWidth: window.prefs.sidebar_width ?? Appearance.sizes.sidebarWidth
        onPageRequested: index => window.page = index
        updatesAvailable: window.updatesAvailable
        onConnectionRequested: window.connectionDialog.open()
        onAppearanceRequested: window.appearanceDialog.open()
        onSensorsRequested: sensorsDialog.open()
        onUpdatesRequested: window.updatesDialog.open()
        onWidthEdited: width => window.setPref("sidebar_width", width)
    }

    // The content sheet.
    Rectangle {
        id: sheet

        x: sidebar.x + sidebar.width + window.gutter
        y: window.gutter
        width: inspector.x - window.gutter - x
        height: parent.height - 2 * window.gutter
        radius: Appearance.rounding.large
        color: Appearance.colors.layer1
        clip: true

        // Faint shapes behind the page's title, a layout of its own for
        // each page, cut off by the sheet's top edge and kept out of
        // the page below.
        Item {
            width: parent.width
            height: Appearance.sizes.topBarHeight
            clip: true

            Stickers {
                anchors.fill: parent
                key: window.pageInfo[window.page].key
                count: 4
                minimumSize: 50
                maximumSize: 110
                fromY: -0.5
                toY: 0
                strength: 0.8
            }
        }

        Item {
            id: pages

            anchors.fill: parent

            Dashboard {
                id: dashboard

                anchors.fill: parent
                topMargin: Appearance.sizes.topBarHeight
                leftMargin: Appearance.sizes.pageMargin
                rightMargin: Appearance.sizes.pageMargin
                bottomMargin: Appearance.sizes.pageMargin
                monitor: window.monitor
                editing: window.editing
                visible: opacity > 0
                opacity: window.page === 0 ? 1 : 0
                onEdited: saveLayout.restart()
                onSettingsRequested: (index, descriptor, values) => widgetSettings.edit(index, descriptor, values)

                Behavior on opacity {
                    NumberAnimation { duration: Appearance.duration(220) }
                }
            }

            AppsPage {
                id: appsPage

                anchors.fill: parent
                anchors.topMargin: Appearance.sizes.topBarHeight
                monitor: window.monitor
                settings: window.prefs.apps
                visible: opacity > 0
                opacity: window.page === 1 ? 1 : 0
                onSettingsEdited: settings => window.setPref("apps", settings)

                Behavior on opacity {
                    NumberAnimation { duration: Appearance.duration(220) }
                }
            }

            ProcessesPage {
                id: processesPage

                anchors.fill: parent
                anchors.topMargin: Appearance.sizes.topBarHeight
                monitor: window.monitor
                settings: window.prefs.processes
                visible: opacity > 0
                opacity: window.page === 2 ? 1 : 0
                onSettingsEdited: settings => window.setPref("processes", settings)

                Behavior on opacity {
                    NumberAnimation { duration: Appearance.duration(220) }
                }
            }
        }

        TopBar {
            id: topBar

            width: parent.width
            height: Appearance.sizes.topBarHeight
            window: window
            monitor: window.monitor
            title: window.pageInfo[window.page].title
            page: window.page
            editing: window.editing
            columnDefs: processesPage.columnDefs
            allColumns: processesPage.allColumns
            processSettings: window.prefs.processes
            onEditRequested: window.editing = true
            onDoneRequested: window.editing = false
            onAddWidgetRequested: type => dashboard.add(type)
            onResetRequested: {
                dashboard.load(Config.defaultLayout())
                saveLayout.restart()
            }
            onProcessSettingsEdited: settings => window.setPref("processes", settings)
        }

        Snackbar {
            id: messages
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 16
            z: 50
        }

        // What fills the sheet clips square; these round its corners off.
        CornerCaps {
            anchors.fill: parent
            radius: sheet.radius
            color: Appearance.colors.layer0
            z: 60
        }
    }

    // The inspector: what's picked on the Apps or Processes page. It
    // opens on a spring, so the page beside it gives a little too, and
    // stays shut while the page would be left narrower than 404 px.
    Item {
        id: inspector

        readonly property bool wanted: (window.page === 1 && appsPage.selectedId !== "")
                                       || (window.page === 2 && processesPage.selectedPid >= 0)
        readonly property bool roomy: window.frame.width - sheet.x - Appearance.sizes.sideSheetWidth - 2 * window.gutter >= 404
        property real openness: wanted && roomy ? 1 : 0

        x: window.frame.width - window.gutter - openness * (Appearance.sizes.sideSheetWidth + window.gutter)
        y: window.gutter
        width: Appearance.sizes.sideSheetWidth
        height: parent.height - 2 * window.gutter
        visible: openness > 0.001

        Behavior on openness {
            enabled: !Appearance.performanceMode
            SpringAnimation { spring: 4; damping: 0.32; epsilon: 0.002 }
        }

        AppDetails {
            anchors.fill: parent
            visible: window.page === 1
            monitor: window.monitor
            appId: appsPage.selectedId
            onCloseRequested: appsPage.selectedId = ""
            onProcessRequested: pid => {
                processesPage.selectedPid = pid
                window.page = 2
            }
        }

        ProcessDetails {
            anchors.fill: parent
            visible: window.page === 2
            monitor: window.monitor
            pid: processesPage.selectedPid
            onCloseRequested: processesPage.selectedPid = -1
            onPidRequested: pid => processesPage.selectedPid = pid
        }
    }

    WidgetSettings {
        id: widgetSettings
        monitor: window.monitor
        onValuesEdited: values => dashboard.setOptions(index, values)
    }

    SensorsDialog {
        id: sensorsDialog
        monitor: window.monitor
        virtualSensors: window.virtualSensors
        onVirtualSensorsEdited: sensors => window.saveVirtualSensors(sensors)
    }

    // Edits come in bursts (drag, then options); write once things settle.
    Timer {
        id: saveLayout
        interval: 400
        onTriggered: Config.saveLayout(dashboard.serialize())
    }

    Shortcut {
        sequence: "Escape"
        enabled: window.editing
        onActivated: window.editing = false
    }

    Component.onCompleted: dashboard.load(Config.layout())
}
