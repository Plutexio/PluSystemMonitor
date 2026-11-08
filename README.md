# PluSystemMonitor

A lightweight, good-looking system monitor for Linux. A small daemon collects
metrics, and a Material 3 (Qt Quick) app shows them, for this machine or any
other on your network.

## What it can do

- **Overview dashboard**: CPU, memory, GPU, temperatures and more as gauges,
  graphs and values on a grid you arrange yourself.
- **Processes and Apps**: everything running, grouped into the apps and Steam
  games you started, with search, sorting and End/Kill.
- **GPUs**: NVIDIA (incl. per-process usage) and AMD.
- **Sensors**: every hwmon sensor, with your own names and scaling, plus
  virtual sensors that work the same across different machines.
- **Fan control (PluFan)**: fixed speeds or temperature curves, run by the
  daemon, with fans safely handed back to the BIOS when anything goes wrong.
- **Remote machines**: connect to any machine running the daemon, always over
  TLS, with SSH-style fingerprint checks.
- **Updates from the app**: every connected machine can update itself.
- **Themes**: follows your wallpaper's colours (Matugen) or picks from
  Catppuccin, Gruvbox, Nord, Tokyo Night and others, or your own.
- **Plasma widget**: any metric on the desktop or a panel.

## Installing

You need Rust, a C compiler and Qt 6.6+ (`qt6-base`, `qt6-declarative`).

```sh
git clone https://github.com/Plutexio/PluSystemMonitor
cd PluSystemMonitor
./install.sh
```

It asks whether you want the monitor, fan control or both, installs everything
to `~/.local` and starts the daemon as a systemd user service. Open
**PluSystemMonitor** (or **PluFan**) from the application menu.

Run `./install.sh` again to update (or use **Updates** in the app), and
`./install.sh --uninstall` to remove it. `./install.sh --help` lists the other
options.

### On a server

Install just the daemon (no Qt needed), and let it listen on the network:

```sh
./install.sh --daemon-only
systemctl --user edit plusysmond
```

```ini
[Service]
Environment="PLUSYSMOND_ARGS=--listen 0.0.0.0:7878"
```

Then connect from the app with the server's address and the token from
`~/.config/plusysmon/token` on the server.

## Development

```sh
cargo run --release -p plusysmond   # daemon, listens on 127.0.0.1:7878
cargo run --release -p plusysmon    # app, connects to it automatically
cargo test
```

The workspace has the daemon (`crates/daemon`), the app (`crates/frontend`),
the fan helper (`crates/fand`) and the shared protocol (`crates/protocol`).
