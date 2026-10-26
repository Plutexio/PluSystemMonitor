pragma Singleton
import QtQuick
import PluSystemMonitor
import "Scheme.js" as Scheme

// Design tokens after end4's Appearance.qml (by way of Swiftify): colours,
// rounding, type, motion curves and sizes. Nothing outside this file names a
// colour; everything takes a role from `colors`.
QtObject {
    id: root

    /// The current scheme (Scheme.js): Material's roles, end4's layers and
    /// the data colours.
    property var colors: Scheme.build(Scheme.baseline)

    /// Takes a scheme from `Config.scheme`.
    function applyScheme(json: var) {
        colors = Scheme.build(json)
    }

    /// Stills what moves on its own, over and over. What answers a click, a
    /// hover or new data still moves. Performance mode includes it.
    property bool reduceMotionSetting: false
    /// As little drawn as can be: nothing animates, not even fades.
    property bool performanceMode: false
    /// No decorative shapes; ones that mean something stay
    /// (`MaterialShape.keepsShape`).
    property bool noShapes: false
    readonly property bool reduceMotion: reduceMotionSetting || performanceMode

    readonly property QtObject rounding: QtObject {
        readonly property int unsharpen: 2
        readonly property int unsharpenMore: 6
        readonly property int verySmall: 8
        readonly property int small: 12
        readonly property int normal: 17
        readonly property int large: 23
        readonly property int veryLarge: 30
        readonly property int full: 9999
        readonly property int window: 18
    }

    readonly property QtObject fontSize: QtObject {
        readonly property int smallest: 10
        readonly property int smaller: 12
        readonly property int smallie: 13
        readonly property int small: 15
        readonly property int normal: 16
        readonly property int large: 17
        readonly property int larger: 19
        readonly property int huge: 22
        readonly property int title: 22
        readonly property int headline: 30
        readonly property int display: 36
    }

    // End4's face, or the system's copy of it, or the one installed beside
    // the app (Config.fontFile); then the next face along.
    readonly property FontLoader interfaceFont: FontLoader {
        source: Qt.fontFamilies().includes("Google Sans Flex") ? "" : Config.fontFile("GoogleSansFlex.ttf")
    }
    readonly property FontLoader iconFont: FontLoader {
        source: Qt.fontFamilies().includes("Material Symbols Rounded") ? "" : Config.fontFile("MaterialSymbolsRounded.ttf")
    }
    readonly property string fontFamily: {
        interfaceFont.status
        const installed = Qt.fontFamilies()
        for (const family of ["Google Sans Flex", "Google Sans", "Rubik", "Inter", "Noto Sans"]) {
            if (installed.includes(family))
                return family
        }
        return Qt.application.font.family
    }
    readonly property string iconFontFamily: "Material Symbols Rounded"
    readonly property string monoFontFamily: "monospace"

    // End4's animationCurves: control points for Easing.BezierSpline.
    readonly property QtObject curves: QtObject {
        readonly property list<real> expressiveFastSpatial: [0.42, 1.67, 0.21, 0.90, 1, 1]
        readonly property list<real> expressiveDefaultSpatial: [0.38, 1.21, 0.22, 1.00, 1, 1]
        readonly property list<real> expressiveSlowSpatial: [0.39, 1.29, 0.35, 0.98, 1, 1]
        readonly property list<real> expressiveEffects: [0.34, 0.80, 0.34, 1.00, 1, 1]
        readonly property list<real> emphasized: [0.05, 0, 2 / 15, 0.06, 1 / 6, 0.4, 5 / 24, 0.82, 0.25, 1, 1, 1]
        readonly property list<real> emphasizedAccel: [0.3, 0, 0.8, 0.15, 1, 1]
        readonly property list<real> emphasizedDecel: [0.05, 0.7, 0.1, 1, 1, 1]
        readonly property list<real> standard: [0.2, 0, 0, 1, 1, 1]
        readonly property list<real> standardAccel: [0.3, 0, 1, 1, 1, 1]
        readonly property list<real> standardDecel: [0, 0, 0, 1, 1, 1]
    }

    readonly property QtObject sizes: QtObject {
        readonly property real gutter: 8
        readonly property real pageMargin: 32
        readonly property real sectionGap: 32
        readonly property real shelfGap: 16
        readonly property real topBarHeight: 64
        readonly property real sideSheetWidth: 320
        readonly property real sidebarWidth: 232
        readonly property real sidebarMinWidth: 132
        readonly property real sidebarMaxWidth: 270
    }

    /// A fade's or a colour's duration in ms; performance mode collapses it.
    function duration(milliseconds: int): int {
        return performanceMode ? 0 : milliseconds
    }

    /// A move's, a growth's or a turn's duration in ms; performance mode
    /// collapses it. Reduced motion keeps it: it stills only what loops.
    function motion(milliseconds: int): int {
        return performanceMode ? 0 : milliseconds
    }

    /// A state colour to ease to or from. "transparent" is black with no
    /// alpha, so an animation from it to a light colour runs through grey;
    /// clear, it takes `other`'s colour at no alpha instead and only fades.
    function fadeable(color: color, other: color): color {
        return color.a > 0 ? color : Qt.alpha(other, 0)
    }
}
