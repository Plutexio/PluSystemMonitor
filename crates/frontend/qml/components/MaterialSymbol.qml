import QtQuick
import PluSystemMonitor

// A Material Symbols Rounded glyph by name. Outlined at rest, filled
// (`fill: 1`) when toggled or chosen; `weight` thickens it under the pointer.
Text {
    id: root

    property real iconSize: 24
    property real fill: 0
    property int weight: 400

    font.family: Appearance.iconFontFamily
    font.pixelSize: iconSize
    font.variableAxes: ({ "FILL": fill, "wght": weight, "GRAD": 0, "opsz": Math.max(20, Math.min(48, iconSize)) })
    color: Appearance.colors.onSurfaceVariant
    horizontalAlignment: Text.AlignHCenter
    verticalAlignment: Text.AlignVCenter
    textFormat: Text.PlainText

    Behavior on fill {
        enabled: !Appearance.performanceMode
        NumberAnimation { duration: 180; easing.type: Easing.OutCubic }
    }
    Behavior on weight {
        enabled: !Appearance.performanceMode
        NumberAnimation { duration: 180; easing.type: Easing.OutCubic }
    }
}
