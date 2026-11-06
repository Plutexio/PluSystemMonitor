#!/usr/bin/env bash
# Builds PluSystemMonitor and installs the daemon with its systemd user service,
# plus what you pick: the system monitor app (with its Plasma widget), fan
# control (the PluFan app and the root helper plusysfand), or both. With
# --daemon-only, no apps: just the daemon, and fan control if asked for.
# Run ./install.sh --help for the options.
set -euo pipefail

usage() {
    cat <<EOF
Usage: ./install.sh [OPTIONS]

Options:
  --prefix <DIR>   Where to install [default: ~/.local]
                   A system prefix like /usr/local uses sudo for the copying.
  --monitor        Install the system monitor app and its Plasma widget
  --no-monitor     Leave it out (and remove it if it was installed)
  --fans           Install fan control: the PluFan app, and the helper that sets
                   fan speeds as root (asks for sudo; a system service)
  --no-fans        Leave it out (and remove it, giving the fans back to the BIOS)
                   Without any of these four, a terminal asks; otherwise the last
                   choice stands (the monitor alone, the first time).
  --daemon-only    No apps, for servers: doesn't need Qt, installs no menu entries,
                   widget or fonts, and keeps the service running after you log out
                   (loginctl enable-linger). --fans still adds fan control, set up
                   from an app on another machine. With --uninstall, removes only
                   the daemon (and fan control).
  --no-build       Install what is already in target/release
  --no-service     Don't enable and start the daemon's systemd user service
  --no-updates     Don't let frontends update this machine (pull, build, install
                   from this clone). Remembered until --updates turns it on again.
  --updates        Allow that again (the default)
  --keep-fan-helper  Don't build or reinstall the fan helper (what updates do: it
                   needs sudo, and the one installed keeps working)
  --uninstall      Remove everything this script installed (keeps ~/.config/plusysmon)
  -h, --help       Print this help
EOF
}

prefix="$HOME/.local"
build=1
service=1
uninstall=0
daemon_only=0
updates=""
monitor=""
fans=""
keep_helper=0
while (($#)); do
    case "$1" in
        --prefix) prefix="${2:?--prefix needs a value}"; shift ;;
        --prefix=*) prefix="${1#*=}" ;;
        --no-build) build=0 ;;
        --no-service) service=0 ;;
        --daemon-only) daemon_only=1 ;;
        --no-updates) updates=0 ;;
        --updates) updates=1 ;;
        --monitor) monitor=1 ;;
        --no-monitor) monitor=0 ;;
        --fans) fans=1 ;;
        --no-fans) fans=0 ;;
        --keep-fan-helper) keep_helper=1 ;;
        --uninstall) uninstall=1 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "unknown option: $1" >&2; usage >&2; exit 1 ;;
    esac
    shift
done

prefix="${prefix%/}"
src="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
bindir="$prefix/bin"
appdir="$prefix/share/applications"
icondir="$prefix/share/icons/hicolor/scalable/apps"
unitdir="$prefix/share/systemd/user"  # in systemd's user unit search path for ~/.local, /usr/local and /usr
fontdir="$prefix/share/plusysmon/fonts"
plasmoid_id="io.github.plutexio.plusysmon"
plasmoiddir="$prefix/share/plasma/plasmoids/$plasmoid_id"
# Tells the daemon where it came from, so it can update itself when asked.
record="$prefix/share/plusysmon/install.conf"
# The fan helper runs as root, so it lives where only root may change it,
# never in a user's prefix: whoever could replace it would get root.
helper="/usr/local/libexec/plusysfand"
helper_unit=/etc/systemd/system/plusysfand.service
helper_conf=/etc/plusysmon/fand.conf

