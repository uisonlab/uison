#!/bin/zsh
# Lessons of the three series (docs/rust, docs/c, docs/cpp): insert example code into the texts, build the lessons,
# run their scenarios and capture windows into docs/<lang>/img — so lessons never go stale.
#   tools/lessons.sh [all|rust|c|cpp|asm] [sync]   # sync — only insert code into the texts
#   tools/lessons.sh asm                       # lessons 1 and 9 in assembly (examples-asm: arm64.S, or x64.asm with NASM)
#   tools/lessons.sh wasm                      # C and C++ lessons in the browser: emcmake, headless Chrome
#   tools/lessons.sh quickstart                # quick starts: a standalone Rust crate (path dependency), one C and C++ command
# Lessons: Rust — crates/uison/examples/NN-name.rs (lesson files in NN-name/), C — examples-c/lessons/NN-name/,
# C++ — examples-cpp/lessons/NN-name/, assembly — examples-asm/lessons/NN-name/. The scenario is the lesson's
# scenario.json; core lessons share one across the languages: scenarios/lessons/<name>.json. 2× screenshots go to out/lessons/<lang>/<lesson>/, 1× into the texts.
# To compare core lessons across languages: tools/lessons-compare.sh.
set -e
cd "${0:A:h}/.."
what=${1:-all}
mode=${2:-}

# Code in a lesson: a line `<!-- code: path -->` followed by a ``` … ``` block, replaced with the file's contents.
sync_code() {
  python3 - <<'PY'
import pathlib, re
for md in sorted(pathlib.Path("docs").rglob("*.md")):
    s = md.read_text()
    def repl(m):
        path, lang = m.group(1), m.group(2)
        if not pathlib.Path(path).exists():
            print(f"  ✗ {md}: no file {path}")
            return m.group(0)
        code = pathlib.Path(path).read_text().rstrip("\n")
        return f"<!-- code: {path} -->\n```{lang}\n{code}\n```"
    # The block runs up to the line that closes it entirely; an empty block (a freshly written lesson) too.
    t = re.sub(r"<!-- code: (\S+) -->\n```(\w*)\n(.*?)^```[ \t]*$", repl, s, flags=re.S | re.M)
    if t != s:
        md.write_text(t)
        print(f"  code → {md}")
PY
}

