import QtQuick

// One choice in a SegmentedControl: a glyph, a label or both.
QtObject {
    property string text
    property string symbol
    property string tooltip
    property bool enabled: true
}
