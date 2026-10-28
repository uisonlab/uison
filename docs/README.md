# uison

**English** · [Русский](ru/README.md)

A small, fast, dependency-free UI framework: you describe a screen as a tree of elements (in JSON or in code), write the logic in Rust, C or C++, and uison takes care of the window, drawing and input. The same program runs on macOS, Windows, Linux (Wayland and X11), iOS, Android and in the browser.

```text
screen (JSON, Rust DSL, C++ widgets, UISON_JSON in C)  +  data  +  handlers in your language
        │  bindings, templates, themes, layout (uison-ui)
        ▼
frame of primitives: rect, line, circle, tri, text, image, clip, hit
        │  diff against the previous frame → redraw only what changed
        ▼
system backend: AppKit, Win32 + Direct2D, Wayland/X11 + Cairo, UIKit, Android Canvas, canvas 2D
```

## Choose a language

| | screen | logic | lesson series |
|---|---|---|---|
| **Rust** | method-chaining DSL or a JSON file | closures on elements, `#[derive(Data)]` | [docs/rust](rust/README.md) |
| **C** | JSON right in the code (`UISON_JSON`) or a file | functions; structs ↔ data via field tables | [docs/c](c/README.md) |
| **C++** | C++20 widgets, as in Flutter | lambdas on elements; structs ↔ data without macros | [docs/cpp](cpp/README.md) |

Each series reads from start to finish without the other two: lesson 0 is setting up your system (macOS, Linux, Windows), and the rest goes from a window with a button to threads and your own frame. Lessons marked with an asterisk are the same program in all three languages, and their screenshots match pixel-for-pixel.

Writing in assembly? [Two lessons in x86-64 (NASM) and AArch64](asm.md) call the same C API.

## Common to all languages

- [Architecture](architecture.md) — how the layers fit together, JSON at the core, console mode and the SSH player.
- [The screen language](screens/README.md) — the screen file, events, `key`, the `@env` environment.
- [Bindings and values](screens/bindings.md) — `@path`, `{@path}`, `$param`, `$theme.x`, selectors, functions.
- [Layout](screens/layout.md) — `Row`, `Column`, `Stack`, sizes, scrolling, lists.
- [Elements](screens/elements.md) — all nodes and library elements with their properties.
- [Themes](screens/themes.md) — tokens, light and dark, your own colors.
- [Phones](mobile.md) — iOS and Android.
- [The browser](wasm.md) — WebAssembly.
- [Console mode and the player](console.md) — a program without a window: frames out, events in, as JSON lines.
- [Agent mode](agents.md) — an LLM agent reads the screen by meaning and acts on it, over JSON lines or MCP, without screenshots.

## Headless testing

You can run any program through a scenario: `UISON_SCRIPT=scenario.json` performs taps, typing and scrolling by element `key`, and `UISON_CAPTURE=folder` saves window screenshots. This is how all the lessons (`tools/lessons.sh`), the examples and the phones are tested.
