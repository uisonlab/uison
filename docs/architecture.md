# Architecture

**English** · [Русский](ru/architecture.md)

This page explains how uison is put together: what goes in, what comes out, which part does what, and why JSON sits at the center of it. It also covers two things that follow from that design: running a program in **console mode**, where JSON messages replace the window, and the **SSH player**, which shows a program that runs on another machine.

## The idea in one picture

```text
 screen: JSON file, Rust DSL, C++ widgets, UISON_JSON in C      data      handlers in Rust / C / C++
        │                                                         │                ▲
        ▼                                                         ▼                │ named actions
 ┌──────────────────────────────────────────────────────────────────────────────────┴───┐
 │ uison-ui: templates, parameters, theme tokens, bindings, layout, widget state       │
 └──────────────────────────────────────────────────────────────────────────────────────┘
        │  frame of primitives (JSON text): rect, line, circle, tri, poly, text, image, clip, hit
        ▼
 ┌──────────────────────────────────────────────────────────────────────────────────────┐
 │ uison-frame: parse, bounds, diff against the previous frame → damage, hit test       │
 └──────────────────────────────────────────────────────────────────────────────────────┘
        │  draw only the damaged regions                         ▲ events (mouse, keys, touch, resize…)
        ▼                                                         │
 backend: AppKit · UIKit · Win32 + Direct2D · Wayland/X11 + Cairo · Android Canvas · canvas 2D
```

Two kinds of JSON flow through this pipeline:

- **The screen** — rich JSON: components with parameters (`$title`), bindings to data (`@inbox.count`), interpolation (`"{@n} unread"`), theme tokens (`$theme.accent`), lists, selectors, named actions. A person writes it (or generates it from the Rust DSL or C++ widgets).
- **The frame** — primitive JSON: absolute rectangles, lines, circles, polygons, single-line text, images, clip groups and invisible hit areas. A program writes it, and a backend draws it.

Everything between the two is ordinary Rust: no code lives in the JSON, and no layout or styling lives in the backends.

## Why JSON

JSON is the contract between the layers, not just a configuration format:

- **One language for every host language.** A screen written once works from Rust, C and C++ alike; C and C++ only pass strings and callbacks across the C ABI. The Rust DSL and the C++ widgets are generated from one schema (`uison-gen`), so all three describe the same tree.
- **A frame is plain data.** `App::frame` and `App::on_event` return the frame as a `String`. Anything that can print JSON can drive a backend, including a C program with `snprintf` ([lesson 9](c/lessons/09-frame.md)), a test, or another process.
- **Everything that crosses a boundary has a JSON form.** Events (`{"type":"down","x":10,"y":20,"button":"left"}`), messages to a screen (`{"action":"remove","key":"del-1"}`, `{"data":{"filter":"done"}}`) and requests to the platform (`{"host":"copy","text":"…"}`) all have one canonical text form ([events](screens/events.md)). The same form is used by the C API, by test scenarios, by console mode and by the players.
- **Inspectable.** A frame can be saved, diffed, replayed and read by a person. A scenario is a JSON list of steps.
- **No dependencies.** uison has its own JSON code (`uison-json`): a pull parser with exact number parsing, strings without copying and `line:col` errors; a mutable `Value` tree; writers; and `compact`, which turns extended JSON (comments, trailing commas, unquoted keys) into strict JSON on one line.

JSON stays **at the edges**. Inside a process everything is typed: a backend turns a native event into `Event`, `uison-frame` parses frame text into structs once per frame, and the backend draws from those structs through the `Canvas` trait. Turning an event into JSON and back costs about 0.2 µs.

## Layers and crates

```text
 uison-json ──▶ uison-core ──▶ uison-frame ──▶ uison-ui ──▶ uison (umbrella: prelude, run, run_console)
                    │              │               │
                    ▼              ▼               ▼
        uison-apple → uison-macos, uison-ios        uison-c (C API: uison.h, uison_kit.h, uison.hpp)
        uison-windows · uison-linux · uison-android · uison-web
```

