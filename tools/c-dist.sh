#!/bin/bash
# Library package for C/C++: dist/uison-<platform>/
#   include/uison.h, uison.hpp, uison_widgets.hpp · lib/ static and dynamic · lib/cmake/uison (find_package) · lib/pkgconfig/uison.pc
#   tools/c-dist.sh [macos|ios|android|linux|wasm|all]
# macos — universal (arm64 + x86_64; UISON_MACOS_ARCHS="aarch64" — Apple silicon only).
# ios — uison.xcframework (device + arm64 simulator), static only.
# android — lib/<ABI>/libuison.a and libuison.so (arm64-v8a, armeabi-v7a, x86_64, x86; UISON_ANDROID_ABIS — a custom list);
#   the program is libmain.so, the APK comes from tools/android.sh.
# linux — uison-linux-x86_64 and uison-linux-aarch64: on a Mac in the OrbStack machines uison64, uison; on Linux the native
#   architecture, the other one cross-compiled if its gcc is present (aarch64-linux-gnu-gcc, x86_64-linux-gnu-gcc).
# wasm — libuison.a for Emscripten (C/C++ program: emcc/em++ or emcmake) and web/shell.html.
set -e
cd "$(dirname "$0")/.."
what=${1:-all}
root=$PWD

# Shared: headers, CMake files, README, LICENSE.
common() {
  local out=$1
  rm -rf "$out"; mkdir -p "$out/include" "$out/lib/cmake/uison" "$out/lib/pkgconfig"
  cp crates/uison-c/include/uison.h crates/uison-c/include/uison_kit.h crates/uison-c/include/uison.hpp \
    crates/uison-c/include/uison_widgets.hpp "$out/include/"
  cp crates/uison-c/cmake/uisonConfig.cmake crates/uison-c/cmake/uison-libs.cmake crates/uison-c/cmake/uison-android.map "$out/lib/cmake/uison/"
  [ -f LICENSE ] && cp LICENSE "$out/" || true
  cp docs/package-readme.md "$out/README.md" 2>/dev/null || true
}

# bin/uison-gen — widgets for custom components (uison_generate_widgets in CMake), a program for the machine
# that builds the app (iOS, Android and browser packages: for the machine that built the package).
gen() {
  local out=$1 triple=${2:-$(rustc -vV | sed -n 's/^host: //p')}
  cargo build --release -q -p uison-gen --target "$triple"
  mkdir -p "$out/bin"
  cp "target/$triple/release/uison-gen" "$out/bin/"
}

# pkg-config: uison is dynamic (`pkg-config --cflags --libs uison`), uison-static is static
# (libuison.a itself plus system libraries; `-luison` would pick the .so if it's next to it).
pc() {
  local out=$1 libs=$2 static=$3
  cat > "$out/lib/pkgconfig/uison.pc" <<PC
prefix=\${pcfiledir}/../..
includedir=\${prefix}/include
libdir=\${prefix}/lib

Name: uison
Description: Tiny UI: screens in JSON, logic in C/C++ (dynamic library)
Version: 0.1.0
Cflags: -I\${includedir}
Libs: -L\${libdir} -Wl,-rpath,\${libdir} -luison
PC
  cat > "$out/lib/pkgconfig/uison-static.pc" <<PC
prefix=\${pcfiledir}/../..
includedir=\${prefix}/include
libdir=\${prefix}/lib

Name: uison-static
Description: Tiny UI: screens in JSON, logic in C/C++ (static library)
Version: 0.1.0
Cflags: -I\${includedir}
Libs: \${libdir}/libuison.a $libs $static
PC
}

macos() {
  local out=dist/uison-macos archs=${UISON_MACOS_ARCHS:-aarch64 x86_64} a
  for a in $archs; do
    cargo build --release -q -p uison-c --target $a-apple-darwin
  done
  common $out
  gen $out
  lipo -create $(for a in $archs; do echo target/$a-apple-darwin/release/libuison.a; done) -output $out/lib/libuison.a
  lipo -create $(for a in $archs; do echo target/$a-apple-darwin/release/libuison.dylib; done) -output $out/lib/libuison.dylib
  strip -x $out/lib/libuison.dylib # strip from rustc breaks the .dylib for Apple's ld — strip it ourselves
  pc $out "-framework AppKit -framework CoreGraphics -framework CoreText -framework ImageIO -framework CoreFoundation -framework Foundation -lobjc" "-Wl,-dead_strip"
  echo "$out: $(du -h $out/lib/libuison.a | cut -f1) static ($archs), $(du -h $out/lib/libuison.dylib | cut -f1) dynamic"
}

# iOS: an app is a single program, no dynamic library needed. The library is in uison.xcframework
# (Xcode takes it whole; clang and CMake use uison.xcframework/ios-arm64[-simulator]/libuison.a).
ios() {
  local out=dist/uison-ios
  export IPHONEOS_DEPLOYMENT_TARGET=${IPHONEOS_DEPLOYMENT_TARGET:-17.0}
  for t in aarch64-apple-ios aarch64-apple-ios-sim; do
    cargo build --release -q -p uison-c --target $t
  done
  common $out
  gen $out
  rm -rf "$out/lib/pkgconfig"
  xcodebuild -create-xcframework -library target/aarch64-apple-ios/release/libuison.a -headers $out/include \
    -library target/aarch64-apple-ios-sim/release/libuison.a -headers $out/include -output $out/uison.xcframework >/dev/null
  echo "$out: $(du -h $out/uison.xcframework/ios-arm64/libuison.a | cut -f1) static (device), uison.xcframework"
}

