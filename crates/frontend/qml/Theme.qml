pragma Singleton
import QtQuick
import PluSystemMonitor

// Design tokens. The values come from theme files through load(); the defaults
// here only cover the moment before the first theme is applied.
QtObject {
    id: theme

    property string current
    property string name

    property color background: "#090b13"
    property color primary: "#8b5cf6"
    property color card: "#0bffffff"
    property color cardBorder: "#14ffffff"
    property color cardHighlight: "#0fffffff"
    property color track: "#12ffffff"
    property color grid: "#0bffffff"
    property color text: "#eceefa"
    property color textDim: "#8b90b2"
    property color good: "#34d399"
    property color warn: "#fbbf24"
    property color bad: "#f87171"
    property color vignette: "#73000000"

    property color cpu: "#8b5cf6"
    property color memory: "#22d3ee"
    property color swap: "#f472b6"
    property color gpu: "#a3e635"
    property color network: "#60a5fa"
    property color disk: "#fb923c"

    property real radius: 22
    property real spacing: 16
    property real padding: 22

    property bool animatedBackground: true
    property real backgroundGlow: 0.32
    property real glow: 0.9

    readonly property var accentKeys: ["cpu", "memory", "swap", "gpu", "network", "disk"]

    // Re-apply when the user edits the active theme file.
    property Connections watcher: Connections {
        target: Config

        function onThemeFileChanged() {
            theme.load(theme.current)
        }
    }

    function load(id) {
        const json = Config.loadTheme(id)
        if (!json)
            return false
        const values = JSON.parse(json)
        for (const key in values) {
            if (theme[key] !== undefined)
                theme[key] = values[key]
        }
        current = id
        return true
    }

    function accent(key) {
        return accentKeys.includes(key) ? theme[key] : cpu
    }

    function formatBytes(bytes) {
        const units = ["B", "KiB", "MiB", "GiB", "TiB"]
        let i = 0
        while (bytes >= 1024 && i < units.length - 1) {
            bytes /= 1024
            i++
        }
        return bytes.toFixed(i >= 3 ? 1 : 0) + " " + units[i]
    }
}