| layer | crate | role |
|---|---|---|
| data | `uison-json` | parse and write JSON, `Value`, `compact` |
| contract | `uison-core` | geometry, `Color`, `Font`, the `Canvas` and `TextMeasure` traits, `Host` (text metrics and system requests), `Event`, the `App` trait, image sizes, the clock |
| frame | `uison-frame` | `Frame::parse`, `diff` → `Damage`, `hit_test`, `paint` onto any `Canvas`, `Writer` (fast frame output), `Runtime` (app + current frame), event ↔ JSON, the scenario runner, console mode |
| screens | `uison-ui` | the screen compiler and runtime `Ui` (an `App`), the element library `lib/*.json`, themes, the Rust DSL |
| platforms | `uison-macos`, `uison-ios` (over `uison-apple`), `uison-windows`, `uison-linux`, `uison-android`, `uison-web` | a window, events, drawing, text measurement |
| languages | `uison`, `uison-derive`, `uison-c`, `uison-gen` | the Rust entry point, `#[derive(Data)]`, the C/C++ API and its packages, wrapper generation |
| tools | `uison-player`, `uison-ssh-player` | players for console mode (below) |

Everything except `uison-ssh-player` depends on `std` only. System APIs are called through hand-written FFI: the Objective-C runtime, CoreGraphics and CoreText; COM for Direct2D and DirectWrite; the Wayland wire protocol and libxcb, Cairo and Pango through `dlopen`; JNI; imports from a small JavaScript file in the browser.

## The core contract

An app is anything that implements a small trait from `uison-core`:

```rust
pub trait App {
    fn frame(&mut self, host: &dyn Host) -> String;                     // the first frame
    fn on_event(&mut self, event: &Event, hit: Option<&str>, host: &dyn Host) -> Option<String>;
    fn wants_tick(&self) -> Option<Duration> { None }                    // timers, animation
    fn set_waker(&mut self, _wake: Waker) {}                             // other threads → Event::Wake
    fn wants_keyboard(&self) -> Option<Keyboard> { None }                // on-screen keyboard
}
```

`hit` is the `id` of the topmost hit area under the pointer, found in the previous frame. `None` from `on_event` means "nothing changed". `Host` is the platform as the app sees it: text measurement, modifier keys, the clipboard, opening a link, the system file chooser, image sizes.

`Ui` from `uison-ui` is one such `App`. A program can also implement `App` by hand and write its own frames.

## The frame

```json
{ "size": [400, 300], "background": "#F5F5F7", "items": [
  { "rect": [10, 10, 380, 230], "radius": 10, "fill": "#FFFFFF", "stroke": "#D2D2D7", "width": 1 },
  { "circle": [120, 80, 8], "fill": "#007AFF" },
  { "clip": [10, 10, 380, 230], "items": [
    { "text": "Dots: 1", "at": [170, 258], "size": 13, "color": "#86868B" } ] },
  { "hit": [290, 250, 100, 32], "id": "clear", "cursor": "pointer" }
] }
```

### The nine primitives

Everything any screen shows is built from nine kinds of items. Each item has exactly one of the keys below; its value is the geometry. All of them are implemented by every backend (macOS, iOS, Windows, Linux, Android, the browser) through the `Canvas` trait, and every item may carry an `id`.

