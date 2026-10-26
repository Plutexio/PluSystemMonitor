import QtQuick
import PluSystemMonitor

// A springy settle with a little overshoot: a press that squishes and a
// release that springs back, a card that lifts. Material 3 Expressive's
// spatial springs, as close as SpringAnimation comes.
SpringAnimation {
    spring: 7
    damping: 0.24
    epsilon: 0.002
}
