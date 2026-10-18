import QtCore
import QtQuick
import QtQuick.Layouts
import PluSystemMonitor

Window {
    id: window

    width: 1180
    height: 760
    minimumWidth: 820
    minimumHeight: 560
    visible: true
    color: Theme.background
    title: monitor.hostname ? monitor.hostname + " — PluSystemMonitor" : "PluSystemMonitor"

    Monitor {
        id: monitor
    }

    Settings {
        id: settings
        property string address: "127.0.0.1"
        property string token: ""
    }

    Background {
        anchors.fill: parent
        running: window.visibility !== Window.Minimized && window.visibility !== Window.Hidden
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 28
        spacing: Theme.spacing + 8

        ConnectionBar {
            Layout.fillWidth: true
            monitor: monitor
            address: settings.address
            token: settings.token
            onConnectRequested: (address, token) => {
                settings.address = address
                settings.token = token
                monitor.connectTo(address, token)
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: Theme.spacing

            Card {
                Layout.preferredWidth: 300
                Layout.fillHeight: true
                title: "Processor"
                accent: Theme.cpu

                RingGauge {
                    anchors.fill: parent
                    value: monitor.cpuTotal
                    color: Theme.cpu
                    label: "usage"
                    caption: monitor.cpuCores.length + " threads"
                }
            }

            Card {
                Layout.fillWidth: true
                Layout.fillHeight: true
                title: "Processor history"
                subtitle: monitor.connected ? monitor.cpuTotal.toFixed(1) + "%" : ""
                accent: Theme.cpu

                LiveGraph {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.bottom: cores.top
                    anchors.bottomMargin: 18
                    values: monitor.cpuHistory
                    interval: monitor.intervalMs
                    color: Theme.cpu
                }

                // One bar per logical core.
                Row {
                    id: cores

                    readonly property int count: Math.max(1, monitor.cpuCores.length)
                    readonly property real barWidth: (width - spacing * (count - 1)) / count

                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    height: 44
                    spacing: 4

                    Repeater {
                        model: monitor.cpuCores

                        Rectangle {
                            required property real modelData

                            width: cores.barWidth
                            height: cores.height
                            radius: Math.min(4, width / 2)
                            color: Theme.track

                            Rectangle {
                                anchors.bottom: parent.bottom
                                width: parent.width
                                height: Math.max(2, parent.height * modelData / 100)
                                radius: parent.radius
                                color: Theme.cpu
                                opacity: 0.45 + 0.55 * modelData / 100

                                Behavior on height {
                                    NumberAnimation { duration: 500; easing.type: Easing.OutCubic }
                                }
                            }
                        }
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: Theme.spacing

            Card {
                Layout.preferredWidth: 300
                Layout.fillHeight: true
                title: "Memory"
                accent: Theme.mem

                RingGauge {
                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.top: parent.top
                    anchors.bottom: bars.top
                    anchors.bottomMargin: 12
                    value: monitor.memTotal > 0 ? monitor.memUsed / monitor.memTotal * 100 : 0
                    color: Theme.mem
                    label: "in use"
                }

                Column {
                    id: bars

                    anchors.left: parent.left
                    anchors.right: parent.right
                    anchors.bottom: parent.bottom
                    spacing: 14

                    UsageBar {
                        width: parent.width
                        label: "RAM"
                        used: monitor.memUsed
                        total: monitor.memTotal
                        color: Theme.mem
                    }
                    UsageBar {
                        width: parent.width
                        label: "Swap"
                        used: monitor.swapUsed
                        total: monitor.swapTotal
                        color: Theme.swap
                    }
                }
            }

            Card {
                Layout.fillWidth: true
                Layout.fillHeight: true
                title: "Memory history"
                subtitle: monitor.connected ? Theme.formatBytes(monitor.memUsed) : ""
                accent: Theme.mem

                LiveGraph {
                    anchors.fill: parent
                    values: monitor.memHistory
                    interval: monitor.intervalMs
                    color: Theme.mem
                }
            }
        }
    }

    Component.onCompleted: monitor.connectTo(settings.address, settings.token)

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
