# Lesson 12. Building, phones, the browser

**English** · [Русский](../../ru/cpp/lessons/12-build.md)

**What you'll learn:** every way to build a program — CMake from source and from a prebuilt package, a single compiler command, static and dynamic linking — and the same program on an iPhone, on Android and in the browser.

## CMake from source

```cmake
add_subdirectory(../uison/crates/uison-c uison)   # CMake calls cargo and builds libuison
add_executable(app main.cpp)
target_link_libraries(app PRIVATE uison::uison)   # static; uison::shared — dynamic
```

You need Rust. The `uison::uison` target adds everything required on its own: system libraries, `-dead_strip` / `--gc-sections`, the entry point on Android, WebAssembly exceptions in the browser. For custom components — `uison_generate_widgets` ([lesson 6](06-components.md)).

## CMake from a package

No Rust needed: the `uison-<platform>` package from a release (or `tools/c-dist.sh macos` / `linux` / `ios` / `android` / `wasm`, on Windows — `tools\c-dist.ps1`). It contains the headers, the static and dynamic libraries, `bin/uison-gen`, and files for CMake and pkg-config.

```cmake
set(uison_DIR /path/to/uison-macos/lib/cmake/uison)   # the iOS, NDK and Emscripten toolchains find packages only this way
find_package(uison REQUIRED)
target_link_libraries(app PRIVATE uison::uison)
```

## A single command

With the library built beforehand (`cargo build --release -p uison-c` in the uison repository, `U` is its path):

```sh
# macOS
clang++ -std=c++20 main.cpp -I$U/crates/uison-c/include $U/target/release/libuison.a \
  -framework AppKit -framework CoreGraphics -framework CoreText -framework ImageIO \
  -framework CoreFoundation -framework Foundation -lobjc -Wl,-dead_strip,-x,-S -o app

# Linux
g++ -std=c++20 main.cpp -I$U/crates/uison-c/include $U/target/release/libuison.a \
  -lgcc_s -lutil -lrt -lpthread -lm -ldl -Wl,--gc-sections -s -o app
```

`-dead_strip` / `--gc-sections` drop unused library code, `-x,-S` / `-s` strip symbols: the program is ≈ 0.75 MB.

### Windows: a single command

In the "Developer Command Prompt for VS"; the library is `target\release\uison.lib`:

```bat
cl /std:c++20 /utf-8 /EHsc main.cpp /I %U%\crates\uison-c\include %U%\target\release\uison.lib ^
  dxgi.lib d2d1.lib d3d11.lib dwrite.lib dwmapi.lib user32.lib shell32.lib gdi32.lib winmm.lib ^
  kernel32.lib advapi32.lib ole32.lib ntdll.lib userenv.lib ws2_32.lib dbghelp.lib ^
  /link /OPT:REF /SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup
```

- `/SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup` — a window without a console with a regular `main()`; without them a console with the output opens alongside, which is handy for debugging. In CMake — `target_link_options(app PRIVATE /SUBSYSTEM:WINDOWS /ENTRY:mainCRTStartup)`.
- `uison.lib` works with either CRT: `/MT` (what `cl` uses) — the program runs on any Windows 10+; `/MD` (what CMake uses) — the Visual C++ Redistributable is required. For distribution, use `/MT` (`set(CMAKE_MSVC_RUNTIME_LIBRARY MultiThreaded)`).

## Static or dynamic

| | static (`uison::uison`) | dynamic (`uison::shared`) |
|---|---|---|
| what you ship | a single program | the program plus `libuison.dylib` / `.so` / `uison.dll` next to it |
| size | ≈ 0.75 MB | program ≈ 35 KB + library 0.7–1 MB |
| when | almost always | several programs sharing one library |

With dynamic linking the program looks for the library next to itself (`@rpath` on macOS, `-Wl,-rpath,'$ORIGIN'` on Linux). On Windows, `uison_copy_dll(app)` copies `uison.dll` next to `app.exe` after the build.

## iPhone and iPad

Static library only; the `uison-ios` package contains `uison.xcframework`.

```sh
cmake -S . -B build-ios -DCMAKE_SYSTEM_NAME=iOS -DCMAKE_OSX_SYSROOT=iphonesimulator \
  -DCMAKE_OSX_ARCHITECTURES=arm64 -DCMAKE_OSX_DEPLOYMENT_TARGET=17.0
cmake --build build-ios
tools/ios-run.sh build-ios/app.app/app sim       # .app → simulator; output goes here
```

A single `clang++` command, a real phone and signing, Xcode — see [iOS and Android](../../mobile.md#ios).

## Android

The program is `libmain.so`: `add_library(main SHARED main.cpp)` instead of `add_executable`; `uison::uison` adds the entry point flags itself.

```sh
cmake -S . -B build-android -DCMAKE_TOOLCHAIN_FILE=$NDK/build/cmake/android.toolchain.cmake \
  -DANDROID_ABI=arm64-v8a -DANDROID_PLATFORM=29 -DCMAKE_BUILD_TYPE=Release -Duison_DIR=$PWD/dist/uison-android/lib/cmake/uison
cmake --build build-android
mkdir -p app/arm64-v8a && cp build-android/libmain.so app/arm64-v8a/
tools/android-run.sh app                         # APK → phone, launch
```

The NDK libc++ is linked statically (`ANDROID_STL=c++_static` by default), so the program doesn't pull in `libc++_shared.so`. Details — [iOS and Android](../../mobile.md#android).

## The browser

[Emscripten](https://emscripten.org): a single `em++` command or `emcmake`.

```sh
D=dist/uison-wasm                                # tools/c-dist.sh wasm
em++ -std=c++20 -Oz -fwasm-exceptions -I$D/include main.cpp $D/lib/libuison.a --shell-file $D/web/shell.html -o site/index.html
```

```cmake
uison_web_page(app)                  # app.html + app.js + app.wasm; outside the browser — nothing
uison_web_files(app screen.json)     # files next to the program go into the page's file system
```

`-fwasm-exceptions` is required: `uison.hpp` reports errors with exceptions. There are no threads in the browser. Details — [The browser (WebAssembly)](../../wasm.md).

## Program icon

The icon of the running window (the Dock on macOS, the title bar and taskbar on Windows, the taskbar on X11) is a PNG you set before `run`:

```cpp
#include "icon.h" // xxd -i icon.png > icon.h: unsigned char icon_png[], unsigned int icon_png_len
uison::set_icon(icon_png, icon_png_len);
return ui.run("Hello");
```

The icon of the program file itself comes from packaging. On Windows, add a resource script to the program's sources — `add_executable(app main.cpp app.rc)`, where `app.rc` is one line `1 ICON "icon.ico"` (id 1 is the one the window uses); on macOS, an `.app` bundle with an `.icns`; on Linux, a `.desktop` file and an icon in the `hicolor` theme (on Wayland the window is matched to it by its `app_id`, `dev.uison.<program>`). A complete example for macOS and Linux — [`bundle.sh`](../../../examples/disk-usage/bundle.sh) of Disk Usage.

End of the series. Reference — [The screen language](../../screens/README.md), [Elements](../../screens/elements.md), API — [`uison.hpp`](../../../crates/uison-c/include/uison.hpp).
