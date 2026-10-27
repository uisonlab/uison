//! Lesson 6: themes and tokens. Element colors are theme tokens (`$theme.primary`, `$theme.card`…);
//! `cx.set_theme` changes the theme, `cx.set_token` a single token.
use uison::prelude::*;

fn main() {
    let accents = row().gap(8).children([for_each(
        "accents",
        "c",
        container().key("accent-{@c}").w(32).h(32).radius(16).fill("@c").on_click(|cx| {
            // key "accent-#34C759" → the color after the dash.
            let color = cx.key.rsplit('-').next().unwrap_or("").to_string();
            cx.set_token("primary", color.as_str());
            cx.data.set("accent", color);
        }),
    )]);
    let screen = column().padding(20).gap(14).cross("stretch").children([
        tabs()
            .key("theme")
            .bind("theme")
            .on_change(|cx| {
                let t = cx.data.str("theme").to_string();
                cx.set_theme(&t);
            })
            .tabs(&[("light", "Light"), ("dark", "Dark"), ("system", "System")]),
        accents,
        card([
            checkbox("Checkbox").bind("agree"),
            slider().bind("volume").min(0).max(100),
            row().gap(8).children([button("Primary"), button("Secondary").variant("secondary")]),
        ])
        .title("Example"),
    ]);
    let mut ui = Ui::from_node(screen).with_size(440.0, 380.0);
    ui.set_theme("light");
    ui.data.set("theme", "light");
    ui.data.set("accent", "#007AFF");
    ui.data.set("agree", true);
    ui.data.set("volume", 60);
    ui.data.set("accents", vec!["#007AFF", "#34C759", "#FF9500", "#AF52DE", "#FF2D55"]);
    run(ui, "Lesson 6 — themes");
}
