import QtQuick
import QtQuick.Controls as QQC2
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.kcmutils as KCM
import org.kde.plasma.plasmoid
import org.kde.plasma.core as PlasmaCore

import "Sources.js" as Sources

// How the sources are drawn, on the desktop and on a panel.
KCM.SimpleKCM {
    id: page

    property string cfg_style
    property string cfg_accent
    property bool cfg_showTitle
    property bool cfg_fit
    property string cfg_styleDefault
    property string cfg_accentDefault
    property bool cfg_showTitleDefault
    property bool cfg_fitDefault
    // Set on every page by Plasma; the other page uses them.
    property string cfg_machine
    property string cfg_machineDefault
    property var cfg_sources
    property var cfg_sourcesDefault
    property string cfg_title
    property string cfg_titleDefault

    readonly property bool onPanel: Plasmoid.formFactor === PlasmaCore.Types.Horizontal
                                    || Plasmoid.formFactor === PlasmaCore.Types.Vertical
    readonly property var colours: Sources.palette(Kirigami.Theme.highlightColor,
        Kirigami.ColorUtils.brightnessForColor(Kirigami.Theme.backgroundColor) === Kirigami.ColorUtils.Dark)

    Kirigami.FormLayout {
        QQC2.ComboBox {
            id: styleBox
            Kirigami.FormData.label: "Style:"
            textRole: "text"
            valueRole: "value"
            // A list doesn't fit on a panel; there it shows as values.
            model: [
                { value: "gauge", text: page.onPanel ? "Gauge: a ring with the value inside" : "Gauge: the first source on a ring, the rest as bars" },
                { value: "graph", text: page.onPanel ? "Graph: a tiny graph under the value" : "Graph: every source over the last two minutes" },
                { value: "value", text: page.onPanel ? "Values: the value over its history" : "Values: big numbers over their history" }
            ].concat(page.onPanel ? [] : [{ value: "list", text: "List: names, values and bars" }])
            Component.onCompleted: currentIndex = Math.max(0, indexOfValue(page.cfg_style === "list" && page.onPanel ? "value" : page.cfg_style))
            onActivated: page.cfg_style = currentValue
        }
        QQC2.Label {
            Layout.maximumWidth: Kirigami.Units.gridUnit * 22
            visible: page.onPanel
            text: "Each source gets its own small one. Clicking the widget opens the large version."
            wrapMode: Text.Wrap
            color: Kirigami.Theme.disabledTextColor
            font: Kirigami.Theme.smallFont
        }

        Item { Kirigami.FormData.isSection: true }

        RowLayout {
            Kirigami.FormData.label: "Colour:"
            spacing: Kirigami.Units.smallSpacing

            QQC2.Button {
                text: "Automatic"
                checkable: true
                checked: page.cfg_accent === "auto" || page.cfg_accent === ""
                onClicked: page.cfg_accent = "auto"
                QQC2.ToolTip.text: "Each source in the colour that suits it"
                QQC2.ToolTip.visible: hovered
            }
            Repeater {
                model: Sources.accentKeys
                QQC2.AbstractButton {
                    id: swatch
                    required property string modelData
                    implicitWidth: Kirigami.Units.gridUnit * 1.6
                    implicitHeight: implicitWidth
                    checkable: true
                    checked: page.cfg_accent === modelData
                    onClicked: page.cfg_accent = modelData
                    background: Rectangle {
                        radius: width / 2
                        color: page.colours[swatch.modelData]
                        border.width: swatch.checked ? 3 : 0
                        border.color: Kirigami.Theme.textColor
                    }
                    QQC2.ToolTip.text: "Like " + swatch.modelData
                    QQC2.ToolTip.visible: hovered
                }
            }
        }
        QQC2.Label {
            Layout.maximumWidth: Kirigami.Units.gridUnit * 22
            text: "The colours follow the accent colour of your Plasma theme. A chosen one goes to the first source."
            wrapMode: Text.Wrap
            color: Kirigami.Theme.disabledTextColor
            font: Kirigami.Theme.smallFont
        }

        Item { Kirigami.FormData.isSection: true }

        QQC2.CheckBox {
            Kirigami.FormData.label: "Show:"
            text: "Title"
            checked: page.cfg_showTitle
            onToggled: page.cfg_showTitle = checked
        }
        QQC2.CheckBox {
            text: "Fit graph lines to their data"
            checked: page.cfg_fit
            onToggled: page.cfg_fit = checked
            QQC2.ToolTip.text: "Scale each line to its own recent peak, so small values stay visible"
            QQC2.ToolTip.visible: hovered
        }
    }
}
