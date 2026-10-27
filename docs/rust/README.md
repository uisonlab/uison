# uison in Rust

**English** · [Русский](../ru/rust/README.md)

The screen is a Rust DSL (`column().gap(12).children([…])`) or a JSON file, the logic is closures right on the elements, and uison handles the window, drawing and input. One program runs on macOS, Windows, Linux (Wayland and X11), iOS, Android and in the browser. No dependencies — only std.

- **[Quick start](quickstart.md)** — your own crate with uison in five commands.
- **[The screen language](../screens/README.md)** — the shared reference: bindings, layout, [elements](../screens/elements.md), themes.
- **[Phones](../mobile.md)** and **[the browser](../wasm.md)**.

## Lessons

Each lesson is an example of the `uison` crate in [`crates/uison/examples`](../../crates/uison/examples): `cargo run --release -p uison --example 01-hello`. The `tools/lessons.sh rust` script builds them all, runs the scenarios and updates the screenshots. Lessons marked with a star are the same program as in the [C](../c/README.md) and [C++](../cpp/README.md) series: the screenshots of all three languages match pixel-for-pixel (`tools/lessons-compare.sh`).

| | lesson | what it covers |
|---|---|---|
| 0 | [Setting up your system](lessons/00-setup.md) | rustup, the system linker, an editor: macOS, Linux, Windows |
| 1 ★ | [A window and a button](lessons/01-hello.md) | DSL, a closure on the button, `Ui`, `run` |
| 2 ★ | [Data and bindings](lessons/02-data.md) | `@path`, `{@path}`, `bind`, `for_each`, `derive` |
| 3 ★ | [A form with validation](lessons/03-forms.md) | a form as a struct (`#[derive(Data)]`), errors, focus |
| 4 ★ | [Lists](lessons/04-lists.md) | `list` with 10,000 rows, row `key`, `Vec<Person>` into data |
| 5 | [A screen from JSON](lessons/05-json.md) | `source!`, `Ui::load`, custom components, `UISON_RELOAD`; DSL or JSON |
| 6 ★ | [Themes and tokens](lessons/06-themes.md) | light, dark, system; accent color on the fly |
| 7 ★ | [Images, tooltips, menus](lessons/07-rich.md) | `image`, `avatar`, `tooltip`, `context_menu`, `link`, clipboard |
| 8 ★ | [Threads](lessons/08-threads.md) | `ui.handle()`, `Handle::post`, `std::thread`, cancellation |
| 9 | [Testing screens](lessons/09-tests.md) | `cargo test` without a window: `tap`, `type_text`, `texts`; a `Headless` screenshot |
| 10 ★ | [Your own frame](lessons/10-frame.md) | the `App` trait, `Writer`: primitives and events by hand |
| 11 | [Everywhere](lessons/11-everywhere.md) | iOS, Android, the browser for your own crate |
