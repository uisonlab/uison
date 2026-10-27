#!/bin/zsh
# A test scenario on the phone: the app runs the steps itself (uison_frame::script, the file is in the APK's assets),
# and this script takes screenshots on `uison: capture <name>` lines from logcat.
#   tools/android-test.sh <package | built program> <scenario.json> [VARIABLE=value ...]
# Screenshots: target/android/capture/<name>.png. The app's output goes to the terminal.
set -e
# Resolve paths from the launch folder (a built program of a crate outside the repository) before changing to the root.
[[ -e ${1:-} ]] && set -- ${1:A} "${@:2}"
[[ -f ${2:-} ]] && set -- $1 ${2:A} "${@:3}"
cd "${0:A:h}/.."
SDK=${ANDROID_HOME:-$HOME/Library/Android/sdk}
adb=$SDK/platform-tools/adb

pkg=${1:?usage: tools/android-test.sh <package> <scenario.json> [VAR=value ...]}
script=${2:?scenario}
out=target/android/capture
mkdir -p $out
rm -f $out/*.png(N)

UISON_DETACH=1 UISON_ANDROID_ASSETS="$UISON_ANDROID_ASSETS $script" tools/android-run.sh $pkg UISON_SCRIPT=${script:t} "${@:3}"
# logcat until the "uison: done" line or an app crash.
coproc $adb logcat -v raw -s uison:I AndroidRuntime:E DEBUG:F libc:F
while IFS= read -r -p line; do
  print -r -- "$line"
  if [[ $line == *"uison: capture "* ]]; then
    name=${line##*uison: capture }
    $adb exec-out screencap -p > $out/$name.png
    echo "  → $out/$name.png"
  fi
  [[ $line == *"uison: done"* || $line == *"Fatal signal"* || $line == *"FATAL EXCEPTION"* ]] && break
done
kill %% 2>/dev/null || true
