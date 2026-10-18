import QtQuick
import QtQuick.Layouts
import PluSystemMonitor

Window {
    id: window

    property var prefs: JSON.parse(Config.preferences())

    function savePrefs() {
        Config.savePreferences(JSON.stringify(prefs))
    }

    // Crossfade: freeze the current look in a snapshot, switch underneath, fade it out.
    function switchTheme(id) {
        if (id === Theme.current)
            return
        const dpr = Screen.devicePixelRatio
        window.contentItem.grabToImage(result => {
            snapshot.source = result.url
            snapshot.opacity = 1
            if (Theme.load(id)) {
                prefs.theme = id
                savePrefs()
            }
            fade.restart()
        }, Qt.size(window.width * dpr, window.height * dpr))
    }

    width: 1180
    height: 760
    minimumWidth: 820
    minimumHeight: 560
    visible: true
    color: Theme.background
    title: monitor.hostname ? monitor.hostname + " — PluSystemMonitor" : "PluSystemMonitor"

    Monitor {
        id: monitor
        // Apps are built from processes too.
        processesEnabled: header.page !== 0
    }

    Background {
        anchors.fill: parent
        running: window.visibility !== Window.Minimized && window.visibility !== Window.Hidden
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 28
        spacing: Theme.spacing + 8

        Header {
            id: header

            Layout.fillWidth: true
            monitor: monitor
            page: window.prefs.page ?? 0
            onPageChanged: {
                if (window.prefs.page !== page) {
                    window.prefs.page = page
                    window.savePrefs()
                }
            }
            address: window.prefs.connection.address
            token: window.prefs.connection.token
            onConnectRequested: (address, token) => {
                window.prefs.connection = { address: address, token: token }
                window.savePrefs()
                monitor.connectTo(address, token)
            }
            onThemeSelected: id => window.switchTheme(id)
            onAddWidgetRequested: type => dashboard.add(type)
            onResetRequested: {
                dashboard.load(Config.defaultLayout())
                saveLayout.restart()
            }
        }

        Item {
            Layout.fillWidth: true
            Layout.fillHeight: true

            Dashboard {
                id: dashboard

                anchors.fill: parent
                monitor: monitor
                editing: header.editing
                visible: opacity > 0
                opacity: header.page === 0 ? 1 : 0
                onEdited: saveLayout.restart()
                onSettingsRequested: (index, descriptor, values) => widgetSettings.edit(index, descriptor, values)

                Behavior on opacity {
                    NumberAnimation { duration: 220 }
                }
            }

            AppsPage {
                anchors.fill: parent
                monitor: monitor
                settings: window.prefs.apps
                visible: opacity > 0
                opacity: header.page === 1 ? 1 : 0
                onSettingsEdited: settings => {
                    window.prefs.apps = settings
                    window.savePrefs()
                }
                onProcessRequested: pid => {
                    processesPage.selectedPid = pid
                    header.page = 2
                }

                Behavior on opacity {
                    NumberAnimation { duration: 220 }
                }
            }

            ProcessesPage {
                id: processesPage

                anchors.fill: parent
                monitor: monitor
                settings: window.prefs.processes
                visible: opacity > 0
                opacity: header.page === 2 ? 1 : 0
                onSettingsEdited: settings => {
                    window.prefs.processes = settings
                    window.savePrefs()
                }

                Behavior on opacity {
                    NumberAnimation { duration: 220 }
                }
            }
        }
    }

    WidgetSettings {
        id: widgetSettings
        anchors.centerIn: parent
        onValuesEdited: values => dashboard.setOptions(index, values)
    }

    Toast {
        id: toast
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.bottom: parent.bottom
        anchors.bottomMargin: 24
        z: 50
    }

    Connections {
        target: Config
        function onErrorOccurred(message) { toast.show(message) }
    }

    Connections {
        target: monitor
        function onActionFailed(message) { toast.show(message) }
    }

    // Edits come in bursts (drag, then options); write once things settle.
    Timer {
        id: saveLayout
        interval: 400
        onTriggered: Config.saveLayout(dashboard.serialize())
    }

    Shortcut {
        sequence: "Escape"
        enabled: header.editing
        onActivated: header.editing = false
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
        if (!Theme.load(prefs.theme))
            Theme.load("aurora")
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
