# Lesson 7. Images, tooltips, menus

**English** · [Русский](../../ru/rust/lessons/07-rich.md)

**What you'll get:** a card with a photo, a link and a button with a tooltip; right-click (long press on a phone) opens a menu; an image below.

![Lesson 7](../img/07-rich.png)

![Menu](../img/07-rich-menu.png)

<!-- code: crates/uison/examples/07-rich.rs -->
```rust
//! Lesson 7: images, tooltips, a context menu, links, the clipboard. Everything except images
//! is just properties; an image is given as bytes (PNG or JPEG): `include_bytes!` embeds the file into the program.
use uison::prelude::*;

const SITE: &str = "https://github.com/MarketKernel/uison";

fn main() {
    let card = row()
        .key("card")
        .gap(14)
        .cross("center")
        .padding(12)
        // Context menu: label, action; "-" is a separator. Actions are called by name (ui.on below).
        .context_menu(json(r#"[{ label: "Copy name", action: "copy" }, "-", { label: "Open website", action: "site" }]"#))
        .children([
            avatar("Alice Parker").src("@photo").size(56),
            column().gap(4).flex(1).children([
                text("Alice Parker").bold().size(16),
                link("github.com/MarketKernel/uison").href(SITE),
            ]),
            // Tooltip: on any node (appears after half a second of hovering).
            icon_button("edit").key("copy").tooltip("Copy name").on("click", "copy"),
        ]);
    let screen = column().padding(20).gap(14).cross("stretch").children([
        card,
        // Image: the size is given or natural (one side given, the other keeps the aspect ratio);
        // fit: cover (crop), contain (fit inside), fill (stretch).
        image("@landscape").h(90).fit("cover").radius(12),
        text("@status").color("$theme.muted"),
    ]);
    let mut ui = Ui::from_node(screen).with_size(440.0, 300.0);
    // Images from bytes → source "mem:photo"; the file path is relative to this lesson file.
    let photo = ui.image("photo", include_bytes!("../../../examples/gallery/images/anna.jpg").as_slice());
    let landscape = ui.image("landscape", include_bytes!("../../../examples/gallery/images/landscape.png").as_slice());
    ui.data.set("photo", photo);
    ui.data.set("landscape", landscape);
    ui.data.set("status", "Right-click (or long-press) the card for a menu");
    ui.on("copy", |cx| {
        cx.copy("Alice Parker");
        cx.data.set("status", "Name copied to clipboard");
    });
    ui.on("site", |cx| cx.open_url(SITE));
    run(ui, "Lesson 7 — images and menus");
}
```

## How it works

1. **Images from bytes.** `ui.image("photo", bytes)` registers a PNG or JPEG and returns the source `"mem:photo"` — you put it into the data and bind to it: `avatar(…).src("@photo")`, `image("@landscape")`. `include_bytes!` embeds the file into the program. A file next to the program is referenced by path: `image("photo.jpg")`; in the browser, by a `https://…` URL as well.
2. **Image size.** If one dimension is set (`h(90)`), the other follows the aspect ratio; `fit("cover")` crops, `"contain"` fits inside, `"fill"` stretches. `radius` rounds the corners.
3. **Tooltip.** `.tooltip("…")` on any node: it appears after half a second of hovering, and on a phone on a long press.
4. **Context menu.** `.context_menu(json(…))` is a list of `{ label, action }`, with `"-"` as a separator. An item calls an action by name (`ui.on("copy", …)`); `cx.key` is the node the menu was opened on.
5. **Link.** `link("text").href("https://…")` opens the address in the browser; from code — `cx.open_url(…)`.
6. **Clipboard.** `cx.copy("…")` copies to the system clipboard; pasting into text fields works by itself (⌘V / Ctrl+V, the field's menu).

## Try it

- A "Delete" menu item with a confirmation: a second menu or `visible` on a panel.
- An avatar without a photo: remove `.src(…)` — you get initials on a colored circle (the color depends on the name).

Next — [lesson 8: threads](08-threads.md).
