# Themes

**English** · [Русский](../ru/screens/themes.md)

A theme is a set of **tokens**: colors, radii, font sizes. Library elements take colors only from tokens (`"$theme.primary"`), so switching the theme recolors the whole screen. Your own components should do the same.

## Choosing a theme

| | how |
|---|---|
| in the screen file | `theme: "dark"` — `"light"`, `"dark"` or `"system"` (the default: follow the system) |
| from the program before launch | C `uison_ui_set_theme(ui, "dark")`, C++ `ui.theme("dark")`, Rust `ui.set_theme("dark")` |
| in a handler | C `uison_cx_set_theme(cx, "light")`, C++ `cx.theme("light")`, Rust `cx.set_theme("light")` |

With `"system"` the screen switches by itself when the system theme changes (macOS, Windows, GNOME, iOS, Android, the browser). Whether the system is currently dark is `@env.dark`.

## Your own token values

An object instead of a name gives tokens on top of the chosen theme:

```js
theme: { primary: "#FF2D55", radius: 4 }
```

On the fly, from a handler: C `uison_cx_set_token(cx, "primary", "\"#34C759\"")` (the value is JSON), C++ `cx.token("primary", "#34C759")`, Rust `cx.set_token("primary", "#34C759")`.

## Tokens

| token | used for |
|---|---|
| `primary`, `primaryHover`, `primaryPressed`, `primaryDisabled` | the primary color: buttons, checkboxes, sliders, links |
| `onPrimary` | text and icons on `primary` |
| `danger`, `dangerHover`, `dangerPressed` | destructive actions, errors |
| `success`, `warning` | success, warning |
| `background` | window background |
| `card` | cards, panels |
| `surface`, `surfaceHover`, `surfacePressed` | secondary buttons, input fields |
| `selection` | selected row or option |
| `textSelection` | selected text in an input field |
| `tooltip`, `onTooltip` | tooltip and its text |
| `text` | main text |
| `label` | element labels |
| `muted` | secondary text |
| `border`, `borderHover` | input field borders |
| `divider` | dividers |
| `track`, `trackHover`, `thumb` | the track and thumb of `Slider`, `Switch` |
| `tabActive` | the active `Tabs` tab |
| `shadow`, `scrollbar` | shadow, scroll bar |
| `avatars` | `Avatar` circle colors (picked by name) |
| `radius`, `radiusSm`, `radiusLg` | corner radii |
| `fontSize`, `fontSizeSm`, `fontSizeLg`, `fontSizeTitle` | font sizes |

The light and dark theme values are in `crates/uison-ui/lib/theme.json`.