# Lesson scenario: its own (scenario.json in the lesson folder) or the shared one for core lessons.
scenario() { # $1 — lesson folder, $2 — NN-name
  if [[ -f $1/scenario.json ]]; then print $1/scenario.json
  elif [[ -f scenarios/lessons/${2#*-}.json ]]; then print scenarios/lessons/${2#*-}.json
  fi
}

fail=0
# Run a lesson with its scenario; screenshots go to docs/<lang>/img/<lesson>[-<shot>].png (1×).
run_lesson() { # $1 — language, $2 — NN-name, $3 — program, $4 — scenario, $5 — working folder
  local cap=$PWD/out/lessons/$1/$2 log shot n sc out
  rm -rf $cap
  if [[ -z $4 ]]; then print "· $1/$2 — no scenario"; return; fi
  sc=${4:a}
  log=$(cd $5 && UISON_CAPTURE=$cap UISON_SCRIPT=$sc $3 2>&1 | grep -v "^capture" || true)
  if print -r -- "$log" | grep -qE "no hit area|panic|error"; then
    print -r -- "✗ $1/$2:"; print -r -- "$log"; fail=1
  fi
  # The texts show macOS windows: elsewhere the screenshots stay in out/lessons; docs/asm.md shows the C ones.
  [[ $(uname) == Darwin && $1 != asm ]] && mkdir -p docs/$1/img && for shot in $cap/*.png(N); do
    n=${shot:t:r}
    out=docs/$1/img/$2${${n:#lesson}:+-$n}.png
    # On Retina the window is captured at 2×; the texts get 1×.
    sips --resampleWidth $(( $(sips -g pixelWidth $shot | awk '/pixelWidth/{print $2}') / 2 )) $shot --out $out >/dev/null
  done
  print "✓ $1/$2 (screenshots: $(ls $cap 2>/dev/null | wc -l | tr -d ' '))"
}

rust_lessons() {
  local f name
  cargo build -q --release -p uison --examples
  # The tests of the "Screen tests" lesson are part of the lesson too.
  cargo test -q -p uison --example 09-tests 2>&1 | grep -E "test result|FAILED|panicked" | sed 's/^/  /'
  for f in crates/uison/examples/[0-9][0-9]-*.rs(N); do
    name=${f:t:r}
    run_lesson rust $name $PWD/target/release/examples/$name "$(scenario crates/uison/examples/$name $name)" $PWD
  done
}

# C, C++ and assembly: CMake (examples-c, examples-cpp, examples-asm) puts the lesson programs (c-NN-name, cpp-NN-name,
# asm-NN-name) and their files into one folder; a lesson runs from there (files next to the program).
cmake_lessons() { # $1 — c, cpp or asm
  local lang=$1 dir name build=out/$1/build
  cmake -S examples-$lang -B $build -DCMAKE_BUILD_TYPE=Release >/dev/null
  cmake --build $build -j 8 2>&1 | grep -E "error|warning" && exit 1 || true
  for dir in examples-$lang/lessons/[0-9][0-9]-*/(N); do
    name=${${dir%/}:t}
    [[ -x $build/$lang-$name ]] || { print "· $lang/$name — no program"; continue; }
    run_lesson $lang $name $PWD/$build/$lang-$name "$(scenario ${dir%/} $name)" $PWD/$build
  done
}

# Emscripten: emcc in PATH or emsdk ($EMSDK, ~/emscripten/emsdk).
emsdk_env() {
  command -v emcc >/dev/null && return
  for d in ${EMSDK:-} ~/emscripten/emsdk ~/emsdk; do
    [[ -f $d/emsdk_env.sh ]] && { source $d/emsdk_env.sh >/dev/null 2>&1; return; }
  done
  print "no emcc: install emsdk (https://emscripten.org/docs/getting_started/downloads.html)"; exit 1
}

wasm_lessons() {
  local lang dir name page sc log
  emsdk_env
  for lang in c cpp; do
    emcmake cmake -S examples-$lang -B out/wasm/$lang -DCMAKE_BUILD_TYPE=Release >/dev/null 2>&1
    cmake --build out/wasm/$lang -j 8 2>&1 | grep -E "error|warning" && exit 1 || true
    for dir in examples-$lang/lessons/[0-9][0-9]-*/(N); do
      name=${${dir%/}:t}; page=out/wasm/$lang/$lang-$name.html
      [[ -f $page ]] || { print "· $lang/$name — no page (threads)"; continue; }
      sc=$(scenario ${dir%/} $name)
      [[ -n $sc ]] || continue
      tools/web.sh $page >/dev/null
      cp $sc target/web/$lang-$name/scenarios/lesson.json
      log=$(tools/web-test.sh $lang-$name lesson.json 2>&1 | grep -v "^capture\|Terminated")
      if ! print -r -- "$log" | grep -q DONE || print -r -- "$log" | grep -qE "no hit area|panic|uison: .*(error|bad)"; then
        print -r -- "✗ $lang/$name:"; print -r -- "$log"; fail=1
      fi
      print "✓ $lang/$name (screenshots: $(ls target/web/$lang-$name/capture 2>/dev/null | wc -l | tr -d ' '), target/web/$lang-$name)"
    done
  done
  # Lambdas capturing main's locals by reference: main doesn't unwind in the browser.
  tools/web.sh out/wasm/cpp/cpp-locals.html >/dev/null
  cp examples-cpp/cpp-test/locals.json target/web/cpp-locals/scenarios/
  log=$(tools/web-test.sh cpp-locals locals.json 2>&1 | grep -v "^capture\|Terminated")
  if [[ ! -f target/web/cpp-locals/capture/locals.png ]] || print -r -- "$log" | grep -qE "error|panic"; then
    print -r -- "✗ cpp-locals:"; print -r -- "$log"; fail=1
  else
    print "✓ cpp-locals (target/web/cpp-locals)"
  fi
}

# Quick starts (docs/<lang>/quickstart.md): what the reader does, outside the repository build.
quickstarts() {
  local q=$PWD/out/quickstart root=$PWD host inc=$PWD/crates/uison-c/include lib libs tmp
  mkdir -p $q
  # Rust: a standalone crate outside the repository (inside it, cargo new would add it to the uison workspace), uison as a path dependency,
  # main.rs is lesson 1; the build cache lives in out/quickstart/target.
  tmp=$(mktemp -d)
  (cd $tmp && cargo new -q hello && cd hello && cargo add -q uison --path $root/crates/uison \
    && cp $root/crates/uison/examples/01-hello.rs src/main.rs \
    && CARGO_TARGET_DIR=$q/target cargo build -q --release) || { print "✗ quickstart rust"; fail=1; }
  rm -rf $tmp
  [[ -x $q/target/release/hello ]] && print "✓ quickstart rust ($(( $(wc -c < $q/target/release/hello) / 1024 )) KB)"
  # C and C++: one compiler command with libuison.a.
  host=$(rustc -vV | sed -n 's/^host: //p')
  cargo build -q --release -p uison-c --target $host
  lib=$PWD/target/$host/release/libuison.a
  if [[ $(uname) == Darwin ]]; then
    libs=(-framework AppKit -framework CoreGraphics -framework CoreText -framework ImageIO -framework CoreFoundation -framework Foundation -lobjc -Wl,-dead_strip,-x,-S)
    clang -std=c99 examples-c/lessons/01-hello/main.c -I$inc $lib $libs -o $q/hello-c && \
      clang++ -std=c++20 examples-cpp/lessons/01-hello/main.cpp -I$inc $lib $libs -o $q/hello-cpp || fail=1
  else
    libs=(-lgcc_s -lutil -lrt -lpthread -lm -ldl -Wl,--gc-sections -s)
    gcc -std=c99 examples-c/lessons/01-hello/main.c -I$inc $lib $libs -o $q/hello-c && \
      g++ -std=c++20 examples-cpp/lessons/01-hello/main.cpp -I$inc $lib $libs -o $q/hello-cpp || fail=1
  fi
  for p in hello-c hello-cpp; do
    [[ -x $q/$p ]] || continue
    if UISON_SCRIPT=$PWD/scenarios/lessons/hello.json UISON_CAPTURE=$q/$p-cap $q/$p 2>&1 | grep -q "uison: done"; then
      print "✓ quickstart ${p#hello-} ($(( $(wc -c < $q/$p) / 1024 )) KB)"
    else
      print "✗ quickstart ${p#hello-}"; fail=1
    fi
  done
}

if [[ $what == quickstart ]]; then
  quickstarts
  exit $fail
fi
if [[ $what == wasm ]]; then
  wasm_lessons
  exit $fail
fi
sync_code
[[ $mode == sync ]] && exit 0
case $what in
  rust) rust_lessons ;;
  c) cmake_lessons c ;;
  cpp) cmake_lessons cpp ;;
  asm) cmake_lessons asm ;;
  all) rust_lessons; cmake_lessons c; cmake_lessons cpp; cmake_lessons asm; quickstarts ;;
  *) print "usage: tools/lessons.sh [all|rust|c|cpp|asm|wasm|quickstart] [sync]"; exit 2 ;;
esac
exit $fail
