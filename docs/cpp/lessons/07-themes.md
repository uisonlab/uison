# Lesson 7. Themes and tokens

**English** · [Русский](../../ru/cpp/lessons/07-themes.md)

**What you'll get:** "Light / Dark / System" tabs and five color circles: picking one recolors the checkbox, the slider and the button.

![Lesson 7](../img/07-themes.png)

<!-- code: examples-cpp/lessons/07-themes/main.cpp -->
```cpp
// Lesson 7: themes and tokens. Element colors are theme tokens ($theme.primary, $theme.card…);
// cx.theme() changes the theme, cx.token() a single token.
#include <string>
#include <vector>

#include <uison_widgets.hpp>

using namespace uison::widgets;

struct State {
    std::string theme = "light", accent = "#007AFF";
    bool agree = true;
    int volume = 60;
    std::vector<std::string> accents{"#007AFF", "#34C759", "#FF9500", "#AF52DE", "#FF2D55"};
};

int main() {
    auto accent = [](uison::Cx& cx) {
        // key "accent-#34C759" → the color after the dash.
        auto color = std::string(cx.key().substr(cx.key().find('-') + 1));
        cx.token("primary", color);
        cx.data().set("accent", color);
    };
    auto screen = Column({.padding = 20, .gap = 14, .cross = "stretch", .children = {
        Tabs({.value = "@theme",
              .tabs = uison::choices({{"light", "Light"}, {"dark", "Dark"}, {"system", "System"}}),
              .key = "theme",
              .on_change = [](uison::Cx& cx) { cx.theme(cx.data().str("theme")); }}),
        Row({.gap = 8, .children = {For({.each = "@accents", .as = "c", .item = Box({
            .fill = "@c", .radius = 16, .w = 32, .h = 32, .key = "accent-{@c}", .on_click = accent})})}}),
        Card({.title = "Example", .children = {
            Checkbox("Checkbox", {.checked = "@agree"}),
            Slider({.value = "@volume", .min = 0, .max = 100}),
            Row({.gap = 8, .children = {Button("Primary"), Button("Secondary", {.variant = "secondary"})}}),
        }}),
    }});
    uison::Ui ui(screen, {.size = {440, 380}, .data = State{}, .theme = "light"});
    return ui.run("Lesson 7 — themes");
}
```

## How it works

1. **Tokens.** Library elements take their colors from theme tokens: `$theme.primary`, `$theme.card`, `$theme.text`… Change the theme and everything is recolored ([themes.md](../../screens/themes.md)).
2. **The whole theme.** `cx.theme("dark")` in a handler, `.theme = "light"` in `uison::Screen` (or `ui.theme("light")`) before launch. `"system"` follows the system, and the screen switches along with the OS on its own.
3. **A single token.** `cx.token("primary", "#AF52DE")` changes the primary color on top of the theme — the checkbox, slider and button are recolored at once.
4. **Color from data.** `Box({.fill = "@c", …})` is a binding: the color is an element of the `accents` array.
5. **State as a struct.** `State` with default values is the screen's initial data (`.data = State{}`).
6. **Tabs.** `Tabs({.value = "@theme", .tabs = uison::choices({{"light", "Light"}, …})})` — the choice is written to `theme`, and `.on_change` is called after that.

## Try it

- Remember the choice between launches: write the theme to a file in `.on_change` and read it at startup.
- `cx.token("radius", 2)` — all elements become almost square.

Next: [lesson 8: images, tooltips, menus](08-rich.md).
