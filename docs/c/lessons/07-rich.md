# Lesson 7. Images, tooltips, menus

**English** · [Русский](../../ru/c/lessons/07-rich.md)

**What you'll get:** a card with a photo, a link and a button with a tooltip; a right click (a long press on a phone) opens a menu; an image below.

![Lesson 7](../img/07-rich.png)

![Menu](../img/07-rich-menu.png)

<!-- code: examples-c/lessons/07-rich/main.c -->
```c
/* Lesson 7: images, tooltips, a context menu, links, the clipboard. Everything except images
   is just properties in JSON; an image is given as a file (PNG or JPEG) or as bytes. */
#include <uison_kit.h>

static void copy(uison_cx* cx, void* user) {
    (void)user;
    uison_cx_copy(cx, "Alice Parker");
    uison_data_set_str(uison_cx_data(cx), "status", "Name copied to clipboard");
}

static void site(uison_cx* cx, void* user) {
    (void)user;
    uison_cx_open_url(cx, "https://github.com/MarketKernel/uison");
}

int main(void) {
    uison_ui* ui = uison_ui_new(UISON_JSON({
        size: [440, 300],
        data: { status: "Right-click (or long-press) the card for a menu" },
        root: { Column: { padding: 20, gap: 14, cross: "stretch", children: [
            { Row: {
                key: "card", gap: 14, cross: "center", padding: 12,
                // Context menu: label, action; "-" is a separator. uison_cx_key is the node it was opened on.
                contextMenu: [{ label: "Copy name", action: "copy" }, "-", { label: "Open website", action: "site" }],
                children: [
                    { Avatar: { name: "Alice Parker", src: "@photo", size: 56 } },
                    { Column: { gap: 4, flex: 1, children: [
                        { Text: { text: "Alice Parker", bold: true, size: 16 } },
                        { Link: { text: "github.com/MarketKernel/uison", href: "https://github.com/MarketKernel/uison" } },
                    ] } },
                    // Tooltip: on any node (appears after half a second of hovering).
                    { IconButton: { key: "copy", icon: "edit", tooltip: "Copy name", on: { click: "copy" } } },
                ] } },
            // Image: the size is given or natural (one side given, the other keeps the aspect ratio);
            // fit: cover (crop), contain (fit inside), fill (stretch).
            { Image: { src: "@landscape", h: 90, fit: "cover", radius: 12 } },
            { Text: { text: "@status", color: "$theme.muted" } },
        ] } }
    }));
    if (!ui) return uison_kit_fail();
    /* An image from a file next to the program → source "mem:photo"; from memory — uison_ui_image. */
    const char* photo = uison_ui_image_file(ui, "photo", "photo.jpg");
    const char* landscape = uison_ui_image_file(ui, "landscape", "landscape.png");
    if (!photo || !landscape) return uison_kit_fail();
    uison_data_set_str(uison_ui_data(ui), "photo", photo);
    uison_data_set_str(uison_ui_data(ui), "landscape", landscape);
    UISON_ON(ui, copy);
    UISON_ON(ui, site);
    return uison_run(ui, "Lesson 7 — images and menus");
}
```

## How it works

1. **Images.** `uison_ui_image_file(ui, "photo", "photo.jpg")` reads a PNG or JPEG next to the program and returns the source `"mem:photo"`; you put it into data and bind to it: `Avatar { src: "@photo" }`, `Image { src: "@landscape" }`. From memory — `uison_ui_image(ui, name, bytes, length)`. If the file is missing — `NULL` and an error in `uison_last_error`.
2. **Image size.** If one dimension is set (`h: 90`), the other follows the aspect ratio; `fit: "cover"` crops, `"contain"` fits inside, `"fill"` stretches; `radius` rounds the corners.
3. **Tooltip.** `tooltip: "…"` on any node: it appears after half a second of hovering, on a phone — on a long press.
4. **Context menu.** `contextMenu: [{ label, action }, "-", …]` — an item invokes an action by name, and `uison_cx_key` is the node the menu was opened on.
5. **Link.** `Link { text, href }` opens the address in the browser; from code — `uison_cx_open_url`.
6. **Clipboard.** `uison_cx_copy(cx, "…")` puts text into the system clipboard; pasting into text fields works by itself.

## Try it

- An image embedded in the program: a byte array (`xxd -i photo.jpg > photo.h`) and `uison_ui_image`.
- An avatar without a photo: remove `src` — the initials on a colored circle remain.

Next: [lesson 8: threads](08-threads.md).
