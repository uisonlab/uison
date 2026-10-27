# uison

<!-- languages -->
<h3 align="center">
<b>🇬🇧 English</b> ·
<a href="docs/readme/README.zh.md">🇨🇳 中文</a> ·
<a href="docs/readme/README.hi.md">🇮🇳 हिन्दी</a> ·
<a href="docs/readme/README.es.md">🇪🇸 Español</a> ·
<a href="docs/readme/README.fr.md">🇫🇷 Français</a> ·
<a href="docs/readme/README.ar.md">🇸🇦 العربية</a> ·
<a href="docs/readme/README.bn.md">🇧🇩 বাংলা</a> ·
<a href="docs/readme/README.pt.md">🇧🇷 Português</a> ·
<a href="docs/readme/README.ru.md">🇷🇺 Русский</a> ·
<a href="docs/readme/README.ur.md">🇵🇰 اردو</a> ·
<a href="docs/readme/README.id.md">🇮🇩 Bahasa Indonesia</a> ·
<a href="docs/readme/README.de.md">🇩🇪 Deutsch</a> ·
<a href="docs/readme/README.ja.md">🇯🇵 日本語</a> ·
<a href="docs/readme/README.mr.md">🇮🇳 मराठी</a> ·
<a href="docs/readme/README.te.md">🇮🇳 తెలుగు</a> ·
<a href="docs/readme/README.tr.md">🇹🇷 Türkçe</a> ·
<a href="docs/readme/README.uk.md">🇺🇦 Українська</a>
</h3>
<!-- /languages -->

**uison** (UI + JSON) is a tiny, fast UI framework written in Rust with no dependencies at
all — only the standard library, down to its own JSON parser. You describe a screen as a
tree of elements: in a JSON file, with a Rust DSL, with C++ widgets in the style of Flutter,
or as JSON right inside C code. The logic is ordinary code in Rust, C or C++: handlers, data,
threads. uison takes care of the window, layout, drawing and input, and the same program
runs on macOS, Windows, Linux (Wayland and X11), iOS, Android and in the browser.

Every backend talks to the system directly, through hand-written FFI: AppKit and CoreText,
Win32 with Direct2D and DirectWrite (COM by hand), the Wayland protocol implemented from
scratch and X11, UIKit, Android's Canvas over JNI, and a 2D canvas from WebAssembly. A
release program is one file of about 0.7–0.9 MB that needs nothing but the system's own
libraries, and an event reaches the screen in about a millisecond on a desktop.

![Disk Usage, an example built with uison: a sunburst chart of a folder, a file list and a tooltip](docs/img/disk-usage.jpg)

## How it works

```text
screen (JSON, Rust DSL, C++ widgets, UISON_JSON in C)  +  data  +  handlers in your language
        │  bindings, templates, themes, layout (uison-ui)
        ▼
frame of primitives: rect, line, circle, tri, poly, text, image, clip, hit
        │  diff against the previous frame → redraw only what changed
        ▼
system backend: AppKit, Win32 + Direct2D, Wayland/X11 + Cairo, UIKit, Android Canvas, canvas 2D
```

A screen compiles into a *frame*: a flat list of a few kinds of primitives — rectangles,
lines, circles, polygons, single lines of text, images, clip regions and invisible hit areas
for the mouse and touch. Each new frame is compared with the previous one, and the backend
repaints only the regions that changed. A backend is small because it only has to draw
these primitives, measure text and turn the system's events into uison's.

The screen language has no code in it. Text and properties are bound to the data
(`"Pressed {@n} times"`, `"disabled": "!@form.valid"`), elements are reusable templates
with parameters, colors and sizes come from theme tokens, and events call named actions
whose handlers are written in Rust, C or C++. When a handler changes the data, the screen
rebuilds itself.

## Hello, world

The same window — a counter and a "+1" button — in the three languages.

**Rust** — the screen is built with a DSL, the handler is a closure on the button:

```rust
use uison::prelude::*;

fn main() {
    let screen = center(column().gap(12).cross("center").children([
        text("Pressed {@n} times").size(20),
        button("+1").key("inc").on_click(|cx| cx.data.edit("n", |n: &mut i32| *n += 1)),
    ]));
    let mut ui = Ui::from_node(screen).with_size(320.0, 200.0);
    ui.data.set("n", 0);
    run(ui, "Hello");
}
```

