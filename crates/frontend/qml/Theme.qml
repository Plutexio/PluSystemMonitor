pragma Singleton
import QtQuick

// Single place for every visual constant; user themes will override these.
QtObject {
    property color background: "#090b13"
    property color card: Qt.rgba(1, 1, 1, 0.045)
    property color cardBorder: Qt.rgba(1, 1, 1, 0.08)
    property color cardHighlight: Qt.rgba(1, 1, 1, 0.06)
    property color track: Qt.rgba(1, 1, 1, 0.07)
    property color grid: Qt.rgba(1, 1, 1, 0.045)

    property color text: "#eceefa"
    property color textDim: "#8b90b2"
    property color good: "#34d399"
    property color warn: "#fbbf24"
    property color bad: "#f87171"

    property color cpu: "#8b5cf6"
    property color mem: "#22d3ee"
    property color swap: "#f472b6"

    property int radius: 22
    property int spacing: 16
    property int padding: 22

    property bool animatedBackground: true
    property real glow: 0.9

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
