# uison in C

**English** · [Русский](../ru/c/README.md)

The screen is JSON right in your code (`UISON_JSON({ … })`, no quote escaping) or in a file, the logic is C functions, and the window, drawing and input are uison's job. One program runs on macOS, Windows, Linux (Wayland and X11), iOS, Android and in the browser. You link a static or dynamic library with the `uison.h` header (C99); helpers live in `uison_kit.h` (header-only).

- **[Quick start](quickstart.md)** — your first program, three ways to build it, static or dynamic.
- **[The screen language](../screens/README.md)** — the shared reference: bindings, layout, [elements](../screens/elements.md), themes.
- **[Phones](../mobile.md)** and **[the browser](../wasm.md)**.
- **API reference** — the comments in [`uison.h`](../../crates/uison-c/include/uison.h) and [`uison_kit.h`](../../crates/uison-c/include/uison_kit.h).
- **Examples** — [`examples-c/`](../../examples-c): `form2-c`, `app-c`, the `abi-test` and `kit-test` tests; `cmake -S examples-c -B out/c/build && cmake --build out/c/build`.

## Lessons

Each lesson is a program in [`examples-c/lessons`](../../examples-c/lessons). `tools/lessons.sh c` builds them all, runs the scenarios and updates the screenshots. Lessons marked with a star are the same program as in the [Rust](../rust/README.md) and [C++](../cpp/README.md) series: the screenshots of all three languages match pixel-for-pixel (`tools/lessons-compare.sh`).

| | lesson | about |
|---|---|---|
| 0 | [Setting up your system](lessons/00-setup.md) | compiler, CMake, Rust or a prebuilt package; macOS, Linux, Windows and Visual Studio |
| 1 ★ | [A window and a button](lessons/01-hello.md) | `UISON_JSON`, an action and a handler function, `uison_run`, an error in JSON and where it is |
| 2 ★ | [Data and bindings](lessons/02-data.md) | `@path`, `{@path}`, a text field, `For`, `derive`; `uison_data_pushf` |
| 3 ★ | [A form with validation](lessons/03-forms.md) | field errors, focus; the form as a struct — a field table |
| 4 ★ | [Lists](lessons/04-lists.md) | a `List` of 10,000 rows, the row `key`; an array of structs into data in one call |
| 5 | [Screens from files](lessons/05-files.md) | `uison_ui_load`, custom components and `$param`, `UISON_RELOAD` |
| 6 ★ | [Themes and tokens](lessons/06-themes.md) | light, dark, system; the accent color on the fly |
| 7 ★ | [Images, tooltips, menus](lessons/07-rich.md) | `Image`, `Avatar`, `tooltip`, `contextMenu`, `Link`, the clipboard |
| 8 ★ | [Threads](lessons/08-threads.md) | a worker thread (pthread / `_beginthreadex`), `uison_post`, progress, cancellation |
| 9 ★ | [Your own frame](lessons/09-frame.md) | `uison_app`: primitives and events by hand |
| 10 | [Everywhere](lessons/10-everywhere.md) | the same program on iPhone, Android and in the browser |