android() {
  local out=dist/uison-android sdk=${ANDROID_HOME:-$HOME/Library/Android/sdk}
  local ndk=${ANDROID_NDK_HOME:-$(ls -d $sdk/ndk/* | sort -V | tail -1)}
  local host; host=$(uname | tr A-Z a-z)-x86_64  # the NDK for Mac and Linux is x86_64 (via Rosetta on an arm64 Mac)
  local clang=$ndk/toolchains/llvm/prebuilt/$host/bin
  common $out
  gen $out
  rm -rf "$out/lib/pkgconfig"
  for abi in ${UISON_ANDROID_ABIS:-arm64-v8a armeabi-v7a x86_64 x86}; do
    local t cc
    case $abi in
      arm64-v8a) t=aarch64-linux-android; cc=aarch64-linux-android29-clang ;;
      armeabi-v7a) t=armv7-linux-androideabi; cc=armv7a-linux-androideabi29-clang ;;
      x86_64) t=x86_64-linux-android; cc=x86_64-linux-android29-clang ;;
      x86) t=i686-linux-android; cc=i686-linux-android29-clang ;;
      *) echo "unknown ABI $abi" >&2; exit 1 ;;
    esac
    local var; var=CARGO_TARGET_$(echo $t | tr a-z- A-Z_)_LINKER
    env $var=$clang/$cc cargo build --release -q -p uison-c --target $t
    mkdir -p $out/lib/$abi
    cp target/$t/release/libuison.{a,so} $out/lib/$abi/
    $clang/llvm-strip --strip-unneeded $out/lib/$abi/libuison.so
  done
  echo "$out: $(du -h $out/lib/arm64-v8a/libuison.a | cut -f1) static, $(du -h $out/lib/arm64-v8a/libuison.so | cut -f1) dynamic (arm64-v8a)"
}

linux_one() { # $1 — the OrbStack machine (or empty on Linux itself), $2 — architecture
  local cmd="cd '$root' && cargo build --release -q -p uison-c -p uison-gen --target $2-unknown-linux-gnu"
  if [ "$(uname)" = Darwin ]; then sleep 1; orb -m "$1" bash -lc "$cmd"; else bash -c "$cmd"; fi
}

linux() {
  local a strip
  for a in x86_64 aarch64; do
    strip="strip --strip-unneeded"
    if [ "$(uname)" = Darwin ]; then
      local vm; vm=$([ $a = x86_64 ] && echo uison64 || echo uison)
      linux_one $vm $a
      strip="orb -m $vm strip --strip-unneeded"
    elif [ "$(uname -m)" = $a ]; then
      linux_one "" $a
    elif command -v $a-linux-gnu-gcc > /dev/null; then
      # Cross-compiling: the library needs only libc, so the target architecture's gcc is enough.
      (export "CARGO_TARGET_$(echo $a | tr a-z A-Z)_UNKNOWN_LINUX_GNU_LINKER=$a-linux-gnu-gcc"; linux_one "" $a)
      strip="$a-linux-gnu-strip --strip-unneeded"
    else
      echo "skip $a (no $a-linux-gnu-gcc: sudo apt install gcc-${a//_/-}-linux-gnu)"; continue
    fi
    local out=dist/uison-linux-$a
    common $out
    cp target/$a-unknown-linux-gnu/release/libuison.{a,so} $out/lib/
    mkdir -p $out/bin && cp target/$a-unknown-linux-gnu/release/uison-gen $out/bin/
    $strip "$root/$out/lib/libuison.so"
    pc $out "-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc" "-Wl,--gc-sections"
    echo "$out: $(du -h $out/lib/libuison.a | cut -f1) static, $(du -h $out/lib/libuison.so | cut -f1) dynamic"
  done
}

# WASM: libuison.a for Emscripten (wasm32-unknown-emscripten), static only. A C/C++ program uses
# emcc/em++ (`em++ main.cpp libuison.a -fwasm-exceptions -o app.html`) or emcmake + find_package(uison);
# all of uison's JS (canvas, input) is inside the library as EM_JS strings. web/shell.html is the page template.
wasm() {
  local out=dist/uison-wasm
  cargo rustc --release -q -p uison-c --target wasm32-unknown-emscripten --crate-type staticlib
  common $out
  gen $out
  rm -rf "$out/lib/pkgconfig"
  cp target/wasm32-unknown-emscripten/release/libuison.a $out/lib/
  mkdir -p $out/web
  cp web/shell.html $out/web/
  echo "$out: $(du -h $out/lib/libuison.a | cut -f1) static (wasm32, Emscripten)"
}

case $what in
  macos) macos ;;
  ios) ios ;;
  android) android ;;
  linux) linux ;;
  wasm) wasm ;;
  all) macos; ios; android; linux; wasm ;;
  *) echo "usage: tools/c-dist.sh [macos|ios|android|linux|wasm|all]"; exit 1 ;;
esac
