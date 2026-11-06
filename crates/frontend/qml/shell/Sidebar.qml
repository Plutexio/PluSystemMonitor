import QtQuick
import PluSystemMonitor

// The navigation drawer, straight on the window's layer0 with no background
// of its own: the machine at the top, the pages, and the settings at the
// bottom with the connection's state. Its right edge drags it wider or
// narrower.
Item {
    id: root

    required property Window window
    required property Monitor monitor
    property int page: 0
    property real preferredWidth: Appearance.sizes.sidebarWidth
    /// Machines with an update waiting, for the badge.
    property int updatesAvailable: 0
    /// What the app is, under the machine's name, and its symbol.
    property string subtitle: qsTr("System Monitor")
    property string badgeSymbol: "monitor_heart"
    property string pagesTitle: qsTr("Monitor")
    /// The fan app has no sensor settings of its own to offer.
    property bool showSensors: true

    signal pageRequested(int index)
    signal connectionRequested()
    signal appearanceRequested()
    signal sensorsRequested()
    signal updatesRequested()
    signal widthEdited(real width)

    property var pages: [
        { name: qsTr("Overview"), symbol: "dashboard", shape: "cookie9" },
        { name: qsTr("Apps"), symbol: "apps", shape: "flower" },
        { name: qsTr("Processes"), symbol: "view_list", shape: "clover4" }
    ]

    // The window's top edge moves it, here as on the top bar.
    Item {
        id: header

        width: parent.width
        height: Appearance.sizes.topBarHeight

        DragHandler {
            target: null
            onActiveChanged: if (active) root.window.startSystemMove()
        }
        TapHandler {
            onDoubleTapped: root.window.visibility === Window.Maximized ? root.window.showNormal() : root.window.showMaximized()
        }
        HoverHandler {
            id: headerHover
        }

        ShapeIcon {
            id: badge
            x: 12
            anchors.verticalCenter: parent.verticalCenter
            size: 38
            iconSize: 20
            shape: "cookie12"
            symbol: root.badgeSymbol
            color: Appearance.colors.primaryContainer
            iconColor: Appearance.colors.onPrimaryContainer
            hovered: headerHover.hovered
        }

        Column {
            anchors.left: badge.right
            anchors.leftMargin: 12
            anchors.right: parent.right
            anchors.rightMargin: 8
            anchors.verticalCenter: parent.verticalCenter
            spacing: 1

            TitleText {
                width: parent.width
                text: root.monitor.hostname || "PluSystemMonitor"
                font.pixelSize: Appearance.fontSize.larger
                elide: Text.ElideRight
                animateChange: true
            }
            StyledText {
                width: parent.width
                text: root.monitor.connected ? root.subtitle : qsTr("Not connected")
                font.pixelSize: Appearance.fontSize.smaller
                color: Appearance.colors.onSurfaceVariant
                elide: Text.ElideRight
            }
        }
    }

    Column {
        id: navigation

        anchors.top: header.bottom
        anchors.topMargin: 8
        width: parent.width
        spacing: 2

        SectionLabel {
            width: parent.width
            text: root.pagesTitle
        }

        Repeater {
            model: root.pages

            NavigationItem {
                required property int index
                required property var modelData

                width: navigation.width
                text: modelData.name
                symbol: modelData.symbol
                iconShape: modelData.shape
                selected: root.page === index
                onClicked: root.pageRequested(index)
            }
        }
    }

    Column {
        id: settings

        anchors.bottom: parent.bottom
        anchors.bottomMargin: 8
        width: parent.width
        spacing: 2

        SectionLabel {
            width: parent.width
            text: qsTr("Settings")
        }

        NavigationItem {
            width: settings.width
            text: qsTr("Connection")
            symbol: "lan"
            iconShape: "clover8"
            onClicked: root.connectionRequested()

            trailing: Item {
                width: 22
                height: 22

                // A ring that spreads from the dot with every sample, while
                // data flows and nothing asks for stillness.
                MaterialShape {
                    id: pulse
                    anchors.centerIn: parent
                    width: 12
                    height: 12
                    shape: "cookie9"
                    color: Qt.alpha(dot.color, 0.5)
                    opacity: 0
                }
                MaterialShape {
                    id: dot
                    anchors.centerIn: parent
                    width: 12
                    height: 12
                    // Offline is the pixel circle, as no network is.
                    shape: root.monitor.connected ? "cookie9" : "pixelCircle"
                    keepsShape: true
                    color: root.monitor.connected ? Appearance.colors.good : Appearance.colors.error

                    Behavior on color {
                        ElementMoveFastColor {}
                    }
                }
                Connections {
                    target: root.monitor
                    function onSampleReceived() {
                        if (!Appearance.reduceMotion)
                            pulseAnimation.restart()
                    }
                }
                ParallelAnimation {
                    id: pulseAnimation
                    NumberAnimation { target: pulse; property: "scale"; from: 0.8; to: 2.2; duration: 900; easing.type: Easing.OutCubic }
                    NumberAnimation { target: pulse; property: "opacity"; from: 0.9; to: 0; duration: 900; easing.type: Easing.OutCubic }
                }
            }
        }

        NavigationItem {
            width: settings.width
            visible: root.showSensors
            text: qsTr("Sensors")
            symbol: "sensors"
            iconShape: "clover4"
            onClicked: root.sensorsRequested()
        }

        NavigationItem {
            width: settings.width
            text: qsTr("Updates")
            symbol: "system_update"
            iconShape: "cookie9"
            onClicked: root.updatesRequested()

            trailing: Rectangle {
                id: updateBadge
                visible: root.updatesAvailable > 0
                width: Math.max(20, count.implicitWidth + 10)
                height: 20
                radius: 10
                color: Appearance.colors.primary

                StyledText {
                    id: count
                    anchors.centerIn: parent
                    text: root.updatesAvailable
                    color: Appearance.colors.onPrimary
                    font.pixelSize: Appearance.fontSize.smaller
                    font.weight: Font.DemiBold
                }
            }
        }

        NavigationItem {
            width: settings.width
            text: qsTr("Appearance")
            symbol: "palette"
            iconShape: "flower"
            onClicked: root.appearanceRequested()
        }
    }

    // Dragging the right edge sets the drawer's width.
    Item {
        x: parent.width - 4
        width: 8
        height: parent.height

        HoverHandler {
            cursorShape: Qt.SizeHorCursor
        }
        DragHandler {
            id: resize

            property real startWidth

            target: null
            xAxis.enabled: true
            yAxis.enabled: false
            onActiveChanged: {
                if (active)
                    startWidth = root.width
                else
                    root.widthEdited(root.preferredWidth)
            }
            onTranslationChanged: {
                if (active) {
                    const max = Math.min(Appearance.sizes.sidebarMaxWidth, root.window.width * 0.28)
                    root.preferredWidth = Math.max(Appearance.sizes.sidebarMinWidth, Math.min(max, startWidth + translation.x))
                }
            }
        }
    }
}
