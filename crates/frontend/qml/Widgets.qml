pragma Singleton
import QtQuick

// Every widget type the dashboard can host. A widget is a QML file in widgets/
// that takes `monitor` and `options`. Its options are described here, so the
// settings form is generated and new widgets need no other wiring.
//
// Option types: "bool", "text", "accent" (one of Sources.accentKeys, or "auto"
// to follow the data source), "sources" (data source IDs, `min`..`max` of them).
QtObject {
    readonly property var types: [
        {
            type: "gauge",
            name: "Gauge",
            description: "One source on a ring, more as bars below it.",
            source: Qt.resolvedUrl("widgets/GaugeWidget.qml"),
            size: [3, 4],
            minSize: [2, 2],
            options: [
                { key: "sources", label: "Sources", type: "sources", default: ["cpu.total"], min: 1, max: 6,
                  hint: "The first one fills the ring." },
                { key: "title", label: "Title", type: "text", default: "", placeholder: "Name of the first source" },
                { key: "accent", label: "Colour", type: "accent", default: "auto" }
            ]
        },
        {
            type: "graph",
            name: "Graph",
            description: "Up to four sources over the last two minutes.",
            source: Qt.resolvedUrl("widgets/GraphWidget.qml"),
            size: [6, 4],
            minSize: [3, 2],
            options: [
                { key: "sources", label: "Sources", type: "sources", default: ["cpu.total"], min: 1, max: 4,
                  hint: "Each line is scaled to its own range." },
                { key: "title", label: "Title", type: "text", default: "", placeholder: "Name of the first source" },
                { key: "fit", label: "Fit Lines to Their Data", type: "bool", default: false },
                { key: "accent", label: "Colour of the First Line", type: "accent", default: "auto" }
            ]
        },
        {
            type: "value",
            name: "Value",
            description: "One source as a big number with a sparkline.",
            source: Qt.resolvedUrl("widgets/ValueWidget.qml"),
            size: [3, 2],
            minSize: [2, 2],
            options: [
                { key: "sources", label: "Source", type: "sources", default: ["cpu.total"], min: 1, max: 1 },
                { key: "title", label: "Title", type: "text", default: "", placeholder: "Name of the source" },
                { key: "accent", label: "Colour", type: "accent", default: "auto" }
            ]
        },
        {
            type: "cpu.cores",
            name: "CPU Cores",
            description: "Load of every logical core.",
            source: Qt.resolvedUrl("widgets/CpuCoresWidget.qml"),
            size: [6, 2],
            minSize: [2, 2],
            options: [
                { key: "accent", label: "Colour", type: "accent", default: "cpu" }
            ]
        }
    ]

    function find(type) {
        return types.find(t => t.type === type) ?? null
    }

    function defaults(type) {
        const values = {}
        const descriptor = find(type)
        if (descriptor) {
            for (const option of descriptor.options)
                values[option.key] = Array.isArray(option.default) ? option.default.slice() : option.default
        }
        return values
    }
}
