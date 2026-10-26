import QtQuick
import PluSystemMonitor

// Material 3 segmented buttons as end4 draws its tab groups: a pill track,
// and the choice a filled pill that slides to it with a little overshoot.
// A segment's hover and press tint lies under the pill, so the pill slides
// over it. Like the switch, it reports choices and leaves `currentIndex` to
// its owner.
Item {
    id: root

    property list<Segment> segments
    property int currentIndex: -1
    property var scheme: Appearance.colors
    property color trackColor: scheme.surfaceContainerHigh
    property color indicatorColor: scheme.secondaryContainer
    property color contentColor: scheme.onSurfaceVariant
    property color selectedContentColor: scheme.onSecondaryContainer
    property real segmentHeight: 36
    property real horizontalPadding: 14
    property real iconSize: 20
    readonly property real inset: 4

    signal activated(int index)

    implicitWidth: row.implicitWidth + 2 * inset
    implicitHeight: segmentHeight + 2 * inset

    // Its first placement jumps straight there; later choices slide.
    property Item selectedSegment: null
    property bool built: false
    property bool sliding: false
    Component.onCompleted: {
        selectedSegment = currentIndex >= 0 ? repeater.itemAt(currentIndex) : null
        Qt.callLater(() => root.built = true)
    }
    onCurrentIndexChanged: {
        sliding = built && currentIndex >= 0
        selectedSegment = currentIndex >= 0 ? repeater.itemAt(currentIndex) : null
    }

    Rectangle {
        anchors.fill: parent
        radius: height / 2
        color: root.trackColor
    }

    Repeater {
        model: repeater.count

        delegate: Rectangle {
            id: tint

            required property int index
            readonly property RippleButton segment: repeater.count > index ? repeater.itemAt(index) as RippleButton : null

            x: segment ? row.x + segment.x : 0
            y: root.inset
            width: segment ? segment.width : 0
            height: root.segmentHeight
            radius: height / 2
            color: segment && segment.hovered ? Appearance.fadeable(root.scheme.layer1Hover, "transparent")
                                              : Appearance.fadeable("transparent", root.scheme.layer1Hover)

            Behavior on color {
                ElementMoveFastColor {}
            }

            Ripple {
                id: ripple
                anchors.fill: parent
                cornerRadius: tint.radius
                color: root.scheme.layer1Active
            }

            Connections {
                target: tint.segment
                function onPressed() { ripple.start(tint.segment.pressX, tint.segment.pressY) }
                function onReleased() { ripple.release() }
                function onCanceled() { ripple.release() }
            }
        }
    }

    Rectangle {
        readonly property Item target: root.selectedSegment

        visible: target !== null
        x: target ? row.x + target.x : 0
        y: root.inset
        width: target ? target.width : 0
        height: root.segmentHeight
        radius: height / 2
        color: root.indicatorColor

        Behavior on x {
            enabled: root.sliding
            ElementMove {}
        }
        Behavior on width {
            enabled: root.sliding
            ElementMove {}
        }
    }

    Row {
        id: row
        x: root.inset
        y: root.inset

        Repeater {
            id: repeater
            model: root.segments
            onItemAdded: (index, item) => {
                if (index === root.currentIndex)
                    root.selectedSegment = item
            }

            delegate: RippleButton {
                id: segment

                required property int index
                required property Segment modelData
                readonly property bool selected: index === root.currentIndex
                readonly property color foreground: selected ? root.selectedContentColor : root.contentColor

                height: root.segmentHeight
                leftPadding: root.horizontalPadding
                rightPadding: root.horizontalPadding
                topPadding: 0
                bottomPadding: 0
                scheme: root.scheme
                enabled: modelData.enabled
                buttonRadius: height / 2
                colBackground: "transparent"
                // Its tint and ripple are drawn under the pill.
                colBackgroundHover: "transparent"
                rippleEnabled: false
                tooltip: modelData.tooltip
                onClicked: root.activated(index)

                contentItem: Item {
                    implicitWidth: content.implicitWidth
                    implicitHeight: content.implicitHeight

                    Row {
                        id: content
                        anchors.centerIn: parent
                        spacing: 6

                        MaterialSymbol {
                            anchors.verticalCenter: parent.verticalCenter
                            visible: segment.modelData.symbol.length > 0
                            text: segment.modelData.symbol
                            iconSize: root.iconSize
                            fill: segment.selected ? 1 : 0
                            color: segment.foreground

                            Behavior on color {
                                ElementMoveFastColor {}
                            }
                        }
                        StyledText {
                            anchors.verticalCenter: parent.verticalCenter
                            visible: segment.modelData.text.length > 0
                            text: segment.modelData.text
                            font.pixelSize: Appearance.fontSize.smallie
                            font.weight: segment.selected ? Font.DemiBold : Font.Medium
                            color: segment.foreground

                            Behavior on color {
                                ElementMoveFastColor {}
                            }
                        }
                    }
                }
            }
        }
    }
}
