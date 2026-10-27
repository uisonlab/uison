// Lesson 8: images, tooltips, a context menu, links, the clipboard. Everything except images
// is just properties; an image is given as a file or as bytes (PNG or JPEG).
#include <uison_widgets.hpp>

using namespace uison::widgets;

static const char* SITE = "https://github.com/MarketKernel/uison";

int main() {
    auto copy = [](uison::Cx& cx) {
        cx.copy("Alice Parker");
        cx.data().set("status", "Name copied to clipboard");
    };
    auto card = Row({.padding = 12, .gap = 14, .cross = "center", .key = "card",
        // Context menu: label, action; "-" is a separator. Actions are called by name (ui.on below).
        .context_menu = uison::json(R"([{ label: "Copy name", action: "copy" }, "-", { label: "Open website", action: "site" }])"),
        .children = {
            Avatar("Alice Parker", {.size = 56, .src = "@photo"}),
            Column({.gap = 4, .flex = 1, .children = {
                Text("Alice Parker", {.size = 16, .bold = true}),
                Link("github.com/MarketKernel/uison", {.href = SITE}),
            }}),
            // Tooltip: on any node (appears after half a second of hovering).
            IconButton("edit", {.key = "copy", .tooltip = "Copy name", .on_click = "copy"}),
        }});
    auto screen = Column({.padding = 20, .gap = 14, .cross = "stretch", .children = {
        card,
        // Image: the size is given or natural (one side given, the other keeps the aspect ratio);
        // fit: cover (crop), contain (fit inside), fill (stretch).
        Image("@landscape", {.fit = "cover", .radius = 12, .h = 90}),
        Text("@status", {.color = "$theme.muted"}),
    }});
    uison::Ui ui(screen, {.size = {440, 300}, .data = uison::obj{{"status", "Right-click (or long-press) the card for a menu"}}});
    // An image from a file next to the program (CMake puts it there) → source "mem:photo".
    // Bytes from memory — ui.image("name", bytes).
    ui.data().set("photo", ui.image_file("photo", "photo.jpg"));
    ui.data().set("landscape", ui.image_file("landscape", "landscape.png"));
    ui.on("copy", copy);
    ui.on("site", [](uison::Cx& cx) { cx.open_url(SITE); });
    return ui.run("Lesson 8 — images and menus");
}
