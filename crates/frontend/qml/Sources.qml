pragma Singleton
import QtQuick
import PluSystemMonitor

// Data sources as plain objects for widgets. Read inside a binding that also
// touches `monitor.revision` so it refreshes with every sample:
//
//     readonly property var sources: { monitor.revision; return Sources.read(monitor, ids) }
QtObject {
    function read(monitor, ids) {
        return (ids ?? []).map(id => {
            const json = monitor ? monitor.metric(id) : ""
            // Unknown sources, and known ones that are gone (their app quit),
            // come back unavailable but keep a name when there is one.
            const m = Object.assign({ id: id, available: false, name: id, unit: "", accent: "cpu" },
                                    json ? JSON.parse(json) : {})
            if (!m.available)
                return Object.assign(m, { value: 0, max: 0, fraction: 0, history: [], text: "—" })
            m.fraction = m.max > 0 ? Math.min(1, m.value / m.max) : 0
            m.text = Format.metric(m.value, m.unit)
            return m
        })
    }

    // Graph scale: the source's full range, or its own recent peak when
    // `fit` is set (small values, e.g. one app's share of the CPU, stay visible).
    function scale(source, fit) {
        if (!source)
            return 1
        if (fit) {
            const peak = source.history.reduce((a, b) => Math.max(a, b), 0)
            return peak > 0 ? peak * 1.15 : 1
        }
        return source.max > 0 ? source.max : 1
    }

    // Why a source shows nothing.
    function absence(source, monitor) {
        if (!source)
            return "Pick a source in the widget's settings"
        if (!monitor || !monitor.connected)
            return "Waiting for data…"
        if (source.id.startsWith("game."))
            return "No game running"
        if (source.id.startsWith("app."))
            return "Not running"
        return "Not available on this machine"
    }

    // App and game sources need the daemon to collect processes.
    function needsProcesses(ids) {
        return (ids ?? []).some(id => id.startsWith("app.") || id.startsWith("game."))
    }

    function catalog(monitor) {
        return monitor ? JSON.parse(monitor.metricCatalog) : []
    }

    // One colour per series: each source's own accent, unless an earlier
    // series took it. `main` (an accent key or "auto") overrides the first.
    function colors(sources, main) {
        const used = []
        return sources.map((source, i) => {
            let key = i === 0 && main && main !== "auto" ? main : source.accent
            if (used.includes(key))
                key = Theme.accentKeys.find(k => !used.includes(k)) ?? key
            used.push(key)
            return Theme.accent(key)
        })
    }
}
