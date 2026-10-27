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
