//! Lesson 2: data and bindings. Elements read data by paths (`@name`), an input field writes it;
//! code changes the data, and the screen follows it by itself.
use uison::prelude::*;

fn main() {
    let screen = column().padding(20).gap(12).cross("stretch").children([
        // Two-way binding: the field writes to name, the text below reads it.
        text_field().key("name").label("Name").bind("name"),
        text("Hello, {@name}!").size(20).bold().visible("@show"),
        switch("Show greeting").key("show").bind("show"),
        // A list: for_each repeats the template for each element of the tags array.
        wrap().gap(6).children([for_each("tags", "t", badge("@t"))]),
        row().gap(8).children([
            text_field().key("draft").bind("draft").placeholder("new tag").flex(1).on_submit(add),
            button("Add").key("add").disabled("!@draft").on_click(add),
        ]),
        // A conditional value: a switch on data, in the screen language, as a string.
        text("Tags: {@count}").color(json(r#"{ switch: "@many", cases: { true: "$theme.danger" }, default: "$theme.muted" }"#)),
    ]);
    let mut ui = Ui::from_node(screen).with_size(420.0, 360.0);
    ui.data.set("name", "World");
    ui.data.set("show", true);
    ui.data.set("tags", vec!["rust", "c++"]);
    ui.data.set("draft", "");
    // Derived data: recomputed after any change.
    ui.derive(|d| {
        let n = d.len("tags");
        d.set("count", n);
        d.set("many", n >= 5);
    });
    run(ui, "Lesson 2 — data");
}

/// One function for two events: Enter in the field and the button.
fn add(cx: &mut Cx) {
    let tag = cx.data.str("draft").to_string();
    cx.data.push("tags", tag);
    cx.data.set("draft", "");
}
