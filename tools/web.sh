#!/bin/sh
# Build a uison app for the browser: tools/web.sh <package>
# Result: target/web/<package>/ (app.wasm + the shared index.html, uison.js, drive.js).
# A built Rust program (your own crate: cargo build --release --target wasm32-unknown-unknown):
# tools/web.sh path/to/hello.wasm → target/web/hello/ (app.wasm + index.html, uison.js).
# A built C/C++ program (Emscripten, an emcc page): tools/web.sh out/wasm/cpp/todo-cpp.html —
# target/web/todo-cpp/ gets the page (index.html), its .js/.wasm/.data and the files next to it (images, JSON).
# Run: python3 tools/serve.py target/web/<package>  →  http://localhost:8000/
set -e
pkg=${1:?usage: tools/web.sh <package | program.html>}
case $pkg in /*) ;; *) pkg=$PWD/$pkg ;; esac
cd "$(dirname "$0")/.."

case $pkg in
*.wasm)
  name=$(basename "$pkg" .wasm)
  out=target/web/$name
  mkdir -p "$out/scenarios"
  cp "$pkg" "$out/app.wasm"
  if [ -z "$NO_WASM_OPT" ] && command -v wasm-opt > /dev/null; then
    wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals \
      -o "$out/app.wasm" "$out/app.wasm"
  fi
  cp web/index.html web/uison.js web/drive.js "$out/"
  cp web/scenarios/*.json scenarios/*.json "$out/scenarios/"
  echo "$out/app.wasm: $(wc -c < "$out/app.wasm" | tr -d ' ') B (gzip $(gzip -9 -c "$out/app.wasm" | wc -c | tr -d ' ') B)"
  exit 0
  ;;
*.html)
  src=$(dirname "$pkg")
  name=$(basename "$pkg" .html)
  out=target/web/$name
  rm -rf "$out"
  mkdir -p "$out/scenarios"
  cp "$pkg" "$out/index.html"
  for f in "$src/$name.js" "$src/$name.wasm" "$src/$name.data"; do
    if [ -f "$f" ]; then cp "$f" "$out/"; fi
  done
  find "$src" -maxdepth 1 \( -name '*.jpg' -o -name '*.png' -o -name '*.json' \) -exec cp {} "$out/" \;
  cp web/drive.js "$out/"
  cp web/scenarios/*.json scenarios/*.json "$out/scenarios/"
  echo "$out: wasm $(wc -c < "$out/$name.wasm" | tr -d ' ') B (gzip $(gzip -9 -c "$out/$name.wasm" | wc -c | tr -d ' ')), js $(wc -c < "$out/$name.js" | tr -d ' ') B (gzip $(gzip -9 -c "$out/$name.js" | wc -c | tr -d ' '))"
  exit 0
  ;;
esac
pkg=$(basename "$pkg")
cargo build --release --target wasm32-unknown-unknown -p "$pkg"
out=target/web/$pkg
mkdir -p "$out"
cp "target/wasm32-unknown-unknown/release/$pkg.wasm" "$out/app.wasm"
# Optional: wasm-opt from binaryen (if installed; NO_WASM_OPT=1 skips it).
if [ -z "$NO_WASM_OPT" ] && command -v wasm-opt > /dev/null; then
  wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext --enable-mutable-globals \
    -o "$out/app.wasm" "$out/app.wasm"
fi
cp web/index.html web/uison.js web/drive.js "$out/"
cp -R web/scenarios "$out/"
# Shared scenarios (uison_frame::script): drive.js understands the same steps; web-specific scenarios take precedence.
# cp -n on macOS returns 1 if the file already exists.
cp -n scenarios/*.json "$out/scenarios/" || true
size=$(wc -c < "$out/app.wasm" | tr -d ' ')
gz=$(gzip -9 -c "$out/app.wasm" | wc -c | tr -d ' ')
echo "$out/app.wasm: $size B (gzip $gz B)"
