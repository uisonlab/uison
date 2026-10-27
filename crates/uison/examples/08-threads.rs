//! Lesson 8: work in another thread. The UI is touched only through `Handle::post`: the function
//! runs on the UI thread (like a handler), and the screen updates itself. Cancellation is an atomic flag.
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use uison::prelude::*;

fn main() {
    let screen = column().padding(24).gap(14).cross("stretch").children([
        text("Downloading").size(20).bold(),
        progress("@progress"),
        text("@status").color("$theme.muted"),
        row().gap(8).children([
            // Actions by name: the handlers need a handle, which only a constructed Ui has.
            button("Download").key("start").icon("arrowDown").disabled("@busy").on("click", "start"),
            button("Cancel").key("cancel").variant("secondary").visible("@busy").on("click", "cancel"),
        ]),
    ]);
    let mut ui = Ui::from_node(screen).with_size(420.0, 240.0);
    ui.data.set("progress", 0);
    ui.data.set("busy", false);
    ui.data.set("status", "Press “Download”");
    let handle = ui.handle(); // cloned into threads
    let cancel = Arc::new(AtomicBool::new(false));
    let stop = cancel.clone();
    ui.on("start", move |cx| {
        cx.data.set("busy", true);
        stop.store(false, Ordering::Relaxed);
        let (h, stop) = (handle.clone(), stop.clone());
        std::thread::spawn(move || {
            for i in 1..=40 {
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50)); // "download" a chunk
                h.post(move |cx| {
                    cx.data.set("progress", i as f64 / 40.0);
                    cx.data.set("status", format!("Downloaded {}%", i * 25 / 10));
                });
            }
            let stopped = stop.load(Ordering::Relaxed);
            h.post(move |cx| {
                cx.data.set("busy", false);
                cx.data.set("status", if stopped { "Cancelled" } else { "Done" });
            });
        });
    });
    ui.on("cancel", move |_| cancel.store(true, Ordering::Relaxed));
    run(ui, "Lesson 8 — threads");
}
