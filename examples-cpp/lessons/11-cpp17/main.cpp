// Lesson 11 (C++17): a screen without C++20 widgets, as a uison::Node method chain; a custom component as a JSON string.
#include <string>

#include <uison.hpp>

// A custom component in the same JSON as in a components.json file (R"json(…)json": JSON may contain `)"`).
static const char* COMPONENTS = R"json({
  Note: {
    params: { text: "" },
    body: { Box: { padding: [8, 12], radius: 8, fill: "$theme.card", stroke: "$theme.divider", width: 1,
                   child: { Text: { text: "$text" } } } },
  },
})json";

int main() {
    // One lambda both for the button and for Enter in the field.
    auto add = [](uison::Cx& cx) {
        auto d = cx.data();
        if (d.str("draft").empty()) return;
        d.push("notes", d.get<std::string>("draft"));
        d.set("draft", "");
    };
    auto screen = uison::column().padding(20).gap(12).cross("stretch").children({
        uison::text("Notes").size(22).bold(),
        uison::row().gap(8).children({
            uison::text_field().bind("draft").placeholder("What to remember?").flex().key("draft").on_submit(add),
            uison::button("Add").icon("plus").key("add").disabled("!@draft").on_click(add),
        }),
        uison::for_each("notes", "n", uison::node("Note").prop("text", "@n")),
        uison::text("Total: {@count}").color("$theme.muted"),
    });
    // The rest of the screen settings as {key, value} pairs (in C++20, the uison::Screen struct).
    uison::Ui ui(screen, {{"size", uison::arr{420, 380}}, {"data", uison::obj{{"draft", ""}, {"notes", uison::arr{"Buy bread"}}}}});
    ui.components(COMPONENTS);
    ui.derive([](uison::Data& d) { d.set("count", d.len("notes")); });
    return ui.run("Lesson 11 — C++17");
}
