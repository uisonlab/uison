#!/bin/zsh
# Build, install and run an example on the simulator or a phone; the app's output goes to the terminal.
#   tools/ios-run.sh <package | path to a program> [sim|device] [--debug] [VARIABLE=value ...]
# Variables (UISON_STATS=1, UISON_FLASH=1, …) are passed to the app.
# Simulator: UISON_SIM (name or UDID, "iPhone 17 Pro" by default). Phone: UISON_DEVICE (UDID; the first connected one by default).
# UISON_DETACH=1 — don't wait for the app to exit (for scripts).
set -e
# Resolve paths from the launch folder (a built program of a crate outside the repository) before changing to the root.
[[ -f ${1:-} ]] && set -- ${1:A} "${@:2}"
cd "${0:A:h}/.."

pkg=${1:?usage: tools/ios-run.sh <package> [sim|device] [--debug] [VAR=value ...]}
kind=sim
[[ $2 == sim || $2 == device ]] && kind=$2
build=()
envs=()
for a in "${@:2}"; do
  [[ $a == --debug ]] && build+=(--debug)
  [[ $a == *=* ]] && envs+=($a)
done
app=$(tools/ios.sh $pkg $kind $build | tail -1)
bundle_id=dev.uison.${${pkg:t}//_/-}

if [[ $kind == sim ]]; then
  sim=${UISON_SIM:-iPhone 17 Pro}
  xcrun simctl boot "$sim" 2>/dev/null || true
  open -a Simulator 2>/dev/null || true  # the simulator window is optional: simctl takes the screenshots
  xcrun simctl install "$sim" $app
  launch=(xcrun simctl launch --terminate-running-process)
  [[ -z $UISON_DETACH ]] && launch+=(--console-pty)
  # Environment variables for the app get the SIMCTL_CHILD_ prefix.
  env ${envs/#/SIMCTL_CHILD_} $launch "$sim" $bundle_id
else
  dev=${UISON_DEVICE:-$(xcrun devicectl list devices 2>/dev/null | awk '/available \(paired\)|connected/ && /physical/ {for (i=1;i<=NF;i++) if ($i ~ /^[0-9A-F]{8}-[0-9A-F]{16}$/) {print $i; exit}}')}
  [[ -z $dev ]] && { echo "no device" >&2; exit 1; }
  xcrun devicectl device install app --device $dev $app >/dev/null
  json="{"
  for kv in $envs; do json+="\"${kv%%=*}\":\"${kv#*=}\","; done
  json="${json%,}}"
  launch=(xcrun devicectl device process launch --device $dev --terminate-existing)
  [[ $json != "{}" ]] && launch+=(--environment-variables "$json")
  [[ -z $UISON_DETACH ]] && launch+=(--console)
  $launch $bundle_id
fi
