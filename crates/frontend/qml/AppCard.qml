import QtQuick
import PluSystemMonitor

// One app in the Apps grid. Games get their Steam artwork and logo.
Item {
    id: card

    required property int index
    required property string appId
    required property string name
    required property bool game
    required property string icon
    required property string art
    required property string logo
    required property real cpu
    required property real memory
    required property int processCount
    required property list<real> history
    required property real gpu
    required property real gpuMemory

    property bool selected: false
    property int interval: 1000

    signal clicked()

    readonly property bool hasArt: card.art.length > 0
    readonly property real peak: {
        let peak = 0
        for (let i = 0; i < history.length; i++)
            peak = Math.max(peak, history[i])
        return peak
    }

    Item {
        id: body

        anchors.fill: parent
        anchors.margins: Theme.spacing / 2
        scale: hover.hovered ? 1.015 : 1

        Behavior on scale {
            NumberAnimation { duration: 180; easing.type: Easing.OutCubic }
        }

        Rectangle {
            anchors.fill: parent
            radius: Theme.radius
            color: Theme.card
        }

        // Artwork fading into the card, as on a store page.
        RoundedImage {
            id: artwork

            width: parent.width
            height: parent.height * 0.62
            radius: Theme.radius
            topOnly: true
            source: card.art
            opacity: 0.75
        }
        Rectangle {
            visible: artwork.visible
            width: artwork.width
            height: artwork.height + 1
            gradient: Gradient {
                GradientStop { position: 0.0; color: Qt.alpha(Theme.background, 0.0) }
                GradientStop { position: 0.55; color: Qt.alpha(Theme.background, 0.35) }
                GradientStop { position: 1.0; color: Qt.alpha(Theme.background, 0.92) }
            }
        }

        Image {
            visible: card.logo.length > 0 && status === Image.Ready
            anchors.horizontalCenter: parent.horizontalCenter
            y: artwork.height * 0.12
            width: parent.width * 0.62
            height: artwork.height * 0.5
            source: card.logo
            fillMode: Image.PreserveAspectFit
            asynchronous: true
            mipmap: true
        }

        Rectangle {
            anchors.fill: parent
            radius: Theme.radius
            color: "transparent"
            border.width: card.selected ? 2 : 1
            border.color: card.selected ? Theme.primary : hover.hovered ? Qt.alpha(Theme.primary, 0.5) : Theme.cardBorder

            Behavior on border.color {
                ColorAnimation { duration: 160 }
            }
        }

        Item {
            anchors.fill: parent
            anchors.margins: Theme.padding - 4

            Row {
                id: title

                // Games put their art first and the title under it.
                y: card.hasArt ? parent.height * 0.42 : 0
                width: parent.width
                spacing: 12

                AppIcon {
                    id: appIcon
                    source: card.icon
                    name: card.name
                    size: card.hasArt ? 34 : 42
                }
                Column {
                    width: parent.width - appIcon.width - parent.spacing
                    anchors.verticalCenter: parent.verticalCenter
                    spacing: 3

                    Text {
                        width: parent.width
                        text: card.name
                        color: Theme.text
                        font.pixelSize: 15
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                    }
                    Row {
                        spacing: 8

                        Rectangle {
                            visible: card.game
                            anchors.verticalCenter: parent.verticalCenter
                            width: gameLabel.implicitWidth + 12
                            height: 18
                            radius: 9
                            color: Qt.alpha(Theme.gpu, 0.2)

                            Text {
                                id: gameLabel
                                anchors.centerIn: parent
                                text: "GAME"
                                color: Theme.gpu
                                font.pixelSize: 10
                                font.weight: Font.Bold
                                font.letterSpacing: 1
                            }
                        }
                        Text {
                            text: card.processCount === 1 ? "1 process" : card.processCount + " processes"
                            color: Theme.textDim
                            font.pixelSize: 12
                        }
                    }
                }
            }

            Row {
                id: numbers

                anchors.bottom: graph.top
                anchors.bottomMargin: 6
                spacing: 18

                Text {
                    text: card.cpu.toFixed(1) + "%"
                    color: Theme.cpu
                    font.pixelSize: 20
                    font.weight: Font.Bold
                    font.features: { "tnum": 1 }
                }
                Text {
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: 2
                    // Many apps hold some VRAM; only actual GPU work is worth a number here.
                    visible: card.gpu >= 0.5
                    text: "GPU " + card.gpu.toFixed(0) + "%"
                    color: Theme.gpu
                    font.pixelSize: 15
                    font.weight: Font.DemiBold
                    font.features: { "tnum": 1 }
                }
                Text {
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: 2
                    text: Format.bytes(card.memory)
                    color: Theme.memory
                    font.pixelSize: 15
                    font.weight: Font.DemiBold
                    font.features: { "tnum": 1 }
                }
            }

            LiveGraph {
                id: graph

                anchors.bottom: parent.bottom
                width: parent.width
                height: card.hasArt ? 26 : Math.max(26, parent.height - title.height - numbers.height - 26)
                values: card.history
                maxPoints: 120
                // Scale to the app's own peak so quiet apps still show their rhythm.
                maxValue: Math.max(2, card.peak) * 1.15
                interval: card.interval
                color: Theme.cpu
            }
        }

        HoverHandler {
            id: hover
            cursorShape: Qt.PointingHandCursor
        }
        TapHandler {
            onTapped: card.clicked()
        }
    }
}