**C** — the screen is JSON written straight into the code, the handler is a function:

```c
#include <uison_kit.h>

static void inc(uison_cx* cx, void* user) {
    (void)user;
    uison_data* d = uison_cx_data(cx);
    uison_data_set_f64(d, "n", uison_data_f64(d, "n") + 1);
}

int main(void) {
    uison_ui* ui = uison_ui_new(UISON_JSON({
        size: [320, 200],
        data: { n: 0 },
        root: { Center: { child: { Column: { gap: 12, cross: "center", children: [
            { Text: { text: "Pressed {@n} times", size: 20 } },
            { Button: { key: "inc", text: "+1", on: { click: "inc" } } },
        ] } } } }
    }));
    if (!ui) return uison_kit_fail(); /* prints the JSON error and where it is */
    UISON_ON(ui, inc);                /* action "inc" → function inc */
    return uison_run(ui, "Hello");
}
```

**C++** — widgets as in Flutter, the handler is a lambda on the button:

```cpp
#include <uison_widgets.hpp>

using namespace uison::widgets;

int main() {
    auto screen = Center({.child = Column({.gap = 12, .cross = "center", .children = {
        Text("Pressed {@n} times", {.size = 20}),
        Button("+1", {.key = "inc", .on_click = [](uison::Cx& cx) {
            cx.data().update<int>("n", [](int n) { return n + 1; });
        }}),
    }})});
    uison::Ui ui(screen, {.size = {320, 200}, .data = uison::obj{{"n", 0}}});
    return ui.run("Hello");
}
```

To start with your own project, see the quick start for
[Rust](docs/rust/quickstart.md), [C](docs/c/quickstart.md) or [C++](docs/cpp/quickstart.md).

## Features

- **No dependencies.** The core, the JSON parser, the layout, the Wayland client, the D-Bus
  client for the Linux settings portal and the COM bindings for Windows are all written
  here, on top of `std`. `Cargo.lock` lists only the workspace's own crates.
- **One program, six platforms.** macOS, Windows (x64 and ARM64), Linux (Wayland and X11,
  arm64 and x86_64), iOS, Android (arm64-v8a and armeabi-v7a) and the browser
  (WebAssembly). On a phone the same code gets touch, a safe area, the on-screen keyboard
  and inertial scrolling.
- **Partial redraw.** Frames are diffed and only damaged regions are repainted, with thin
  lines snapped to physical pixels the same way on every backend.
- **A library of elements**: buttons, icon buttons and 25 icons, checkboxes, radio
  buttons, switches, sliders, selects, tabs, text fields with selection and the clipboard,
  progress bars, cards, list tiles, badges, avatars, tooltips, context menus, links and
  images. Elements are themselves JSON templates (`lib/*.json`), and your own components
  are written the same way.
- **Layout in the spirit of Flutter**: `Row`, `Column`, `Stack`, `Wrap`, `Align`, flex,
  scrolling, virtualized lists for tens of thousands of rows, and `Flow` — a paragraph of
  inline text runs with wrapping across them.
- **Bindings and state**: paths into the data (`@user.name`), text interpolation, local
  state of a component (`@~open`), selectors by state (hover, pressed, focus), `switch`, and
  a few functions (`eq`, `not`, `len`, `ratio`…). Structs map to data and back:
  `#[derive(Data)]` in Rust, field tables in C, reflection without macros in C++20.
- **Themes**: light, dark and "follow the system", built from tokens you can override.
- **Your own drawing**: a `Canvas` node calls your function to write primitives into its
  rectangle, next to ordinary elements.
- **Threads**: a worker thread posts results back to the UI thread (`post`), on every
  platform except the browser.
- **Live reload** in debug builds: with `UISON_RELOAD=folder` the screen's JSON files are
  re-read when you save them.
- **Testing without a window**: a scenario (`UISON_SCRIPT=scenario.json`) taps, types,
  scrolls and takes screenshots by element `key`; the same scenarios run on every platform,
  in headless Chrome, in the iOS simulator and on a phone.
- **C and C++ API**: `uison.h` (C99), `uison_kit.h` (helpers for C), `uison.hpp` (C++17,
  conveniences in C++20) and `uison_widgets.hpp` (C++20 widgets generated from the same
  schema as the Rust DSL); static and dynamic libraries, a CMake package and `pkg-config`.
  C and C++ also build for the browser with Emscripten: `em++ main.cpp libuison.a -o index.html`.

