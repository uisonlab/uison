# Lesson 10. Everywhere

**English** · [Русский](../../ru/c/lessons/10-everywhere.md)

**What you'll get:** the program from lesson 1 on iPhone, on Android and in the browser. The code doesn't change: each platform has its own `uison_run`.

## The browser

[Emscripten](https://emscripten.org) builds a C program into WebAssembly the usual way — `emcc` or `emcmake`. You need the `dist/uison-wasm` package (`tools/c-dist.sh wasm`):

```sh
D=dist/uison-wasm
emcc -std=c11 -Oz -I$D/include main.c $D/lib/libuison.a --shell-file $D/web/shell.html -o site/index.html
python3 tools/serve.py site 8000          # http://localhost:8000/
```

No other flags are needed: the JS for drawing and input lives inside `libuison.a`. With CMake — `emcmake cmake …` and `uison_web_page(app)`; files next to the program (lesson 5) — `uison_web_files(app screen.json …)`. Details — [the browser](../../wasm.md).

## iPhone and iPad

```sh
D=dist/uison-ios                          # tools/c-dist.sh ios
clang -target arm64-apple-ios17.0-simulator -isysroot $(xcrun --sdk iphonesimulator --show-sdk-path) \
  main.c -I$D/include $D/uison.xcframework/ios-arm64-simulator/libuison.a \
  -framework UIKit -framework QuartzCore -framework CoreGraphics -framework CoreText -framework ImageIO \
  -framework CoreFoundation -framework Foundation -lobjc -Wl,-dead_strip,-x,-S -o hello
tools/ios-run.sh ./hello sim                                      # .app → simulator; output goes here
tools/ios-test.sh ./hello scenarios/lessons/hello.json sim        # scenario and screenshot
```

For a phone — `-target arm64-apple-ios17.0`, `--sdk iphoneos`, `uison.xcframework/ios-arm64` and `device` (signing required). Files next to the program (lesson 5) — `UISON_IOS_FILES="screen.json components.json"`. CMake, Xcode — [phones](../../mobile.md#ios).

## Android

On Android the program is a `libmain.so` library: the uison Java layer loads it, and `JNI_OnLoad` from `libuison.a` calls your `main`.

```sh
D=dist/uison-android                      # tools/c-dist.sh android
NDK=~/Library/Android/sdk/ndk/<version>; CC=$NDK/toolchains/llvm/prebuilt/darwin-x86_64/bin   # Linux: linux-x86_64
mkdir -p hello/arm64-v8a
$CC/aarch64-linux-android29-clang -shared -fPIC main.c -I$D/include $D/lib/arm64-v8a/libuison.a \
  -llog -landroid -ldl -lm -Wl,--gc-sections,--version-script=$D/lib/cmake/uison/uison-android.map -s \
  -o hello/arm64-v8a/libmain.so
tools/android-run.sh hello                                        # APK → phone, launch
tools/android-test.sh hello scenarios/lessons/hello.json          # scenario and screenshot
```

The app name comes from the folder (`hello`). Files next to the program — `UISON_ANDROID_ASSETS="screen.json components.json"`. A second ABI (`armeabi-v7a`), CMake with the NDK toolchain — [phones](../../mobile.md#android).

## What changes on a phone

Nothing in the code, but the screen is different — and you can see it in the `@env` data: touches instead of a mouse (`@env.touch`), a safe area under the notch and the home bar, the on-screen keyboard (the layout shrinks above it), and the size is the whole screen. For a narrow screen — `visible: { lt: ["@env.width", 600] }`.

## Program icon

The icon of the running window (the Dock on macOS, the title bar and taskbar on Windows, the taskbar on X11) is a PNG you set before `uison_run`:

```c
#include "icon.h" /* xxd -i icon.png > icon.h: unsigned char icon_png[], unsigned int icon_png_len */
uison_set_icon(icon_png, icon_png_len);
return uison_run(ui, "Hello");
```

The icon of the program file itself comes from packaging. On Windows, add a resource script to the program's sources — `add_executable(app main.c app.rc)`, where `app.rc` is one line `1 ICON "icon.ico"` (id 1 is the one the window uses); on macOS, an `.app` bundle with an `.icns`; on Linux, a `.desktop` file and an icon in the `hicolor` theme (on Wayland the window is matched to it by its `app_id`, `dev.uison.<program>`). A complete example for macOS and Linux — [`bundle.sh`](../../../examples/disk-usage/bundle.sh) of Disk Usage.

That's the end of the series. Reference — [the screen language](../../screens/README.md), [elements](../../screens/elements.md), API — [`uison.h`](../../../crates/uison-c/include/uison.h).
