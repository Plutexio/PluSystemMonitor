import QtQuick
import PluSystemMonitor

// PluFan's window: the same frame, drawer and dialogs as the monitor
// (AppWindow), with the fans and their curves on the sheet.
AppWindow {
    id: window

    readonly property var pageInfo: [
        { title: qsTr("Fans"), key: "fans" },
        { title: qsTr("Curves"), key: "curves" }
    ]
    readonly property int shownPage: Math.min(page, pageInfo.length - 1)

    appName: "PluFan"
    snackbar: messages

    /// Makes a curve for a fan that has none to pick, following the
    /// temperature that most likely matters to it, and shows it.
    function newCurveFor(fanId) {
        const catalog = Sources.catalog(window.monitor)
        const gpuFan = fanId.startsWith("nvidia.") || (store.outputs.find(o => o.id === fanId)?.device ?? "").match(/GPU|RTX|GTX|RX /)
        const wanted = gpuFan ? ["gpu.0.temperature", "cpu.temperature"] : ["cpu.temperature", "gpu.0.temperature"]
        const source = wanted.map(id => catalog.find(s => s.id === id)).find(s => s)
                    ?? catalog.find(s => s.unit === "celsius")
        const name = source ? source.name.replace(/ temperature$/, "") : qsTr("Curve")
        const id = store.addCurve(name, source ? [source.id] : [])
        store.editFan(fanId, fan => {
            fan.mode = "curve"
            fan.curve = id
        })
        curvesPage.selectedId = id
        window.page = 1
    }

    FanStore {
        id: store
        monitor: window.monitor
        onFailed: message => messages.show(message)
    }

    Sidebar {
        id: sidebar

        x: window.gutter
        y: 0
        width: preferredWidth
        height: parent.height
        window: window
        monitor: window.monitor
        page: window.shownPage
        pages: [
            { name: qsTr("Fans"), symbol: "mode_fan", shape: "clover4" },
            { name: qsTr("Curves"), symbol: "show_chart", shape: "sunny" }
        ]
        pagesTitle: qsTr("Fan Control")
        subtitle: qsTr("Fan Control")
        badgeSymbol: "mode_fan"
        showSensors: false
        preferredWidth: window.prefs.sidebar_width ?? Appearance.sizes.sidebarWidth
        onPageRequested: index => window.page = index
        updatesAvailable: window.updatesAvailable
        onConnectionRequested: window.connectionDialog.open()
        onAppearanceRequested: window.appearanceDialog.open()
        onUpdatesRequested: window.updatesDialog.open()
        onWidthEdited: width => window.setPref("sidebar_width", width)
    }

    // The content sheet.
    Rectangle {
        id: sheet

        x: sidebar.x + sidebar.width + window.gutter
        y: window.gutter
        width: parent.width - x - window.gutter
        height: parent.height - 2 * window.gutter
        radius: Appearance.rounding.large
        color: Appearance.colors.layer1
        clip: true

        Item {
            width: parent.width
            height: Appearance.sizes.topBarHeight
            clip: true

            Stickers {
                anchors.fill: parent
                key: window.pageInfo[window.shownPage].key
                count: 4
                minimumSize: 50
                maximumSize: 110
                fromY: -0.5
                toY: 0
                strength: 0.8
            }
        }

        FansPage {
            anchors.fill: parent
            anchors.topMargin: Appearance.sizes.topBarHeight
            store: store
            monitor: window.monitor
            visible: opacity > 0
            opacity: window.shownPage === 0 ? 1 : 0
            onNewCurveRequested: fanId => window.newCurveFor(fanId)

            Behavior on opacity {
                NumberAnimation { duration: Appearance.duration(220) }
            }
        }

        CurvesPage {
            id: curvesPage

            anchors.fill: parent
            anchors.topMargin: Appearance.sizes.topBarHeight
            store: store
            monitor: window.monitor
            virtualSensors: window.virtualSensors
            visible: opacity > 0
            opacity: window.shownPage === 1 ? 1 : 0
            onFailed: message => messages.show(message)

            Behavior on opacity {
                NumberAnimation { duration: Appearance.duration(220) }
            }
        }

        TopBar {
            width: parent.width
            height: Appearance.sizes.topBarHeight
            window: window
            monitor: window.monitor
            title: window.pageInfo[window.shownPage].title
            page: -1
        }

        Snackbar {
            id: messages
            anchors.horizontalCenter: parent.horizontalCenter
            anchors.bottom: parent.bottom
            anchors.bottomMargin: 16
            z: 50
        }

        CornerCaps {
            anchors.fill: parent
            radius: sheet.radius
            color: Appearance.colors.layer0
            z: 60
        }
    }
}
