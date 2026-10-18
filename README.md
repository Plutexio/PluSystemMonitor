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

### Widgets and data sources

Widgets don't hard-code what they show. Every number the daemon reports is a
**data source** with an ID, a name, a unit, a scale and two minutes of
history: `cpu.total`, `cpu.core.3`, `memory.used`, `memory.swap`,
`gpu.0.usage`, `gpu.0.vram`, `gpu.0.temperature`, `gpu.0.power`,
`gpu.0.core_clock`, `gpu.0.memory_clock`, `gpu.0.fan`, `gpu.0.encoder`,
`gpu.0.decoder` (the GPU ones exist when the driver reports them).

Apps are sources too: `app.<app id>.cpu`, `.memory`, `.gpu` and `.vram` for
every running app (e.g. `app.steam:1062090.gpu`), and `game.cpu`,
`game.memory`, `game.gpu`, `game.vram` for **whichever game is running** (the
one using the most GPU). A game widget can be set up before any game runs; its
title follows the game. Using app or game sources makes the daemon collect
processes while the Overview is open.

The generic widgets show whichever sources you pick in their settings:

- **Gauge**: the first source on a ring, the others as bars below it.
- **Graph**: up to four sources over time, each scaled to its own range, or
  to its recent peak with "Fit lines to their data" (handy for one app's
  share of the CPU, which is small next to the whole machine).
- **Value**: one source as a big number with a sparkline.

The title comes from the first source unless you type your own; colours follow
the sources unless you pick one. In `layout.toml`:

```toml
[[widgets]]
type = "graph"
x = 0
y = 0
w = 6
h = 4
options = { sources = ["gpu.0.usage", "gpu.0.temperature"], title = "Gaming" }
```

Layouts made before data sources existed (`cpu.gauge`, `gpu.graph`, ...) are
converted to these widgets when loaded.

New sources are added in `crates/frontend/src/metrics.rs`; every widget can
use them right away. A new kind of widget is a QML file in
`crates/frontend/qml/widgets/` (it gets `monitor` and `options`, and reads
sources with `Sources.read`), an entry in `qml/Widgets.qml`, and a line in
`crates/frontend/build.rs`.

## GPUs

- **NVIDIA** through NVML (`libnvidia-ml.so`, part of the driver), loaded at
  runtime: the daemon runs fine without it. Load, VRAM, temperature, power,
  clocks, fan, encoder/decoder, and per-process GPU load and video memory
  (Processes columns, app totals). NVML adds about 20 MiB to the daemon.
- **AMD** through amdgpu's sysfs files (`gpu_busy_percent`, VRAM, hwmon);
  names come from the PCI ID database. Not yet tested on real hardware, and
  without per-process figures for now.
- Intel GPUs are not shown yet: their load needs perf counters.

`cargo test -p plusysmond live_gpus -- --ignored --nocapture` prints what the
daemon sees on the current machine.

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

## Apps

The Apps page groups processes into the applications you started, with icons,
totals, a CPU sparkline per app, and End/Kill for the whole app. Clicking a
process in an app's details opens it on the Processes page.

How processes are assigned, in order:

1. **Steam games**: everything below Steam's `reaper SteamLaunch AppId=<id>`
   process. Games run inside Steam's own systemd unit, which is why
   cgroup-based monitors file them under Steam; the reaper doesn't lie. Names
   come from the library's `appmanifest_<id>.acf`, artwork and logo from
   Steam's library cache.
2. **Desktop apps**: the systemd unit desktop sessions start apps in
   (`app-<id>@….service`, `app-<launcher>-<id>-<pid>.scope`), matched to a
   `.desktop` file for the name and icon (looked up in your icon theme).
3. Everything else counts as a background process.

Known limits: programs started from a terminal belong to the terminal's app,
and all Chromium-based browsers share one unit (`org.chromium.Chromium`), so a
browser and its installed web apps can't be told apart.

Icons and artwork are fetched from the daemon (so they work for remote
machines too; it only serves files it registered itself) and cached in
`~/.cache/plusysmon/assets/`.

`cargo test -p plusysmond live_apps -- --ignored --nocapture` prints how the
current machine's processes are grouped.

## Development

- `cargo test` runs the parser, protocol, theme and layout tests.
- `plusysmon --screenshot out.png` renders the window, saves it after 5 s and quits.
- With stderr redirected, Qt logs to the journal instead; set
  `QT_FORCE_STDERR_LOGGING=1` to see QML warnings in the terminal.
- Design tokens are declared in `crates/frontend/qml/Theme.qml` and filled from
  the theme files.
