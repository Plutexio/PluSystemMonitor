import QtQuick
import PluSystemMonitor

// A Material 3 common button, a pill: filled, tonal, outlined, text or
// elevated, plus filled and tonal error ones for what can't be undone. An
// optional leading symbol; `iconOnly` shrinks it to a circle with the label
// as its tooltip.
RippleButton {
    id: root

    enum Variant {
        Filled,
        Tonal,
        Outlined,
        Text,
        Elevated,
        Danger,
        DangerTonal
    }

    property int variant: TextButton.Filled
    property string symbol
    property bool iconOnly: false
    property real compactSize: 40
    readonly property real naturalWidth: label.implicitWidth + (symbol.length > 0 ? 26 + 16 : 24) + 24
    property color labelColor: {
        switch (variant) {
        case TextButton.Filled: return scheme.onPrimary
        case TextButton.Tonal: return scheme.onSecondaryContainer
        case TextButton.Danger: return scheme.onError
        case TextButton.DangerTonal: return scheme.onErrorContainer
        default: return scheme.primary
        }
    }

    implicitWidth: iconOnly ? compactSize : naturalWidth
    implicitHeight: iconOnly ? compactSize : 40
    leftPadding: iconOnly ? 0 : (symbol.length > 0 ? 16 : 24)
    rightPadding: iconOnly ? 0 : 24
    buttonRadius: Appearance.rounding.full
    tooltip: iconOnly ? text : ""
    colBackground: {
        switch (variant) {
        case TextButton.Filled: return scheme.primary
        case TextButton.Tonal: return scheme.secondaryContainer
        case TextButton.Elevated: return scheme.surfaceContainerLow
        case TextButton.Danger: return scheme.error
        case TextButton.DangerTonal: return scheme.errorContainer
        default: return "transparent"
        }
    }
    colBackgroundHover: {
        switch (variant) {
        case TextButton.Filled: return scheme.primaryHover
        case TextButton.Tonal: return scheme.secondaryContainerHover
        case TextButton.Elevated: return scheme.layer1Hover
        case TextButton.Danger: return scheme.errorHover
        case TextButton.DangerTonal: return scheme.errorContainerHover
        default: return Qt.alpha(scheme.primary, 0.08)
        }
    }
    colRipple: {
        switch (variant) {
        case TextButton.Filled: return scheme.primaryActive
        case TextButton.Tonal: return scheme.secondaryContainerActive
        case TextButton.Danger: return scheme.errorActive
        case TextButton.DangerTonal: return scheme.errorContainerActive
        default: return Qt.alpha(scheme.primary, 0.14)
        }
    }
    backgroundItem.border.width: variant === TextButton.Outlined ? 1 : (visualFocus ? 2 : 0)
    backgroundItem.border.color: variant === TextButton.Outlined && !visualFocus ? scheme.outlineVariant : scheme.secondary

    contentItem: Item {
        implicitWidth: row.implicitWidth
        implicitHeight: row.implicitHeight

        Row {
            id: row
            anchors.centerIn: parent
            spacing: 8

            MaterialSymbol {
                anchors.verticalCenter: parent.verticalCenter
                visible: root.symbol.length > 0
                text: root.symbol
                iconSize: root.iconOnly ? Math.min(22, root.height * 0.55) : 18
                fill: 1
                color: root.labelColor

                Behavior on color {
                    ElementMoveFastColor {}
                }
            }

            StyledText {
                id: label
                anchors.verticalCenter: parent.verticalCenter
                visible: !root.iconOnly
                text: root.text
                font.pixelSize: Appearance.fontSize.small
                font.weight: Font.DemiBold
                color: root.labelColor

                Behavior on color {
                    ElementMoveFastColor {}
                }
            }
        }
    }
}
