//! Settings screen: a long JSON screen with scrolling and dependent elements.
//!
//! `settings` opens a window; `settings --png out.png` takes a headless screenshot.

use uison::ui::Ui;

pub fn app() -> Ui {
    let mut ui = Ui::new(include_str!("../settings.json")).unwrap_or_else(|e| panic!("settings.json: {e}"));
    let defaults = ui.data.get("s").cloned().unwrap();
    ui.on("theme", |cx| {
        let t = cx.data.str("s.theme").to_string();
        cx.set_theme(&t);
    });
    ui.on("save", |cx| cx.data.set("status", "Settings saved"));
    ui.on("reset", move |cx| {
        cx.data.set("s", defaults.clone());
        cx.data.set("status", "Reset to defaults");
        cx.set_theme("light");
    });
    for action in ["password", "mail", "logout"] {
        ui.on(action, move |cx| cx.data.set("status", format!("Action: {action}")));
    }
    ui
}
