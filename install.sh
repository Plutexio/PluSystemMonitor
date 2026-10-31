#!/usr/bin/env bash
# Builds PluSystemMonitor and installs the app, its menu entry and the daemon's
# systemd user service, or with --daemon-only just the daemon and its service.
# Run ./install.sh --help for the options.
set -euo pipefail

usage() {
    cat <<EOF
Usage: ./install.sh [OPTIONS]

Options:
  --prefix <DIR>   Where to install [default: ~/.local]
                   A system prefix like /usr/local uses sudo for the copying.
  --daemon-only    Only the daemon, for servers: doesn't need Qt, installs no app,
                   menu entry or fonts, and keeps the service running after you
                   log out (loginctl enable-linger). With --uninstall, removes
                   only the daemon.
  --no-build       Install what is already in target/release
  --no-service     Don't enable and start the daemon's systemd user service
  --uninstall      Remove everything this script installed (keeps ~/.config/plusysmon)
  -h, --help       Print this help
EOF
}

prefix="$HOME/.local"
build=1
service=1
uninstall=0
daemon_only=0
while (($#)); do
    case "$1" in
        --prefix) prefix="${2:?--prefix needs a value}"; shift ;;
        --prefix=*) prefix="${1#*=}" ;;
        --no-build) build=0 ;;
        --no-service) service=0 ;;
        --daemon-only) daemon_only=1 ;;
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

if ((uninstall)); then
    if ((user_systemd)) && systemctl --user cat plusysmond.service &>/dev/null; then
        say "Stopping the daemon service"
        systemctl --user disable --now plusysmond.service || true
    fi
    say "Removing files from $prefix"
    "${sudo[@]}" rm -f "$bindir/plusysmond" "$unitdir/plusysmond.service"
    if ((user_systemd)); then
        systemctl --user daemon-reload
    fi
    if ((!daemon_only)); then
        "${sudo[@]}" rm -f "$bindir/plusysmon" "$appdir/plusysmon.desktop" "$icondir/plusysmon.svg"
        "${sudo[@]}" rm -rf "$prefix/share/plusysmon"
        refresh_menus
    fi
    say "Done. Settings, layout, token and certificate are still in ~/.config/plusysmon."
    exit 0
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
    packages=(-p plusysmond)
    ((daemon_only)) || packages+=(-p plusysmon)
    cargo build --release --manifest-path "$src/Cargo.toml" "${packages[@]}"
fi

target="$(cargo metadata --format-version 1 --no-deps --manifest-path "$src/Cargo.toml" 2>/dev/null |
    sed -n 's/.*"target_directory":"\([^"]*\)".*/\1/p' || true)"
target="${target:-$src/target}/release"
bins=(plusysmond)
((daemon_only)) || bins+=(plusysmon)
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
if ((!daemon_only)); then
    "${sudo[@]}" install -Dm755 "$target/plusysmon" "$bindir/plusysmon"
    "${sudo[@]}" install -Dm644 "$src/packaging/plusysmon.svg" "$icondir/plusysmon.svg"
    sed "s|@BINDIR@|$bindir|g" "$src/packaging/plusysmon.desktop" >"$tmp/plusysmon.desktop"
    "${sudo[@]}" install -Dm644 "$tmp/plusysmon.desktop" "$appdir/plusysmon.desktop"
    refresh_menus

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
    say "Done. PluSystemMonitor is in your application menu (or run $bindir/plusysmon)."
fi
case ":$PATH:" in
    *":$bindir:"*) ;;
    *) warn "$bindir is not in your PATH, so '${bins[-1]}' only works with the full path" ;;
esac
