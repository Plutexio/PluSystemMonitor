import QtQuick
import PluSystemMonitor

// One app in the Apps grid, as a media card: it lifts and leans toward the
// pointer on a spring, and gives under a press. Games get their Steam
// artwork and logo.
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

        // How far it leans toward the pointer, in degrees.
        property real tiltX: hover.hovered ? (hover.point.position.x / width - 0.5) * 12 : 0
        property real tiltY: hover.hovered ? (0.5 - hover.point.position.y / height) * 10 : 0

        anchors.fill: parent
        anchors.margins: Appearance.sizes.shelfGap / 2
        scale: tap.pressed ? 0.95 : hover.hovered ? 1.03 : 1
        opacity: tap.pressed ? 0.86 : 1

        Behavior on scale {
            enabled: !Appearance.performanceMode
            Bounce {}
        }
        Behavior on opacity {
            NumberAnimation { duration: Appearance.duration(120) }
        }
        Behavior on tiltX {
            enabled: !Appearance.performanceMode
            SpringAnimation { spring: 4; damping: 0.3; epsilon: 0.01 }
        }
        Behavior on tiltY {
            enabled: !Appearance.performanceMode
            SpringAnimation { spring: 4; damping: 0.3; epsilon: 0.01 }
        }

        transform: [
            Rotation {
                origin.x: body.width / 2
                origin.y: body.height / 2
                axis { x: 0; y: 1; z: 0 }
                angle: body.tiltX
            },
            Rotation {
                origin.x: body.width / 2
                origin.y: body.height / 2
                axis { x: 1; y: 0; z: 0 }
                angle: body.tiltY
            }
        ]

        Rectangle {
            anchors.fill: parent
            radius: Appearance.rounding.large
            color: card.selected ? Appearance.colors.secondaryContainer : Appearance.colors.layer2

            Behavior on color {
                ElementMoveFastColor {}
            }
        }

        // Artwork fading into the card, as on a store page.
        RoundedImage {
            id: artwork

            width: parent.width
            height: parent.height * 0.62
            radius: Appearance.rounding.large
            topOnly: true
            source: card.art
            opacity: 0.8
        }
        Rectangle {
            visible: artwork.visible
            width: artwork.width
            height: artwork.height + 1
            gradient: Gradient {
                GradientStop { position: 0.0; color: Qt.alpha(Appearance.colors.layer2, 0.0) }
                GradientStop { position: 0.55; color: Qt.alpha(Appearance.colors.layer2, 0.35) }
                GradientStop { position: 1.0; color: card.selected ? Appearance.colors.secondaryContainer : Appearance.colors.layer2 }
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
            radius: Appearance.rounding.large
            color: "transparent"
            border.width: card.selected ? 2 : 0
            border.color: Appearance.colors.primary
        }

        Item {
            anchors.fill: parent
            anchors.margins: 18

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

                    StyledText {
                        width: parent.width
                        text: card.name
                        color: card.selected ? Appearance.colors.onSecondaryContainer : Appearance.colors.onSurface
                        font.weight: Font.DemiBold
                        elide: Text.ElideRight
                    }
                    Row {
                        spacing: 8

                        Rectangle {
                            visible: card.game
                            anchors.verticalCenter: parent.verticalCenter
                            width: gameLabel.implicitWidth + 16
                            height: 20
                            radius: height / 2
                            color: Appearance.colors.tertiaryContainer

                            StyledText {
                                id: gameLabel
                                anchors.centerIn: parent
                                text: "Game"
                                color: Appearance.colors.onTertiaryContainer
                                font.pixelSize: Appearance.fontSize.smaller
                                font.weight: Font.DemiBold
                            }
                        }
                        StyledText {
                            text: card.processCount === 1 ? "1 process" : card.processCount + " processes"
                            color: Appearance.colors.onSurfaceVariant
                            font.pixelSize: Appearance.fontSize.smaller
                        }
                    }
                }
            }

            Row {
                id: numbers

                anchors.bottom: graph.top
                anchors.bottomMargin: 6
                spacing: 18

                TitleText {
                    text: card.cpu.toFixed(1) + "%"
                    color: Appearance.colors.cpu
                    font.pixelSize: 20
                    tabularNumbers: true
                }
                StyledText {
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: 2
                    // Many apps hold some VRAM; only actual GPU work is worth a number here.
                    visible: card.gpu >= 0.5
                    text: "GPU " + card.gpu.toFixed(0) + "%"
                    color: Appearance.colors.gpu
                    font.weight: Font.DemiBold
                    tabularNumbers: true
                }
                StyledText {
                    anchors.bottom: parent.bottom
                    anchors.bottomMargin: 2
                    text: Format.bytes(card.memory)
                    color: Appearance.colors.memory
                    font.weight: Font.DemiBold
                    tabularNumbers: true
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
                color: Appearance.colors.cpu
            }
        }

        HoverHandler {
            id: hover
            cursorShape: Qt.PointingHandCursor
        }
        TapHandler {
            id: tap
            onTapped: card.clicked()
        }
    }
}
