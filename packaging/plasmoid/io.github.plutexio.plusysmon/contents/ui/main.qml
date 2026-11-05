import QtQuick
import QtQuick.Layouts
import org.kde.plasma.plasmoid
import org.kde.plasma.core as PlasmaCore
import org.kde.kirigami as Kirigami

import "Sources.js" as Sources

// PluSystemMonitor's data sources on the panel or the desktop. The daemon
// (plusysmond) serves them on the session bus; see crates/daemon/src/widgets.rs.
PlasmoidItem {
    id: root

    readonly property var config: Plasmoid.configuration
    readonly property bool onPanel: Plasmoid.formFactor === PlasmaCore.Types.Horizontal
                                    || Plasmoid.formFactor === PlasmaCore.Types.Vertical
    readonly property var ids: (config.sources ?? []).filter(id => id.length > 0)
    readonly property var sources: Sources.read(backend.reply, ids)
    readonly property var colours: Sources.palette(Kirigami.Theme.highlightColor,
        Kirigami.ColorUtils.brightnessForColor(Kirigami.Theme.backgroundColor) === Kirigami.ColorUtils.Dark)
    readonly property var colors: Sources.colors(sources, config.accent, colours)
    readonly property bool live: backend.up && backend.reply !== null && backend.reply.connected
    /// Gone and being retried: what shows are its last values.
    readonly property bool offline: backend.up && Sources.isOffline(backend.reply)
    readonly property string machineLabel: config.machine && backend.reply && backend.reply.hostname
                                           ? backend.reply.hostname : (config.machine ?? "")

    Plasmoid.icon: "plusysmon"
    Plasmoid.backgroundHints: PlasmaCore.Types.DefaultBackground | PlasmaCore.Types.ConfigurableBackground
    preferredRepresentation: onPanel ? compactRepresentation : fullRepresentation

    toolTipMainText: config.title || (sources[0] ? sources[0].name : "PluSystemMonitor")
    toolTipSubText: {
        if (offline) {
            const last = sources.filter(s => s.available).map(s => s.name + ": " + s.text)
            return Sources.offlineText(backend.reply) + (last.length ? "\n\nLast seen:\n" + last.join("\n") : "")
        }
        if (!live)
            return Sources.absence(null, backend.reply, backend.up)
        const lines = sources.map(s => s.name + ": " + s.text)
        if (machineLabel)
            lines.push("on " + machineLabel)
        return lines.join("\n")
    }

    Backend {
        id: backend
        machine: root.config.machine ?? ""
        ids: root.ids
        // On a panel the values show all the time; the popup only adds to that.
        running: root.onPanel || root.expanded || Plasmoid.formFactor === PlasmaCore.Types.Planar
                 || Plasmoid.formFactor === PlasmaCore.Types.MediaCenter
    }

    compactRepresentation: CompactView {
        sources: root.sources
        colors: root.colors
        live: root.live || root.offline
        offline: root.offline
        style: root.config.style === "list" ? "value" : root.config.style
        vertical: Plasmoid.formFactor === PlasmaCore.Types.Vertical
        interval: backend.intervalMs
        // All of the panel's thickness, as Plasma's own monitors take it;
        // as long as the sources need along it.
        anchors.fill: parent
        Layout.fillHeight: !vertical
        Layout.fillWidth: vertical
        Layout.minimumWidth: vertical ? -1 : implicitWidth
        Layout.preferredWidth: vertical ? -1 : implicitWidth
        Layout.maximumWidth: vertical ? Infinity : implicitWidth
        Layout.minimumHeight: vertical ? implicitHeight : -1
        Layout.preferredHeight: vertical ? implicitHeight : -1
        Layout.maximumHeight: vertical ? implicitHeight : Infinity
        onClicked: root.expanded = !root.expanded
    }

    fullRepresentation: FullView {
        Layout.minimumWidth: Kirigami.Units.gridUnit * 6
        Layout.minimumHeight: Kirigami.Units.gridUnit * 6
        Layout.preferredWidth: Kirigami.Units.gridUnit * (root.config.style === "graph" ? 20 : 14)
        Layout.preferredHeight: Kirigami.Units.gridUnit * 12
        sources: root.sources
        colors: root.colors
        reply: backend.reply
        serviceUp: backend.up
        style: root.config.style
        title: root.config.title
        showTitle: root.config.showTitle
        fit: root.config.fit
        interval: backend.intervalMs
        machineLabel: root.machineLabel
    }
}
