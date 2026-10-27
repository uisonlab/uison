# Lesson 11. Everywhere

**English** · [Русский](../../ru/rust/lessons/11-everywhere.md)

**What you'll get:** the same program — your crate from the [quick start](../quickstart.md) — on an iPhone, on Android and in the browser. The code doesn't change: each platform has its own `uison::run`.

Packaging (`.app`, APK, web page) is done by scripts from the uison repository — all they need is the built program. Below, `UISON` is the path to your clone of the repository, and `hello` is your crate.

## The browser

```sh
rustup target add wasm32-unknown-unknown
cargo build --release --target wasm32-unknown-unknown
$UISON/tools/web.sh target/wasm32-unknown-unknown/release/hello.wasm     # → target/web/hello/
python3 $UISON/tools/serve.py target/web/hello 8000                        # http://localhost:8000/
```

The page is `index.html`, `uison.js` (canvas drawing, input) and `app.wasm`. No wasm-bindgen and no glue code: the program is the same, and `main` is called from `uison.js`. Details — [the browser](../../wasm.md).

## iPhone and iPad

You need Xcode (for the simulator) — and, for a phone, a developer signature ([phones](../../mobile.md#ios)).

```sh
rustup target add aarch64-apple-ios-sim aarch64-apple-ios
cargo build --release --target aarch64-apple-ios-sim
$UISON/tools/ios-run.sh target/aarch64-apple-ios-sim/release/hello sim   # .app → simulator; output goes here
```

With a scenario and a screenshot, in one command: `$UISON/tools/ios-test.sh target/aarch64-apple-ios-sim/release/hello scenario.json sim`. For a phone — `--target aarch64-apple-ios` and `device`.

## Android

You need the Android SDK and NDK ([phones](../../mobile.md#android)). On Android the program is a `libmain.so` library: uison's Java layer loads it and calls `main`.

```sh
rustup target add aarch64-linux-android
NDK=~/Library/Android/sdk/ndk/<version>   # on Linux — ~/Android/Sdk/ndk/<version>
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$NDK/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android29-clang
export CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS="-C link-arg=-shared"
cargo build --release --target aarch64-linux-android
mkdir -p hello/arm64-v8a && cp target/aarch64-linux-android/release/hello hello/arm64-v8a/libmain.so
$UISON/tools/android-run.sh hello                                         # APK → phone, launch
```

The app name comes from the folder (`hello`). With a scenario and a screenshot — `$UISON/tools/android-test.sh hello scenario.json`.

## What changes on a phone

Nothing in the code, but the screen is different — and you can see it in the `@env` data:
- `@env.touch` — touch instead of the mouse: tap, finger scrolling with inertia, long press instead of the right button;
- the safe area (camera cutout, home indicator) — by default the screen root is placed inside it (`@env.safe.*`);
- the on-screen keyboard slides up when a field has focus, and the layout shrinks above it;
- the size is the whole screen: the screen's `size` has no effect, and `@env.width` tells you whether it's narrow: `visible: { lt: ["@env.width", 600] }`.

## Program icon

The icon of the running window (the Dock on macOS, the title bar and taskbar on Windows, the taskbar on X11) is a PNG you set before `run`:

```rust
uison::set_icon(include_bytes!("../assets/icon.png")); // 256×256 is enough
run(ui, "Hello");
```

The icon of the program file itself comes from packaging. On macOS it's an `.app` bundle with an `.icns`; on Windows, a resource in the `.exe` (written by `build.rs`); on Linux, a `.desktop` file and an icon in the `hicolor` theme (on Wayland the window is matched to it by its `app_id`, `dev.uison.<program>`). The Disk Usage example does all three: [`build.rs`](../../../examples/disk-usage/build.rs), [`bundle.sh`](../../../examples/disk-usage/bundle.sh).

End of the series. Reference — [the screen language](../../screens/README.md), [elements](../../screens/elements.md).
