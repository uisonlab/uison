# The screen language

**English** · [Русский](../ru/screens/README.md)

A screen in uison is a tree of nodes. You can write it as JSON (a file or a string in code) or build it in code: with C++ widgets, the Rust DSL, or `UISON_JSON` in C. Either way you get the same tree, so this reference is shared by all three lesson series. There is no code in JSON: events trigger **actions by name**, and handlers are written in your language.

- [Bindings and values](bindings.md) — `@path`, `{@path}`, `$param`, `$theme.x`, selectors, functions.
- [Layout](layout.md) — `Row`, `Column`, `Stack`, sizes `w`/`h`/`flex`, scrolling, lists.
- [Elements](elements.md) — all nodes and library elements with their properties (generated from the schema).
- [Themes](themes.md) — tokens, light and dark themes, your own themes.
- [Events as JSON](events.md) — the event form, messages to a screen (`ui.apply`), events in scenarios.

## The screen file

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

Extended JSON: keys may be unquoted, a trailing comma after the last element is allowed, and `//` and `/* */` comments are permitted. Strings use double quotes only.

## Nodes

A node is an object with a single key. The key is the node type, capitalized; the value holds its properties:

```js
{ Text: { text: "Hello", size: 15, bold: true } }
{ Column: { gap: 8, children: [ { Text: { text: "A" } }, { Text: { text: "B" } } ] } }
```

There are three kinds of types:
- **built-in nodes**: `Box`, `Row`, `Column`, `Stack`, `Text`, `Input`, `Image`, `Scroll`, `List`, `For`, shapes;
- **library elements**: `Button`, `TextField`, `Checkbox`, `Select`, `Tabs`… They are built from built-in nodes in `lib/components.json`;
- **your own components** from the screen file's `components` or from a separate file (the "Custom components" lesson in each series).

An unknown property is an error with a location: `Button at root/0/3: unknown parameter 'txt' (parameters: text, variant, …)`. The window then shows an error screen instead of crashing.

## Events and actions

```js
{ Button: { key: "add", text: "Add", on: { click: "add" } } }
{ TextField: { key: "draft", value: "@draft", on: { submit: "add", change: "typed" } } }
```

- `on` maps an event (`click`, `change`, `submit`) to an **action name**. The program registers the handler: C — `UISON_ON(ui, add)`, C++ — `ui.on("add", …)`, Rust — `ui.on("add", |cx| …)`. In C++ and Rust you can also skip the name and attach a lambda right to the element: `Button("+", {.on_click = …})`, `button("+").on_click(|cx| …)`.
- Events bubble up to ancestors: `on` on a list row catches taps on all of its children.
- The handler knows the action name, the event and the node's `key` (`cx.key`). One action shared by all rows of a list tells the rows apart by key: `"del-{@t.id}"`.
- Keyboard shortcuts (`shortcut("cmd+n", "add")`) and timers (`timer(500, "tick")`) work by action name too.

## key

`key` is the node's name. It is used for:
- events: `cx.key`, `uison_cx_key`;
- state: hover, focus, caret and scroll position are kept as long as the key stays the same;
- focus: `cx.focus("draft")`;
- tree lookup: `cx.tree.find("title")`;
- test scenarios (`scenarios/*.json`): `["tap", "add"]`.

A node without a `key` gets its path in the tree (`root/0/3`), which changes whenever the screen changes. So give interactive elements their own key: `Ui` warns you if a key is repeated.

## The `@env` environment

Data filled in by the library itself:

| path | what |
|---|---|
| `@env.width`, `@env.height` | window size, pt |
| `@env.platform` | `macos`, `windows`, `linux`, `ios`, `android`, `web` |
| `@env.touch` | touch screen (phone) |
| `@env.dark` | the system uses a dark theme |
| `@env.mod` | the primary modifier for labels: `⌘` (Apple) or `Ctrl+` — `"{@env.mod}N"` |
| `@env.keyboard` | the on-screen keyboard is open |
| `@env.safe.top`, `.right`, `.bottom`, `.left` | the phone's safe area, pt |

For example, `visible: { lt: ["@env.width", 600] }` shows a node only on a narrow screen.
