#!/bin/zsh
# Core lessons in the three languages are the same program: Rust, C and C++ (and assembly) screenshots (out/lessons, taken by
# tools/lessons.sh) must match pixel for pixel below the window title. This checks the wrappers: if the Rust DSL,
# UISON_JSON or the C++ widgets produce a different tree, the screenshot differs. Plus C/C++ examples against Rust.
#   tools/lessons-compare.sh            # lessons (run tools/lessons.sh first) and examples in windows
#   tools/lessons-compare.sh web        # examples in the browser: Rust (tools/web.sh) against C/C++ (emcmake)
# Needs Pillow (`pip3 install pillow`).
set -e
cd "${0:A:h}/.."

# Compare screenshot folders: $1 is the reference, the rest are checked; $CROP — how many pixels to skip at the top.
compare() {
  python3 - "$@" <<'PY'
import os, pathlib, sys
from PIL import Image, ImageChops
crop = int(os.environ.get("CROP", "0"))
ref, *others = map(pathlib.Path, sys.argv[1:])
shots = sorted(p.name for p in ref.glob("*.png"))
bad = 0
for other in others:
    for n in shots:
        if not (other / n).exists():
            print(f"  {other}: no screenshot {n}"); bad += 1; continue
        a, b = Image.open(ref / n).convert("RGBA"), Image.open(other / n).convert("RGBA")
        box = (0, crop, a.width, a.height)
        d = ImageChops.difference(a.crop(box), b.crop(box)).getbbox() if a.size == b.size else "size"
        print(f"  {other}/{n}: {'match' if d is None else f'DIFFERENT {d}'}")
        bad += d is not None
sys.exit(1 if bad or not shots else 0)
PY
}

fail=0

if [[ $1 == web ]]; then
  if ! command -v emcc >/dev/null; then
    for d in ${EMSDK:-} ~/emscripten/emsdk ~/emsdk; do [[ -f $d/emsdk_env.sh ]] && source $d/emsdk_env.sh >/dev/null 2>&1 && break; done
  fi
  emcmake cmake -S examples-c -B out/wasm/c -DCMAKE_BUILD_TYPE=Release >/dev/null 2>&1
  cmake --build out/wasm/c --target form2-c -j 8 >/dev/null 2>&1
  emcmake cmake -S examples-cpp -B out/wasm/cpp -DCMAKE_BUILD_TYPE=Release >/dev/null 2>&1
  cmake --build out/wasm/cpp --target todo-cpp list10k-cpp -j 8 >/dev/null 2>&1
  for rs c sc in form2 c/form2-c form2.json todo cpp/todo-cpp todo.json list10k cpp/list10k-cpp list10k.json; do
    tools/web.sh $rs >/dev/null 2>&1
    tools/web.sh out/wasm/$c.html >/dev/null
    for p in $rs ${c:t}; do
      if tools/web-test.sh $p $sc 2>&1 | grep -E "no hit area|error"; then fail=1; fi
    done
    compare target/web/$rs/capture target/web/${c:t}/capture || fail=1
  done
  exit $fail
fi

# Core lessons: the same name after the number (01-hello, 06-themes / 07-themes…) means the same program.
# On Retina the window is captured at 2×: the title bar (28 pt) is 56 px, so take everything below 64.
export CROP=64
for name in hello data forms lists themes rich threads frame; do
  local dirs=(out/lessons/{rust,c,cpp,asm}/[0-9][0-9]-$name(N/))
  if (( ${#dirs} < 2 )); then print "· $name: no screenshots (tools/lessons.sh)"; continue; fi
  print "$name:"
  compare $dirs || fail=1
done

# Examples: the Rust example, the C/C++ program (out/c/build or out/cpp/build), the scenario.
pairs=(
  form2 c/build/form2-c form2.json
  todo cpp/build/todo-cpp todo.json
  list10k cpp/build/list10k-cpp list10k.json
)
cmake -S examples-c -B out/c/build -DCMAKE_BUILD_TYPE=Release >/dev/null
cmake --build out/c/build --target form2-c -j 8 >/dev/null
cmake -S examples-cpp -B out/cpp/build -DCMAKE_BUILD_TYPE=Release >/dev/null
cmake --build out/cpp/build --target todo-cpp list10k-cpp -j 8 >/dev/null
cargo build -q --release -p form2 -p todo -p list10k
for rs c sc in $pairs; do
  out=out/c/compare/${c:t}
  rm -rf $out && mkdir -p $out
  UISON_SCRIPT=scenarios/$sc UISON_CAPTURE=$out/rs target/release/$rs 2>$out/rs.log
  UISON_SCRIPT=scenarios/$sc UISON_CAPTURE=$out/c out/$c 2>$out/c.log
  if grep -qE "no hit area|panic|error" $out/rs.log $out/c.log; then
    grep -hE "no hit area|panic|error" $out/rs.log $out/c.log
    fail=1
  fi
  print "${c:t}:"
  compare $out/rs $out/c || fail=1
done
exit $fail
