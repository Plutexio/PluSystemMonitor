import QtQuick
import org.kde.plasma.workspace.dbus as DBus

// The PluSystemMonitor daemon's widget service on the session bus
// (crates/daemon/src/widgets.rs). Calls answer with parsed JSON, or null.
Item {
    id: daemon

    readonly property string service: "io.github.plutexio.PluSystemMonitor"
    readonly property bool up: watcher.registered

    DBus.DBusServiceWatcher {
        id: watcher
        busType: DBus.BusType.Session
        watchedService: daemon.service
    }

    function call(member, args, done) {
        const reply = DBus.SessionBus.asyncCall({
            service: daemon.service,
            path: "/Widgets",
            iface: daemon.service + ".Widgets",
            member: member,
            arguments: args
        })
        reply.finished.connect(() => {
            let result = null
            if (!reply.isError) {
                let value = reply.value
                // Some replies come wrapped once more.
                if (value && value.value !== undefined)
                    value = value.value
                try {
                    result = JSON.parse(String(value))
                } catch (e) {
                    console.warn("plusysmon: malformed reply to", member, e)
                }
            }
            // Destroyed here, or the connection keeps the reply alive.
            reply.destroy()
            done(result)
        })
    }
}
