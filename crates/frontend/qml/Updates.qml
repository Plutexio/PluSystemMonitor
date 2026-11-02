pragma Singleton
import QtQuick
import PluSystemMonitor

// What a machine's probe (Fleet's `probed`) and update progress mean, in words
// and in a tone: good, busy, warn, error or muted.
QtObject {
    function busy(progress) {
        return !!progress && ["starting", "updating", "restarting"].includes(progress.state)
    }

    // Its daemon can update itself and has something to update to.
    function updatable(status, progress) {
        if (!status || !status.online || !status.updates || busy(progress))
            return false
        const u = status.updates
        return u.unavailable === "" && u.behind > 0 && u.state !== "updating" && u.state !== "restarting"
    }

    function describe(status, progress, appProtocol) {
        if (busy(progress)) {
            const last = progress.lines.length > 0 ? progress.lines[progress.lines.length - 1] : ""
            return { text: qsTr("Updating… %1").arg(last), tone: "busy" }
        }
        if (progress && progress.state === "failed")
            return { text: qsTr("Update failed: %1").arg(progress.error), tone: "error" }
        if (!status)
            return { text: qsTr("Looking…"), tone: "muted" }
        if (!status.online)
            return { text: qsTr("Offline"), tone: "error" }
        switch (status.state) {
        case "untrusted":
            return { text: qsTr("Its certificate isn't trusted yet; connect to it once"), tone: "warn" }
        case "rejected":
            return { text: qsTr("Refused: %1").arg(status.message), tone: "error" }
        case "error":
            return { text: status.message, tone: "error" }
        }
        if (status.legacy) {
            return status.compatible
                ? { text: qsTr("Protocol %1, too old to update itself. Run install.sh there once").arg(status.version), tone: "warn" }
                : { text: qsTr("Incompatible and REQUIRES an update. Run install.sh there"), tone: "error" }
        }
        const u = status.updates ?? {}
        if (!status.compatible) {
            const daemonOlder = status.version < appProtocol
            if (u.unavailable === "" && daemonOlder)
                return { text: qsTr("Incompatible and REQUIRES an update"), tone: "error" }
            return daemonOlder
                ? { text: qsTr("Incompatible and REQUIRES an update: %1").arg(u.unavailable), tone: "error" }
                : { text: qsTr("Newer than this app, which REQUIRES an update to talk to it"), tone: "error" }
        }
        if (u.unavailable)
            return { text: qsTr("No remote updates: %1").arg(u.unavailable), tone: "muted" }
        if (status.checking || u.state === "checking")
            return { text: qsTr("Checking for updates…"), tone: "busy" }
        if (u.state === "updating" || u.state === "restarting")
            return { text: qsTr("Updating…"), tone: "busy" }
        if (u.state === "failed" && u.error)
            return { text: qsTr("Failed: %1").arg(u.error), tone: "warn" }
        if (u.behind > 0) {
            const count = u.behind === 1 ? qsTr("1 update") : qsTr("%1 updates").arg(u.behind)
            let text = qsTr("%1 · %2").arg(count).arg(u.latest)
            if (u.latest_oldest_compatible > appProtocol)
                text += qsTr(" · afterwards it needs a newer app here too")
            return { text: text, tone: "good" }
        }
        if (!u.checked_ms)
            return { text: qsTr("Not checked yet"), tone: "muted" }
        return { text: qsTr("Up to date"), tone: "muted" }
    }

    function toneColor(tone) {
        switch (tone) {
        case "good": return Appearance.colors.primary
        case "warn": return Appearance.colors.warn
        case "error": return Appearance.colors.error
        default: return Appearance.colors.onSurfaceVariant
        }
    }
}