say() { printf '\033[1;35m::\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }

# Copy into the prefix with sudo only when it isn't ours to write.
sudo=()
probe="$prefix"
while [[ ! -e "$probe" ]]; do probe="$(dirname "$probe")"; done
if [[ ! -w "$probe" ]]; then
    sudo=(sudo)
fi

# The user service can only be managed from the user's own session.
user_systemd=1
if [[ $EUID -eq 0 ]] || ! systemctl --user show-environment &>/dev/null; then
    user_systemd=0
fi

refresh_menus() {
    if command -v update-desktop-database &>/dev/null; then
        "${sudo[@]}" update-desktop-database -q "$appdir" || true
    fi
    if [[ -f "$prefix/share/icons/hicolor/index.theme" ]] && command -v gtk-update-icon-cache &>/dev/null; then
        "${sudo[@]}" gtk-update-icon-cache -q -t -f "$prefix/share/icons/hicolor" || true
    fi
    if command -v kbuildsycoca6 &>/dev/null; then
        kbuildsycoca6 &>/dev/null || true
    fi
}

# Stopping the helper gives every fan back to the firmware.
remove_fan_helper() {
    if [[ -e "$helper_unit" || -e "$helper" ]]; then
        say "Removing the fan helper (fans go back to the BIOS)"
        sudo systemctl disable --now plusysfand.service 2>/dev/null || true
        sudo rm -f "$helper" "$helper_unit" "$helper_conf"
        sudo rmdir /etc/plusysmon 2>/dev/null || true
        sudo systemctl daemon-reload || true
    fi
}

if ((uninstall)); then
    # The helper serves the whole machine; only the install that set it up removes it.
    if [[ -f "$record" ]] && grep -qx 'fans=1' "$record"; then
        remove_fan_helper
    fi
    # Only the service this prefix installed: another prefix's daemon isn't ours to stop.
    if ((user_systemd)) &&
        [[ "$(systemctl --user show -p FragmentPath --value plusysmond.service 2>/dev/null)" == "$unitdir/plusysmond.service" ]]; then
        say "Stopping the daemon service"
        systemctl --user disable --now plusysmond.service || true
    fi
    say "Removing files from $prefix"
    "${sudo[@]}" rm -f "$bindir/plusysmond" "$unitdir/plusysmond.service" "$record"
    if ((user_systemd)); then
        systemctl --user daemon-reload
    fi
    if ((!daemon_only)); then
        "${sudo[@]}" rm -f "$bindir/plusysmon" "$appdir/plusysmon.desktop" "$icondir/plusysmon.svg" \
            "$bindir/plusysfan" "$appdir/plusysfan.desktop" "$icondir/plusysfan.svg"
        "${sudo[@]}" rm -rf "$prefix/share/plusysmon" "$plasmoiddir"
        refresh_menus
    fi
    say "Done. Settings, layout, token and certificate are still in ~/.config/plusysmon."
    exit 0
fi

# What to install: the flags, else a question in a terminal, else what was
# installed last time, else the monitor alone.
recorded() {
    if [[ -f "$record" ]]; then
        sed -n "s/^$1=//p" "$record" | tail -1
    fi
}
if [[ -z "$monitor" && -z "$fans" && -t 0 && -t 1 ]]; then
    if ((daemon_only)); then
        read -rp "Also install fan control (needs sudo once)? [y/N] " answer
        if [[ "$answer" == [yY]* ]]; then fans=1; else fans=0; fi
    else
        echo "What should be installed?"
        echo "  1) System monitor"
        echo "  2) Fan control (PluFan; needs sudo once for its helper)"
        echo "  3) Both"
        read -rp "Choice [1]: " answer
        case "$answer" in
            2) monitor=0; fans=1 ;;
            3) monitor=1; fans=1 ;;
            *) monitor=1; fans=0 ;;
        esac
    fi
fi
had_fans="$(recorded fans)"
fans="${fans:-$had_fans}"
fans="${fans:-0}"
if [[ -z "$monitor" ]]; then
    monitor="$(recorded monitor)"
    if [[ -z "$monitor" ]]; then
        # A first install of fan control alone leaves the monitor out; records
        # from before fan control had it with every desktop install.
        if [[ ! -f "$record" && "$fans" == 1 ]]; then
            monitor=0
        else
            monitor=$((!daemon_only))
        fi
    fi
fi
if ((daemon_only)); then
    monitor=0
