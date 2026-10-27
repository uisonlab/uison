# Lesson 1. A window and a button

**English** · [Русский](../../ru/cpp/lessons/01-hello.md)

**What you'll get:** a window with the label "Pressed N times" and a "+1" button.

![Lesson 1](../img/01-hello.png)

<!-- code: examples-cpp/lessons/01-hello/main.cpp -->
```cpp
// Lesson 1: a window with a counter. The screen is C++ widgets (like in Flutter), the handler is a lambda right on the button.
#include <uison_widgets.hpp>

using namespace uison::widgets;

int main() {
    // Center → Column → Text + Button: the same tree as { Center: { child: { Column: … } } } in JSON.
    auto screen = Center({.child = Column({.gap = 12, .cross = "center", .children = {
        // {@n} substitutes the value of n from data; the screen updates itself when n changes.
        Text("Pressed {@n} times", {.size = 20}),
        Button("+1", {.key = "inc", .on_click = [](uison::Cx& cx) {
            cx.data().update<int>("n", [](int n) { return n + 1; });
        }}),
    }})});
    uison::Ui ui(screen, {.size = {320, 200}, .data = uison::obj{{"n", 0}}});
    return ui.run("Lesson 1");
}
```

```sh
cmake --build out/cpp/build --target cpp-01-hello && ./out/cpp/build/cpp-01-hello
```

## How it works

1. **The screen is widgets.** `Center`, `Column`, `Text`, `Button` are functions from `uison_widgets.hpp`: properties are passed as designated initializers (`{.gap = 12, .cross = "center"}`), the main property (text, icon) as the first argument. You get the same tree as the JSON `{ Center: { child: { Column: { gap: 12, … } } } }` ([elements](../../screens/elements.md)).
2. **Fields go in declaration order.** C++20 requires it: the widget's own properties → sizes (`w`, `flex`…) → common ones (`key`, `visible`, `disabled`, `tooltip`…) → events (`on_click`…) → `child` / `children`. The IDE suggests the fields and their descriptions; a misspelled field name is a compile error.
3. **Data and bindings.** `.data = uison::obj{{"n", 0}}` is the initial data. `"Pressed {@n} times"` substitutes `n`; when the data changes, the screen rebuilds itself ([bindings](../../screens/bindings.md)).
4. **The handler is a lambda on the button.** `.on_click = [](uison::Cx& cx) { … }`; `cx.data()` is the data, `update<int>("n", f)` reads the value as an `int`, changes it and writes it back. You can also drop the parameter: `[] { … }`, or give an action name as a string: `.on_click = "inc"` + `ui.on("inc", …)`.
5. **`key`.** The node's name: scenarios press the button by it (`["tap", "inc"]`), and it shows which button was pressed.
6. **`uison::Ui`.** The screen and everything else about it come as a `uison::Screen` struct: `.size`, `.data`, `.theme`, `.components`. `ui.run("…")` opens the window; an error in the screen description throws a `uison::Error` exception with the location.

## Try it

- Add a "−1" button.
- Disable "+1" when `n` is greater than 10: `.disabled = "@big"`, and let `derive` write `big` ([lesson 2](02-data.md)).
- Print `screen.json().str()` — the same screen in JSON.

Next: [lesson 2: data and bindings](02-data.md).