| JSON key | geometry | other keys | drawn with |
|---|---|---|---|
| `rect` | `[x, y, w, h]` | `fill`, `stroke`, `width` (stroke, default 1), `radius` (rounded corners) | `Canvas::rect` |
| `line` | `[x1, y1, x2, y2]` | `color` (default black), `width`, `cap`: `"butt"` (default) or `"round"` | `Canvas::line` |
| `circle` | `[cx, cy, r]` | `fill`, `stroke`, `width` | `Canvas::circle` |
| `tri` | `[x1, y1, x2, y2, x3, y3]` | `fill`, `stroke`, `width` | `Canvas::triangle` (a polygon by default) |
| `poly` | `[[x, y], [x, y], …]`, closed | `fill`, `stroke`, `width`; nonzero fill rule, rounded joins | `Canvas::poly` |
| `text` | `"one line of text"` | `at: [x, y]` (top left, required), `size` (default 13), `color`, `font` (a family or `monospace` / `serif` / `sans-serif`), `bold`, `italic` | `Canvas::text` |
| `image` | `[x, y, w, h]` | `src` (a file or `mem:` name, required), `fit`: `"cover"` (default), `"contain"`, `"fill"`; `radius` | `Canvas::image` |
| `clip` | `[x, y, w, h]` | `items`: nested items, cut to the rectangle | `Canvas::push_clip` / `pop_clip` |
| `hit` | `[x, y, w, h]` | `id` (what `on_event` receives as `hit`), `cursor`: `"arrow"`, `"pointer"`, `"text"` | not drawn: hit testing and cursors |

Shapes need `fill`, `stroke` or both; colors are `"#RGB"`, `"#RRGGBB"` or `"#RRGGBBAA"`. An unknown key, a key that does not belong to the shape, or a duplicate is an error with its `line:col`. The parser rejects them rather than ignoring them, so a typo in a frame is found at once.

- Coordinates are logical points with the origin at the top left. Text is one line; `at` is its top-left corner, and wrapping is done by the screen layer.
- `uison-frame` parses the frame and computes bounds (text is measured through `TextMeasure`, so bounds are exact).
- **Diff.** The new frame is aligned with the previous one item by item (a longest common subsequence; clip groups with the same rectangle are matched and compared inside). Changed, added and removed items give damaged regions, and the backend redraws only those. On iOS and Android the system always asks for the whole view, so there the frame is drawn in full; a full Studio frame takes 0.4–0.7 ms to draw on a phone.
- **Hit test and cursors** come from the `hit` areas, so the backend needs no knowledge of widgets.
- Thin lines are snapped to physical pixels in the same way on every backend.

## The screen layer (`uison-ui`)

`Ui` holds the screen, the data, the widget state and the handlers. On every change it rebuilds the frame:

1. **Expand** — components from the library (`Button`, `TextField`, `Tabs`…) and your own are expanded with their parameters; `For` and `List` repeat templates over data (`List` is virtualized).
2. **Bind** — `@path` values, `{@path}` interpolation, local state `@~name`, selectors (`{state: {…}}`, `{switch, cases}`), functions (`eq`, `ratio`, `pick`…) and theme tokens are resolved against the data, the environment (`@env.width`, `@env.dark`, `@env.safe.top`…) and the theme.
3. **Lay out** — a Flutter-style model: `Row`, `Column`, `Stack`, `Wrap`, `Flow` (inline text with links), flex, alignment, scrolling, text wrapping.
4. **Emit** — primitives are written with `Writer`; what is outside the visible part of a scroll view produces nothing.

The runtime handles hover, pressed, focus, the caret and text selection, scrolling with inertia and bounce, taps versus scrolls on touch screens, tooltips and context menus, and keyboard shortcuts. An event that means something to the app calls a **named action**: `on: { click: "remove" }` in the screen, `ui.on("remove", |cx| …)` in Rust (or a function or a lambda in C and C++). A handler changes data or the tree; the next frame reflects the change.

## Backends

A backend is small because all of the above is shared. It provides:

- `run(app, title)` — the window and the event loop;
- native events → `Event` (mouse, wheel, touch, keys and text input, resize, safe area, theme, paste);
- an implementation of `Canvas` and `TextMeasure`, with a two-generation cache of laid-out text lines;
- after each event, a look at `wants_tick` (20 ms or less ticks with the display: CADisplayLink, Choreographer, `WaitForVBlank`, frame callbacks; longer waits use a timer) and at `wants_keyboard`.

