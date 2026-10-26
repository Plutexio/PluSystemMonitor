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

### Installing

```sh
./install.sh                      # builds and installs to ~/.local
```

This puts `plusysmon` and `plusysmond` in `~/.local/bin`, adds
PluSystemMonitor to the application menu, and enables the daemon as a systemd
user service, so it runs whenever you are logged in:

```sh
systemctl --user status plusysmond
journalctl --user -u plusysmond   # its log
systemctl --user edit plusysmond  # extra options, see below
```

```ini
[Service]
Environment="PLUSYSMOND_ARGS=--listen 0.0.0.0:7878"
```

Run `./install.sh` again after pulling to update (it restarts the daemon).
Other options: `--prefix /usr/local` (asks for sudo), `--no-service`,
`--no-build`, `--uninstall` (keeps `~/.config/plusysmon`).

### Remote machines

```sh
plusysmond --listen 0.0.0.0:7878
cat ~/.config/plusysmon/token              # paste this into the frontend
```

On localhost the frontend reads the token file itself. Traffic is **not encrypted
yet**, so only expose the daemon on networks you trust.

## Look and themes

The interface follows the Swiftify style guide: Material 3 Expressive as end4's
dots-hyprland draws it, with Material colour roles, shapes from Material's shape
library, springy motion and round "bubbly" titles in Google Sans Flex. Icons are
Material Symbols Rounded. Both fonts are used from the system when installed,
otherwise from `<prefix>/share/plusysmon/fonts` (`install.sh` downloads them),
`~/.local/share/plusysmon/fonts` or Swiftify's copy.

Colours come from a theme, picked under **Appearance** in the drawer:

- **Desktop** (default): the Material scheme Matugen generated for your
  wallpaper (end4's quickshell, `~/.config/matugen`, DankMaterialShell, …),
  reloaded live when it changes. Without one it falls back to the app's accent.
- **PluSystemMonitor**: a scheme seeded from the app's violet.
- Rice palettes: Catppuccin, Rosé Pine, Gruvbox, Nord, Tokyo Night, Dracula,
  Everforest, Kanagawa, Solarized, One and a plain Default, each light and dark.

Light or dark follows the desktop unless set in the same dialog, which also has
Reduce Motion, Performance Mode and No Shapes. To make your own theme, put a TOML
file in `~/.config/plusysmon/themes/` (the "Open Themes Folder" button creates
it), either a seed colour:

```toml
# ~/.config/plusysmon/themes/neon.toml
name = "Neon"
seed = "#ff2d55"
variant = "vibrant"   # tonal_spot (default), content, fidelity, expressive, …
```

or the 13 colours of a rice palette, for `[dark]`, `[light]` or both (one is
used for both modes when the other is missing):

```toml
[dark]
lowest = "#11111b"    # the deepest ground
window = "#181825"
page = "#1e1e2e"
raised = "#313244"
raised_more = "#45475a"
line = "#585b70"
text = "#cdd6f4"
subtext = "#a6adc8"
muted = "#7f849c"
primary = "#cba6f7"
secondary = "#f5c2e7"
tertiary = "#94e2d5"
error = "#f38ba8"
```

The colours the data is drawn in (CPU, memory, GPU, …) keep their own hues and
are pulled toward each theme's primary. Edits to the active theme apply live;
mistakes show up as an error in the app and in the theme list.

## Layout

"Edit Layout" lets you drag widgets around a 12-column grid, resize them from the
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

## Sensors

Every hwmon sensor the kernel exposes becomes a data source
(`sensor.<device>.<feature>`, e.g. `sensor.coretemp.0.temp1`,
`sensor.it87.2624.fan1`, `sensor.nvme0.temp1`): temperatures, voltages,
fans, power and current, grouped by what they belong to (CPU, motherboard,
NVMe drives, network adapter, ...). IDs use the device, not the `hwmonN`
number, so they survive reboots. Fan headers and voltage inputs that never
read anything are left out. The Processor also gets `cpu.temperature`
(package), `cpu.frequency` (average clock), `cpu.frequency_peak` (fastest
core) and, when readable, `cpu.power`.

Things to know:

- **Motherboard voltages are raw.** Super I/O chips (it87, nct67xx, ...)
  measure most rails through resistor dividers the kernel doesn't know about,
  so e.g. the 12 V rail may read about 2 V and most inputs are unnamed. Rename
  a widget to what the input is on your board.
- **CPU power** comes from the RAPL energy counter, which only root can read
  (it leaks information through a power side channel, see PLATYPUS). Running
  the daemon as root shows it; so does making the file world-readable, e.g.
  with a tmpfiles rule `z /sys/class/powercap/intel-rapl:0/energy_uj 0444 - - -`,
  if you accept that trade-off on your machine.
- Fans controlled by software (fan curves, zero-RPM modes) legitimately read 0.

`cargo test -p plusysmond live_sensors -- --ignored --nocapture` lists what the
daemon finds.

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
- Design tokens are in `crates/frontend/qml/design/Appearance.qml`; the colour
  scheme is built by `design/Scheme.js` from the roles `src/theme.rs` computes.
  Components after Swiftify's are in `qml/components`, the window's shell in
  `qml/shell`, and shaders (baked with Qt's `qsb` at build time) in `qml/shaders`.
