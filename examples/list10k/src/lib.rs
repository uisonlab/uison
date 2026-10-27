//! A virtualized list of 10,000 rows (Rust DSL).
//!
//! `list10k` opens a window (or runs in the browser); natively `list10k --bench [w h]` scrolls headless.

use uison::ui::dsl::*;
use uison::ui::{IntoJson, Ui, obj};

const NAMES: [&str; 8] = ["Anna Peterson", "Ben Irving", "Clara Smith", "David Owens", "Emma Cole", "Frank White", "Grace Fox", "Henry Kim"];
const TEXTS: [&str; 5] = [
    "Hi! How is the project going?",
    "Sent over the mockups, take a look when you have time",
    "Is the 15:00 call still on?",
    "Great, thanks 👍",
    "Found a rendering bug, fixing it now",
];

/// The list in a 420×720 window.
pub fn app() -> Ui {
    sized(420.0, 720.0)
}

pub fn sized(w: f32, h: f32) -> Ui {
    let tile = node("ListTile")
        .title("@m.name")
        .subtitle("@m.text")
        .leading(avatar("@m.name").prop("src", "@m.photo"))
        .trailing(text("@m.time").size(12).color("$theme.muted"))
        .on("click", "open");
    let screen = column().fill_w().fill_h().cross("stretch").children([
        container().padding([14, 20]).child(row().cross("center").gap(8).children([
            text("Messages").size(20).bold(),
            badge("{@count}").prop("color", "$theme.primary"),
            spacer(),
            text("@opened").size(12).color("$theme.muted"),
        ])),
        divider(),
        list("messages", "m", 56.0, container().padding([0, 8]).child(tile)).key("list").fill_w().flex(1.0),
    ]);
    let mut ui = Ui::from_node(screen).with_size(w, h);
    // Photos for the first four names (scrolling with images), initials for the rest.
    let photos = [
        ui.image("anna", &include_bytes!("../../gallery/images/anna.jpg")[..]),
        ui.image("boris", &include_bytes!("../../gallery/images/boris.jpg")[..]),
        ui.image("vera", &include_bytes!("../../gallery/images/vera.jpg")[..]),
        ui.image("gleb", &include_bytes!("../../gallery/images/gleb.jpg")[..]),
    ];
    let messages: Vec<_> = (0..10_000)
        .map(|i| {
            obj([
                ("name", NAMES[i % NAMES.len()].into_json()),
                ("photo", photos.get(i % NAMES.len()).map_or(uison::ui::Json::Null, |p| p.clone().into_json())),
                ("text", format!("#{i}: {}", TEXTS[i % TEXTS.len()]).into_json()),
                ("time", format!("{:02}:{:02}", 9 + i % 12, (i * 7) % 60).into_json()),
            ])
        })
        .collect();
    ui.data.set("count", messages.len());
    ui.data.set("messages", messages);
    ui.data.set("opened", "");
    ui.on("open", |cx| {
        let i = cx.key.rsplit('/').nth(1).unwrap_or("?").to_string();
        cx.data.set("opened", format!("opened message {i}"));
    });
    ui
}

