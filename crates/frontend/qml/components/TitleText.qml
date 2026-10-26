import QtQuick
import PluSystemMonitor

// A title in end4's manner: Google Sans Flex drawn fully round, a touch wide
// and heavy through its variable axes, so headings read bubbly while body
// text stays plain. Without Google Sans Flex it is simply bold.
StyledText {
    /// display, headline or title: the app's name, a page's, a section's
    property string level: "title"

    font.pixelSize: level === "display" ? Appearance.fontSize.display
                  : level === "headline" ? Appearance.fontSize.headline : Appearance.fontSize.title
    font.weight: Font.Bold
    font.letterSpacing: level === "display" ? -0.4 : 0
    font.variableAxes: ({ "ROND": 100, "wdth": 110, "wght": 720 })
}
