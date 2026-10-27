//! Gallery of the standard element library.
//!
//! `gallery` opens a window; `gallery --png out.png [dark]` takes a headless screenshot.

use uison::ui::Ui;

pub fn app() -> Ui {
    let mut ui = Ui::load(uison::ui::source!("../gallery.json")).unwrap_or_else(|e| panic!("gallery.json: {e}"));
    // Images as bytes from the program (the same on every platform): src "mem:name".
    for (name, bytes) in [
        ("anna", &include_bytes!("../images/anna.jpg")[..]),
        ("boris", include_bytes!("../images/boris.jpg")),
        ("vera", include_bytes!("../images/vera.jpg")),
        ("gleb", include_bytes!("../images/gleb.jpg")),
        ("landscape", include_bytes!("../images/landscape.png")),
    ] {
        ui.image(name, bytes);
    }
    // Derived data: the email error is recomputed on any model change.
    ui.derive(|d| {
        let email = d.str("email");
        let ok = email.is_empty() || email.split_once('@').is_some_and(|(a, b)| !a.is_empty() && b.contains('.'));
        d.set("emailError", if ok { "" } else { "This address looks wrong" });
    });
    ui.derive(|d| {
        let pct = (d.num("progress") * 100.0).round() as i64;
        d.set("progressPct", pct);
    });
    // Work on another thread: Handle::post runs the function on the UI thread.
    let handle = ui.handle();
    ui.on("load", move |cx| {
        cx.data.set("loading", true);
        cx.data.set("progress", 0.0);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let h = handle.clone();
            std::thread::spawn(move || {
                for i in 1..=20 {
                    std::thread::sleep(std::time::Duration::from_millis(60));
                    h.post(move |cx| cx.data.set("progress", i as f64 / 20.0));
                }
                h.post(|cx| cx.data.set("loading", false));
            });
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = &handle;
            cx.data.set("progress", 1.0);
            cx.data.set("loading", false);
        }
    });
    ui.on("theme", |cx| {
        let t = cx.data.str("theme").to_string();
        cx.set_theme(&t);
    });
    ui.on("count", |cx| cx.data.update("clicks", |v| v.as_f64().unwrap_or(0.0) + 1.0));
    ui.on("copyLink", |cx| cx.copy("https://github.com/MarketKernel/uison"));
    // Context menu items: cx.key is the node it was opened on.
    ui.on("menuOpen", |cx| eprintln!("gallery: menu open on {}", cx.key));
    ui.on("menuDelete", |cx| eprintln!("gallery: menu delete on {}", cx.key));
    ui
}
