import QtQuick
import PluSystemMonitor

// What both apps' windows have: the frameless window on layer0 with its
// edges, the preferences, the theme, the connection to a machine, the other
// machines' updates, and the dialogs for those. An app puts its drawer and
// sheet inside (they land in the frame) and its own dialogs beside them.
Window {
    id: window

    /// "PluSystemMonitor", "PluFan": the window title's second half.
    property string appName: "PluSystemMonitor"
    property var prefs: JSON.parse(Config.preferences())
    property int page: prefs.page ?? 0
    /// Whether the daemon should send processes now.
    property bool processesWanted: false
    /// Where messages show; the app's sheet holds it.
    property Snackbar snackbar: null
    readonly property bool maximized: visibility === Window.Maximized || visibility === Window.FullScreen
    readonly property real gutter: Appearance.sizes.gutter
    readonly property bool systemDark: Application.styleHints.colorScheme !== Qt.ColorScheme.Light
    readonly property bool wantsDark: prefs.theme_mode === "light" ? false : prefs.theme_mode === "dark" ? true : systemDark

    readonly property alias monitor: monitor
    readonly property alias fleet: fleet
    readonly property alias frame: frame
    readonly property alias connectionDialog: connectionDialog
    readonly property alias updatesDialog: updatesDialog
    readonly property alias appearanceDialog: appearanceDialog
    default property alias content: frame.data

    /// `--open <name>` on the command line, for screenshots of a dialog.
    /// Connection, updates and appearance open by themselves.
    signal openRequested(string name)

    function savePrefs() {
        Config.savePreferences(JSON.stringify(prefs))
    }

    function setPref(key, value) {
        prefs[key] = value
        prefs = Object.assign({}, prefs)
        savePrefs()
    }

    // Called once connected, so the list only holds machines that worked.
    // It is also the list of machines the Updates dialog looks after.
    function rememberConnection() {
        const connection = prefs.connection
        const others = connection.recent.filter(entry => entry.address !== connection.address)
        const latest = { address: connection.address, token: connection.token, hostname: monitor.hostname }
        setPref("connection", Object.assign({}, connection, { recent: [latest, ...others].slice(0, 64) }))
    }

    /// Every machine connected to before, the current one included.
    function machines() {
        const connection = prefs.connection
        const list = connection.recent.map(entry => ({ address: entry.address, token: entry.token, hostname: entry.hostname }))
        if (!list.some(entry => entry.address === connection.address))
            list.unshift({ address: connection.address, token: connection.token, hostname: "" })
        return list
    }

    function probeMachines(check) {
        fleet.probe(JSON.stringify(machines()), check)
    }

    function forgetConnection(address) {
        const connection = prefs.connection
        const recent = connection.recent.filter(entry => entry.address !== address)
        setPref("connection", Object.assign({}, connection, { recent: recent }))
    }

    // Sources of the user's own, linked to real ones per machine (see
    // virtual_sensors.rs). Saved right away; the file keeps what was accepted.
    property var virtualSensors: JSON.parse(Config.virtualSensors())

    function saveVirtualSensors(sensors) {
        const kept = Config.saveVirtualSensors(JSON.stringify(sensors))
        if (kept) {
            virtualSensors = JSON.parse(kept)
            monitor.setVirtualSensors(kept)
        }
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
    title: monitor.hostname ? monitor.hostname + " — " + appName : appName

    onPageChanged: {
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
        processesEnabled: window.processesWanted
    }

    // What the other machines are up to: reachable or not, their versions
    // and updates, keyed by address.
    property var machineStatus: ({})
    property var updateProgress: ({})
    readonly property int updatesAvailable: {
        let count = 0
        for (const address in machineStatus) {
            if (Updates.updatable(machineStatus[address], updateProgress[address]))
                count++
        }
        return count
    }

    Fleet {
        id: fleet

        onProbed: (address, json) => {
            window.machineStatus = Object.assign({}, window.machineStatus, { [address]: JSON.parse(json) })
        }
        onUpdateEvent: (address, json) => {
            const event = JSON.parse(json)
            const before = window.updateProgress[address] ?? { state: "", lines: [], error: "" }
            const after = {
                state: event.state ?? before.state,
                lines: event.line ? [...before.lines, event.line].slice(-200) : before.lines,
                error: event.error ?? before.error
            }
            window.updateProgress = Object.assign({}, window.updateProgress, { [address]: after })
            // A daemon that refused us for its age speaks again once updated.
            if (event.state === "done" && address === window.prefs.connection.address && !monitor.connected)
                monitor.connectTo(address, window.prefs.connection.token)
        }
    }

    // Looking for updates in the background, unless that's turned off.
    Timer {
        running: window.prefs.check_updates ?? true
        interval: 6 * 3600 * 1000
        repeat: true
        triggeredOnStart: false
        onTriggered: window.probeMachines(true)
    }
    Timer {
        running: true
        interval: 8000
        onTriggered: window.probeMachines(window.prefs.check_updates ?? true)
    }

    // The window itself: layer0, rounded unless maximised.
    Rectangle {
        id: frame

        anchors.fill: parent
        radius: window.maximized ? 0 : Appearance.rounding.window
        color: Appearance.colors.layer0
        border.width: window.maximized ? 0 : 1
        border.color: Appearance.colors.layer0Border

        WindowEdges {
            anchors.fill: parent
            visible: !window.maximized
            window: window
            z: 100
        }
    }

    ConnectionDialog {
        id: connectionDialog
        monitor: monitor
        address: window.prefs.connection.address
        token: window.prefs.connection.token
        currentAddress: window.prefs.connection.address
        recent: window.prefs.connection.recent
        machineStatus: window.machineStatus
        onProbeRequested: window.probeMachines(false)
        onUpdatesRequested: {
            close()
            updatesDialog.open()
        }
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

    UpdatesDialog {
        id: updatesDialog
        monitor: monitor
        fleet: fleet
        machines: window.machines()
        machineStatus: window.machineStatus
        updateProgress: window.updateProgress
        checkUpdates: window.prefs.check_updates ?? true
        onCheckUpdatesEdited: value => window.setPref("check_updates", value)
        onCheckRequested: window.probeMachines(true)
        onLookRequested: window.probeMachines(window.prefs.check_updates ?? true)
        onUpdateRequested: (address, token) => {
            window.updateProgress = Object.assign({}, window.updateProgress, { [address]: { state: "starting", lines: [], error: "" } })
            fleet.update(address, token)
        }
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
        function onErrorOccurred(message) { window.snackbar?.show(message) }
        function onSchemeFilesChanged() { window.applyScheme() }
    }

    Connections {
        target: monitor
        function onActionFailed(message) { window.snackbar?.show(message) }
        function onConnectedChanged() {
            if (monitor.connected)
                window.rememberConnection()
        }
        function onTrustRequested(authority, fingerprint, previous) {
            trustDialog.ask(authority, fingerprint, previous)
        }
        function onIncompatibleChanged() {
            if (monitor.incompatible)
                window.snackbar?.show(monitor.status)
        }
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
        monitor.setVirtualSensors(JSON.stringify(virtualSensors))
        monitor.connectTo(prefs.connection.address, prefs.connection.token)
    }

    // `--open <dialog>`: open a dialog on start, for screenshots of it.
    Timer {
        readonly property int flag: Qt.application.arguments.indexOf("--open")

        running: flag > 0 && flag + 1 < Qt.application.arguments.length
        interval: 1500
        onTriggered: {
            const name = Qt.application.arguments[flag + 1]
            const dialogs = { connection: connectionDialog, updates: updatesDialog, appearance: appearanceDialog }
            if (dialogs[name])
                dialogs[name].open()
            else
                window.openRequested(name)
        }
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
