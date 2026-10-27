//! Lesson 5: the screen and custom components are JSON files next to the program. `source!` embeds the file
//! into the program and remembers its name: with `UISON_RELOAD=folder` (debug build) the screen is read
//! from disk and rebuilt when the file is saved, without a restart.
use uison::prelude::*;

fn main() {
    let mut ui = match Ui::load(source!("05-json/screen.json")) {
        Ok(ui) => ui,
        Err(e) => return eprintln!("{e}"),
    };
    if let Err(e) = ui.load_components(source!("05-json/components.json")) {
        return eprintln!("{e}");
    }
    // One handler for several buttons: which one was pressed comes from the key ("inc-Tasks", "dec-Done").
    ui.on("inc", |cx| change(cx, 1.0));
    ui.on("dec", |cx| change(cx, -1.0));
    run(ui, "Lesson 5 — screen from JSON");
}

fn change(cx: &mut Cx, step: f64) {
    let path = if cx.key.contains("Tasks") { "tasks" } else { "done" };
    let v = (cx.data.num(path) + step).max(0.0);
    cx.data.set(path, v);
}
