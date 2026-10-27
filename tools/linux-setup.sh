#!/bin/bash
# Packages for building and checking uison-linux on Ubuntu 24.04 (an OrbStack machine or a real one).
# Headless compositors, screenshot and input utilities, fonts. No -dev packages needed: libraries are dlopen'ed.
#   tools/linux-setup.sh          — the basics (Mutter, Sway, Weston, Xvfb)
#   tools/linux-setup.sh kwin     — plus KWin (heavy, ~1 GB)
set -e
[ "$(id -u)" = 0 ] || exec sudo "$0" "$@"
export DEBIAN_FRONTEND=noninteractive

pkgs=(
  build-essential curl ca-certificates git cmake pkgconf
  mutter sway weston xvfb openbox xdotool x11-apps
  grim wtype wayland-utils dbus dbus-user-session wl-clipboard xclip libgdk-pixbuf-2.0-0
  libcairo2 libpango-1.0-0 libpangocairo-1.0-0 libxkbcommon0 libxkbcommon-x11-0
  libxcb1 libxcb-shm0 libxcb-xkb1 libxcb-xinput0 libxcb-present0 libxcb-render0
  fonts-ubuntu fonts-noto-core fonts-noto-color-emoji fonts-dejavu-core
  xdg-desktop-portal xdg-desktop-portal-gtk adwaita-icon-theme
)
[ "$1" = kwin ] && pkgs+=(kwin-wayland)

apt-get update -q
apt-get install -y -q --no-install-recommends "${pkgs[@]}"

# Rust for the user, not root
user=${SUDO_USER:-$USER}
if ! sudo -u "$user" bash -lc 'command -v cargo' >/dev/null; then
  sudo -u "$user" bash -c 'curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal -c clippy'
fi
echo done
