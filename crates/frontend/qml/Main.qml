import QtQuick
import PluSystemMonitor

// The frameless window: the drawer on layer0, the content sheet beside it
// with a top bar that has no background, and the inspector sheet that opens
// on a spring when something is picked. 8 px of window between them all.
Window {
    id: window

    property var prefs: JSON.parse(Config.preferences())
    property int page: prefs.page ?? 0
    property bool editing: false
    readonly property bool maximized: visibility === Window.Maximized || visibility === Window.FullScreen
    readonly property real gutter: Appearance.sizes.gutter
    readonly property bool systemDark: Application.styleHints.colorScheme !== Qt.ColorScheme.Light
    readonly property bool wantsDark: prefs.theme_mode === "light" ? false : prefs.theme_mode === "dark" ? true : systemDark

    readonly property var pageInfo: [
        { title: qsTr("Overview"), key: "overview" },
        { title: qsTr("Apps"), key: "apps" },
        { title: qsTr("Processes"), key: "processes" }
    ]

    function savePrefs() {
        Config.savePreferences(JSON.stringify(prefs))
    }

    function setPref(key, value) {
        prefs[key] = value
        prefs = Object.assign({}, prefs)
        savePrefs()
    }

    // Called once connected, so the list only holds machines that worked.
    function rememberConnection() {
        const connection = prefs.connection
        const others = connection.recent.filter(entry => entry.address !== connection.address)
        const latest = { address: connection.address, token: connection.token, hostname: monitor.hostname }
        setPref("connection", Object.assign({}, connection, { recent: [latest, ...others].slice(0, 8) }))
    }

    function forgetConnection(address) {
        const connection = prefs.connection
        const recent = connection.recent.filter(entry => entry.address !== address)
        setPref("connection", Object.assign({}, connection, { recent: recent }))
    }

    function applyScheme() {
        Appearance.applyScheme(JSON.parse(Config.scheme(prefs.theme, wantsDark)))
    }

    // Crossfade: freeze the current look in a snapshot, switch underneath,
    // fade the snapshot out.
    function changeTheme(change) {
        if (Appearance.performanceMode) {
            change()
            applyScheme()
            return
        }
        const dpr = Screen.devicePixelRatio
        window.contentItem.grabToImage(result => {
            snapshot.source = result.url
            snapshot.opacity = 1
            change()
            applyScheme()
            fade.restart()
        }, Qt.size(window.width * dpr, window.height * dpr))
    }

    width: 1280
    height: 860
    minimumWidth: 820
    minimumHeight: 560
    visible: true
    flags: Qt.Window | Qt.FramelessWindowHint
    color: "transparent"
    title: monitor.hostname ? monitor.hostname + " — PluSystemMonitor" : "PluSystemMonitor"

    onPageChanged: {
        editing = false
        if (prefs.page !== page)
            setPref("page", page)
    }
    onWantsDarkChanged: applyScheme()

    Binding {
        target: Appearance
        property: "reduceMotionSetting"
        value: window.prefs.reduce_motion ?? false
    }
    Binding {
        target: Appearance
        property: "performanceMode"
        value: window.prefs.performance_mode ?? false
    }
    Binding {
        target: Appearance
        property: "noShapes"
        value: window.prefs.no_shapes ?? false
    }

    Monitor {
        id: monitor
        // Apps are built from processes too, as are app and game widget sources.
        processesEnabled: window.page !== 0 || dashboard.usesProcesses
    }

    // The window itself: layer0, rounded unless maximised.
    Rectangle {
        id: frame

        anchors.fill: parent
        radius: window.maximized ? 0 : Appearance.rounding.window
        color: Appearance.colors.layer0
        border.width: window.maximized ? 0 : 1
        border.color: Appearance.colors.layer0Border

        Sidebar {
            id: sidebar

            x: window.gutter
            y: 0
            width: preferredWidth
            height: parent.height
            window: window
            monitor: monitor
            page: window.page
            preferredWidth: window.prefs.sidebar_width ?? Appearance.sizes.sidebarWidth
            onPageRequested: index => window.page = index
            onConnectionRequested: connectionDialog.open()
            onAppearanceRequested: appearanceDialog.open()
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
                    monitor: monitor
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
                    monitor: monitor
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
                    monitor: monitor
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
                monitor: monitor
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
                id: snackbar
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
            readonly property bool roomy: frame.width - sheet.x - Appearance.sizes.sideSheetWidth - 2 * window.gutter >= 404
            property real openness: wanted && roomy ? 1 : 0

            x: frame.width - window.gutter - openness * (Appearance.sizes.sideSheetWidth + window.gutter)
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
                monitor: monitor
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
                monitor: monitor
                pid: processesPage.selectedPid
                onCloseRequested: processesPage.selectedPid = -1
                onPidRequested: pid => processesPage.selectedPid = pid
            }
        }

        WindowEdges {
            anchors.fill: parent
            visible: !window.maximized
            window: window
            z: 100
        }
    }

    WidgetSettings {
        id: widgetSettings
        monitor: monitor
        onValuesEdited: values => dashboard.setOptions(index, values)
    }

    ConnectionDialog {
        id: connectionDialog
        monitor: monitor
        address: window.prefs.connection.address
        token: window.prefs.connection.token
        currentAddress: window.prefs.connection.address
        recent: window.prefs.connection.recent
        onConnectRequested: (address, token) => {
            window.setPref("connection", Object.assign({}, window.prefs.connection, { address: address, token: token }))
            monitor.connectTo(address, token)
        }
        onForgetRequested: address => window.forgetConnection(address)
    }

    TrustDialog {
        id: trustDialog
        onTrusted: (authority, fingerprint) => monitor.trustFingerprint(authority, fingerprint)
    }

    AppearanceDialog {
        id: appearanceDialog
        prefs: window.prefs
        dark: window.wantsDark
        onThemeSelected: id => window.changeTheme(() => window.setPref("theme", id))
        onModeSelected: mode => window.changeTheme(() => window.setPref("theme_mode", mode))
        onPreferenceEdited: (key, value) => window.setPref(key, value)
    }

    Connections {
        target: Config
        function onErrorOccurred(message) { snackbar.show(message) }
        function onSchemeFilesChanged() { window.applyScheme() }
    }

    Connections {
        target: monitor
        function onActionFailed(message) { snackbar.show(message) }
        function onConnectedChanged() {
            if (monitor.connected)
                window.rememberConnection()
        }
        function onTrustRequested(authority, fingerprint, previous) {
            trustDialog.ask(authority, fingerprint, previous)
        }
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

    Image {
        id: snapshot

        anchors.fill: parent
        z: 100
        opacity: 0
        visible: opacity > 0

        NumberAnimation on opacity {
            id: fade
            running: false
            to: 0
            duration: 450
            easing.type: Easing.InOutQuad
            onFinished: snapshot.source = ""
        }
    }

    Component.onCompleted: {
        applyScheme()
        dashboard.load(Config.layout())
        monitor.connectTo(prefs.connection.address, prefs.connection.token)
    }

    // `--screenshot <file.png>`: let data and animations settle, save the window, quit.
    Timer {
        readonly property int flag: Qt.application.arguments.indexOf("--screenshot")

        running: flag > 0 && flag + 1 < Qt.application.arguments.length
        interval: 5000
        onTriggered: window.contentItem.grabToImage(result => {
            result.saveToFile(Qt.application.arguments[flag + 1])
            Qt.quit()
        })
    }
}
