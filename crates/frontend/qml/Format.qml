pragma Singleton
import QtQuick

// Turning numbers into text the same way everywhere.
QtObject {
    function bytes(value) {
        const units = ["B", "KiB", "MiB", "GiB", "TiB"]
        let i = 0
        while (value >= 1024 && i < units.length - 1) {
            value /= 1024
            i++
        }
        return value.toFixed(i >= 3 ? 1 : 0) + " " + units[i]
    }

    // Disk rates: negative means "not readable", zero is left blank to cut noise.
    function rate(value) {
        if (value < 0)
            return "—"
        return value === 0 ? "" : bytes(value) + "/s"
    }

    // A data source's value in its unit (see metrics.rs `Unit`).
    function metric(value, unit) {
        switch (unit) {
        case "percent": return value.toFixed(value < 10 ? 1 : 0) + "%"
        case "bytes": return bytes(value)
        case "celsius": return temperature(value)
        case "watts": return watts(value)
        case "megahertz": return Math.round(value) + " MHz"
        }
        return value.toFixed(1)
    }

    function temperature(celsius) {
        return Math.round(celsius) + " °C"
    }

    function watts(value) {
        return Math.round(value) + " W"
    }

    // "NVIDIA GeForce RTX 5070 Ti" → "RTX 5070 Ti": the vendor is implied.
    function gpuName(name) {
        return name.replace(/^(NVIDIA )?(GeForce )?|^AMD (Radeon )?/, "") || name
    }

    function processState(letter) {
        switch (letter) {
        case "R": return "Running"
        case "S": return "Sleeping"
        case "D": return "Waiting on I/O"
        case "T": return "Paused"
        case "t": return "Traced"
        case "Z": return "Zombie"
        case "I": return "Idle"
        case "X": return "Dead"
        default: return letter
        }
    }

    // "3 h 12 min", "45 s", "2 d 4 h": the two largest units are enough.
    function duration(ms) {
        let s = Math.max(0, Math.floor(ms / 1000))
        const parts = [[86400, "d"], [3600, "h"], [60, "min"], [1, "s"]]
        const out = []
        for (const [size, unit] of parts) {
            if (s >= size || (unit === "s" && out.length === 0)) {
                out.push(Math.floor(s / size) + " " + unit)
                s %= size
            }
            if (out.length === 2)
                break
        }
        return out.join(" ")
    }
}
