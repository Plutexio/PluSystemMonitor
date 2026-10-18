import QtQuick
import PluSystemMonitor

// One source as a labelled bar: "RAM   11.4 GiB / 31.2 GiB". Without a scale
// it is just the label and the value.
Column {
    id: root

    /// An object from `Sources.read`.
    property var source
    property color color: Theme.memory

    readonly property bool scaled: source && source.max > 0

    spacing: 7

    Item {
        width: root.width
        height: valueText.height

        Text {
            anchors.left: parent.left
            anchors.right: valueText.left
            anchors.rightMargin: 8
            text: root.source ? root.source.name : ""
            color: Theme.textDim
            font.pixelSize: 13
            elide: Text.ElideRight
        }
        Text {
            id: valueText
            anchors.right: parent.right
            text: {
                const s = root.source
                if (!s || !s.available)
                    return "—"
                // "used / total" reads well for amounts, not for rates or temperatures.
                return s.unit === "bytes" && s.max > 0 ? s.text + " / " + Format.metric(s.max, s.unit) : s.text
            }
            color: Theme.text
            font.pixelSize: 13
            font.weight: Font.DemiBold
            font.features: { "tnum": 1 }
        }
    }

    Rectangle {
        visible: root.scaled
        width: root.width
        height: 8
        radius: 4
        color: Theme.track

        Rectangle {
            width: Math.max(height, parent.width * (root.source ? root.source.fraction : 0))
            height: parent.height
            radius: parent.radius
            visible: root.source && root.source.fraction > 0
            gradient: Gradient {
                orientation: Gradient.Horizontal
                GradientStop { position: 0.0; color: Qt.darker(root.color, 1.4) }
                GradientStop { position: 1.0; color: root.color }
            }

            Behavior on width {
                NumberAnimation { duration: 650; easing.type: Easing.OutCubic }
            }
        }
    }
}
