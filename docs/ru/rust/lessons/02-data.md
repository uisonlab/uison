# Урок 2. Данные и привязки

[English](../../../rust/lessons/02-data.md) · **Русский**

**Что будет:** поле «Name» и приветствие, которое меняется по мере ввода; переключатель, который его прячет; теги-значки и поле, которое добавляет новые.

![Урок 2](../../../rust/img/02-data.png)

<!-- code: crates/uison/examples/02-data.rs -->
```rust
//! Lesson 2: data and bindings. Elements read data by paths (`@name`), an input field writes it;
//! code changes the data, and the screen follows it by itself.
use uison::prelude::*;

fn main() {
    let screen = column().padding(20).gap(12).cross("stretch").children([
        // Two-way binding: the field writes to name, the text below reads it.
        text_field().key("name").label("Name").bind("name"),
        text("Hello, {@name}!").size(20).bold().visible("@show"),
        switch("Show greeting").key("show").bind("show"),
        // A list: for_each repeats the template for each element of the tags array.
        wrap().gap(6).children([for_each("tags", "t", badge("@t"))]),
        row().gap(8).children([
            text_field().key("draft").bind("draft").placeholder("new tag").flex(1).on_submit(add),
            button("Add").key("add").disabled("!@draft").on_click(add),
        ]),
        // A conditional value: a switch on data, in the screen language, as a string.
        text("Tags: {@count}").color(json(r#"{ switch: "@many", cases: { true: "$theme.danger" }, default: "$theme.muted" }"#)),
    ]);
    let mut ui = Ui::from_node(screen).with_size(420.0, 360.0);
    ui.data.set("name", "World");
    ui.data.set("show", true);
    ui.data.set("tags", vec!["rust", "c++"]);
    ui.data.set("draft", "");
    // Derived data: recomputed after any change.
    ui.derive(|d| {
        let n = d.len("tags");
        d.set("count", n);
        d.set("many", n >= 5);
    });
    run(ui, "Lesson 2 — data");
}

/// One function for two events: Enter in the field and the button.
fn add(cx: &mut Cx) {
    let tag = cx.data.str("draft").to_string();
    cx.data.push("tags", tag);
    cx.data.set("draft", "");
}
```

## Как это работает

1. **Двусторонняя привязка.** `text_field().bind("name")` — поле показывает `name` и пишет в него при вводе. `text("Hello, {@name}!")` читает то же значение — приветствие меняется на каждую букву, без обработчиков. `bind` у `checkbox` и `switch` привязывает `checked`, у остальных — `value`.
2. **Видимость.** `.visible("@show")` — узел есть, пока `show` истинно. `"!@show"` — наоборот. Истинность — как в JavaScript: ложь — `false`, `0`, `""`, `[]`, `null`.
3. **Списки.** `for_each("tags", "t", badge("@t"))` повторяет шаблон для каждого элемента массива `tags`; внутри `@t` — текущий элемент. Добавили элемент в массив — значок появился.
4. **Одна функция на два события.** `add` — обычная функция `fn(&mut Cx)`: её можно отдать и `on_submit` (Enter в поле), и `on_click`. `cx.data.push("tags", …)` добавляет в конец массива.
5. **Производные данные.** `ui.derive(|d| …)` вызывается после каждого изменения данных: здесь он считает теги. Экран привязан к результату (`{@count}`), поэтому счётчик всегда верный — где бы ни менялись теги.
6. **Выбор значения.** `json(r#"{ switch: "@many", cases: { true: "$theme.danger" }, default: "$theme.muted" }"#)` — значение на языке экранов: цвет по данным. Такие селекторы и функции (`eq`, `not`, `len`…) — в [справочнике привязок](../../screens/bindings.md).

## Попробуйте

- Не добавлять тег, который уже есть.
- Удаление по нажатию на значок: `.key("tag-{@t}")` и `.on_click(…)` у `badge`; какой — по `cx.key`.

Дальше — [урок 3: форма с проверкой](03-forms.md).
