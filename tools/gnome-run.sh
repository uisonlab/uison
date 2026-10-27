#!/bin/bash
# Run a program in the real GNOME session with the session's own environment, outside VS Code's snap:
#   tools/gnome-run.sh <program> [arguments…]
# The snap terminal leaks GTK/GIO paths (zenity, gnome-text-editor crash on core20 libraries) and its cgroup (the portal
# answers slowly). The AppArmor label snap.code.code stays even here — GNOME then shows the window under the VS Code
# icon; for the dock and Alt+Tab start the program as a service: systemd-run --user -q --collect <program>.
mapfile -t envs < <(systemctl --user show-environment)
exec systemd-run --user --scope -q env -i "${envs[@]}" PATH="$HOME/.cargo/bin:/usr/local/bin:/usr/bin:/bin" "$@"
