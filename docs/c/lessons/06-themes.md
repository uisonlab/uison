# Lesson 6. Themes and tokens

**English** · [Русский](../../ru/c/lessons/06-themes.md)

**What you'll get:** "Light / Dark / System" tabs and five color circles: picking one recolors the checkbox, the slider and the button.

![Lesson 6](../img/06-themes.png)

<!-- code: examples-c/lessons/06-themes/main.c -->
```c
/* Lesson 6: themes and tokens. Element colors are theme tokens ($theme.primary, $theme.card…);
   uison_cx_set_theme changes the theme, uison_cx_set_token a single token. */
#include <stdlib.h>
#include <string.h>

#include <uison_kit.h>

static void theme(uison_cx* cx, void* user) {
    (void)user;
    uison_cx_set_theme(cx, uison_data_str(uison_cx_data(cx), "theme"));
}

static void accent(uison_cx* cx, void* user) {
    /* key "accent-#34C759" → the color after the dash; the token value is JSON: a quoted string (uison_fmt). */
    const char* color = strrchr(uison_cx_key(cx), '-') + 1;
    char* json = uison_fmt("%s", color);
    (void)user;
    uison_cx_set_token(cx, "primary", json);
    uison_data_set_str(uison_cx_data(cx), "accent", color);
    free(json);
}

int main(void) {
    uison_ui* ui = uison_ui_new(UISON_JSON({
        size: [440, 380],
        data: { theme: "light", accent: "#007AFF", agree: true, volume: 60,
                accents: ["#007AFF", "#34C759", "#FF9500", "#AF52DE", "#FF2D55"] },
        root: { Column: { padding: 20, gap: 14, cross: "stretch", children: [
            { Tabs: { key: "theme", value: "@theme", on: { change: "theme" },
                      tabs: [{ value: "light", label: "Light" }, { value: "dark", label: "Dark" }, { value: "system", label: "System" }] } },
            { Row: { gap: 8, children: [ { For: { each: "@accents", as: "c", do: {
                Box: { key: "accent-{@c}", w: 32, h: 32, radius: 16, fill: "@c", on: { click: "accent" } } } } } ] } },
            { Card: { title: "Example", children: [
                { Checkbox: { label: "Checkbox", checked: "@agree" } },
                { Slider: { value: "@volume", min: 0, max: 100 } },
                { Row: { gap: 8, children: [ { Button: { text: "Primary" } }, { Button: { text: "Secondary", variant: "secondary" } } ] } },
            ] } },
        ] } }
    }));
    if (!ui) return uison_kit_fail();
    uison_ui_set_theme(ui, "light");
    UISON_ON(ui, theme);
    UISON_ON(ui, accent);
    return uison_run(ui, "Lesson 6 — themes");
}
```

## How it works

1. **Tokens.** Library elements take their colors from theme tokens: `$theme.primary`, `$theme.card`, `$theme.text`… Change the theme and everything is recolored ([themes.md](../../screens/themes.md)).
2. **The whole theme.** `uison_cx_set_theme(cx, "dark")` in a handler, `uison_ui_set_theme(ui, "light")` before running. `"system"` follows the system, and the screen switches by itself when the user changes the OS theme.
3. **A single token.** `uison_cx_set_token(cx, "primary", value)` changes the primary color on top of the theme. The value is JSON: a quoted string `"\"#AF52DE\""`. `uison_fmt("%s", color)` builds it without escaping by hand (free the string with `free`).
4. **A color from data.** `fill: "@c"` on a circle is a binding: the color is an element of the `accents` array.
5. **Tabs.** `Tabs { value: "@theme", tabs: [{ value, label }…] }` writes the choice into `theme`, and `on: { change: "theme" }` is invoked afterwards.

## Try it

- Remember the choice between runs: write the theme to a file in the `theme` handler and read it at startup.
- `uison_cx_set_token(cx, "radius", "2")` — all elements become almost square.

Next: [lesson 7: images, tooltips, menus](07-rich.md).
