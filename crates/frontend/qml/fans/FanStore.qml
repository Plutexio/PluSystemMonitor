import QtQuick
import PluSystemMonitor

// The connected machine's fan control, as its daemon keeps it: the settings
// (curves, and what each fan follows), the fans its helper offers, and
// whether the helper is there. Edits apply at once here and reach the daemon
// a moment later, in one go, so dragging a curve point doesn't send a storm.
QtObject {
    id: root

    required property Monitor monitor
    /// Daemon answered at least once since connecting.
    property bool loaded: false
    /// The daemon knows of fan control (version 7 and up).
    property bool supported: true
    property var settings: ({ interval_ms: 3000, curves: [], fans: {} })
    /// `[{ id, name, device, rpm_source, percent, min_percent }]`
    property var outputs: []
    property var status: ({ helper: false, problem: "" })
    /// `{ id: { id, name, percent, input, problem } }`, as of the latest sample.
    readonly property var states: {
        const list = JSON.parse(monitor.fanStates)
        const map = {}
        for (const state of list)
            map[state.id] = state
        return map
    }
    readonly property bool usable: monitor.connected && supported && status.helper

    signal failed(string message)

    readonly property var modes: [
        { id: "highest", name: qsTr("Hottest") },
        { id: "average", name: qsTr("Average") },
        { id: "lowest", name: qsTr("Coolest") },
        { id: "first", name: qsTr("First") }
    ]

    function refresh() {
        supported = monitor.requestFans()
        if (!supported)
            loaded = true
    }

    /// Copies the settings, lets `update` change the copy, keeps it, and saves soon.
    function edit(update) {
        const copy = JSON.parse(JSON.stringify(settings))
        update(copy)
        settings = copy
        saveTimer.restart()
    }

    function fan(id) {
        return settings.fans[id] ?? { mode: "auto", percent: 50, curve: "", min: 20, max: 100, allow_stop: false }
    }

    function editFan(id, update) {
        edit(s => {
            const fan = Object.assign(root.fan(id), s.fans[id] ?? {})
            update(fan)
            s.fans[id] = fan
        })
    }

    function curve(id) {
        return settings.curves.find(c => c.id === id) ?? null
    }

    function editCurve(id, update) {
        edit(s => {
            const curve = s.curves.find(c => c.id === id)
            if (curve)
                update(curve)
        })
    }

    /// A new curve on `sources`; returns its ID.
    function addCurve(name, sources) {
        const used = settings.curves.map(c => parseInt((c.id.split(".")[1]) || "0") || 0)
        const id = "curve." + (Math.max(0, ...used) + 1)
        edit(s => s.curves.push({
            id: id,
            name: name,
            sources: sources,
            combine: "highest",
            points: [[30, 25], [50, 35], [70, 60], [85, 100]],
            hysteresis: 2,
            average_secs: 0,
            step_up: 0,
            step_down: 5
        }))
        return id
    }

    /// Fans that followed it go back to the firmware.
    function removeCurve(id) {
        edit(s => {
            s.curves = s.curves.filter(c => c.id !== id)
            for (const fanId in s.fans) {
                if (s.fans[fanId].mode === "curve" && s.fans[fanId].curve === id)
                    s.fans[fanId].mode = "auto"
            }
        })
    }

    function fansFollowing(curveId) {
        return outputs.filter(o => {
            const fan = settings.fans[o.id]
            return fan && fan.mode === "curve" && fan.curve === curveId
        })
    }

    function outputName(id) {
        const label = settings.fans[id]?.label
        if (label)
            return label
        return outputs.find(o => o.id === id)?.name ?? id
    }

    // The same arithmetic as the daemon (plusysmon_protocol::fans), for
    // showing where a curve stands now.
    function speedAt(points, input) {
        if (!points.length)
            return 100
        if (input <= points[0][0])
            return points[0][1]
        for (let i = 1; i < points.length; i++) {
            const [x0, y0] = points[i - 1]
            const [x1, y1] = points[i]
            if (input <= x1)
                return x1 <= x0 ? y1 : y0 + (y1 - y0) * (input - x0) / (x1 - x0)
        }
        return points[points.length - 1][1]
    }

    function combine(mode, values) {
        if (!values.length)
            return null
        switch (mode) {
        case "average": return values.reduce((a, b) => a + b, 0) / values.length
        case "highest": return Math.max(...values)
        case "lowest": return Math.min(...values)
        case "sum": return values.reduce((a, b) => a + b, 0)
        }
        return values[0]
    }

    /// A curve's input now, from the latest sample (unaveraged). Read inside a
    /// binding that touches `monitor.revision`.
    function inputOf(curve) {
        if (!curve)
            return null
        const values = Sources.read(monitor, curve.sources).filter(s => s.available).map(s => s.value)
        return combine(curve.combine, values)
    }

    /// The links a virtual sensor has on the connected machine:
    /// `{ sources, mode }`, or null.
    function expandVirtual(virtualSensors, id) {
        const sensor = (virtualSensors.sensors ?? []).find(s => s.id === id)
        if (!sensor)
            return null
        const binding = sensor.bindings.find(b => b.machine && b.machine === monitor.fingerprint)
                     ?? sensor.bindings.find(b => b.machine === "")
        return binding ? { sources: binding.sources.slice(), mode: sensor.mode, name: sensor.name } : null
    }

    property Timer saveTimer: Timer {
        interval: 500
        onTriggered: root.monitor.saveFans(JSON.stringify(root.settings))
    }

    // Uncontrolled fans' speeds and the helper's state change behind our back.
    property Timer pollTimer: Timer {
        running: root.monitor.connected && root.supported
        interval: 5000
        repeat: true
        onTriggered: root.refresh()
    }

    property Connections connections: Connections {
        target: root.monitor
        function onFansReceived(json) {
            const reply = JSON.parse(json)
            if (reply.error)
                root.failed(reply.error)
            // Edits still waiting to go out win over what the daemon had.
            if (!root.saveTimer.running)
                root.settings = reply.settings
            root.outputs = reply.outputs
            root.status = reply.status
            root.loaded = true
        }
        function onConnectedChanged() {
            root.loaded = false
            root.outputs = []
            root.status = { helper: false, problem: "" }
            if (root.monitor.connected)
                root.refresh()
        }
    }
}
