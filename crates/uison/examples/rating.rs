//! A custom control: star rating. Drawn by code in a `Canvas`, input — `click` and `hover`
//! with `cx.pos` (cursor relative to the node). One draw function serves every rating on the
//! screen: the node `key` ("rating-food") tells which value to show.
use uison::prelude::*;
use uison::{Color, Point};

const STARS: usize = 5;
const CELL: f32 = 28.0; // one star with its gap

/// The control itself: a `Canvas` node bound to `ratings.<name>`.
fn rating(name: &str) -> Node {
    canvas()
        .key(format!("rating-{name}"))
        .draw("rating")
        .w(CELL * STARS as f32)
        .h(CELL)
        .on("click", "rate")
        .on("hover", "rate-hover")
}

/// Star number (1…5) under the cursor.
fn star_at(cx: &Cx) -> Option<usize> {
    cx.pos.map(|p| ((p.x / CELL) as usize + 1).min(STARS))
}

/// A five-pointed star: 10 vertices, outer and inner radius.
fn star(c: Point, r: f32) -> [Point; 10] {
    std::array::from_fn(|i| {
        let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
        let r = if i % 2 == 0 { r } else { r * 0.45 };
        Point::new(c.x + r * a.cos(), c.y + r * a.sin())
    })
}

fn main() {
    let row_for = |label: &str, name: &str| {
        row().gap(12).cross("center").children([text(label).w(90), rating(name), text(format!("{{@ratings.{name}}} / 5"))])
    };
    let screen = column().padding(20).gap(14).children([
        text("Rate your stay").bold().size(18),
        row_for("Food", "food"),
        row_for("Room", "room"),
        row_for("Service", "service"),
        text("@status").color("$theme.muted"),
    ]);
    let mut ui = Ui::from_node(screen).with_size(420.0, 240.0);
    ui.data.set("ratings", obj([("food", 4.into_json()), ("room", 3.into_json()), ("service", 0.into_json())]));
    ui.data.set("status", "Click a star; click it again to clear");

    ui.draw("rating", |d| {
        let name = d.key.trim_start_matches("rating-");
        // While hovering, show the would-be value instead of the stored one.
        let hover = d.data.num(&format!("hover.{name}")) as usize;
        let value = if hover > 0 { hover } else { d.data.num(&format!("ratings.{name}")) as usize };
        let (on, off) = if hover > 0 {
            (Color::rgb(255, 200, 80), Color::rgb(200, 200, 200))
        } else {
            (Color::rgb(245, 166, 35), Color::rgb(200, 200, 200))
        };
        for i in 0..STARS {
            let c = Point::new(d.rect.x + CELL * (i as f32 + 0.5), d.rect.y + CELL / 2.0);
            d.w.poly(&star(c, CELL * 0.45)).fill(if i < value { on } else { off });
        }
    });
    ui.on("rate", |cx| {
        let name = cx.key.trim_start_matches("rating-").to_string();
        let Some(n) = star_at(cx) else { return };
        let path = format!("ratings.{name}");
        let n = if cx.data.num(&path) as usize == n { 0 } else { n };
        cx.data.set(&path, n);
        cx.data.set(&format!("hover.{name}"), 0);
        cx.data.set("status", format!("{name}: {n} star(s)"));
    });
    ui.on("rate-hover", |cx| {
        let name = cx.key.trim_start_matches("rating-").to_string();
        cx.data.set(&format!("hover.{name}"), star_at(cx).unwrap_or(0));
    });
    run(ui, "Custom control — rating");
}
