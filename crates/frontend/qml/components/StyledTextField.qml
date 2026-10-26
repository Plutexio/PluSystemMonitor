import QtQuick
import QtQuick.Templates as T
import PluSystemMonitor

// A Material outlined text field: a hairline outline that turns into a
// 2 px primary one while typed in, its label above the text.
T.TextField {
    id: root

    property string label

    implicitWidth: 280
    implicitHeight: 52
    leftPadding: 16
    rightPadding: 16
    topPadding: label.length > 0 ? 22 : 14
    bottomPadding: label.length > 0 ? 8 : 14
    color: Appearance.colors.onSurface
    placeholderTextColor: Appearance.colors.onSurfaceVariant
    selectionColor: Appearance.colors.primary
    selectedTextColor: Appearance.colors.onPrimary
    selectByMouse: true
    font.family: Appearance.fontFamily
    font.pixelSize: Appearance.fontSize.small
    verticalAlignment: TextInput.AlignVCenter

    background: Rectangle {
        radius: Appearance.rounding.small
        color: root.hovered && !root.activeFocus ? Appearance.colors.layer2Hover : Appearance.colors.layer2
        border.width: root.activeFocus ? 2 : 1
        border.color: root.activeFocus ? Appearance.colors.primary : Appearance.colors.outlineVariant

        Behavior on color {
            ElementMoveFastColor {}
        }
        Behavior on border.color {
            ElementMoveFastColor {}
        }

        // The template draws no placeholder of its own.
        StyledText {
            x: root.leftPadding
            y: root.topPadding
            width: parent.width - root.leftPadding - root.rightPadding
            height: parent.height - root.topPadding - root.bottomPadding
            visible: root.text.length === 0 && !root.preeditText
            text: root.placeholderText
            color: root.placeholderTextColor
            elide: Text.ElideRight
        }

        StyledText {
            x: root.leftPadding
            y: 7
            visible: root.label.length > 0
            text: root.label
            font.pixelSize: Appearance.fontSize.smaller
            font.weight: Font.Medium
            color: root.activeFocus ? Appearance.colors.primary : Appearance.colors.onSurfaceVariant
        }
    }
}
