.pragma library

// Data sources as the widget shows them: the daemon's readings (see the
// widget service, crates/daemon/src/widgets.rs) with text, scale and colour.
// Mirrors the app's Sources.qml and Format.qml.

var accentKeys = ["cpu", "memory", "swap", "gpu", "network", "disk"]

function bytes(value) {
    var units = ["B", "KiB", "MiB", "GiB", "TiB"]
    var i = 0
    while (value >= 1024 && i < units.length - 1) {
        value /= 1024
        i++
    }
    return value.toFixed(i >= 3 ? 1 : 0) + " " + units[i]
}

// A value in its unit (metrics.rs `Unit`).
function metric(value, unit) {
    switch (unit) {
    case "percent": return value.toFixed(value < 10 ? 1 : 0) + "%"
    case "bytes": return bytes(value)
    case "celsius": return Math.round(value) + " °C"
    case "watts": return Math.round(value) + " W"
    case "megahertz": return value >= 1000 ? (value / 1000).toFixed(2) + " GHz" : Math.round(value) + " MHz"
    case "volts": return value.toFixed(3) + " V"
    case "rpm": return Math.round(value) + " RPM"
    case "amps": return value.toFixed(2) + " A"
    }
    return value.toFixed(1)
}

// The same, as short as it goes, for a panel.
function brief(value, unit) {
    switch (unit) {
    case "percent": return value.toFixed(value < 10 ? 1 : 0) + "%"
    case "celsius": return Math.round(value) + "°"
    case "bytes": return bytes(value).replace("iB", "").replace(" ", "")
    case "megahertz": return value >= 1000 ? (value / 1000).toFixed(1) + "G" : Math.round(value) + "M"
    case "rpm": return Math.round(value) + ""
    }
    return metric(value, unit)
}

// The reply's sources, ready to draw.
function read(reply, ids) {
    var known = reply && reply.sources ? reply.sources : []
    return (ids || []).map(function (id) {
        var m = known.find(function (s) { return s.id === id }) || { id: id, available: false, name: id }
        m = Object.assign({ unit: "", accent: "cpu", category: "" }, m)
        if (!m.available)
            return Object.assign(m, { value: 0, max: 0, fraction: 0, history: [], text: "—", brief: "—" })
        m.fraction = m.max > 0 ? Math.min(1, m.value / m.max) : 0
        m.text = metric(m.value, m.unit)
        m.brief = brief(m.value, m.unit)
        return m
    })
}

// Graph scale: the full range, or the recent peak with `fit`.
function scale(source, fit) {
    if (!source)
        return 1
    if (fit || !(source.max > 0)) {
        var peak = (source.history || []).reduce(function (a, b) { return Math.max(a, b) }, 0)
        return peak > 0 ? peak * 1.15 : 1
    }
    return source.max
}

// The data colours, turned from the Plasma accent so they suit the theme:
// the accent itself for the processor, the others spaced round the wheel.
function palette(base, dark) {
    var shifts = { cpu: 0, gpu: 0.16, memory: 0.36, network: 0.5, swap: 0.64, disk: 0.82 }
    var hue = base.hslHue < 0 ? 0.6 : base.hslHue
    var saturation = Math.max(0.55, Math.min(0.85, base.hslSaturation))
    var lightness = dark ? 0.68 : 0.46
    var out = {}
    accentKeys.forEach(function (key) {
        var h = (hue + shifts[key]) % 1
        out[key] = key === "cpu" ? Qt.hsla(hue, saturation, lightness, 1) : Qt.hsla(h, saturation, lightness, 1)
    })
    return out
}

// One colour per source: its own accent unless an earlier one took it;
// `main` (a key or "auto") overrides the first.
function colors(sources, main, colours) {
    var used = []
    return sources.map(function (source, i) {
        var key = i === 0 && main && main !== "auto" ? main : source.accent
        if (used.indexOf(key) >= 0)
            key = accentKeys.find(function (k) { return used.indexOf(k) < 0 }) || key
        used.push(key)
        return colours[key] || colours.cpu
    })
}

// A tonal ground for something drawn in `tint`.
function tonal(tint, ground, dark) {
    var t = dark ? 0.24 : 0.2
    return Qt.rgba(tint.r * t + ground.r * (1 - t), tint.g * t + ground.g * (1 - t),
                   tint.b * t + ground.b * (1 - t), 1)
}

// The icon a source is shown with.
function icon(source) {
    var id = source ? source.id : ""
    var unit = source ? source.unit : ""
    if (unit === "celsius")
        return "temperature-normal"
    if (unit === "watts" || unit === "volts" || unit === "amps")
        return "flash"
    if (unit === "rpm" || unit === "megahertz")
        return "speedometer"
    if (id.indexOf("game.") === 0)
        return "input-gaming"
    if (id.indexOf("app.") === 0)
        return "application-x-executable"
    if (id.indexOf("gpu.") === 0)
        return "preferences-desktop-display"
    if (id.indexOf("memory.") === 0)
        return "memory"
    if (id.indexOf("cpu.") === 0)
        return "cpu"
    return "view-statistics"
}

// Whether the machine went away (and is being retried), so the values
// shown are its last ones.
function isOffline(reply) {
    return !!reply && reply.state === "offline"
}

// "since 14:32" today, "since 3 Nov 14:32" before.
function since(ms) {
    if (!(ms > 0))
        return ""
    var date = new Date(ms)
    var today = new Date()
    var sameDay = date.toDateString() === today.toDateString()
    return "since " + Qt.formatDateTime(date, sameDay ? "hh:mm" : "d MMM hh:mm")
}

// "12 min", "3 h 5 min", "2 d 4 h": how long ago `ms` was.
function ago(ms) {
    var s = Math.max(0, Math.floor((Date.now() - ms) / 1000))
    var parts = [[86400, "d"], [3600, "h"], [60, "min"], [1, "s"]]
    var out = []
    for (var i = 0; i < parts.length && out.length < 2; i++) {
        if (s >= parts[i][0] || (parts[i][1] === "s" && out.length === 0)) {
            out.push(Math.floor(s / parts[i][0]) + " " + parts[i][1])
            s %= parts[i][0]
        }
    }
    return out.join(" ")
}

// "Offline since 14:32 (12 min). It doesn't answer. Reconnecting…"
function offlineText(reply) {
    var when = reply.offlineSince > 0 ? " " + since(reply.offlineSince) + " (" + ago(reply.offlineSince) + ")" : ""
    return "Offline" + when + ". " + (reply.status ? reply.status + " " : "") + "Reconnecting…"
}

// Why a source shows nothing.
function absence(source, reply, serviceUp) {
    if (!serviceUp)
        return "The PluSystemMonitor daemon isn't running here. Start it with: systemctl --user start plusysmond"
    if (!reply)
        return "Waiting for data…"
    if (isOffline(reply))
        return offlineText(reply)
    if (!reply.connected)
        return reply.status || "Not connected"
    if (!source)
        return "Pick a source in the widget's settings."
    if (source.id.indexOf("game.") === 0)
        return "No game is running."
    if (source.id.indexOf("app.") === 0)
        return "It isn't running."
    if (source.id.indexOf("virtual.") === 0)
        return "Nothing is linked to it on this machine."
    return "This machine doesn't report it."
}
