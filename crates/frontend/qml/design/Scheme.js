.pragma library

// One Material 3 colour scheme as a plain object: the 49 roles Material
// defines (from `Config.scheme`), the layer, hover and pressed colours end4
// derives from them, and the colours the data is drawn in. States are mixes
// of what is there, never new hues. (An object rather than a QML type: QML
// can't declare properties named like `onPrimary`, which it reads as signal
// handlers.)

// Material's baseline dark scheme, for the moment before the first one comes.
var baseline = {
    dark: true,
    roles: {
        background: "#141218", onBackground: "#e6e0e9", surface: "#141218", surfaceDim: "#141218",
        surfaceBright: "#3b383e", surfaceContainerLowest: "#0f0d13", surfaceContainerLow: "#1d1b20",
        surfaceContainer: "#211f26", surfaceContainerHigh: "#2b2930", surfaceContainerHighest: "#36343b",
        onSurface: "#e6e0e9", surfaceVariant: "#49454f", onSurfaceVariant: "#cac4d0",
        inverseSurface: "#e6e0e9", inverseOnSurface: "#322f35", outline: "#938f99", outlineVariant: "#49454f",
        shadow: "#000000", scrim: "#000000", surfaceTint: "#d0bcff", primary: "#d0bcff", onPrimary: "#381e72",
        primaryContainer: "#4f378b", onPrimaryContainer: "#eaddff", inversePrimary: "#6750a4",
        secondary: "#ccc2dc", onSecondary: "#332d41", secondaryContainer: "#4a4458",
        onSecondaryContainer: "#e8def8", tertiary: "#efb8c8", onTertiary: "#492532",
        tertiaryContainer: "#633b48", onTertiaryContainer: "#ffd8e4", error: "#f2b8b5", onError: "#601410",
        errorContainer: "#8c1d18", onErrorContainer: "#f9dedc", primaryFixed: "#eaddff",
        primaryFixedDim: "#d0bcff", onPrimaryFixed: "#21005d", onPrimaryFixedVariant: "#4f378b",
        secondaryFixed: "#e8def8", secondaryFixedDim: "#ccc2dc", onSecondaryFixed: "#1d192b",
        onSecondaryFixedVariant: "#4a4458", tertiaryFixed: "#ffd8e4", tertiaryFixedDim: "#efb8c8",
        onTertiaryFixed: "#31111d", onTertiaryFixedVariant: "#633b48"
    },
    data: {
        cpu: "#d0bcff", memory: "#7dd3e8", swap: "#f7b0d0", gpu: "#bfe58a", network: "#a8c8ff",
        disk: "#ffb68a", good: "#8fd8b0", warn: "#f2c66d"
    }
}

// `amount` of `a`, the rest of `b` (end4's ColorUtils.mix).
function mix(a, b, amount) {
    const t = Math.max(0, Math.min(1, amount))
    return Qt.rgba(a.r * t + b.r * (1 - t), a.g * t + b.g * (1 - t), a.b * t + b.b * (1 - t), a.a * t + b.a * (1 - t))
}

function build(json) {
    const s = { dark: json.dark }
    for (const key in json.roles)
        s[key] = Qt.color(json.roles[key])
    for (const key in json.data)
        s[key] = Qt.color(json.data[key])

    s.subtext = s.outline

    // End4's layers: what sits on what, from the window up.
    s.layer0 = s.background
    s.onLayer0 = s.onBackground
    s.layer0Hover = mix(s.background, s.onBackground, 0.9)
    s.layer0Active = mix(s.background, s.onBackground, 0.8)
    s.layer0Border = mix(s.outlineVariant, s.background, 0.4)
    s.layer1 = s.surfaceContainerLow
    s.onLayer1 = s.onSurfaceVariant
    s.onLayer1Inactive = mix(s.onSurfaceVariant, s.surfaceContainerLow, 0.45)
    s.layer1Hover = mix(s.surfaceContainerLow, s.onSurfaceVariant, 0.92)
    s.layer1Active = mix(s.surfaceContainerLow, s.onSurfaceVariant, 0.85)
    s.layer2 = s.surfaceContainer
    s.onLayer2 = s.onSurface
    s.layer2Hover = mix(s.surfaceContainer, s.onSurface, 0.9)
    s.layer2Active = mix(s.surfaceContainer, s.onSurface, 0.8)
    s.layer2Disabled = mix(s.surfaceContainer, s.background, 0.8)
    s.onLayer2Disabled = mix(s.onSurface, s.background, 0.4)
    s.layer3 = s.surfaceContainerHigh
    s.onLayer3 = s.onSurface
    s.layer3Hover = mix(s.surfaceContainerHigh, s.onSurface, 0.9)
    s.layer3Active = mix(s.surfaceContainerHigh, s.onSurface, 0.8)
    s.layer4 = s.surfaceContainerHighest
    s.onLayer4 = s.onSurface
    s.layer4Hover = mix(s.surfaceContainerHighest, s.onSurface, 0.9)
    s.layer4Active = mix(s.surfaceContainerHighest, s.onSurface, 0.8)

    s.primaryHover = mix(s.primary, s.layer1Hover, 0.87)
    s.primaryActive = mix(s.primary, s.layer1Active, 0.7)
    s.primaryContainerHover = mix(s.primaryContainer, s.onPrimaryContainer, 0.9)
    s.primaryContainerActive = mix(s.primaryContainer, s.onPrimaryContainer, 0.8)
    s.secondaryHover = mix(s.secondary, s.layer1Hover, 0.85)
    s.secondaryActive = mix(s.secondary, s.layer1Active, 0.4)
    s.secondaryContainerHover = mix(s.secondaryContainer, s.onSecondaryContainer, 0.9)
    s.secondaryContainerActive = mix(s.secondaryContainer, s.onSecondaryContainer, 0.54)
    s.tertiaryHover = mix(s.tertiary, s.layer1Hover, 0.85)
    s.tertiaryActive = mix(s.tertiary, s.layer1Active, 0.4)
    s.tertiaryContainerHover = mix(s.tertiaryContainer, s.onTertiaryContainer, 0.9)
    s.tertiaryContainerActive = mix(s.tertiaryContainer, s.layer1Active, 0.54)
    s.surfaceContainerHighestHover = mix(s.surfaceContainerHighest, s.onSurface, 0.95)
    s.surfaceContainerHighestActive = mix(s.surfaceContainerHighest, s.onSurface, 0.85)
    s.tooltip = s.inverseSurface
    s.onTooltip = s.inverseOnSurface
    s.scrimOverlay = Qt.alpha(s.scrim, 0.5)
    s.shadowColor = Qt.alpha(s.shadow, 0.3)
    s.errorHover = mix(s.error, s.layer1Hover, 0.85)
    s.errorActive = mix(s.error, s.layer1Active, 0.7)
    s.errorContainerHover = mix(s.errorContainer, s.onErrorContainer, 0.9)
    s.errorContainerActive = mix(s.errorContainer, s.onErrorContainer, 0.7)

    // A data colour by its key (cpu, memory, swap, gpu, network, disk).
    s.accent = function (key) {
        switch (key) {
        case "memory": return s.memory
        case "swap": return s.swap
        case "gpu": return s.gpu
        case "network": return s.network
        case "disk": return s.disk
        default: return s.cpu
        }
    }
    // A tonal ground for something drawn in `tint`, as a container is for
    // its accent.
    s.tonal = function (tint) {
        return mix(Qt.color(tint), s.surfaceContainer, s.dark ? 0.24 : 0.2)
    }
    return s
}
