import QtQuick
import QtQuick.Layouts
import QtQuick.Templates as T
import PluSystemMonitor

// Colours and motion: the theme (the desktop's Matugen scheme, the app's own
// accent, a rice palette or a file of yours), light or dark, and how much
// moves and how many shapes there are.
StyledDialog {
    id: root

    property var prefs: ({})
    property bool dark: true
    property var themes: []

    signal themeSelected(string id)
    signal modeSelected(string mode)
    signal preferenceEdited(string key, var value)

    readonly property var modes: ["auto", "light", "dark"]

    headline: qsTr("Appearance")
    symbol: "palette"
    iconShape: "flower"
    preferredWidth: 560

    onAboutToShow: themes = JSON.parse(Config.themes(dark))
    onDarkChanged: if (opened) themes = JSON.parse(Config.themes(dark))

    component SettingRow: RippleButton {
        id: setting

        property string label
        property string note
        property bool value

        width: parent.width
        height: 56
        buttonRadius: Appearance.rounding.small
        colBackgroundHover: Appearance.colors.layer3Hover
        colRipple: Appearance.colors.layer3Active
        bounce: false

        contentItem: Item {
            Column {
                anchors.left: parent.left
                anchors.leftMargin: 4
                anchors.right: toggle.left
                anchors.rightMargin: 12
                anchors.verticalCenter: parent.verticalCenter
                spacing: 1

                StyledText {
                    text: setting.label
                }
                StyledText {
                    width: parent.width
                    text: setting.note
                    color: Appearance.colors.onSurfaceVariant
                    font.pixelSize: Appearance.fontSize.smaller
                    elide: Text.ElideRight
                }
            }
            StyledSwitch {
                id: toggle
                anchors.right: parent.right
                anchors.verticalCenter: parent.verticalCenter
                checked: setting.value
                onClicked: setting.clicked()
            }
        }
    }

    // A theme as a little window: its ground, a sheet, its three accents.
    component Swatch: RippleButton {
        id: swatch

        required property var modelData
        readonly property bool selected: modelData.id === (root.prefs.theme ?? "system")
        readonly property bool broken: modelData.error !== undefined

        height: 84
        enabled: !broken
        buttonRadius: Appearance.rounding.normal
        colBackground: selected ? Appearance.colors.secondaryContainer : Appearance.colors.layer4
        colBackgroundHover: selected ? Appearance.colors.secondaryContainerHover : Appearance.colors.layer4Hover
        colRipple: Appearance.colors.layer4Active
        tooltip: broken ? modelData.error : ""
        onClicked: root.themeSelected(modelData.id)

        contentItem: Item {
            Rectangle {
                id: preview
                x: 2
                y: 2
                width: parent.width - 4
                height: 40
                radius: Appearance.rounding.small
                color: swatch.broken ? Appearance.colors.errorContainer : swatch.modelData.background
                border.width: 1
                border.color: Appearance.colors.outlineVariant

                Rectangle {
                    visible: !swatch.broken
                    anchors.fill: parent
                    anchors.margins: 6
                    anchors.leftMargin: 18
                    radius: 6
                    color: swatch.modelData.surface ?? "transparent"

                    Row {
                        anchors.centerIn: parent
                        spacing: 5

                        Repeater {
                            model: swatch.broken ? [] : [swatch.modelData.primary, swatch.modelData.secondary, swatch.modelData.tertiary]

                            Rectangle {
                                required property string modelData
                                width: 10
                                height: 10
                                radius: 5
                                color: modelData
                            }
                        }
                    }
                }

                MaterialSymbol {
                    anchors.centerIn: parent
                    visible: swatch.broken
                    text: "warning"
                    iconSize: 20
                    color: Appearance.colors.onErrorContainer
                }
            }

            StyledText {
                anchors.left: parent.left
                anchors.leftMargin: 4
                anchors.right: check.left
                anchors.rightMargin: 4
                anchors.bottom: parent.bottom
                anchors.bottomMargin: 2
                text: swatch.modelData.name
                elide: Text.ElideRight
                font.pixelSize: Appearance.fontSize.smallie
                font.weight: swatch.selected ? Font.DemiBold : Font.Medium
                color: swatch.selected ? Appearance.colors.onSecondaryContainer : Appearance.colors.onSurface
            }

            ShapeIcon {
                id: check
                anchors.right: parent.right
                anchors.bottom: parent.bottom
                size: 22
                iconSize: 14
                shape: "cookie9"
                symbol: "check"
                color: Appearance.colors.primary
                iconColor: Appearance.colors.onPrimary
            }
            Reveal {
                target: check
                shown: swatch.selected
            }
        }
    }

    Flickable {
        width: parent.width
        height: Math.min(contents.implicitHeight, root.maximumHeight - 180)
        contentHeight: contents.implicitHeight
        boundsBehavior: Flickable.StopAtBounds
        clip: true

        T.ScrollBar.vertical: StyledScrollBar {}

        Column {
            id: contents

            width: parent.width
            spacing: 10

            SectionLabel {
                width: parent.width
                inset: 0
                text: qsTr("Theme")
            }

            GridLayout {
                width: parent.width
                columns: 3
                columnSpacing: 8
                rowSpacing: 8

                Repeater {
                    model: root.themes

                    Swatch {
                        Layout.fillWidth: true
                        Layout.preferredWidth: 1
                        Layout.preferredHeight: 84
                    }
                }
            }

            Row {
                width: parent.width
                spacing: 12

                TextButton {
                    id: folderButton
                    anchors.verticalCenter: parent.verticalCenter
                    variant: TextButton.Tonal
                    symbol: "folder_open"
                    text: qsTr("Open Themes Folder")
                    onClicked: Qt.openUrlExternally(Config.themesFolder())
                }
                StyledText {
                    anchors.verticalCenter: parent.verticalCenter
                    width: parent.width - folderButton.width - parent.spacing
                    text: qsTr("a .toml with a seed colour, or [dark] and [light] palettes. edits show as you save")
                    color: Appearance.colors.onSurfaceVariant
                    font.pixelSize: Appearance.fontSize.smaller
                    wrapMode: Text.Wrap
                }
            }

            Item {
                width: 1
                height: 4
            }

            SectionLabel {
                width: parent.width
                inset: 0
                text: qsTr("Mode")
            }

            SegmentedControl {
                currentIndex: Math.max(0, root.modes.indexOf(root.prefs.theme_mode ?? "auto"))
                segments: [
                    Segment { text: qsTr("Auto"); symbol: "brightness_auto" },
                    Segment { text: qsTr("Light"); symbol: "light_mode" },
                    Segment { text: qsTr("Dark"); symbol: "dark_mode" }
                ]
                onActivated: index => root.modeSelected(root.modes[index])
            }

            StyledText {
                width: parent.width
                text: qsTr("the desktop's own scheme picks for itself, whatever this says")
                visible: (root.prefs.theme ?? "system") === "system"
                color: Appearance.colors.onSurfaceVariant
                font.pixelSize: Appearance.fontSize.smaller
                wrapMode: Text.Wrap
            }

            Item {
                width: 1
                height: 4
            }

            SectionLabel {
                width: parent.width
                inset: 0
                text: qsTr("Motion and Shapes")
            }

            Column {
                width: parent.width
                spacing: 2

                SettingRow {
                    label: qsTr("Reduce Motion")
                    note: qsTr("nothing moves on its own; clicks and new data still do")
                    value: root.prefs.reduce_motion ?? false
                    onClicked: root.preferenceEdited("reduce_motion", !value)
                }
                SettingRow {
                    label: qsTr("Performance Mode")
                    note: qsTr("no animations at all, not even fades")
                    value: root.prefs.performance_mode ?? false
                    onClicked: root.preferenceEdited("performance_mode", !value)
                }
                SettingRow {
                    label: qsTr("No Shapes")
                    note: qsTr("less shapes if you prefer a minimalistic look")
                    value: root.prefs.no_shapes ?? false
                    onClicked: root.preferenceEdited("no_shapes", !value)
                }
            }
        }
    }

    buttons: [
        TextButton {
            variant: TextButton.Filled
            text: qsTr("Done")
            onClicked: root.close()
        }
    ]
}
