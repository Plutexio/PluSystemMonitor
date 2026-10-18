# PluSystemMonitor

A lightweight, good-looking system monitor split into a daemon and a frontend.

- **`plusysmond`** (`crates/daemon`) collects metrics from `/proc`, keeps a history
  buffer and streams samples over WebSocket (MessagePack frames).
- **`plusysmon`** (`crates/frontend`) is a Qt Quick / QML app (via cxx-qt) that can
  connect to a daemon on this machine or any other.
- **`plusysmon-protocol`** (`crates/protocol`) holds the shared message types.

## Running

```sh
cargo run --release -p plusysmond          # listens on 127.0.0.1:7878
cargo run --release -p plusysmon           # connects to 127.0.0.1 automatically
```

Requires Qt 6.6+ (`qt6-base`, `qt6-declarative`).

### Remote machines

```sh
plusysmond --listen 0.0.0.0:7878
cat ~/.config/plusysmon/token              # paste this into the frontend
```

On localhost the frontend reads the token file itself. Traffic is **not encrypted
yet**, so only expose the daemon on networks you trust.

## Themes

Built in: Aurora (default), Frost, Daylight. Pick one with the drop button in the
header. To make your own, put a TOML file in `~/.config/plusysmon/themes/` (the
"Open themes folder" button creates it). A theme only needs what it changes:

```toml
# ~/.config/plusysmon/themes/neon.toml
name = "Neon"
extends = "aurora"      # optional, Aurora is the default base

[colors]
primary = "#ff2d55"     # buttons, focus, selection

[accents]
cpu = "#22c55e"         # #rgb, #rrggbb or #rrggbbaa
```

Every key is listed, with comments, in `crates/frontend/themes/aurora.toml`.
Edits to the active theme apply live; mistakes (unknown keys, bad colours) show
up as an error in the app and in the theme list.

## Layout

"Edit layout" lets you drag widgets around a 12-column grid, resize them from the
corner handle, change their options and add or remove them. The result is saved
to `~/.config/plusysmon/layout.toml`. A layout file that can't be read is moved
to `layout.toml.broken` and the default layout is used.

Adding a widget type: create `crates/frontend/qml/widgets/MyWidget.qml` (it gets
`monitor` and `options`), register it in `qml/Widgets.qml` with its sizes and
options, and list the file in `crates/frontend/build.rs`.

## Processes

The Processes page lists everything running on the connected machine: search by
name, command, user or PID, sort by any column, pick columns and toggle kernel
threads in the columns menu. Selecting a process opens its details with Pause /
End (SIGTERM) / Kill (SIGKILL). Keyboard: arrows and Page Up/Down move the
selection, Delete ends the selected process, Esc closes the details.

The daemon only walks `/proc` while some client has this page open, and then
sends only what changed since the previous update. It can signal processes of
its own user; others are refused by the kernel, and PIDs 0 and 1 are never
signalled. Disk rates of other users' processes need privileges and show as "—".

Grouping processes into applications (including Steam/Proton games) is planned
as a separate Apps view.

## Development

- `cargo test` runs the parser, protocol, theme and layout tests.
- `plusysmon --screenshot out.png` renders the window, saves it after 5 s and quits.
- With stderr redirected, Qt logs to the journal instead; set
  `QT_FORCE_STDERR_LOGGING=1` to see QML warnings in the terminal.
- Design tokens are declared in `crates/frontend/qml/Theme.qml` and filled from
  the theme files.