fi
# One program serves both apps; servers get neither.
app=0
if ((!daemon_only && (monitor || fans))); then
    app=1
fi
install_helper=0
if ((fans && !keep_helper)); then
    install_helper=1
    # Asked now rather than after a long build.
    if [[ $EUID -ne 0 ]]; then
        say "Fan control's helper runs as root, so installing it needs sudo"
        sudo -v
    fi
fi

if ((build)); then
    if [[ $EUID -eq 0 ]]; then
        warn "building as root; run the script as your user, it asks for sudo when it needs it"
    fi
    if ! command -v cargo &>/dev/null; then
        echo "cargo is missing; install Rust (https://rustup.rs) and a C compiler" >&2
        exit 1
    fi
    say "Building (release)"
    # The widget service serves the monitor's Plasma widget; fan control is a
    # feature of its own. A server builds neither D-Bus nor Qt.
    features=()
    if ((monitor)); then features+=(plusysmond/widgets); fi
    if ((fans)); then features+=(plusysmond/fans); fi
    packages=(-p plusysmond)
    if ((app)); then packages+=(-p plusysmon); fi
    if ((install_helper)); then packages+=(-p plusysfand); fi
    cargo build --release --manifest-path "$src/Cargo.toml" "${packages[@]}" \
        --no-default-features --features "$(IFS=,; echo "${features[*]}")"
fi

target="$(cargo metadata --format-version 1 --no-deps --manifest-path "$src/Cargo.toml" 2>/dev/null |
    sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p' || true)"
target="${target:-$src/target}/release"
bins=(plusysmond)
if ((app)); then bins+=(plusysmon); fi
if ((install_helper)); then bins+=(plusysfand); fi
for bin in "${bins[@]}"; do
    [[ -x "$target/$bin" ]] || { echo "$target/$bin is missing; run without --no-build" >&2; exit 1; }
done

say "Installing to $prefix"
# `install` replaces files by unlinking them first, so a running daemon is fine.
"${sudo[@]}" install -Dm755 "$target/plusysmond" "$bindir/plusysmond"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
sed "s|@BINDIR@|$bindir|g" "$src/packaging/plusysmond.service" >"$tmp/plusysmond.service"
"${sudo[@]}" install -Dm644 "$tmp/plusysmond.service" "$unitdir/plusysmond.service"

# An earlier --no-updates stands until --updates says otherwise.
if [[ -z "$updates" ]]; then
    updates=1
    if [[ -f "$record" ]] && grep -qx 'updates=0' "$record"; then
        updates=0
    fi
fi
cat >"$tmp/install.conf" <<EOF
# Written by install.sh. plusysmond reads it to update itself from this clone
# when a frontend asks: git pull, then install.sh again with these options.
source=$src
prefix=$prefix
daemon_only=$daemon_only
monitor=$monitor
fans=$fans
updates=$updates
path=$PATH
EOF
"${sudo[@]}" install -Dm644 "$tmp/install.conf" "$record"

# The interface's two faces, unless the system has them: Google Sans Flex for
# text and Material Symbols Rounded for icons. Pinned like Swiftify pins them.
fetch_font() {
    local family="$1" file="$2" url="$3" sum="$4"
    if fc-list -q "$family" 2>/dev/null || [[ -f "$fontdir/$file" ]]; then
        return
    fi
    if ! command -v curl &>/dev/null; then
        warn "curl is missing, so $family isn't installed; the app falls back to other faces"
        return
    fi
    say "Downloading $family"
    if curl -fsSL "$url" -o "$tmp/$file" && echo "$sum  $tmp/$file" | sha256sum -c --quiet -; then
        "${sudo[@]}" install -Dm644 "$tmp/$file" "$fontdir/$file"
    else
        warn "couldn't download $family; the app falls back to other faces"
    fi
}
if ((app)); then
    "${sudo[@]}" install -Dm755 "$target/plusysmon" "$bindir/plusysmon"
elif ((!daemon_only)); then
    "${sudo[@]}" rm -f "$bindir/plusysmon"
