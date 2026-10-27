# Урок 5. Экран из JSON

[English](../../../rust/lessons/05-json.md) · **Русский**

**Что будет:** экран и свои элементы — JSON-файлы рядом с программой: две карточки-счётчика и строки с кнопками «−» и «+». Файлы можно править, пока программа работает.

![Урок 5](../../../rust/img/05-json.png)

<!-- code: crates/uison/examples/05-json.rs -->
```rust
//! Lesson 5: the screen and custom components are JSON files next to the program. `source!` embeds the file
//! into the program and remembers its name: with `UISON_RELOAD=folder` (debug build) the screen is read
//! from disk and rebuilt when the file is saved, without a restart.
use uison::prelude::*;

fn main() {
    let mut ui = match Ui::load(source!("05-json/screen.json")) {
        Ok(ui) => ui,
        Err(e) => return eprintln!("{e}"),
    };
    if let Err(e) = ui.load_components(source!("05-json/components.json")) {
        return eprintln!("{e}");
    }
    // One handler for several buttons: which one was pressed comes from the key ("inc-Tasks", "dec-Done").
    ui.on("inc", |cx| change(cx, 1.0));
    ui.on("dec", |cx| change(cx, -1.0));
    run(ui, "Lesson 5 — screen from JSON");
}

fn change(cx: &mut Cx, step: f64) {
    let path = if cx.key.contains("Tasks") { "tasks" } else { "done" };
    let v = (cx.data.num(path) + step).max(0.0);
    cx.data.set(path, v);
}
```

Экран — `05-json/screen.json`:

<!-- code: crates/uison/examples/05-json/screen.json -->
```js
// The lesson 5 screen: custom components Stat and Counter from components.json.
{
  size: [460, 300],
  data: { tasks: 3, done: 1 },
  root: { Column: { padding: 20, gap: 16, cross: "stretch", children: [
    { Row: { gap: 12, children: [
      { Stat: { title: "Tasks", value: "@tasks", icon: "check" } },
      { Stat: { title: "Done", value: "@done", icon: "star", color: "$theme.success" } },
    ] } },
    { Counter: { label: "Tasks" } },
    { Counter: { label: "Done" } },
  ] } }
}
```

Свои компоненты — `05-json/components.json`:

<!-- code: crates/uison/examples/05-json/components.json -->
```js
// Custom components: params are the parameters with default values, body is the tree,
// inside it parameters are $name. The screen uses them like built-in ones: { Stat: { title: … } }.
{
  Stat: {
    params: { title: "", value: 0, icon: "star", color: "$theme.primary" },
    body: {
      Box: {
        flex: 1, padding: 14, radius: "$theme.radius", fill: "$theme.card", stroke: "$theme.divider",
        child: { Column: { gap: 6, children: [
          { Row: { gap: 6, cross: "center", children: [
            { Icon: { name: "$icon", size: 16, color: "$color" } },
            { Text: { text: "$title", size: 12, color: "$theme.muted" } },
          ] } },
          { Text: { text: "$value", size: 26, bold: true } },
        ] } },
      },
    },
  },

  // The button keys are built from a parameter ("inc-{$label}"): each Counter has its own buttons,
  // while the handler is one; it tells which was pressed by the key.
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

1. **`source!`** встраивает файл в программу (как `include_str!`) и запоминает его имя. Программа — по-прежнему один файл, JSON рядом с ней не нужен.
2. **Живая перезагрузка.** Отладочная сборка с `UISON_RELOAD=folder` читает файлы с теми же именами из этой папки и перестраивает экран при сохранении — без перезапуска, данные сохраняются:

   ```sh
   UISON_RELOAD=crates/uison/examples/05-json cargo run -p uison --example 05-json
   ```

   Поменяйте в `screen.json` подпись или цвет и сохраните — окно обновится. В release-сборке перезагрузки нет.
3. **Свои компоненты.** `Stat` и `Counter` — шаблоны: `params` (параметры и значения по умолчанию) и `body` (дерево, параметры внутри — `$name`). В экране они используются как встроенные: `{ Stat: { title: "Tasks", value: "@tasks" } }`. Подробно — [справочник](../../screens/bindings.md#параметры-компонента-param).
4. **Обработчики по имени.** В JSON нет замыканий: кнопки называют действие (`on: { click: "inc" }`), а программа регистрирует его — `ui.on("inc", |cx| …)`.
5. **Один обработчик на много кнопок.** key кнопки собирается из параметра: `"inc-{$label}"` → `inc-Tasks`, `inc-Done`. Обработчик узнаёт кнопку по `cx.key`.

## DSL или JSON

| | DSL | JSON |
|---|---|---|
| обработчики | замыкания прямо у элементов | по имени, `ui.on` |
| проверка | имена методов — компилятор | имена свойств — при загрузке, с местом ошибки |
| правка | пересборка | живая перезагрузка |
| кто правит | программист | и дизайнер, и программа на другом языке (тот же файл открывают C и C++) |

Их можно смешивать: экран в JSON, а свой компонент — DSL-ом, и наоборот.

## Попробуйте

- Добавьте в `components.json` компонент `Progress` (полоса по `value` и `max`) и покажите его под счётчиками — не пересобирая программу.
- Сгенерируйте DSL для своих компонентов: `cargo run -p uison-gen -- --rust components.json` — функции `stat()`, `counter()` и методы их свойств.

Дальше — [урок 6: темы и токены](06-themes.md).
