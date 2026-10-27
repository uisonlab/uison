#!/bin/zsh
# Build, install and run an example on a phone (or emulator); the app's output (logcat, tag uison) goes to the terminal.
#   tools/android-run.sh <package | built program> [--debug] [VARIABLE=value ...]
# Variables (UISON_STATS=1, UISON_FLASH=1, UISON_SCRIPT=…) are passed as Intent extras.
# Device: ANDROID_SERIAL (the only connected one by default).
# UISON_DETACH=1 — only launch, don't read the output.
set -e
# Resolve paths from the launch folder (a built program of a crate outside the repository) before changing to the root.
[[ -e ${1:-} ]] && set -- ${1:A} "${@:2}"
cd "${0:A:h}/.."
SDK=${ANDROID_HOME:-$HOME/Library/Android/sdk}
adb=$SDK/platform-tools/adb

pkg=${1:?usage: tools/android-run.sh <package> [--debug] [VAR=value ...]}
build=()
extras=()
for a in "${@:2}"; do
  [[ $a == --debug ]] && build+=(--debug)
  [[ $a == *=* ]] && extras+=(--es ${a%%=*} ${a#*=})
done
apk=$(tools/android.sh $pkg $build | tail -1)
id=dev.uison.${${apk:t:r}//-/_}  # the APK name is the package name or the program's folder name

$adb install -r -t $apk >/dev/null
$adb shell am force-stop $id
$adb logcat -c
$adb shell am start -n $id/dev.uison.UisonActivity $extras >/dev/null
[[ -n $UISON_DETACH ]] && exit 0
# The app's output while it's alive (Ctrl+C to quit).
$adb logcat -v raw -s uison:I AndroidRuntime:E DEBUG:F libc:F
