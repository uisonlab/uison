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
