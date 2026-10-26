import QtQuick
import PluSystemMonitor

// One source as a labelled wavy bar: "RAM   11.4 GiB / 31.2 GiB". Without a
// scale it is just the label and the value.
Column {
    id: root

    /// An object from `Sources.read`.
    property var source
    property color color: Appearance.colors.memory

    readonly property bool scaled: source && source.max > 0

    spacing: 6

    Item {
        width: root.width
        height: valueText.height

        StyledText {
            anchors.left: parent.left
            anchors.right: valueText.left
            anchors.rightMargin: 8
            text: root.source ? root.source.name : ""
            color: Appearance.colors.onSurfaceVariant
            font.pixelSize: Appearance.fontSize.smallie
            elide: Text.ElideRight
        }
        StyledText {
            id: valueText
            anchors.right: parent.right
            text: {
                const s = root.source
                if (!s || !s.available)
                    return "—"
                // "used / total" reads well for amounts, not for rates or temperatures.
                return s.unit === "bytes" && s.max > 0 ? s.text + " / " + Format.metric(s.max, s.unit) : s.text
            }
            font.pixelSize: Appearance.fontSize.smallie
            font.weight: Font.DemiBold
            tabularNumbers: true
        }
    }

    WavyProgress {
        visible: root.scaled
        width: root.width
        value: root.source ? root.source.fraction : 0
        color: root.color
        trackColor: Appearance.colors.tonal(root.color)
        thickness: 5
    }
}
