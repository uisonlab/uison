# Lesson 8. Images, tooltips, menus

**English** · [Русский](../../ru/cpp/lessons/08-rich.md)

**What you'll get:** a card with a photo, a link and a button with a tooltip; a right click (a long press on a phone) opens a menu; an image below.

![Lesson 8](../img/08-rich.png)

![Menu](../img/08-rich-menu.png)

<!-- code: examples-cpp/lessons/08-rich/main.cpp -->
```cpp
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
```

## How it works

1. **Images.** `ui.image_file("photo", "photo.jpg")` reads a PNG or JPEG next to the program and returns the source `"mem:photo"`; you put it into the data and bind to it: `Avatar(…, {.src = "@photo"})`, `Image("@landscape")`. From memory — `ui.image("name", bytes)`; a missing file throws `uison::Error`.
2. **Image size.** If one dimension is set (`.h = 90`), the other follows the aspect ratio; `.fit = "cover"` crops, `"contain"` fits inside, `"fill"` stretches; `.radius` rounds the corners.
3. **Tooltip.** `.tooltip = "…"` on any widget: shows after half a second of hovering, on a phone — on a long press.
4. **Context menu.** `.context_menu = uison::json(R"([{ label, action }, "-", …])")` — an item calls an action by name (`ui.on("copy", …)`), `cx.key()` is the node where the menu was opened.
5. **Handler by name.** `.on_click = "copy"` is the same function as the menu item's; use a lambda for a single place, a name when the action is needed in several.
6. **Link, clipboard.** `Link("…", {.href = …})` opens the address; `cx.copy("…")` puts text into the system clipboard.

## Try it

- An image embedded in the program: a byte array (`xxd -i photo.jpg`) and `ui.image("photo", bytes)`.
- An avatar without a photo: remove `.src` — the initials on a colored circle remain.

Next: [lesson 9: threads](09-threads.md).