fi
if ((monitor)); then
    "${sudo[@]}" install -Dm644 "$src/packaging/plusysmon.svg" "$icondir/plusysmon.svg"
    sed "s|@BINDIR@|$bindir|g" "$src/packaging/plusysmon.desktop" >"$tmp/plusysmon.desktop"
    "${sudo[@]}" install -Dm644 "$tmp/plusysmon.desktop" "$appdir/plusysmon.desktop"

    # The Plasma widget (Add Widgets → PluSystemMonitor). Replaced whole, so
    # files a newer version dropped don't linger.
    "${sudo[@]}" rm -rf "$plasmoiddir"
    "${sudo[@]}" mkdir -p "$(dirname "$plasmoiddir")"
    "${sudo[@]}" cp -r "$src/packaging/plasmoid/$plasmoid_id" "$plasmoiddir"
elif ((!daemon_only)); then
    "${sudo[@]}" rm -f "$appdir/plusysmon.desktop" "$icondir/plusysmon.svg"
    "${sudo[@]}" rm -rf "$plasmoiddir"
fi
if ((fans && app)); then
    # PluFan is the same program under its own name: a hard link, not a
    # symlink. KWin names windows after /proc/<pid>/exe, which follows
    # symlinks, and the taskbar would then group PluFan under the monitor.
    "${sudo[@]}" ln -f "$bindir/plusysmon" "$bindir/plusysfan"
    "${sudo[@]}" install -Dm644 "$src/packaging/plusysfan.svg" "$icondir/plusysfan.svg"
    sed "s|@BINDIR@|$bindir|g" "$src/packaging/plusysfan.desktop" >"$tmp/plusysfan.desktop"
    "${sudo[@]}" install -Dm644 "$tmp/plusysfan.desktop" "$appdir/plusysfan.desktop"
elif ((!fans)); then
    "${sudo[@]}" rm -f "$bindir/plusysfan" "$appdir/plusysfan.desktop" "$icondir/plusysfan.svg"
fi
if ((!daemon_only)); then
    refresh_menus
fi

if ((install_helper)); then
    say "Installing the fan helper to $helper (sudo)"
    sudo install -Dm755 -o root -g root "$target/plusysfand" "$helper"
    sed "s|@HELPER@|$helper|g" "$src/packaging/plusysfand.service" >"$tmp/plusysfand.service"
    sudo install -Dm644 -o root -g root "$tmp/plusysfand.service" "$helper_unit"
    # Who may set fan speeds: whoever was allowed before, and this user.
    uids="$(id -u)"
    if [[ -f "$helper_conf" ]]; then
        uids="$({ sed -n 's/^users *= *//p' "$helper_conf" | tr ',' '\n' | tr -d ' '; id -u; } |
            grep -E '^[0-9]+$' | sort -un | paste -sd, -)"
    fi
    cat >"$tmp/fand.conf" <<EOF
# Written by install.sh. Users (by ID) whose plusysmond may set fan speeds
# through plusysfand; root always may. Restart plusysfand after changing it.
users = $uids
EOF
    sudo install -Dm644 -o root -g root "$tmp/fand.conf" "$helper_conf"
    sudo systemctl daemon-reload
    sudo systemctl enable plusysfand.service
    # Restart rather than start, so a new helper runs. The fans are the BIOS's
    # for a moment, until the daemon takes them again.
    sudo systemctl restart plusysfand.service
elif ((!fans)) && [[ "$had_fans" == 1 ]]; then
    remove_fan_helper
fi