| platform | window and input | drawing and text |
|---|---|---|
| macOS | AppKit, an `NSView` subclass registered at runtime | CoreGraphics, CoreText |
| iOS | UIKit, `UIKeyInput`, CADisplayLink | CoreGraphics, CoreText |
| Windows | Win32, per-monitor DPI v2, `WM_POINTER` | Direct2D, DirectWrite, WIC |
| Linux | Wayland (own protocol code) and X11 (libxcb), xkbcommon, a D-Bus client for the theme | Cairo, Pango, all through `dlopen` |
| Android | a ~300-line Java activity and view, JNI | `android.graphics.Canvas` |
| Browser | WebAssembly without wasm-bindgen | canvas 2D |

The same environment variables work on every backend for checking a program without a person: `UISON_SCRIPT` (a scenario of taps, typing, scrolling by element `key`), `UISON_CAPTURE` (screenshots), `UISON_HITMAP` (the map of hit areas for driving a real mouse), `UISON_STATS`, `UISON_FLASH`, `UISON_RELOAD` (reload screen files on save while debugging).

## Languages

- **Rust** — `uison::prelude`: `Ui`, the DSL (`column`, `button`, `.on_click(|cx| …)`), `#[derive(Data)]` for structs ↔ data, `source!` for screen files.
- **C** — `uison.h` (C99) over a static or shared `libuison`; `uison_kit.h` adds `UISON_JSON({…})` for screens written right in the code and field tables for structs.
- **C++** — `uison.hpp` (RAII, lambdas, struct reflection without macros) and `uison_widgets.hpp` (C++20 widgets in the Flutter style, generated from the schema).

All three reach the same `Ui` and the same backends. A message from another thread (`post`) wakes the UI thread through the `Waker`.

## Console mode: JSON instead of a window

Because an app's whole output is a frame in JSON and its whole input is events with a JSON form, the window is optional. In **console mode** a program has no window: it writes frames to stdout and reads events from stdin, one message per line.

```rust
if std::env::args().any(|a| a == "--console") {
    uison::run_console(app(), "My app");  // the same app, no window
}
uison::run(app(), "My app");
```

```text
 program (any App)                                   whoever is on the other end
┌───────────────────────────────┐   stdout   ┌──────────────────────────────────────┐
│ uison_frame::console::serve   │ ─────────▶ │ {"uison":0,"title":"Disk Usage"}     │
│  reader thread → channel →    │            │ {"frame":{"size":[…],"items":[…]}}   │
│  on_event → frame → one line  │            │ {"host":"copy","text":"…"}           │
│  timers, animation, threads   │   stdin    │                                      │
│  stay inside the program      │ ◀───────── │ {"type":"resize","w":1280,"h":796}   │
│                               │            │ {"type":"down","x":412,"y":230,…}    │
└───────────────────────────────┘            │ {"action":"rescan"}                  │
                                             └──────────────────────────────────────┘
```

How it works:

- `uison_frame::console::serve` is a loop that knows nothing about `Ui`: a thread reads lines into a channel; the main loop waits for a line or for the `wants_tick` deadline, hit-tests against the last frame, calls `on_event` and writes the new frame as strict JSON on one line (`compact`, without reformatting numbers). Each message is one `write_all`.
- `Host` requests become messages: copy, open a link and paste go to the other side as `{"host":…}`; the paste answer comes back as a `paste` event.
- stdout belongs to the protocol. On macOS and Linux the protocol writes to a duplicate of descriptor 1, and descriptor 1 is pointed at stderr, so a stray `println!` cannot break the stream.
- Text is still measured by the program. On a machine without fonts (a Linux server without Cairo and Pango), or with `UISON_TEXT=estimate`, it estimates widths from typical character widths of a sans-serif font and says so on stderr; the player draws the frame with its own fonts.
- The program exits when stdin closes.

