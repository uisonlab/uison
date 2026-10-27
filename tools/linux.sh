#!/bin/bash
# Build and check an example on Linux in a compositor without a screen (headless):
#   tools/linux.sh <example> [scenario.json] [-c mutter|sway|weston|kwin|x11|all] [-s scale] [-t seconds] [-e VAR=value]…
#                  [-b command] [-a 'pattern|command'] [-p package]… [-x 'arguments']
# <example> — a Cargo package or a path to a built program (C/C++ from examples-c, examples-cpp).
# The scenario comes from scenarios/ or web/scenarios/ (or a path). Without a scenario the example runs for -t seconds
# (3 by default) with UISON_CAPTURE screenshots. Screenshots and the log go to out/linux/<compositor>/<example>/.
# -b — a command in the compositor's environment before the example starts (wl-copy, xclip), output in before.txt;
# -a — a command run when the pattern appears in the example's log (the example is still running): wl-paste, xclip -o → after.txt.
# -p — one more Cargo package to build; -x — the example's arguments, @bin@ is the folder of built programs:
#   tools/linux.sh uison-player player-du-basic.json -c sway -p disk-usage -x '@bin@/disk-usage --console /usr/lib'
# On a Mac the command goes into the OrbStack machine (tools/linux-setup.sh installs its packages).
set -e
cd "$(dirname "$0")/.."

if [ "$(uname)" = Darwin ]; then
  # The folder shared with the Mac shows fresh edits to the machine with a delay — cargo might miss them.
  sleep 1
  exec orb -m "${UISON_ORB:-uison}" bash -l "tools/linux.sh" "$@"
fi

# The VS Code terminal from snap replaces GSettings schemas and GTK paths — mutter crashes with them.
if [ -n "$SNAP" ]; then
  unset GSETTINGS_SCHEMA_DIR LOCPATH GTK_PATH GTK_EXE_PREFIX GTK_IM_MODULE_FILE GIO_MODULE_DIR GTK_MODULES
  [ -n "$XDG_DATA_DIRS_VSCODE_SNAP_ORIG" ] && export XDG_DATA_DIRS=$XDG_DATA_DIRS_VSCODE_SNAP_ORIG
fi

example=$1; shift || true
[ -n "$example" ] || { sed -n '2,6p' "$0"; exit 1; }
scenario=""; comps=mutter; scale=1; secs=3; envs=(); before=""; after=""; pkgs=(); xargs=""
while [ $# -gt 0 ]; do
  case $1 in
    -c) comps=$2; shift 2 ;;
    -s) scale=$2; shift 2 ;;
    -t) secs=$2; shift 2 ;;
    -e) envs+=("$2"); shift 2 ;;
    -b) before=$2; shift 2 ;;
    -a) after=$2; shift 2 ;;
    -p) pkgs+=(-p "$2"); shift 2 ;;
    -x) xargs=$2; shift 2 ;;
    *) scenario=$1; shift ;;
  esac
done
[ "$comps" = all ] && comps="mutter sway weston kwin x11"

