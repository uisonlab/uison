# Урок 6. Свои компоненты

[English](../../../cpp/lessons/06-components.md) · **Русский**

**Что будет:** свои элементы `Stat` (карточка с числом) и `Counter` (строка с кнопками «−» и «+») — в JSON-файле, а в коде — как стандартные виджеты, с подсказками и проверкой имён.

![Урок 6](../../../cpp/img/06-components.png)

<!-- code: examples-cpp/lessons/06-components/main.cpp -->
```cpp
// Lesson 6: custom components. components.json holds the Stat and Counter templates; CMake generates
// C++ widgets for them (uison_generate_widgets → app_widgets.hpp), so custom elements are written the same
// way as standard ones: app::Stat({.title = …}), with hints and field names checked by the compiler.
#include <string>

#include "app_widgets.hpp"

using namespace uison::widgets;

int main() {
    auto screen = Column({.padding = 20, .gap = 16, .cross = "stretch", .children = {
        Row({.gap = 12, .children = {
            app::Stat({.title = "Tasks", .value = "@tasks", .icon = "check"}),
            app::Stat({.title = "Done", .value = "@done", .icon = "star", .color = "$theme.success"}),
        }}),
        app::Counter({.label = "Tasks"}),
        app::Counter({.label = "Done"}),
    }});
    // Components come from the same JSON as for the generator (CMake puts the file next to the program).
    uison::Ui ui(screen, {.size = {460, 300}, .data = uison::obj{{"tasks", 3}, {"done", 1}}});
    ui.components_file("components.json");
    // One handler for several buttons: which one was pressed comes from the key ("inc-Tasks", "dec-Done").
    auto change = [](uison::Cx& cx, int step) {
        auto path = cx.key().find("Tasks") != std::string_view::npos ? "tasks" : "done";
        int v = cx.data().get<int>(path) + step;
        cx.data().set(path, v < 0 ? 0 : v);
    };
    ui.on("inc", [change](uison::Cx& cx) { change(cx, 1); });
    ui.on("dec", [change](uison::Cx& cx) { change(cx, -1); });
    return ui.run("Lesson 6 — custom components");
}
```

Компоненты — `components.json`:

<!-- code: examples-cpp/lessons/06-components/components.json -->
```js
// Custom components: params are the parameters with default values (a comment on a parameter ends up
// in the widget's hints), body is the tree, inside it parameters are $name.
{
  Stat: {
    params: {
      title: "", // caption
      value: 0, // the number, large
      icon: "star", // an icon from the Icon set
      color: "$theme.primary", // icon color
    },
    body: {
      Box: {
        flex: 1, padding: 14, radius: "$theme.radius", fill: "$theme.card", stroke: "$theme.divider",
        child: { Column: { gap: 6, children: [
          { Row: { gap: 6, cross: "center", children: [
            { Icon: { name: "$icon", size: 16, color: "$color" } },
            { Text: { text: "$title", size: 12, color: "$theme.muted" } },
          ] } },
          { Text: { text: "{$value}", size: 26, bold: true } },
        ] } },
      },
    },
  },

  // Button keys come from a parameter: each Counter has its own buttons, one handler (it tells the button by key).
  Counter: {
    params: { label: "" },
    body: {
      Row: { gap: 8, cross: "center", children: [
        { Text: { text: "$label", flex: 1 } },
        { IconButton: { icon: "minus", size: "sm", key: "dec-{$label}", on: { click: "dec" } } },
        { IconButton: { icon: "plus", size: "sm", key: "inc-{$label}", on: { click: "inc" } } },
      ] },
    },
  },
}
```

## Как это работает

1. **Компонент** — шаблон: `params` (параметры и значения по умолчанию) и `body` (дерево, параметры внутри — `$name`). Комментарий у параметра попадает в подсказку поля виджета.
2. **Виджеты из генератора.** В `CMakeLists.txt`:

   ```cmake
   uison_generate_widgets(cpp-06-components lessons/06-components/components.json)
   ```

   CMake генерирует `app_widgets.hpp` (пространство имён `app`) и пересобирает его, когда меняется `components.json`. В коде — `app::Stat({.title = …, .value = …})`: поля — параметры компонента, плюс общие (`w`, `flex`, `key`…) и события. Генератор — `uison-gen` (собирается из исходников или лежит в пакете, `bin/uison-gen`); имя пространства и файла — `NAMESPACE` и `OUTPUT`.
3. **Компоненты в `Ui`.** Виджет строит только узел `{ Stat: { … } }`, а что такое `Stat` — `Ui` узнаёт из того же JSON: `ui.components_file("components.json")` (файл CMake кладёт рядом с программой) или `.components = R"json({…})json"` в `uison::Screen`.
4. **Один обработчик, много кнопок.** key кнопки собирается из параметра: `"inc-{$label}"` → `inc-Tasks`. Обработчик узнаёт кнопку по `cx.key()`.

## Попробуйте

- Добавьте параметр `hint` в `Stat` — после сборки он сразу появится полем `app::Stat({.hint = …})`.
- Компонент `Progress` (полоса по `value` и `max`) под счётчиками.

Дальше — [урок 7: темы и токены](07-themes.md).
