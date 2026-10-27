# Lesson 1. A window and a button

**English** · [Русский](../../ru/rust/lessons/01-hello.md)

**What you'll get:** a window with the label "Pressed N times" and a "+1" button.

![Lesson 1](../img/01-hello.png)

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
cargo run --release -p uison --example 01-hello
```

## How it works

1. **A screen is a tree of nodes.** `center`, `column`, `text`, `button` are DSL functions; each returns a node (`Node`), and chained methods set properties: `.gap(12)`, `.size(20)`. You get the same tree as in the JSON screen file `{ Center: { child: { Column: { gap: 12, … } } } }` — `screen.to_json()` prints it. All elements and properties are in the [reference](../../screens/elements.md); rust-analyzer hints show their descriptions.
2. **Data.** `ui.data` is the application model: a JSON tree whose values you reach by paths (`"n"`, `"form.email"`, `"todos.3.done"`). `ui.data.set("n", 0)` stores the initial value.
3. **Bindings.** `"Pressed {@n} times"` substitutes the value of `n`. When the data changes, the screen rebuilds itself: you don't have to update the text by hand ([bindings](../../screens/bindings.md)).
4. **The handler is a closure on the button.** `.on_click(|cx| …)` receives the context `cx`: `cx.data` is the same data. `edit("n", |n: &mut i32| *n += 1)` reads the value as `i32`, changes it and writes it back.
5. **`key`.** The node's name: scenarios press the button by it (`["tap", "inc"]`), and it also tells you which button was pressed when one handler serves several.
6. **Running.** `Ui::from_node(screen).with_size(320.0, 200.0)` sets the screen and the window size; `run(ui, "Lesson 1")` opens the window. On a phone and in the browser it is the same `main` — [lesson 11](11-everywhere.md).

`use uison::prelude::*` brings in everything you need in one line: the DSL, `Ui`, `Cx`, data, `run`.

## Try it

- Add a "−1" button.
- Make "+1" disabled when `n` is greater than 10: `.disabled("@big")`, and let a handler write `big` (or `derive` from [lesson 2](02-data.md)).
- Print `screen.to_json()` — the same screen in JSON.

Next — [lesson 2: data and bindings](02-data.md).
