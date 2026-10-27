# Lesson 2. Data and bindings

**English** · [Русский](../../ru/cpp/lessons/02-data.md)

**What you'll get:** a "Name" field and a greeting that changes as you type; a switch that hides it; tag badges and a field that adds new ones.

![Lesson 2](../img/02-data.png)

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

## How it works

1. **Two-way binding.** `TextField({.value = "@name"})` shows `name` and writes to it as you type. `Text("Hello, {@name}!")` reads the same value — the greeting changes with every letter, with no handlers.
2. **Visibility.** `.visible = "@show"` — the node exists while `show` is true; `"!@show"` — the opposite. False values are `false`, `0`, `""`, `[]`, `null`.
3. **Lists.** `For({.each = "@tags", .as = "t", .item = Badge("@t")})` repeats the template for each element of `tags`; inside it, `@t` is the current element.
4. **One lambda for two events.** `add` is given both to `.on_submit` (Enter in the field) and to `.on_click`.
5. **Data in code.** `d.get<std::string>("draft")`, `d.set("draft", "")`, `d.push("tags", …)` — values are converted to C++ types and back: numbers, strings, `bool`, `vector`, `map`, `optional`, structs ([lesson 5](05-structs.md)).
6. **Derived data.** `ui.derive([](uison::Data& d) { … })` is called after every change: here it counts the tags, and the screen is bound to the result (`{@count}`).
7. **Choosing a value.** `uison::json(R"({ switch: "@many", cases: { true: "$theme.danger" }, default: "$theme.muted" })")` is a value in the screen language right inside the widget: a color chosen by the data ([bindings](../../screens/bindings.md)).

## Try it

- Don't add a tag that already exists.
- Remove a tag by clicking its badge: `Badge("@t", {.key = "tag-{@t}", .on_click = …})`, which one — from `cx.key()`.

Next: [lesson 3: a form with validation](03-forms.md).
