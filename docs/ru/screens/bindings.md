# Привязки и значения

[English](../../screens/bindings.md) · **Русский**

Значение свойства — число, строка, `true`/`false`, массив или объект. Некоторые строки и объекты имеют особый смысл: они читают данные, параметры компонента, токены темы или выбирают одно из значений. Это не код: нет переменных, ветвлений и вызовов, только подстановка и выбор.

## Данные: `@path`

```js
{ Text: { text: "@user.name" } }                       // value from the model
{ TextField: { value: "@form.email" } }                // two-way: typing writes to the model
{ Checkbox: { label: "I agree", checked: "@form.agree" } }
{ Button: { text: "Submit", disabled: "!@form.agree" } }  // negation
```

- Путь — через точку; элемент массива — номером: `@todos.3.title`.
- `!@path` — отрицание (истинность — как ниже).
- Элементы ввода (`TextField`, `Checkbox`, `Switch`, `Slider`, `Select`, `Tabs`, `RadioGroup`) пишут в модель по своему пути сами. Программа читает результат из данных.
- Истинность (`visible`, `disabled`, `!@`): ложь — `false`, `null`, `0`, `""`, `[]` и отсутствующее значение. Всё остальное — истина.

## Текст с подстановкой: `{@path}`

```js
{ Text: { text: "Hello, {@user.name}! Unread: {@inbox.unread}" } }
{ Button: { key: "del-{@t.id}", text: "Delete" } }
```

`{@path}` внутри строки подставляет значение. Так же работают `{$param}` и `{$theme.x}`. Литеральные фигурные скобки — `{{` и `}}`. Если строка должна начинаться с `$`, `@` или `!@` как обычный текст, первый знак удваивается: `"$$5"` покажет `$5`, `"@@anna"` — `@anna`. Текст извне (ввод пользователя, веб-страница) экранирует `uison::ui::literal(text)`.

## Списки: `For` и `List`

```js
{ For: { each: "@todos", as: "t", do: { Text: { text: "@t.title" } } } }
{ For: { each: ["Morning", "Afternoon"], as: "p", index: "i", do: { Text: { text: "{@i}: {@p}" } } } }
{ List: { items: "@messages", as: "m", itemHeight: 56, item: { ListTile: { title: "@m.from" } } } }
```

- Внутри шаблона `@t` — текущий элемент (`@t.title` — его поле). `index` задаёт имя номера элемента, без него номер доступен как `@index`.
- Привязка внутри шаблона — путь к настоящему элементу (`@todos.3.done`): `Checkbox` в строке меняет данные этой строки.
- `List` — прокрутка с виртуализацией: раскрываются только видимые строки, поэтому 10 000 строк не медленнее 20. `itemHeight` у строк одинаковый.

## Параметры компонента: `$param`

Внутри своего компонента параметр — `$name` (у литерала — `$name.field`):

```js
components: {
  Stat: {
    params: { label: "", value: 0 },           // parameters and default values
    body: { Column: { children: [
      { Text: { text: "$label", size: 12 } },
      { Text: { text: "{$value}", size: 22, bold: true } },
    ] } },
  },
}
// usage:
{ Stat: { label: "Tasks", value: "@stats.total" } }
```

- Параметры подставляются в контексте вызывающего: `value: "@stats.total"` внутри становится привязкой к `stats.total`.
- Значение по умолчанию может ссылаться на другие параметры (`fg` у `Button` зависит от `$variant`).
- Слот — параметр-узел или список узлов: `children`, `leading`. Вставка — `"$children"`.
- Внутри компонента привязки вызывающего не видны: пишите `"$param"`, а не `"@alias"`. Например, `key: "write-{$c.id}"`.
- `key` внутри тела компонента остаётся как написан: префикса экземпляра нет. Если экземпляров несколько, собирайте key из параметра (`key: "{$id}-head"`), иначе `Ui` предупредит о повторе.

## Локальное состояние: `@~name`

```js
Collapse: {
  params: { title: "", children: [] },
  body: { Column: { children: [
    { Row: { toggle: "@~open", children: [ { Text: { text: "$title" } } ] } },
    { Box: { visible: "@~open", child: "$children" } },
  ] } },
}
```

`@~open` — значение, своё у каждого экземпляра компонента (открыт ли `Select`, развёрнут ли раздел). Хранится в модели под `~.<key>.open`, поэтому у экземпляра должен быть устойчивый `key`.

## Токены темы: `$theme.name`

```js
{ Box: { fill: "$theme.card", radius: "$theme.radiusLg", child: { Text: { text: "…", color: "$theme.muted" } } } }
```

Цвета, радиусы и размеры шрифта темы. У параметров и токенов разные пространства имён: у кнопки есть параметр `text`, а в теме есть цвет `text`. `"$text"` — это всегда параметр, а цвет — `"$theme.text"`. Список токенов — в [themes.md](themes.md).

## Выбор по состоянию: `{ state: … }`

```js
fill: { state: { normal: "$theme.surface", hover: "$theme.surfaceHover", pressed: "$theme.surfacePressed", disabled: null } }
```

Состояния: `normal`, `hover`, `pressed`, `focus`, `disabled`. Приоритет: `disabled` > `pressed` > `focus` > `hover` > `normal`. Состояния есть у интерактивных узлов (с `on`, поведением, `interactive: true`). Состояние учитывает потомков (как `:focus-within`): фокус в поле подсвечивает рамку `TextField`.

## Выбор по значению: `{ switch: … }`

```js
color: { switch: "@t.priority", cases: { high: "$theme.danger", low: "$theme.muted" }, default: "$theme.text" }
visible: { switch: "@filter", cases: { active: "!@t.done", done: "@t.done" }, default: true }
```

Значение (`switch`) сравнивается с ключами `cases` как строка. Нет совпадения — `default`, без него — `null`.

## Функции-значения

| запись | результат |
|---|---|
| `{ eq: [A, B] }` | `true`, если A и B равны как строки (выбранная вкладка, радиокнопка) |
| `{ not: X }` | отрицание истинности |
| `{ lt: [A, B] }`, `{ gt: [A, B] }` | сравнение чисел: `{ lt: ["@env.width", 600] }` — узкий экран |
| `{ ratio: X, min, max, invert }` | доля X в диапазоне, 0…1 (заполнение `Slider`, `ProgressBar`) |
| `{ len: X }` | длина массива, строки или число полей объекта |
| `{ initials: X }` | первые буквы двух первых слов: «Anna Petrova» → «AP» |
| `{ pick: X, from: [...] }` | элемент списка по хешу строки (постоянный цвет аватара) |

Аргументы — любые значения: привязки, параметры, другие функции.

## Производные данные

То, что вычисляется из данных (счётчики, ошибки формы, итоги), считает программа в `derive`. Он вызывается после каждого изменения модели, и экран просто привязывается к результату:

```js
{ Text: { text: "Left: {@stats.left}" } }
```

C: `uison_ui_derive(ui, f, user, drop)`, C++: `ui.derive([](uison::Data& d) { … })`, Rust: `ui.derive(|d| …)`.