## Platforms

| platform | backend | checked on |
|---|---|---|
| macOS | AppKit, CoreGraphics, CoreText, ImageIO through the Objective-C runtime | Apple Silicon Mac — the main development machine |
| iOS | UIKit, CADisplayLink, the same CoreGraphics/CoreText code as macOS | simulator and iPhone 12 mini |
| Android | JNI by hand, `android.graphics.Canvas`, a ~300-line Java layer; APK built without Gradle | Huawei P40 lite (Android 10) |
| Windows | Win32, Direct2D, DirectWrite and WIC, COM by hand; per-monitor DPI, dark title bar | Windows 11 PC at 100/125/150 % and two monitors with different scales |
| Linux | own Wayland protocol code and X11 (xcb), Cairo + Pango, xkbcommon — all through `dlopen`, so the binary depends only on libc | headless Mutter, KWin, Sway, Weston and X11 (arm64 and x86_64); Ubuntu 24.04 with GNOME in a virtual machine |
| Browser | `wasm32-unknown-unknown` without wasm-bindgen, canvas 2D; C/C++ through Emscripten | Chrome (and headless Chrome) |

Not checked yet: a real touchscreen on Windows, Windows on ARM (it builds but hasn't been
run), Linux on real hardware and KDE, mouse and keyboard input in GNOME, Safari and Firefox.

## Size and speed

Release builds (`opt-level = "s"`, LTO, `panic = "abort"`, stripped). Sizes are from the
project's own measurements as it grew, so the newest builds may differ by a few percent.

| program | size |
|---|---|
| lesson 1 (the counter above), Rust, macOS | ~0.7 MB |
| the same in C / C++ with `libuison.a`, macOS | 672 KB / 816 KB |
| Studio — the main demo (mail with 2,000 messages, kanban, contacts, charts, forms), macOS | ~840 KB |
| Studio, Windows x64 / ARM64 | 672 KB / 613 KB, no Visual C++ Redistributable needed |
| Studio, Linux arm64 / x86_64 | 857 KB / 936 KB |
| Studio, Android `libmain.so` arm64 | ~820 KB |
| Studio, browser (`.wasm`, gzip) | ~220 KB |

| measurement | result |
|---|---|
| event → painted window, macOS | ~0.8 ms, of which ~0.2 ms is uison's own work |
| touch → frame, iPhone 12 mini | 3.6 ms; scrolling a list at an even 60 fps |
| full repaint of Studio, iPhone 12 mini / Huawei P40 lite | 0.7 ms / 0.4–0.7 ms |
| mouse event → frame, Windows (Iris Xe, 60 Hz) | p50 1.4–1.7 ms |
| event → frame, Chrome | median 2.2 ms |
| launch → first frame | iOS 37–41 ms, Android 58–68 ms, Windows 73–103 ms, browser 24–42 ms |

[Disk Usage](examples/disk-usage/README.md), a port of an egui program, gives a direct
comparison: on macOS arm64 the binary is 857 KB instead of 5.1 MB, with no dependencies
instead of 362 crates in `Cargo.lock`, and with the same scanning speed and memory use.

## Documentation

The documentation is in English, with a [Russian version](docs/ru/README.md) of every page.

- [Choose a language](docs/README.md) — where to start.
- Lesson series, each readable on its own, from setting up your system to threads and your
  own frame: [Rust](docs/rust/README.md), [C](docs/c/README.md), [C++](docs/cpp/README.md).
  The core lessons are the same program in all three languages, and their screenshots match
  pixel-for-pixel.
- [The screen language](docs/screens/README.md): [bindings and values](docs/screens/bindings.md),
  [layout](docs/screens/layout.md), [elements](docs/screens/elements.md),
  [themes](docs/screens/themes.md).
- [iOS and Android](docs/mobile.md), [the browser (WebAssembly)](docs/wasm.md).

## Examples

