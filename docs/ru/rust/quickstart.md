# Быстрый старт: свой крейт с uison

[English](../../rust/quickstart.md) · **Русский**

Нужен Rust ([урок 0](lessons/00-setup.md)). Пять команд — и своё окно.

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

Окно со счётчиком откроется через минуту (первая сборка), дальше — за секунды. Что здесь происходит — [урок 1](lessons/01-hello.md).

## Размер программы

uison не тянет зависимостей: программа — один файл, которому нужны только системные библиотеки. Чтобы она была маленькой (~700 КБ на macOS, ~900 КБ на Linux x86_64), добавьте в `Cargo.toml` профиль, как у самого uison:

```toml
[profile.release]
opt-level = "s"
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

На Windows положите рядом с `Cargo.toml` ещё и `.cargo/config.toml`. Без него программе нужна `VCRUNTIME140.dll` из Visual C++ Redistributable, которой на чистой Windows нет; с ним vcruntime входит в программу, а библиотека C — системная (часть Windows 10+). Урок 1 с обоими: 618 КБ вместо 1042 КБ, без лишних DLL.

```toml
[target.'cfg(all(windows, target_env = "msvc"))']
rustflags = ["-C", "target-feature=+crt-static", "-C", "link-arg=/NODEFAULTLIB:libucrt.lib", "-C", "link-arg=/DEFAULTLIB:ucrt.lib"]
```

## Дальше

- [Уроки](README.md#уроки) — от окна с кнопкой до потоков, тестов и своего кадра.
- [Язык экранов](../screens/README.md) — справочник, общий для Rust, C и C++.
- Телефоны и браузер — [урок 11](lessons/11-everywhere.md).
