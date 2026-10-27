# Bindings and values

**English** · [Русский](../ru/screens/bindings.md)

A property value is a number, a string, `true`/`false`, an array or an object. Some strings and objects have a special meaning: they read data, component parameters or theme tokens, or pick one of several values. This is not code: there are no variables, branches or calls, only substitution and selection.

## Data: `@path`

```js
{ Text: { text: "@user.name" } }                       // value from the model
{ TextField: { value: "@form.email" } }                // two-way: typing writes to the model
{ Checkbox: { label: "I agree", checked: "@form.agree" } }
{ Button: { text: "Submit", disabled: "!@form.agree" } }  // negation
```

- Path segments are separated by dots; an array element is addressed by its index: `@todos.3.title`.
- `!@path` is negation (truthiness as described below).
- Input elements (`TextField`, `Checkbox`, `Switch`, `Slider`, `Select`, `Tabs`, `RadioGroup`) write to the model at their path themselves. The program reads the result from the data.
- Truthiness (`visible`, `disabled`, `!@`): `false`, `null`, `0`, `""`, `[]` and a missing value are false. Everything else is true.

## Text with substitution: `{@path}`

```js
{ Text: { text: "Hello, {@user.name}! Unread: {@inbox.unread}" } }
{ Button: { key: "del-{@t.id}", text: "Delete" } }
```

`{@path}` inside a string substitutes the value. `{$param}` and `{$theme.x}` work the same way. Literal braces are written `{{` and `}}`. A string that should start with `$`, `@` or `!@` as plain text doubles the first character: `"$$5"` shows `$5`, `"@@anna"` shows `@anna`. Text that comes from outside (user input, a web page) is escaped by `uison::ui::literal(text)`.

## Lists: `For` and `List`

```js
{ For: { each: "@todos", as: "t", do: { Text: { text: "@t.title" } } } }
{ For: { each: ["Morning", "Afternoon"], as: "p", index: "i", do: { Text: { text: "{@i}: {@p}" } } } }
{ List: { items: "@messages", as: "m", itemHeight: 56, item: { ListTile: { title: "@m.from" } } } }
```

- Inside the template, `@t` is the current item (`@t.title` is its field). `index` names the item's index; without it the index is available as `@index`.
- A binding inside the template is a path to the actual item (`@todos.3.done`): a `Checkbox` in a row changes that row's data.
- `List` is a virtualized scroll view: only visible rows are expanded, so 10,000 rows are no slower than 20. All rows have the same `itemHeight`.

## Component parameters `$param`

Inside your own component, a parameter is `$name` (for a literal, `$name.field`):

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

- Parameters are substituted in the caller's context: `value: "@stats.total"` becomes a binding to `stats.total` inside the component.
- A default value can refer to other parameters (`fg` in `Button` depends on `$variant`).
- A slot is a parameter holding a node or a list of nodes: `children`, `leading`. Insert it with `"$children"`.
- The caller's bindings are not visible inside a component: write `"$param"`, not `"@alias"`. For example, `key: "write-{$c.id}"`.
- A `key` inside a component body stays as written: there is no per-instance prefix. If there are several instances, build the key from a parameter (`key: "{$id}-head"`), otherwise `Ui` warns about a duplicate.

## Local state: `@~name`

```js
Collapse: {
  params: { title: "", children: [] },
  body: { Column: { children: [
    { Row: { toggle: "@~open", children: [ { Text: { text: "$title" } } ] } },
    { Box: { visible: "@~open", child: "$children" } },
  ] } },
}
```

`@~open` is a value of its own for each component instance (whether a `Select` is open, whether a section is expanded). It is stored in the model under `~.<key>.open`, so the instance needs a stable `key`.

## Theme tokens: `$theme.name`

```js
{ Box: { fill: "$theme.card", radius: "$theme.radiusLg", child: { Text: { text: "…", color: "$theme.muted" } } } }
```

Theme colors, radii and font sizes. Parameters and tokens live in different namespaces: a button has a `text` parameter, and the theme has a `text` color. `"$text"` is always the parameter; the color is `"$theme.text"`. The list of tokens is in [themes.md](themes.md).

## Selecting by state: `{ state: … }`

```js
fill: { state: { normal: "$theme.surface", hover: "$theme.surfaceHover", pressed: "$theme.surfacePressed", disabled: null } }
```

States: `normal`, `hover`, `pressed`, `focus`, `disabled`. Priority: `disabled` > `pressed` > `focus` > `hover` > `normal`. Interactive nodes have states (those with `on`, a behavior, or `interactive: true`). State takes descendants into account (like `:focus-within`): focus in the input highlights the `TextField` border.

## Selecting by value: `{ switch: … }`

```js
color: { switch: "@t.priority", cases: { high: "$theme.danger", low: "$theme.muted" }, default: "$theme.text" }
visible: { switch: "@filter", cases: { active: "!@t.done", done: "@t.done" }, default: true }
```

The value (`switch`) is compared with the `cases` keys as a string. If nothing matches, the result is `default`, or `null` without it.

## Value functions

| form | result |
|---|---|
| `{ eq: [A, B] }` | `true` if A and B are equal as strings (selected tab, radio button) |
| `{ not: X }` | negated truthiness |
| `{ lt: [A, B] }`, `{ gt: [A, B] }` | numeric comparison: `{ lt: ["@env.width", 600] }` means a narrow screen |
| `{ ratio: X, min, max, invert }` | the fraction of X within the range, 0…1 (`Slider`, `ProgressBar` fill) |
| `{ len: X }` | length of an array or string, or the number of fields in an object |
| `{ initials: X }` | first letters of the first two words: "Anna Petrova" → "AP" |
| `{ pick: X, from: [...] }` | a list item chosen by the string's hash (a stable avatar color) |

Arguments can be any values: bindings, parameters, other functions.

## Derived data

Anything computed from data (counters, form errors, totals) is calculated by the program in `derive`. It is called after every model change, and the screen simply binds to the result:

```js
{ Text: { text: "Left: {@stats.left}" } }
```

C: `uison_ui_derive(ui, f, user, drop)`, C++: `ui.derive([](uison::Data& d) { … })`, Rust: `ui.derive(|d| …)`.
