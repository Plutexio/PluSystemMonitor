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
            if (!json)
                return { id: id, available: false, name: id, unit: "", value: 0, max: 0,
                         fraction: 0, history: [], text: "—", accent: "cpu" }
            const m = JSON.parse(json)
            m.available = true
            m.fraction = m.max > 0 ? Math.min(1, m.value / m.max) : 0
            m.text = Format.metric(m.value, m.unit)
            return m
        })
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
