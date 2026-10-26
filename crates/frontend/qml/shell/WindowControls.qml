import QtQuick
import PluSystemMonitor

// Minimise, maximise and close for the frameless window, each on a shape of
// its own like the top bar's other buttons.
Row {
    id: root

    required property Window window

    spacing: 4

    ToolbarButton {
        size: 36
        iconSize: 18
        shape: "pill"
        turnsOnHover: false
        symbol: "remove"
        tooltip: qsTr("Minimise")
        onClicked: root.window.showMinimized()
    }
    ToolbarButton {
        size: 36
        iconSize: 16
        shape: "cookie6"
        symbol: root.window.visibility === Window.Maximized ? "close_fullscreen" : "open_in_full"
        tooltip: root.window.visibility === Window.Maximized ? qsTr("Restore") : qsTr("Maximise")
        onClicked: root.window.visibility === Window.Maximized ? root.window.showNormal() : root.window.showMaximized()
    }
    ToolbarButton {
        size: 36
        iconSize: 18
        shape: "cookie9"
        shapeHoverColor: Appearance.colors.errorContainer
        shapePressedColor: Appearance.colors.errorContainerActive
        symbol: "close"
        tooltip: qsTr("Close Window")
        onClicked: root.window.close()
    }
}