arch=$(uname -m); target=$arch-unknown-linux-gnu
if [[ $example == */* ]]; then
  # A path to a built program (a C/C++ example from examples-c or examples-cpp, built by CMake).
  bin=$(realpath "$example"); example=$(basename "$example")
else
  cargo build --release -q -p "$example" "${pkgs[@]}" --target "$target"
  bin=$PWD/target/$target/release/$example
fi
# shellcheck disable=SC2206 # the arguments are split into words on purpose
args=(${xargs//@bin@/$PWD/target/$target/release})

if [ -n "$scenario" ]; then
  for f in "$scenario" "scenarios/$scenario" "web/scenarios/$scenario"; do
    [ -f "$f" ] && { scenario=$(realpath "$f"); break; }
  done
  [ -f "$scenario" ] || { echo "no scenario $scenario"; exit 1; }
fi

rt=${XDG_RUNTIME_DIR:-/run/user/$(id -u)}
comp_pid=""
stop_comp() {
  if [ -n "$comp_pid" ]; then
    kill "$comp_pid" 2>/dev/null || true
    wait "$comp_pid" 2>/dev/null || true
  fi
  comp_pid=""
}
trap stop_comp EXIT

# Start the compositor and wait for its socket; sets WAYLAND_DISPLAY or DISPLAY.
start_comp() {
  local c=$1 log=$2
  unset WAYLAND_DISPLAY DISPLAY
  case $c in
    weston)
      rm -f "$rt/uison-weston"*
      weston --backend=headless --socket=uison-weston --width=1600 --height=1000 --scale="${scale%.*}" >"$log" 2>&1 &
      comp_pid=$!; sock=uison-weston ;;
    mutter)
      rm -f "$rt/uison-mutter"*
      dbus-run-session -- mutter --headless --wayland --no-x11 --wayland-display=uison-mutter --virtual-monitor 1600x1000@60 >"$log" 2>&1 &
      comp_pid=$!; sock=uison-mutter ;;
    sway)
      printf 'xwayland disable\noutput HEADLESS-1 resolution 1600x1000 scale %s\n' "$scale" > /tmp/uison-sway.conf
      local before; before=$(ls "$rt" | grep -E '^wayland-[0-9]+$' || true)
      WLR_BACKENDS=headless WLR_RENDERER=pixman WLR_LIBINPUT_NO_DEVICES=1 sway -c /tmp/uison-sway.conf >"$log" 2>&1 &
      comp_pid=$!; sock=""
      # For swaymsg in -a commands (window position etc.).
      export SWAYSOCK=$rt/sway-ipc.$(id -u).$comp_pid.sock
      for _ in $(seq 50); do
        sock=$(comm -13 <(echo "$before") <(ls "$rt" | grep -E '^wayland-[0-9]+$') | head -1)
        [ -n "$sock" ] && break; sleep 0.1
      done ;;
    kwin)
      rm -f "$rt/uison-kwin"*
      dbus-run-session -- kwin_wayland --virtual --no-lockscreen --socket uison-kwin --width 1600 --height 1000 --scale "$scale" >"$log" 2>&1 &
      comp_pid=$!; sock=uison-kwin ;;
    x11)
      Xvfb :71 -noreset -screen 0 1600x1000x24 >"$log" 2>&1 &
      comp_pid=$!
      for _ in $(seq 50); do [ -S /tmp/.X11-unix/X71 ] && break; sleep 0.1; done
      export DISPLAY=:71
      # X11 scale is Xft.dpi (the way GNOME/KDE/Xfce set it).
      if [ "$scale" != 1 ]; then
        xprop -root -f RESOURCE_MANAGER 8s -set RESOURCE_MANAGER "Xft.dpi:	$(echo "$scale * 96" | bc | cut -d. -f1)"
      fi
      # Wait until openbox has fully started (--startup runs at the end of startup): a window
      # opened earlier gets grabbed and never shown (_NET_SUPPORTING_WM_CHECK is set too early).
      rm -f "$rt/uison-openbox-ready"
      (openbox --startup "touch $rt/uison-openbox-ready" >/dev/null 2>&1 &)
      for _ in $(seq 100); do [ -f "$rt/uison-openbox-ready" ] && break; sleep 0.05; done
      return 0 ;;
    *) echo "unknown compositor $c"; return 1 ;;
  esac
  for _ in $(seq 50); do [ -n "$sock" ] && [ -S "$rt/$sock" ] && break; sleep 0.1; done
  [ -S "$rt/$sock" ] || { echo "$c failed to start:"; tail -5 "$log"; return 1; }
  export WAYLAND_DISPLAY=$sock
}

status=0
for c in $comps; do
  out=out/linux/$c/$example
  rm -rf "$out"; mkdir -p "$out"
  start_comp "$c" "$out/compositor.log" || { status=1; continue; }
  run_env=(UISON_STATS=1 UISON_CAPTURE="$out" UISON_HITMAP="$out/hitmap.txt" "${envs[@]}")
  [ -n "$before" ] && { bash -c "$before" >"$out/before.txt" 2>&1 || true; }
  if [ -n "$scenario" ]; then
    env "${run_env[@]}" UISON_SCRIPT="$scenario" timeout 120 "$bin" "${args[@]}" >"$out/log.txt" 2>&1 &
    app_pid=$!
    if [ -n "$after" ]; then
      for _ in $(seq 600); do
        if grep -q -- "${after%%|*}" "$out/log.txt"; then
          timeout 5 bash -c "${after#*|}" >"$out/after.txt" 2>&1 || true
          break
        fi
        kill -0 $app_pid 2>/dev/null || break
        sleep 0.1
      done
    fi
    wait $app_pid || status=$?
  else
    env "${run_env[@]}" timeout "$secs" "$bin" "${args[@]}" >"$out/log.txt" 2>&1 &
    app_pid=$!
    # A screenshot of the compositor (with the window frame) where there is a tool for it.
    sleep "$(echo "$secs - 0.7" | bc)"
    case $c in
      sway) grim "$out/screen.png" 2>/dev/null || true ;;
      x11) xwd -root -silent 2>/dev/null | convert xwd:- "$out/screen.png" 2>/dev/null || true ;;
    esac
    wait $app_pid || true
  fi
  stop_comp
  shots=$(ls "$out"/*.png 2>/dev/null | wc -l)
  echo "== $c: screenshots: $shots, uison messages: $(grep -c 'uison: ' "$out/log.txt" || true); log $out/log.txt"
  grep -E "uison: (done|bad|Wayland|compositor)|error|panicked" "$out/log.txt" | head -5 || true
done
exit $status
