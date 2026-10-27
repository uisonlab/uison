# Quick start: your own crate with uison

**English** · [Русский](../ru/rust/quickstart.md)

You need Rust ([lesson 0](lessons/00-setup.md)). Five commands, and you have your own window.

```sh
cargo new hello && cd hello
cargo add uison --git https://github.com/MarketKernel/uison     # or --path ../uison/crates/uison
```

`src/main.rs`:

<!-- code: crates/uison/examples/01-hello.rs -->
```rust
//! Lesson 1: a window with a counter. The screen is the Rust DSL, the handler is a closure right on the button.
use uison::prelude::*;

fn main() {
    // center → column → text + button: the same tree as { Center: { child: { Column: … } } } in JSON.
    let screen = center(column().gap(12).cross("center").children([
        // {@n} substitutes the value of n from data; the screen updates itself when n changes.
        text("Pressed {@n} times").size(20),
        button("+1").key("inc").on_click(|cx| cx.data.edit("n", |n: &mut i32| *n += 1)),
    ]));
    let mut ui = Ui::from_node(screen).with_size(320.0, 200.0);
    ui.data.set("n", 0);
    run(ui, "Lesson 1");
}
```

```sh
cargo run --release
```

The window with the counter opens in about a minute (the first build), after that in seconds. What is going on here — [lesson 1](lessons/01-hello.md).

## Program size

uison pulls in no dependencies: the program is a single file that needs only the system libraries. To keep it small (~700 KB on macOS, ~900 KB on Linux x86_64), add a profile to `Cargo.toml`, like uison itself has:

```toml
[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

On Windows, also put `.cargo/config.toml` next to `Cargo.toml`. Without it the program needs `VCRUNTIME140.dll` from the Visual C++ Redistributable, which a clean Windows doesn't have; with it vcruntime is linked in and the C runtime is the system one (part of Windows 10+). Lesson 1 with both: 618 KB instead of 1042 KB, no extra DLLs.

```toml
[target.'cfg(all(windows, target_env = "msvc"))']
rustflags = ["-C", "target-feature=+crt-static", "-C", "link-arg=/NODEFAULTLIB:libucrt.lib", "-C", "link-arg=/DEFAULTLIB:ucrt.lib"]
```

## Next steps

- [Lessons](README.md#lessons) — from a window with a button to threads, tests and your own frame.
- [The screen language](../screens/README.md) — the reference shared by Rust, C and C++.
- Phones and the browser — [lesson 11](lessons/11-everywhere.md).
