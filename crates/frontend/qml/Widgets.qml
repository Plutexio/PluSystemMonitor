pragma Singleton
import QtQuick

// Every widget type the dashboard can host. A widget is a QML file in widgets/
// that takes `monitor` and `options`. Its options are described here, so the
// settings form is generated and new widgets need no other wiring.
//
// Option types: "bool", "accent" (one of Theme.accentKeys).
QtObject {
    readonly property var types: [
        {
            type: "cpu.gauge",
            name: "CPU usage",
            description: "Overall processor load as a gauge.",
            source: Qt.resolvedUrl("widgets/CpuGaugeWidget.qml"),
            size: [3, 4],
            minSize: [2, 3],
            options: [
                { key: "accent", label: "Colour", type: "accent", default: "cpu" }
            ]
        },
        {
            type: "cpu.graph",
            name: "CPU history",
            description: "Processor load over the last two minutes.",
            source: Qt.resolvedUrl("widgets/CpuGraphWidget.qml"),
            size: [9, 4],
            minSize: [3, 2],
            options: [
                { key: "accent", label: "Colour", type: "accent", default: "cpu" },
                { key: "showCores", label: "Per-core bars", type: "bool", default: true }
            ]
        },
        {
            type: "cpu.cores",
            name: "CPU cores",
            description: "Load of every logical core.",
            source: Qt.resolvedUrl("widgets/CpuCoresWidget.qml"),
            size: [6, 2],
            minSize: [2, 2],
            options: [
                { key: "accent", label: "Colour", type: "accent", default: "cpu" }
            ]
        },
        {
            type: "memory.gauge",
            name: "Memory",
            description: "RAM in use, with RAM and swap bars.",
            source: Qt.resolvedUrl("widgets/MemoryGaugeWidget.qml"),
            size: [3, 4],
            minSize: [2, 3],
            options: [
                { key: "accent", label: "Colour", type: "accent", default: "memory" },
                { key: "showSwap", label: "Swap bar", type: "bool", default: true }
            ]
        },
        {
            type: "gpu.gauge",
            name: "GPU",
            description: "Graphics load, VRAM, temperature and power.",
            source: Qt.resolvedUrl("widgets/GpuGaugeWidget.qml"),
            size: [3, 4],
            minSize: [2, 3],
            options: [
                { key: "accent", label: "Colour", type: "accent", default: "gpu" }
            ]
        },
        {
            type: "gpu.graph",
            name: "GPU history",
            description: "Graphics load over time, with clocks, fan and video engines.",
            source: Qt.resolvedUrl("widgets/GpuGraphWidget.qml"),
            size: [9, 4],
            minSize: [3, 2],
            options: [
                { key: "accent", label: "Colour", type: "accent", default: "gpu" }
            ]
        },
        {
            type: "memory.graph",
            name: "Memory history",
            description: "RAM usage over the last two minutes.",
            source: Qt.resolvedUrl("widgets/MemoryGraphWidget.qml"),
            size: [9, 4],
            minSize: [3, 2],
            options: [
                { key: "accent", label: "Colour", type: "accent", default: "memory" }
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
                values[option.key] = option.default
        }
        return values
    }
}
