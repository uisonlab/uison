#!/bin/zsh
# A test scenario on the simulator or a phone: the app runs the steps itself
# (uison_frame::script), and this script takes screenshots on `uison: capture <name>` lines.
#   tools/ios-test.sh <package | path to a program> <scenario.json> [sim|device] [VARIABLE=value ...]
# Screenshots: target/ios/capture/<sim|device>/<name>.png. The app's output goes to the terminal.
set -e
# Resolve paths from the launch folder (a built program of a crate outside the repository) before changing to the root.
[[ -f ${1:-} ]] && set -- ${1:A} "${@:2}"
[[ -f ${2:-} ]] && set -- $1 ${2:A} "${@:3}"
cd "${0:A:h}/.."

pkg=${1:?usage: tools/ios-test.sh <package> <scenario.json> [sim|device] [VAR=value ...]}
script=${2:?scenario}
kind=sim
[[ $3 == sim || $3 == device ]] && kind=$3
out=target/ios/capture/$kind
mkdir -p $out
rm -f $out/*.png(N)

sim=${UISON_SIM:-iPhone 17 Pro}
dev=${UISON_DEVICE:-$(xcrun devicectl list devices 2>/dev/null | awk '/available \(paired\)|connected/ && /physical/ {for (i=1;i<=NF;i++) if ($i ~ /^[0-9A-F]{8}-[0-9A-F]{16}$/) {print $i; exit}}')}

shot() {
  if [[ $kind == sim ]]; then
    xcrun simctl io "$sim" screenshot $out/$1.png >/dev/null 2>&1
  else
    xcrun devicectl device capture screenshot --device $dev --destination $out/$1.png >/dev/null 2>&1
  fi
  echo "  → $out/$1.png"
}

UISON_IOS_FILES="$UISON_IOS_FILES $script" tools/ios-run.sh $pkg $kind UISON_SCRIPT=${script:t} "${@:4}" 2>&1 | while IFS= read -r line; do
  line=${line%$'\r'}  # the simulator's pty yields \r\n
  print -r -- "$line"
  [[ $line == *"uison: capture "* ]] && shot ${line##*uison: capture }
done
