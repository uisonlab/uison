# Урок 2. Данные и привязки

[English](../../../cpp/lessons/02-data.md) · **Русский**

**Что будет:** поле «Name» и приветствие, которое меняется по мере ввода; переключатель, который его прячет; теги-значки и поле, которое добавляет новые.

![Урок 2](../../../cpp/img/02-data.png)

<!-- code: examples-cpp/lessons/02-data/main.cpp -->
```cpp
// Lesson 2: data and bindings. Elements read data by paths (@name), an input field writes it;
// code changes the data, and the screen follows it by itself.
#include <string>

#include <uison_widgets.hpp>

using namespace uison::widgets;

int main() {
    // One lambda for two events: Enter in the field and the button.
    auto add = [](uison::Cx& cx) {
        auto d = cx.data();
        d.push("tags", d.get<std::string>("draft"));
        d.set("draft", "");
    };
    auto screen = Column({.padding = 20, .gap = 12, .cross = "stretch", .children = {
        // Two-way binding: the field writes to name, the text below reads it.
        TextField({.value = "@name", .label = "Name", .key = "name"}),
        Text("Hello, {@name}!", {.size = 20, .bold = true, .visible = "@show"}),
        Switch("Show greeting", {.checked = "@show", .key = "show"}),
        // A list: For repeats the template for each element of the tags array.
        Wrap({.gap = 6, .children = {For({.each = "@tags", .as = "t", .item = Badge("@t")})}}),
        Row({.gap = 8, .children = {
            TextField({.value = "@draft", .placeholder = "new tag", .flex = 1, .key = "draft", .on_submit = add}),
            Button("Add", {.key = "add", .disabled = "!@draft", .on_click = add}),
        }}),
        // A conditional value: a switch on data, in the screen language, as a string.
        Text("Tags: {@count}", {.color = uison::json(R"({ switch: "@many", cases: { true: "$theme.danger" }, default: "$theme.muted" })")}),
    }});
    uison::Ui ui(screen, {.size = {420, 360},
                          .data = uison::obj{{"name", "World"}, {"show", true}, {"tags", uison::arr{"rust", "c++"}}, {"draft", ""}}});
    // Derived data: recomputed after any change.
    ui.derive([](uison::Data& d) {
        auto n = d.len("tags");
        d.set("count", n);
        d.set("many", n >= 5);
    });
    return ui.run("Lesson 2 — data");
}
```

## Как это работает

1. **Двусторонняя привязка.** `TextField({.value = "@name"})` показывает `name` и пишет в него при вводе. `Text("Hello, {@name}!")` читает то же значение — приветствие меняется на каждую букву, без обработчиков.
2. **Видимость.** `.visible = "@show"` — узел есть, пока `show` истинно; `"!@show"` — наоборот. Ложь — `false`, `0`, `""`, `[]`, `null`.
3. **Списки.** `For({.each = "@tags", .as = "t", .item = Badge("@t")})` повторяет шаблон для каждого элемента `tags`; внутри `@t` — текущий элемент.
4. **Одна лямбда на два события.** `add` отдана и `.on_submit` (Enter в поле), и `.on_click`.
5. **Данные в коде.** `d.get<std::string>("draft")`, `d.set("draft", "")`, `d.push("tags", …)` — значения переводятся в типы C++ и обратно: числа, строки, `bool`, `vector`, `map`, `optional`, структуры ([урок 5](05-structs.md)).
6. **Производные данные.** `ui.derive([](uison::Data& d) { … })` вызывается после каждого изменения: здесь он считает теги, а экран привязан к результату (`{@count}`).
7. **Выбор значения.** `uison::json(R"({ switch: "@many", cases: { true: "$theme.danger" }, default: "$theme.muted" })")` — значение на языке экранов прямо в виджете: цвет по данным ([привязки](../../screens/bindings.md)).

## Попробуйте

- Не добавлять тег, который уже есть.
- Удаление по нажатию на значок: `Badge("@t", {.key = "tag-{@t}", .on_click = …})`, какой — по `cx.key()`.

Дальше — [урок 3: форма с проверкой](03-forms.md).