| example | what it shows |
|---|---|
| `studio` | the main demo: mail with 2,000 messages, kanban, contacts, animated charts, a form, settings; a phone layout under 700 pt |
| `gallery` | every element of the library |
| `disk-usage` | a port of an egui app: a sunburst chart drawn in a `Canvas` node, panels and menus as a uison screen, a parallel scanner on `std` |
| `browser` | a small web browser: its own HTML and CSS parser and cascade, HTTP, and HTTPS through the system's TLS; a page becomes a uison screen |
| `todo` | a to-do list in the Rust DSL |
| `list10k` | a virtualized list of 10,000 rows |
| `form2`, `settings` | forms and settings screens |
| `demo`, `scroll`, `uibench` | primitives by hand, scrolling, a benchmark |

```sh
cargo run --release -p studio
cargo run --release -p disk-usage -- ~/Downloads
cargo run --release -p browser
```

C and C++ examples live in [`examples-c`](examples-c) and [`examples-cpp`](examples-cpp),
each a CMake project of its own.

## Build and test

You need stable Rust; nothing else on macOS, Windows (with the MSVC toolchain) and Linux
(no `-dev` packages: system libraries are loaded at run time). Setting up each system is
lesson 0 of every series ([Rust](docs/rust/lessons/00-setup.md)).

```sh
cargo test --workspace && cargo clippy --workspace   # tests
cargo run --release -p studio                        # a window on macOS, Windows or Linux
tools/web.sh studio && python3 tools/serve.py target/web/studio 8000   # the browser
tools/ios.sh studio sim                              # iOS simulator (Xcode)
tools/android.sh studio                              # an APK without Gradle (Android SDK and NDK)
tools/c-dist.sh macos                                # the C/C++ package in dist/uison-macos
tools/lessons.sh                                     # build every lesson, run its scenario, update the screenshots
```

On Windows the scripts are `tools\*.ps1`: `tools\win.ps1 studio` builds, runs a scenario and
saves screenshots, `tools\c-dist.ps1 -Test` builds and tests the C/C++ package.

Backend switches for debugging, the same everywhere: `UISON_STATS=1` prints timings of
events, frames and animation, `UISON_FLASH=1` highlights repainted regions, and
`UISON_SCRIPT` / `UISON_CAPTURE` run a scenario and save screenshots.

## Layout

```
crates/
  uison-json      JSON pull parser and value tree, with comments, trailing commas and unquoted keys
  uison-core      geometry, colors, fonts, events, the Canvas and Host traits, the App trait
  uison-frame     the primitive frame: parsing, diff → damage, hit testing, the scenario runner
  uison-ui        the screen compiler: templates, bindings, themes, layout, the runtime, the Rust DSL, lib/*.json
  uison-apple     shared by macOS and iOS: Objective-C FFI, CoreGraphics, CoreText, ImageIO
  uison-macos     AppKit backend
  uison-ios       UIKit backend
  uison-android   JNI backend and its Java layer (java/dev/uison)
  uison-windows   Win32 + Direct2D + DirectWrite backend
  uison-linux     Wayland and X11 backend, Cairo + Pango, all through dlopen
  uison-web       WebAssembly backend (web/uison.js)
  uison-c         the C API and C++ headers, static and dynamic library, CMake package
  uison-gen       generates the C++ widgets, the Rust DSL and the element reference from the screen schema
  uison-derive    #[derive(Data)] without syn
  uison           the umbrella crate: everything for the current platform, uison::prelude; the Rust lessons
examples/         Rust examples (see above)
examples-c/       C examples and lessons, a CMake project
examples-cpp/     C++ examples and lessons, a CMake project
scenarios/        test scenarios shared by all platforms
tools/            build, run and test scripts for every platform; drivers of real mouse and keyboard input
web/              the page and the JavaScript side of the WebAssembly backend
docs/             documentation in English; ru/, the same in Russian; readme/, this README in other languages
```

## Limitations

- One window per program; no drag-and-drop.
- Text input covers Latin and Cyrillic and the phones' on-screen keyboards; composed input
  for Chinese, Japanese and Korean (IME) on the desktop is not done, nor is right-to-left text.
- No accessibility support (screen readers) yet.
- No threads in the browser.
- The framework is young: version 0.1, the API may change.

## Translations

The documentation is in English, and `docs/ru/` holds the same pages in Russian, with a
language switch at the top of each. This README is translated as well:
`docs/readme/README.<code>.md`, one per language, with the list of languages at the top of
each. A change here belongs in the translations too.

## License

The license has not been chosen yet.
