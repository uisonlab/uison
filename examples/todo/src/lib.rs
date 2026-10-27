//! A todo list: the screen in the Rust DSL, data and logic right here.
//!
//! `todo` opens a window; `todo --png out.png` takes a headless screenshot.

use uison::ui::dsl::*;
use uison::ui::{IntoJson, Json, Ui, obj};

pub fn screen() -> Node {
    // A task row; `@t` is an item of `todos`, the filter is declarative through switch.
    let item = row()
        .key("{@t.id}")
        .gap(10)
        .cross("center")
        .padding([6, 4])
        .visible(json(r#"{ switch: "@filter", cases: { active: "!@t.done", done: "@t.done" }, default: true }"#))
        .children([
            node("Checkbox").bind("t.done"),
            text("@t.title")
                .flex(1.0)
                .max_lines(1)
                .prop("color", json(r#"{ switch: "@t.done", cases: { true: "$theme.muted" }, default: "$theme.text" }"#)),
            icon_button("trash").prop("size", "sm").key("del-{@t.id}").on("click", "remove"),
        ]);

    let card = container()
        .w(460)
        .padding(20)
        .radius("$theme.radiusLg")
        .fill("$theme.card")
        .border("$theme.divider", 1.0)
        .child(column().gap(14).cross("stretch").children([
            row().cross("center").gap(8).children([
                text("Todos").size(22).bold(),
                badge("{@stats.left}").prop("color", "$theme.primary").visible("@stats.left"),
                spacer(),
                text("{@stats.done} of {@stats.total} done").size(12).color("$theme.muted"),
            ]),
            row().gap(8).cross("center").children([
                text_field().bind("draft").placeholder("What needs to be done?").key("draft").flex(1.0).on("submit", "add"),
                button("Add").icon("plus").on("click", "add").disabled("!@draft"),
            ]),
            tabs().bind("filter").tabs(&[("all", "All"), ("active", "Active"), ("done", "Completed")]),
            column().cross("stretch").children([for_each("todos", "t", item)]),
            text("Nothing here yet: add your first todo").text_align("center").color("$theme.muted").visible("!@todos"),
            divider(),
            row().cross("center").children([
                text("Remaining: {@stats.left}").size(12).color("$theme.muted"),
                spacer(),
                button("Clear completed").variant("text").prop("size", "sm").on("click", "clear").visible("@stats.done"),
            ]),
        ]));
    scroll(center(card).prop("h", "hug").margin([24, 0])).fill_w().fill_h()
}

fn todo(title: &str, id: i32, done: bool) -> Json {
    obj([("id", id.into_json()), ("title", title.into_json()), ("done", done.into_json())])
}

pub fn app() -> Ui {
    let mut ui = Ui::from_node(screen()).with_size(520.0, 620.0);
    ui.data.set("filter", "all");
    ui.data.set("draft", "");
    ui.data.set("next", 4);
    ui.data.set(
        "todos",
        vec![todo("Buy bread", 1, false), todo("Write Task2.md", 2, true), todo("Build the element library", 3, false)],
    );
    // Derived data: the counters are recomputed after any change.
    ui.derive(|d| {
        let todos = d.get("todos").and_then(Json::as_array).map(<[Json]>::to_vec).unwrap_or_default();
        let done = todos.iter().filter(|t| t.get("done").is_some_and(Json::truthy)).count();
        d.set("stats.total", todos.len());
        d.set("stats.done", done);
        d.set("stats.left", todos.len() - done);
    });
    ui.on("add", |cx| {
        let title = cx.data.str("draft").trim().to_string();
        if title.is_empty() {
            return;
        }
        let id = cx.data.num("next") as i32;
        cx.data.push("todos", todo(&title, id, false));
        cx.data.set("next", id + 1);
        cx.data.set("draft", "");
        cx.focus("draft/1/0");
    });
    ui.on("remove", |cx| {
        let id: f64 = cx.key.trim_start_matches("del-").parse().unwrap_or(-1.0);
        cx.data.retain("todos", |t| t.get("id").and_then(Json::as_f64) != Some(id));
    });
    ui.on("clear", |cx| cx.data.retain("todos", |t| !t.get("done").is_some_and(Json::truthy)));
    ui
}
