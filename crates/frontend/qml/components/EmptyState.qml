import QtQuick
import PluSystemMonitor

// What a view shows with nothing in it: a symbol on a tonal shape picked for
// what it says, a round title and one sentence on what to do. The shape
// springs in with a twist each time it appears, among a few stickers.
Column {
    id: root

    property string symbol: "monitoring"
    property string shape: {
        switch (symbol) {
        case "search_off": return "ghostish"
        case "cloud_off":
        case "wifi_off":
        case "link_off": return "pixelCircle"
        case "apps": return "flower"
        case "data_usage": return "pentagon"
        default: return "cookie9"
        }
    }
    property string title
    property string description
    property var scheme: Appearance.colors

    spacing: 10
    width: Math.min(parent ? parent.width : 400, 420)

    onVisibleChanged: if (visible && !Appearance.performanceMode) arrive.restart()

    MaterialShape {
        id: badge
        anchors.horizontalCenter: parent.horizontalCenter
        width: 80
        height: 80
        shape: root.shape
        keepsShape: true
        color: root.scheme.secondaryContainer

        Stickers {
            anchors.centerIn: parent
            width: 190
            height: 120
            z: -1
            key: root.title
            count: 3
            minimumSize: 16
            maximumSize: 30
            strength: 1.8
            scheme: root.scheme
        }

        ParallelAnimation {
            id: arrive
            NumberAnimation { target: badge; property: "scale"; from: 0.4; to: 1; duration: 700; easing.type: Easing.OutBack; easing.overshoot: 2.2 }
            NumberAnimation { target: badge; property: "rotation"; from: -60; to: 0; duration: 900; easing.type: Easing.OutBack; easing.overshoot: 1.4 }
        }

        MaterialSymbol {
            anchors.centerIn: parent
            text: root.symbol
            iconSize: 34
            fill: 1
            color: root.scheme.onSecondaryContainer
        }
    }

    Item {
        width: 1
        height: 6
    }

    TitleText {
        width: parent.width
        horizontalAlignment: Text.AlignHCenter
        text: root.title
        font.pixelSize: Appearance.fontSize.larger
        color: root.scheme.onSurface
        wrapMode: Text.Wrap
    }

    StyledText {
        width: parent.width
        horizontalAlignment: Text.AlignHCenter
        visible: root.description.length > 0
        text: root.description
        color: root.scheme.onSurfaceVariant
        wrapMode: Text.Wrap
    }
}
