# Язык экранов

[English](../../screens/README.md) · **Русский**

Экран в uison — дерево узлов. Его можно записать JSON-ом (файл, строка в коде) или построить кодом: виджетами C++, Rust DSL, `UISON_JSON` в C. Во всех случаях получается одно и то же дерево, поэтому этот справочник общий для трёх серий уроков. В JSON нет кода: события вызывают **действия по имени**, а обработчики пишутся на вашем языке.

- [Привязки и значения](bindings.md) — `@path`, `{@path}`, `$param`, `$theme.x`, селекторы, функции.
- [Раскладка](layout.md) — `Row`, `Column`, `Stack`, размеры `w`/`h`/`flex`, прокрутка, списки.
- [Элементы](elements.md) — все узлы и элементы библиотеки со свойствами (генерируется из схемы).
- [Темы](themes.md) — токены, светлая и тёмная тема, свои темы.
- [События как JSON](events.md) — форма событий, сообщения экрану (`ui.apply`), события в сценариях.

## Файл экрана

```js
{
  size: [800, 600],                // window size in points (on a phone, the whole screen)
  theme: "system",                 // "light", "dark", "system" or an object of tokens on top of light
  data: { form: { name: "" } },    // initial data: the model the screen is bound to
  components: { /* your own elements: { Name: { params, body } } */ },
  safeArea: true,                  // root inside the phone's safe area (the default)
  root: { Column: { /* … */ } },   // the screen root: the only required key
}
```

Расширенный JSON: ключи можно писать без кавычек, после последнего элемента можно ставить запятую, разрешены комментарии `//` и `/* */`. Строки — только в двойных кавычках.

## Узел

Узел — объект с одним ключом. Ключ — тип узла с заглавной буквы, значение — свойства:

```js
{ Text: { text: "Hello", size: 15, bold: true } }
{ Column: { gap: 8, children: [ { Text: { text: "A" } }, { Text: { text: "B" } } ] } }
```

Типы бывают трёх видов:
- **встроенные узлы**: `Box`, `Row`, `Column`, `Stack`, `Text`, `Input`, `Image`, `Scroll`, `List`, `For`, фигуры;
- **элементы библиотеки**: `Button`, `TextField`, `Checkbox`, `Select`, `Tabs`… Они собраны из встроенных узлов в `lib/components.json`;
- **свои компоненты** из `components` файла экрана или из отдельного файла (урок «Свои компоненты» в каждой серии).

Неизвестное свойство — ошибка с местом: `Button at root/0/3: unknown parameter 'txt' (parameters: text, variant, …)`. Окно при этом показывает экран ошибки, а не падает.

## События и действия

```js
{ Button: { key: "add", text: "Add", on: { click: "add" } } }
{ TextField: { key: "draft", value: "@draft", on: { submit: "add", change: "typed" } } }
```

- `on` связывает событие (`click`, `change`, `submit`) с **именем действия**. Обработчик регистрирует программа: C — `UISON_ON(ui, add)`, C++ — `ui.on("add", …)`, Rust — `ui.on("add", |cx| …)`. В C++ и Rust можно и без имени — лямбдой прямо у элемента: `Button("+", {.on_click = …})`, `button("+").on_click(|cx| …)`.
- События всплывают к предкам: `on` у строки списка ловит нажатия всех её детей.
- В обработчике известны имя действия, событие и `key` узла (`cx.key`). Одно действие на все строки списка различает строки по key: `"del-{@t.id}"`.
- По имени действия работают и горячие клавиши (`shortcut("cmd+n", "add")`), и таймеры (`timer(500, "tick")`).

## key

`key` — имя узла. По нему работают:
- события: `cx.key`, `uison_cx_key`;
- состояние: наведение, фокус, каретка, прокрутка сохраняются, пока key тот же;
- фокус: `cx.focus("draft")`;
- поиск в дереве: `cx.tree.find("title")`;
- сценарии проверки (`scenarios/*.json`): `["tap", "add"]`.

Без `key` узел получает путь в дереве (`root/0/3`). Он меняется, когда меняется экран. Поэтому интерактивным элементам лучше давать свой key: `Ui` предупреждает, если key повторяется.

## Окружение `@env`

Данные, которые заполняет сама библиотека:

| путь | что |
|---|---|
| `@env.width`, `@env.height` | размер окна, pt |
| `@env.platform` | `macos`, `windows`, `linux`, `ios`, `android`, `web` |
| `@env.touch` | экран касаний (телефон) |
| `@env.dark` | тёмная тема системы |
| `@env.mod` | основной модификатор для подписей: `⌘` (Apple) или `Ctrl+` — `"{@env.mod}N"` |
| `@env.keyboard` | открыта экранная клавиатура |
| `@env.safe.top`, `.right`, `.bottom`, `.left` | безопасная зона телефона, pt |

Например, `visible: { lt: ["@env.width", 600] }` показывает узел только на узком экране.
