# Урок 1. Окно и кнопка

[English](../../../cpp/lessons/01-hello.md) · **Русский**

**Что будет:** окно с надписью «Pressed N times» и кнопкой «+1».

![Урок 1](../../../cpp/img/01-hello.png)

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

## Как это работает

1. **Экран — виджеты.** `Center`, `Column`, `Text`, `Button` — функции из `uison_widgets.hpp`: свойства — назначенными инициализаторами (`{.gap = 12, .cross = "center"}`), главное свойство (текст, иконка) — первым аргументом. Получается то же дерево, что в JSON `{ Center: { child: { Column: { gap: 12, … } } } }` ([элементы](../../screens/elements.md)).
2. **Поля — по порядку объявления.** Так требует C++20: свои свойства → размеры (`w`, `flex`…) → общие (`key`, `visible`, `disabled`, `tooltip`…) → события (`on_click`…) → `child` / `children`. IDE подсказывает поля и их описания; ошибка в имени поля — ошибка компиляции.
3. **Данные и привязки.** `.data = uison::obj{{"n", 0}}` — начальные данные. `"Pressed {@n} times"` — подставить `n`; данные изменились — экран пересобрался сам ([привязки](../../screens/bindings.md)).
4. **Обработчик — лямбда у кнопки.** `.on_click = [](uison::Cx& cx) { … }`; `cx.data()` — данные, `update<int>("n", f)` читает значение как `int`, меняет и записывает. Можно и без параметра: `[] { … }`, и имя действия строкой: `.on_click = "inc"` + `ui.on("inc", …)`.
5. **`key`.** Имя узла: по нему кнопку нажимают сценарии проверки (`["tap", "inc"]`) и видно, какая кнопка нажата.
6. **`uison::Ui`.** Экран и остальное о нём — структурой `uison::Screen`: `.size`, `.data`, `.theme`, `.components`. `ui.run("…")` открывает окно; ошибка в описании экрана — исключение `uison::Error` с местом.

## Попробуйте

- Добавьте кнопку «−1».
- Сделайте «+1» недоступной, когда `n` больше 10: `.disabled = "@big"`, а `big` пусть пишет `derive` ([урок 2](02-data.md)).
- Напечатайте `screen.json().str()` — тот же экран в JSON.

Дальше — [урок 2: данные и привязки](02-data.md).
