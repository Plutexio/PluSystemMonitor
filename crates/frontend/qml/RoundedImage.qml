import QtQuick
import QtQuick.Effects

// An image cropped to fill, clipped to rounded corners (plain `clip` only
// does rectangles).
Item {
    id: root

    property alias source: image.source
    property real radius: 0
    /// Round only the top corners, for banners sitting on a card.
    property bool topOnly: false

    visible: image.status === Image.Ready

    Image {
        id: image

        anchors.fill: parent
        fillMode: Image.PreserveAspectCrop
        asynchronous: true
        visible: false
    }

    Item {
        id: mask

        anchors.fill: parent
        layer.enabled: true
        visible: false

        Rectangle {
            width: parent.width
            // Extend past the bottom so its rounded corners fall outside.
            height: root.topOnly ? parent.height + root.radius : parent.height
            radius: root.radius
        }
    }

    MultiEffect {
        anchors.fill: parent
        source: image
        maskEnabled: true
        maskSource: mask
        maskThresholdMin: 0.5
        maskSpreadAtMin: 1.0
    }
}
