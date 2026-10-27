// Lesson 1: a window with a counter. The screen is C++ widgets (like in Flutter), the handler is a lambda right on the button.
#include <uison_widgets.hpp>

using namespace uison::widgets;

int main() {
    // Center → Column → Text + Button: the same tree as { Center: { child: { Column: … } } } in JSON.
    auto screen = Center({.child = Column({.gap = 12, .cross = "center", .children = {
        // {@n} substitutes the value of n from data; the screen updates itself when n changes.
        Text("Pressed {@n} times", {.size = 20}),
        Button("+1", {.key = "inc", .on_click = [](uison::Cx& cx) {
            cx.data().update<int>("n", [](int n) { return n + 1; });
        }}),
    }})});
    uison::Ui ui(screen, {.size = {320, 200}, .data = uison::obj{{"n", 0}}});
    return ui.run("Lesson 1");
}