You can talk to such a program with nothing but a shell:

```sh
printf '%s\n' '{"type":"resize","w":900,"h":600}' '{"action":"about"}' \
  | target/release/disk-usage --console | cut -c1-120
```

This turns any uison program into something that can be tested by pipes, recorded as a stream of lines, driven by another program, or shown somewhere else. The protocol in full is on the [console mode page](console.md).

## Players

A **player** is an ordinary uison program that starts another program in console mode and shows it.

- **`uison-player`** (local) starts the program as a child process. The **App** tab embeds the program's frames into its own (`Writer::frame` moves them into place and prefixes their hit ids, so the program's areas and the player's never mix), sends the mouse and keys over the program's area, sends `resize` and the system theme. The **JSON** tab lists every message in both directions with stderr, filters and a field for sending your own line. The status line shows traffic, frames per second and latency. `uison-player` is also a library: the way to reach the program is a `Link` trait, and the start screen is a replaceable `Launcher` component.
- **`uison-ssh-player`** is the same library with an SSH link. It connects to a server (key or password), checks the host key against `~/.ssh/known_hosts` and its own list (a new host is remembered with its fingerprint, a changed key is refused), optionally uploads the program (`mkdir -p … && cat > path && chmod +x path`, so no SFTP and nothing to install on the server), and runs it with `--console`. The connection stays open between runs; **Stop** ends the program and keeps the connection. It also lives in a repository of its own, [uisonlab/uison-ssh-player](https://github.com/uisonlab/uison-ssh-player).

```text
 uison-ssh-player                           server: any sshd, nothing installed
┌──────────────────────────┐      SSH       ┌────────────────────────────────────┐
│ 1. connect, check host   │ ─────────────▶ │ key or password                    │
│ 2. upload (optional)     │ ──── exec ───▶ │ cat > ~/uison/prog && chmod +x     │
│ 3. run                   │ ──── exec ───▶ │ ~/uison/prog args --console        │
│ App / JSON tabs          │ ◀─── stdout ── │ frames                             │
│                          │ ──── stdin ──▶ │ events, actions                    │
└──────────────────────────┘                └────────────────────────────────────┘
```

The SSH library (russh with `ring`, and tokio on one thread) is the **only external dependency in the repository** and lives in this crate alone: the framework and the program on the server still know nothing but stdin and stdout. The player also runs on a phone (checked on Android 16): **Choose key…** takes a key through the system file chooser and keeps a private copy in the app's folder, **New key** creates an ed25519 key on the device and shows the line for `authorized_keys`, and a program laid out for a desktop is drawn scaled down.

## What it costs

Measured with Disk Usage on `/System/Library/Frameworks` (about 3,000 chart sectors):

| | |
|---|---|
| a frame with the chart | ~125 KB of JSON |
| the program per hover event | 1.7 ms (app 1.0, parsing its own frame 0.4, `compact` 0.3) |
| event → frame back through a local pipe | 1.85 ms (in a normal window: 1.4 ms) |
| zoom animation through a pipe | ~55 frames per second, 6–7 MB/s |
| the same over SSH to a cloud server | round trip 40 ms; hover → frame 204 ms (median) |

Locally, a frame per change costs almost nothing. Over the internet, sending the whole frame is the slow part. That is why the next step for remote windows sends the **screen once and then data changes and actions**, with layout and fonts on the viewer's side, rather than frames.

## Design rules

- `std` only; system APIs through hand-written FFI. Binary size matters (Studio, the main demo, is ~840 KB on macOS and ~220 KB of gzipped WebAssembly), but not more than quality.
- Logic in the host language, never in JSON: JSON holds data, templates, bindings and action names.
- One frame model for every platform: a backend draws primitives and reports events, and that is all it knows.
- Every step can be checked automatically: scenarios, screenshots and hit maps on every backend; the core lessons in three languages produce pixel-identical screenshots.
