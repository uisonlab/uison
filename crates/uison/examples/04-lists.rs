//! Lesson 4: lists. `list` draws only the visible rows, so 10,000 elements don't slow it down.
//! Which row was pressed comes from the node's key (`cx.key`: "row-17", "del-17").
use uison::prelude::*;

#[derive(Data, Default)]
struct Person {
    id: i32,
    name: String,
}

fn main() {
    // The row template: @r is the current element of rows; one closure for all rows.
    let item = row().key("row-{@r.id}").padding([0, 16]).gap(10).cross("center").on_click(pick).children([
        avatar("@r.name").size(28),
        text("@r.name").flex(1),
        text("#{@r.id}").size(12).color("$theme.muted"),
        icon_button("trash").key("del-{@r.id}").size("sm").on_click(remove),
    ]);
    let screen = column().w("fill").h("fill").cross("stretch").children([
        container().padding([12, 16]).child(text("@picked").bold()),
        divider(),
        list("rows", "r", 44.0, item).key("list").flex(1),
    ]);
    let mut ui = Ui::from_node(screen).with_size(440.0, 520.0);
    let names = ["Alice Parker", "Brian Evans", "Clara Smith", "David Owen", "Emma Collins"];
    let rows: Vec<Person> = (0..10_000).map(|i| Person { id: i, name: names[i as usize % 5].into() }).collect();
    ui.data.set("rows", rows);
    ui.data.set("picked", "Pick a row");
    run(ui, "Lesson 4 — lists");
}

/// Index of the row with this id in rows (after deletions index and id differ).
fn find(d: &Data, id: f64) -> Option<usize> {
    d.get("rows")?.as_array()?.iter().position(|r| r.get("id").and_then(Json::as_f64) == Some(id))
}

/// The number at the end of the key: "row-17" → 17.
fn key_num(key: &str) -> f64 {
    key.rsplit('-').next().and_then(|n| n.parse().ok()).unwrap_or(-1.0)
}

fn pick(cx: &mut Cx) {
    if let Some(i) = find(cx.data, key_num(cx.key)) {
        let name = cx.data.str(&format!("rows.{i}.name")).to_string();
        cx.data.set("picked", format!("Selected: {name}"));
    }
}

fn remove(cx: &mut Cx) {
    if let Some(i) = find(cx.data, key_num(cx.key)) {
        cx.data.remove("rows", i);
    }
}
