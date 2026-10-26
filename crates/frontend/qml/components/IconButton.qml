import QtQuick
import PluSystemMonitor

// A Material 3 icon button. The variant picks its container; a toggled
// button fills its glyph and takes the selected container's colours.
RippleButton {
    id: root

    enum Variant {
        Standard,
        Filled,
        Tonal,
        Outlined
    }

    property int variant: IconButton.Standard
    property string symbol
    property real iconSize: 24
    property real size: 40
    property bool fillWhenToggled: true
    property bool alwaysFilled: false
    /// Toggle buttons show an unselected container until toggled.
    property bool toggleable: false
    property color iconColor: {
        switch (variant) {
        case IconButton.Filled:
            return toggled || !toggleable ? scheme.onPrimary : scheme.primary
        case IconButton.Tonal:
            return toggled || !toggleable ? scheme.onSecondaryContainer : scheme.onSurfaceVariant
        default:
            return toggled ? scheme.primary : scheme.onSurfaceVariant
        }
    }

    implicitWidth: size
    implicitHeight: size
    padding: 0
    buttonRadius: Appearance.rounding.full
    colBackground: {
        switch (variant) {
        case IconButton.Filled:
            return toggled || !toggleable ? scheme.primary : scheme.surfaceContainerHighest
        case IconButton.Tonal:
            return toggled || !toggleable ? scheme.secondaryContainer : scheme.surfaceContainerHighest
        default:
            return "transparent"
        }
    }
    colBackgroundHover: {
        switch (variant) {
        case IconButton.Filled:
            return toggled || !toggleable ? scheme.primaryHover : scheme.surfaceContainerHighestHover
        case IconButton.Tonal:
            return toggled || !toggleable ? scheme.secondaryContainerHover : scheme.surfaceContainerHighestHover
        default:
            return scheme.layer1Hover
        }
    }
    colBackgroundToggled: variant === IconButton.Standard || variant === IconButton.Outlined ? "transparent" : colBackground
    colBackgroundToggledHover: variant === IconButton.Standard || variant === IconButton.Outlined ? scheme.layer1Hover : colBackgroundHover
    colRipple: variant === IconButton.Filled ? scheme.primaryActive : scheme.layer1Active
    colRippleToggled: colRipple
    backgroundItem.border.width: variant === IconButton.Outlined && !toggled ? 1 : (visualFocus ? 2 : 0)
    backgroundItem.border.color: variant === IconButton.Outlined && !visualFocus ? scheme.outlineVariant : scheme.secondary

    contentItem: Item {
        MaterialSymbol {
            anchors.centerIn: parent
            text: root.symbol
            iconSize: root.iconSize
            fill: root.alwaysFilled || (root.toggled && root.fillWhenToggled) ? 1 : 0
            color: root.iconColor

            Behavior on color {
                ElementMoveFastColor {}
            }
        }
    }
}
