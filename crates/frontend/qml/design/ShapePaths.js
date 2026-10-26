.pragma library

.import "../third_party/rounded-polygon-qmljs/material-shapes.js" as MaterialShapes
.import "../third_party/rounded-polygon-qmljs/shapes/morph.js" as MorphModule

// Material 3 Expressive's shapes as SVG paths in a unit square, for
// MaterialShape. Each shape's path and each pair's morph is built once.

var getters = {
    circle: "getCircle", square: "getSquare", slanted: "getSlanted", arch: "getArch", fan: "getFan",
    arrow: "getArrow", semiCircle: "getSemiCircle", oval: "getOval", pill: "getPill",
    triangle: "getTriangle", diamond: "getDiamond", clamShell: "getClamShell", pentagon: "getPentagon",
    gem: "getGem", sunny: "getSunny", verySunny: "getVerySunny", cookie4: "getCookie4Sided",
    cookie6: "getCookie6Sided", cookie7: "getCookie7Sided", cookie9: "getCookie9Sided",
    cookie12: "getCookie12Sided", ghostish: "getGhostish", clover4: "getClover4Leaf",
    clover8: "getClover8Leaf", burst: "getBurst", softBurst: "getSoftBurst", boom: "getBoom",
    softBoom: "getSoftBoom", flower: "getFlower", puffy: "getPuffy", puffyDiamond: "getPuffyDiamond",
    pixelCircle: "getPixelCircle", pixelTriangle: "getPixelTriangle", bun: "getBun", heart: "getHeart"
}

var paths = {}
var morphs = {}

function polygon(name) {
    return MaterialShapes[getters[name] || "getCircle"]()
}

function number(value) {
    return Math.round(value * 10000) / 10000
}

function svg(cubics) {
    if (cubics.length === 0)
        return ""
    const parts = ["M", number(cubics[0].anchor0X), number(cubics[0].anchor0Y)]
    for (const c of cubics) {
        parts.push("C", number(c.control0X), number(c.control0Y), number(c.control1X), number(c.control1Y),
                   number(c.anchor1X), number(c.anchor1Y))
    }
    parts.push("Z")
    return parts.join(" ")
}

function path(name) {
    if (!(name in paths))
        paths[name] = svg(polygon(name).cubics)
    return paths[name]
}

// From one shape to another; a spring may carry `progress` a little past 1.
function morph(from, to, progress) {
    if (from === to || progress <= 0)
        return path(from)
    if (progress === 1)
        return path(to)
    const key = from + ">" + to
    if (!(key in morphs))
        morphs[key] = new MorphModule.Morph(polygon(from), polygon(to))
    return svg(morphs[key].asCubics(progress))
}
