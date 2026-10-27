# Урок 1. Окно и кнопка

[English](../../../rust/lessons/01-hello.md) · **Русский**

**Что будет:** окно с надписью «Pressed N times» и кнопкой «+1».

![Урок 1](../../../rust/img/01-hello.png)

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

## Как это работает

1. **Экран — дерево узлов.** `center`, `column`, `text`, `button` — функции DSL; каждая возвращает узел (`Node`), а методы цепочкой задают свойства: `.gap(12)`, `.size(20)`. Получается то же дерево, что в JSON-файле экрана `{ Center: { child: { Column: { gap: 12, … } } } }` — его печатает `screen.to_json()`. Все элементы и свойства — в [справочнике](../../screens/elements.md); подсказки rust-analyzer показывают их описания.
2. **Данные.** `ui.data` — модель приложения: JSON-дерево, к значениям — по путям (`"n"`, `"form.email"`, `"todos.3.done"`). `ui.data.set("n", 0)` кладёт начальное значение.
3. **Привязки.** `"Pressed {@n} times"` — подставить значение `n`. Когда данные меняются, экран пересобирается сам: обновлять текст руками не нужно ([привязки](../../screens/bindings.md)).
4. **Обработчик — замыкание у кнопки.** `.on_click(|cx| …)` получает контекст `cx`: `cx.data` — те же данные. `edit("n", |n: &mut i32| *n += 1)` читает значение как `i32`, меняет и записывает обратно.
5. **`key`.** Имя узла: по нему кнопку нажимают сценарии проверки (`["tap", "inc"]`), по нему же видно, какая кнопка нажата, если обработчик один на несколько.
6. **Запуск.** `Ui::from_node(screen).with_size(320.0, 200.0)` — экран и размер окна; `run(ui, "Lesson 1")` открывает окно. На телефоне и в браузере тот же `main` — [урок 11](11-everywhere.md).

`use uison::prelude::*` подключает всё нужное одной строкой: DSL, `Ui`, `Cx`, данные, `run`.

## Попробуйте

- Добавьте кнопку «−1».
- Сделайте «+1» недоступной, когда `n` больше 10: `.disabled("@big")`, а `big` пусть пишет обработчик (или `derive` из [урока 2](02-data.md)).
- Напечатайте `screen.to_json()` — тот же экран в JSON.

Дальше — [урок 2: данные и привязки](02-data.md).