if ((app)); then

    fetch_font "Google Sans Flex" GoogleSansFlex.ttf \
        "https://raw.githubusercontent.com/google/fonts/a0e3dbcdc3a3ecfafff3f071159ae0221628922d/ofl/googlesansflex/GoogleSansFlex%5BGRAD,ROND,opsz,slnt,wdth,wght%5D.ttf" \
        c31a482fbecbf2e07e6890134d20078723aadf732c9b9c6c9a44f86f8265b6fe
    fetch_font "Material Symbols Rounded" MaterialSymbolsRounded.ttf \
        "https://raw.githubusercontent.com/google/material-design-icons/737e3324305806514d7909874fa1818ae1808232/variablefont/MaterialSymbolsRounded%5BFILL,GRAD,opsz,wght%5D.ttf" \
        95b24392bb49efd1bc3e92cff4e2452ad094461bab7c97e7d8723fab97e330ca
    if [[ -f "$fontdir/GoogleSansFlex.ttf" && ! -f "$fontdir/OFL.txt" ]] && command -v curl &>/dev/null; then
        curl -fsSL "https://raw.githubusercontent.com/google/fonts/a0e3dbcdc3a3ecfafff3f071159ae0221628922d/ofl/googlesansflex/OFL.txt" \
            -o "$tmp/OFL.txt" && "${sudo[@]}" install -Dm644 "$tmp/OFL.txt" "$fontdir/OFL.txt" || true
    fi
fi

if ((service)); then
    if ((user_systemd)); then
        say "Starting the daemon (systemd user service plusysmond)"
        systemctl --user daemon-reload
        systemctl --user enable plusysmond.service
        # Restart rather than start, so an update runs the new binary.
        systemctl --user restart plusysmond.service
        sleep 1
        if [[ "$(systemctl --user is-active plusysmond.service)" != active ]]; then
            warn "the daemon didn't come up; see: journalctl --user -u plusysmond"
            if ss -Hltn 'sport = :7878' 2>/dev/null | grep -q .; then
                warn "port 7878 is taken, probably by a plusysmond started by hand." \
                    "Stop it and the service takes over within a few seconds."
            fi
        fi
        # User services normally end with the last login; a server's has to stay.
        if ((daemon_only)) && [[ "$(loginctl show-user "$(id -un)" -p Linger --value 2>/dev/null)" != yes ]]; then
            say "Keeping the service running without a login"
            loginctl enable-linger "$(id -un)" ||
                warn "couldn't do that, so the daemon stops when you log out. Ask an admin for: loginctl enable-linger $(id -un)"
        fi
    else
        warn "can't reach your systemd user session (running as root?). Start the daemon as your user with:"
        echo "    systemctl --user daemon-reload && systemctl --user enable --now plusysmond"
        if ((daemon_only)); then
            echo "    loginctl enable-linger \$USER   # keeps it running after you log out"
        fi
    fi
fi

if ((fans)) && ! compgen -G "/sys/class/hwmon/*/pwm[0-9]" >/dev/null && ! command -v nvidia-smi &>/dev/null; then
    warn "found no fan that software may control here. Many boards need the driver for" \
        "their fan chip first (often it87 or nct6775: sudo modprobe it87)."
fi

if ((daemon_only)); then
    say "Done. The daemon is in $bindir/plusysmond and only listens on this machine for now."
    cat <<EOF
To let other machines connect, add this with 'systemctl --user edit plusysmond':

    [Service]
    Environment="PLUSYSMOND_ARGS=--listen 0.0.0.0:7878"

then run 'systemctl --user restart plusysmond'. A frontend connecting from elsewhere needs:

    Token:        cat ~/.config/plusysmon/token
    Fingerprint:  $bindir/plusysmond --fingerprint
EOF
else
    if ((monitor)); then
        say "Done. PluSystemMonitor is in your application menu (or run $bindir/plusysmon)."
        echo "The PluSystemMonitor widget is under Add Widgets on the desktop or a panel. If Plasma"
        echo "was running an older version of it, log out and back in (or: systemctl --user restart plasma-plasmashell)."
    fi
    if ((fans)); then
        say "Fan control: PluFan is in your application menu (or run $bindir/plusysfan)."
    fi
    if ((!monitor && !fans)); then
        say "Done. Only the daemon is installed; ./install.sh --monitor or --fans adds an app."
    fi
fi
case ":$PATH:" in
    *":$bindir:"*) ;;
    *) warn "$bindir is not in your PATH, so '${bins[-1]}' only works with the full path" ;;
esac
