import QtQuick
import PluSystemMonitor

// Big, faint Material shapes scattered behind something, after end4's
// desktop stickers but kept quiet: tonal, low in contrast and partly cut off
// by whatever clips them. The same key always lays them out the same way
// (FNV-1a, then mulberry32), so a page keeps its own. They hold still. With
// no shapes there are none.
Item {
    id: root

    property string key: "plusysmon"
    property int count: 4
    property real minimumSize: 70
    property real maximumSize: 200
    property var scheme: Appearance.colors
    property real strength: 1
    /// Where their centres may fall, as shares of the height.
    property real fromY: -0.15
    property real toY: 1.1
    readonly property real alpha: (scheme.dark ? 0.32 : 0.5) * strength

    readonly property list<string> shapes: ["ghostish", "bun", "flower", "sunny", "cookie9", "clover4", "softBoom",
                                            "puffy", "pixelCircle", "softBurst", "clamShell", "arch", "fan", "gem",
                                            "cookie6", "clover8", "verySunny", "puffyDiamond", "burst", "pentagon"]

    function seedOf(text: string): int {
        let hash = 2166136261
        for (let index = 0; index < text.length; ++index) {
            hash ^= text.charCodeAt(index)
            hash = Math.imul(hash, 16777619)
        }
        return hash >>> 0
    }

    readonly property var layout: {
        let state = seedOf(key)
        const next = () => {
            state = (state + 0x6d2b79f5) >>> 0
            let t = state
            t = Math.imul(t ^ (t >>> 15), t | 1)
            t ^= t + Math.imul(t ^ (t >>> 7), t | 61)
            return ((t ^ (t >>> 14)) >>> 0) / 4294967296
        }
        const stickers = []
        for (let index = 0; index < count; ++index) {
            // A slot each across the width, jittered within it.
            stickers.push({
                x: (index + 0.15 + 0.7 * next()) / count,
                y: fromY + (toY - fromY) * next(),
                size: minimumSize + (maximumSize - minimumSize) * next(),
                shape: shapes[Math.floor(next() * shapes.length)],
                tone: Math.floor(next() * 3),
                angle: next() * 360
            })
        }
        return stickers
    }

    function tone(which: int): color {
        const base = which === 0 ? scheme.primaryContainer : (which === 1 ? scheme.tertiaryContainer : scheme.secondaryContainer)
        return Qt.alpha(base, alpha)
    }

    Repeater {
        model: Appearance.noShapes ? 0 : root.count

        MaterialShape {
            required property int index
            readonly property var spec: root.layout[index]

            width: spec.size
            height: spec.size
            x: spec.x * root.width - width / 2
            y: spec.y * root.height - height / 2
            shape: spec.shape
            color: root.tone(spec.tone)
            spinAngle: spec.angle
        }
    }
}
