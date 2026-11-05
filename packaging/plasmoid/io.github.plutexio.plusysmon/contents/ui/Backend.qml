import QtQuick

// Reads the widget's sources from the daemon once per sampling interval.
Daemon {
    id: backend

    property string machine
    property var ids: []
    /// Polls only while something shows the data.
    property bool running: true

    /// The latest `Read` reply: { connected, status, hostname, intervalMs, sources }.
    property var reply: null
    readonly property int intervalMs: reply && reply.intervalMs > 0 ? reply.intervalMs : 1000
    property bool busy: false

    onMachineChanged: reply = null
    onUpChanged: if (!up) reply = null

    function poll() {
        if (busy || !up)
            return
        busy = true
        const machine = backend.machine
        call("Read", [machine, (ids ?? []).slice()], result => {
            busy = false
            if (machine === backend.machine)
                backend.reply = result
        })
    }

    Timer {
        interval: Math.max(250, backend.intervalMs)
        repeat: true
        triggeredOnStart: true
        running: backend.running && backend.up && (backend.ids ?? []).length > 0
        onTriggered: backend.poll()
    }

    // New sources show without waiting for the next tick.
    onIdsChanged: Qt.callLater(poll)
}
