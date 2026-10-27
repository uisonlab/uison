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
