//! The stage 1 form built with the element library: the screen is JSON, the logic is here.
//!
//! `form2` opens a window; `form2 --png out.png` takes a headless screenshot.

use uison::ui::{Data, Ui};

fn valid_email(e: &str) -> bool {
    e.split_once('@').is_some_and(|(a, b)| !a.is_empty() && b.contains('.') && !b.ends_with('.'))
}

/// Errors show after the first submit attempt; after that, as you type.
fn validate(d: &mut Data) {
    if !d.bool("tried") {
        return;
    }
    let name = if d.str("form.name").trim().is_empty() { "Enter your name" } else { "" };
    let email = if valid_email(d.str("form.email")) { "" } else { "Check the address" };
    d.set("errors.name", name);
    d.set("errors.email", email);
}

pub fn app() -> Ui {
    let mut ui = Ui::new(include_str!("../form.json")).unwrap_or_else(|e| panic!("form.json: {e}"));
    ui.derive(validate);
    ui.on("submit", |cx| {
        cx.data.set("tried", true);
        validate(cx.data);
        if cx.data.bool("errors.name") {
            cx.focus("name/1/0");
        } else if cx.data.bool("errors.email") {
            cx.focus("email/1/0");
        } else {
            cx.data.update("form.sent", |n| n.as_f64().unwrap_or(0.0) + 1.0);
        }
    });
    ui
}
