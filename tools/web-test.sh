#!/bin/sh
# Run a scenario in headless Chrome: tools/web-test.sh <package> <scenario.json> [width height]
# Screenshots go to target/web/<package>/capture/, page logs to stdout.
set -e
pkg=${1:?usage: tools/web-test.sh <package> <scenario.json> [w h]}
script=${2:?scenario}
w=${3:-1220}; h=${4:-780}
cd "$(dirname "$0")/.."
dir=target/web/$pkg
rm -rf "$dir/capture"
log=$(mktemp)
python3 tools/serve.py "$dir" 8766 > "$log" 2>&1 &
server=$!
sleep 0.5
chrome="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
"$chrome" ${HEADLESS---headless=new} --user-data-dir="$(mktemp -d)" --no-first-run --hide-scrollbars ${CHROME_FLAGS:-} \
  --force-device-scale-factor=2 --window-size="$w,$h" \
  "http://localhost:8766/?test=1&stats=${STATS:-0}${URL_EXTRA:-}&script=scenarios/$script" > /dev/null 2>&1 &
browser=$!
for _ in $(seq 120); do
  grep -q DONE "$log" && break
  sleep 0.25
done
kill $browser $server 2>/dev/null || true
grep -v "^uison: http" "$log"
rm -f "$log"
