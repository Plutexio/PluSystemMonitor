import QtQuick
import PluSystemMonitor

// The squiggle that runs on from a section's title to the end of its row,
// in `outlineVariant`. Straight with no shapes.
WavyProgress {
    value: 1
    color: Appearance.colors.outlineVariant
    trackColor: "transparent"
    thickness: 2
    amplitude: Appearance.noShapes ? 0 : 2
    wavelength: 14
    height: 8
}
