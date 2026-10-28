//! Compiler and runtime tests on deterministic text metrics.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use uison_core::{App, Event, Font, Host, Key, KeyCode, MouseButton, Point, TextMeasure, TextMetrics};
use uison_frame::{Frame, Shape};

use crate::dsl::*;
use crate::*;

/// 7 points per character, line height = font size × 1.2.
pub struct Fake;

impl TextMeasure for Fake {
    fn measure(&self, t: &str, f: &Font) -> TextMetrics {
        TextMetrics { width: t.chars().count() as f32 * 7.0, ascent: f.size * 0.9, descent: f.size * 0.3, leading: 0.0 }
    }
}

impl Host for Fake {}

fn frame(ui: &mut Ui) -> Frame {
    let src = ui.render(&Fake);
    Frame::parse(&src, &Fake).unwrap_or_else(|e| panic!("{e}\n{src}"))
}

fn texts(f: &Frame) -> Vec<String> {
    fn walk(items: &[uison_frame::Item], out: &mut Vec<String>) {
        for it in items {
            match &it.shape {
                Shape::Text { text, .. } => out.push(text.to_string()),
                Shape::Clip { items, .. } => walk(items, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(&f.items, &mut out);
    out
}

/// Rectangle of a hit area by id.
fn hit_rect(f: &Frame, id: &str) -> uison_core::Rect {
    fn walk(items: &[uison_frame::Item], id: &str) -> Option<uison_core::Rect> {
        items.iter().find_map(|it| match &it.shape {
            Shape::Hit { rect, .. } if it.id.as_deref() == Some(id) => Some(*rect),
            Shape::Clip { items, .. } => walk(items, id),
            _ => None,
        })
    }
    walk(&f.items, id).unwrap_or_else(|| panic!("no hit '{id}'"))
}

fn center(r: uison_core::Rect) -> Point {
    Point::new(r.x + r.w / 2.0, r.y + r.h / 2.0)
}

/// Click at the center of a hit area: move, down, up — like a real mouse.
fn click(ui: &mut Ui, id: &str) {
    click_with(ui, id, &Fake);
}

fn click_with(ui: &mut Ui, id: &str, host: &dyn Host) {
    let f = frame(ui);
    let p = center(hit_rect(&f, id));
    let hit = uison_frame::hit_test(&f, p).map(str::to_string);
    for e in [Event::MouseMove(p), Event::MouseDown(p, MouseButton::Left), Event::MouseUp(p, MouseButton::Left)] {
        ui.on_event(&e, hit.as_deref(), host);
    }
}

#[test]
fn layout_row_column_flex() {
    let mut ui = Ui::new(
        r##"{ size: [300, 200], root: { Column: { w: "fill", h: "fill", gap: 10, padding: 0, children: [
            { Row: { key: "r", w: "fill", gap: 10, children: [
                { Box: { key: "a", w: 50, h: 20, fill: "#F00", on: { click: "x" } } },
                { Box: { key: "b", flex: 1, h: 20, fill: "#0F0", on: { click: "x" } } },
                { Box: { key: "c", flex: 2, h: 20, fill: "#00F", on: { click: "x" } } },
            ] } },
            { Box: { key: "d", w: "50%", h: "fill", fill: "#000", on: { click: "x" } } },
        ] } } }"##
            .replace("padding: 0, ", "")
            .as_str(),
    )
    .unwrap();
    let f = frame(&mut ui);
    assert_eq!(hit_rect(&f, "a"), uison_core::Rect::new(0.0, 0.0, 50.0, 20.0));
    // Free 300 - 50 - 2·10 = 230 → 1:2.
    let b = hit_rect(&f, "b");
    let c = hit_rect(&f, "c");
    assert!((b.w - 230.0 / 3.0).abs() < 0.01 && (c.w - 460.0 / 3.0).abs() < 0.01, "{b:?} {c:?}");
    assert_eq!(c.right(), 300.0);
    // "fill" in height inside a Column — the remainder: 200 - 20 - 10.
    assert_eq!(hit_rect(&f, "d"), uison_core::Rect::new(0.0, 30.0, 150.0, 170.0));
}

#[test]
fn layout_alignment_and_text_wrap() {
    let mut ui = Ui::new(
        r##"{ size: [120, 100], root: { Center: { child: { Box: { key: "box", padding: 10, fill: "#EEE", on: { click: "x" },
            child: { Text: { text: "zero one two three", maxLines: 2 } } } } } } }"##,
    )
    .unwrap();
    let f = frame(&mut ui);
    // Text wider than 100 wraps; 2 lines of 13·1.2 ≈ 16 → height 32 + 20.
    assert_eq!(texts(&f), ["zero one two", "three"]);
    let r = hit_rect(&f, "box");
    assert_eq!((r.w, r.h), (12.0 * 7.0 + 20.0, 32.0 + 20.0));
    assert_eq!((r.x, r.y), ((120.0 - r.w) / 2.0, (100.0 - r.h) / 2.0));
}

#[test]
fn templates_params_slots_and_theme() {
    let mut ui = Ui::new(
        r##"{ size: [400, 300],
          components: {
            Greeting: { params: { name: "world", children: [] },
              body: { Column: { children: [ { Text: { text: "Hello, {$name}!", color: "$theme.primary" } }, "$children" ] } } },
          },
          data: { user: "Anna" },
          root: { Greeting: { name: "@user", children: [ { Text: { text: "slot" } } ] } } }"##,
    )
    .unwrap();
    assert_eq!(texts(&frame(&mut ui)), ["Hello, Anna!", "slot"]);
    ui.data.set("user", "Boris");
    assert_eq!(texts(&frame(&mut ui)), ["Hello, Boris!", "slot"]);
}

#[test]
fn literal_text_is_not_an_expression() {
    // Text from outside (a web page) passed through a component's slot and shown directly.
    let raw = ["$initial", "@user", "!@flag", "{@user} and {$x}", "a } b {{", "$theme.primary"];
    let items: Vec<String> = raw.iter().map(|s| format!(r#"{{ Text: {{ text: "{}" }} }}"#, crate::literal(s))).collect();
    let src = format!(
        r#"{{ size: [400, 300], data: {{ user: "Anna" }},
          components: {{ Holder: {{ params: {{ children: [] }}, body: {{ Column: {{ children: [ "$children" ] }} }} }} }},
          root: {{ Holder: {{ children: [ {} ] }} }} }}"#,
        items.join(", ")
    );
    let mut ui = Ui::new(&src).unwrap();
    assert_eq!(texts(&frame(&mut ui)), raw);
}

#[test]
fn errors_point_at_the_node() {
    let bad = |root: &str| {
        let mut ui = Ui::new(&format!("{{ root: {root} }}")).unwrap();
        let src = ui.render(&Fake);
        texts(&Frame::parse(&src, &Fake).unwrap()).join(" ")
    };
    let e = bad(r#"{ Column: { children: [ { Text: { text: "ok" } }, { Buton: { text: "x" } } ] } }"#);
    assert!(e.contains("unknown node type 'Buton'") && e.contains("root/1"), "{e}");
    let e = bad(r#"{ Button: { txt: "x" } }"#);
    assert!(e.contains("unknown parameter 'txt'"), "{e}");
    let e = bad(r##"{ Text: { text: "x", colour: "#000" } }"##);
    assert!(e.contains("unknown property 'colour'"), "{e}");
    let e = bad(r#"{ Text: { text: "$nope" } }"#);
    assert!(e.contains("$nope"), "{e}");
    assert!(Ui::new("{ root: { Text: {} }, extra: 1 }").is_err());
}

#[test]
fn for_each_with_keys() {
    let mut ui = Ui::new(
        r#"{ data: { todos: [{ id: 7, t: "bread" }, { id: 9, t: "milk" }] },
             root: { Column: { children: { For: { each: "@todos", key: "@item.id",
               do: { Text: { key: "@item.id", text: "{@index}. {@item.t}" } } } } } } }"#,
    )
    .unwrap();
    assert_eq!(texts(&frame(&mut ui)), ["0. bread", "1. milk"]);
}

#[test]
fn checkbox_toggles_bound_data_and_fires_change() {
    let mut ui = Ui::new(
        r#"{ data: { agree: false },
             root: { Column: { children: [ { Checkbox: { key: "cb", label: "I agree", checked: "@agree", on: { change: "changed" } } } ] } } }"#,
    )
    .unwrap();
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let c = calls.clone();
    ui.on("changed", move |cx| {
        assert_eq!(cx.key, "cb");
        assert!(cx.data.bool("agree"));
        c.set(c.get() + 1);
    });
    click(&mut ui, "cb");
    assert!(ui.data.bool("agree"));
    assert_eq!(calls.get(), 1);
}

#[test]
fn messages_run_actions_and_set_data() {
    let mut ui = Ui::new(
        r#"{ data: { q: "", n: 0 }, root: { Column: { children: [ { Button: { key: "ok", text: "Save", on: { click: "save" } } } ] } } }"#,
    )
    .unwrap();
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let s = seen.clone();
    ui.on("save", move |cx| {
        s.borrow_mut().push((cx.event.to_string(), cx.key.to_string(), cx.pos, cx.clicks));
        cx.data.update("n", |n| n.as_f64().unwrap_or(0.0) + 1.0);
    });
    ui.derive(|d| {
        let q = d.str("q").to_uppercase();
        d.set("upper", q);
    });
    frame(&mut ui);
    // An action as if the button had fired it, with the position and click count from the message.
    let msg = Event::Message(r#"{"action":"save","event":"click","key":"ok","pos":[12,8.5],"clicks":2}"#.into());
    assert!(ui.on_event(&msg, None, &Fake).is_some());
    assert_eq!(*seen.borrow(), [("click".to_string(), "ok".to_string(), Some(Point::new(12.0, 8.5)), 2)]);
    assert_eq!(ui.data.num("n"), 1.0);
    // Data: set and derived data recomputed.
    assert!(ui.on_event(&Event::Message(r#"{"data":{"q":"jpg"}}"#.into()), None, &Fake).is_some());
    assert_eq!(ui.data.str("upper"), "JPG");
    // Defaults: event `click`, no position, one click; `apply` from code does the same.
    assert!(ui.apply(r#"{"action":"save"}"#));
    assert_eq!(seen.borrow()[1], ("click".to_string(), String::new(), None, 1));
    assert!(!ui.apply(r#"{"hello":1}"#));
    assert!(!ui.apply("not json"));
}

#[test]
fn file_chooser_unsupported_calls_the_action_at_once() {
    let mut ui = Ui::new(r#"{ root: { Button: { key: "b", text: "Choose", on: { click: "choose" } } } }"#).unwrap();
    let got = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    ui.on("choose", |cx| cx.pick_file("chosen"));
    let g = got.clone();
    ui.on("chosen", move |cx| *g.borrow_mut() = format!("{}:{}", cx.event, cx.key));
    click(&mut ui, "b");
    // `Fake` has no chooser: the action learns it right away; a phone's answer is a message.
    assert_eq!(*got.borrow(), "unsupported:");
    assert!(ui.apply(r#"{"action":"chosen","event":"picked","key":"/data/files/picked/k.pem"}"#));
    assert_eq!(*got.borrow(), "picked:/data/files/picked/k.pem");
}

#[test]
fn hover_and_pressed_states_change_colors() {
    let mut ui = Ui::new(r#"{ root: { Button: { key: "b", text: "OK" } } }"#).unwrap();
    let color_of = |f: &Frame| {
        f.items.iter().find_map(|it| match &it.shape {
            Shape::Rect { paint, .. } => paint.fill,
            _ => None,
        })
    };
    let f = frame(&mut ui);
    let normal = color_of(&f);
    let p = center(hit_rect(&f, "b"));
    let up = ui.on_event(&Event::MouseMove(p), Some("b"), &Fake).expect("hover changes the frame");
    let hover = color_of(&Frame::parse(&up, &Fake).unwrap());
    assert_ne!(normal, hover);
    let down = ui.on_event(&Event::MouseDown(p, MouseButton::Left), Some("b"), &Fake).unwrap();
    assert_ne!(hover, color_of(&Frame::parse(&down, &Fake).unwrap()));
    // Moving again without changing the node — the frame is not rebuilt.
    assert!(ui.on_event(&Event::MouseMove(Point::new(p.x + 1.0, p.y)), Some("b"), &Fake).is_none());
}

#[test]
fn text_field_typing_caret_and_submit() {
    let mut ui = Ui::new(
        r#"{ data: { name: "" }, root: { Column: { children: [
            { TextField: { key: "name", value: "@name", placeholder: "Name", on: { submit: "go" } } } ] } } }"#,
    )
    .unwrap();
    let submitted = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
    let s = submitted.clone();
    ui.on("go", move |cx| *s.borrow_mut() = cx.data.str("name").to_string());
    assert!(texts(&frame(&mut ui)).contains(&"Name".to_string()));
    // Focus — by clicking the input itself (the Input inside TextField).
    click(&mut ui, "name/1/0");
    assert!(ui.wants_tick().is_some(), "caret blinks");
    for k in [Key::char("П"), Key::char("ё"), Key::char("т"), Key::char("р")] { // Cyrillic on purpose: multibyte typing and caret
        ui.on_event(&Event::Key(k), None, &Fake);
    }
    // Пётр → replace "ё" with "е": two arrows left, Backspace, "е".
    ui.on_event(&Event::Key(Key::new(KeyCode::Left)), None, &Fake);
    ui.on_event(&Event::Key(Key::new(KeyCode::Left)), None, &Fake);
    ui.on_event(&Event::Key(Key::new(KeyCode::Backspace)), None, &Fake);
    ui.on_event(&Event::Key(Key::char("е")), None, &Fake);
    assert_eq!(ui.data.str("name"), "Петр");
    ui.on_event(&Event::Key(Key::new(KeyCode::Enter)), None, &Fake);
    assert_eq!(*submitted.borrow(), "Петр");
    ui.on_event(&Event::Key(Key::new(KeyCode::Escape)), None, &Fake);
    assert!(ui.wants_tick().is_none());
}

#[test]
fn select_opens_picks_and_dismisses() {
    let mut ui = Ui::new(
        r#"{ size: [300, 300], data: { c: "pl" }, root: { Column: { children: [
            { Select: { key: "s", value: "@c", options: [{ value: "ua", label: "Ukraine" }, { value: "pl", label: "Poland" }] } },
            { Button: { key: "btn", text: "Next", on: { click: "next" } } } ] } } }"#,
    )
    .unwrap();
    let pressed = std::rc::Rc::new(std::cell::Cell::new(false));
    let p2 = pressed.clone();
    ui.on("next", move |_| p2.set(true));
    assert!(texts(&frame(&mut ui)).contains(&"Poland".to_string()));
    click(&mut ui, "s/1/0"); // field
    let f = frame(&mut ui);
    assert!(texts(&f).contains(&"Ukraine".to_string()), "open: {:?}", texts(&f));
    // The panel fits all options (not squeezed to the field height).
    let panel = hit_rect(&f, "s/1/1");
    assert!(panel.h >= 2.0 * 28.0, "{panel:?}");
    click(&mut ui, "s/1/1/0/0/ua"); // option
    assert_eq!(ui.data.str("c"), "ua");
    assert!(!texts(&frame(&mut ui)).contains(&"Poland".to_string()), "closed");
    // A click outside the open list only closes it.
    click(&mut ui, "s/1/0");
    click(&mut ui, "btn");
    assert!(!pressed.get());
    click(&mut ui, "btn");
    assert!(pressed.get());
}

#[test]
fn slider_drag_and_scroll() {
    let mut ui = Ui::new(
        r#"{ size: [220, 100], data: { v: 0 }, root: { Column: { w: "fill", children: [
            { Slider: { key: "s", value: "@v", min: 0, max: 100, step: 10, w: "fill" } } ] } } }"#,
    )
    .unwrap();
    let f = frame(&mut ui);
    let r = hit_rect(&f, "s");
    let p = Point::new(r.x + 10.0 + (r.w - 20.0) * 0.52, r.y + r.h / 2.0);
    ui.on_event(&Event::MouseDown(p, MouseButton::Left), Some("s"), &Fake);
    assert_eq!(ui.data.num("v"), 50.0);
    ui.on_event(&Event::MouseMove(Point::new(r.right(), p.y)), Some("s"), &Fake);
    ui.on_event(&Event::MouseUp(p, MouseButton::Left), Some("s"), &Fake);
    assert_eq!(ui.data.num("v"), 100.0);
}

#[test]
fn divider_drags_by_shift_and_keeps_the_pointer() {
    let mut ui = Ui::new(
        r##"{ size: [300, 100], data: { w: 100 }, root: { Row: { w: "fill", h: "fill", cross: "stretch", children: [
            { Box: { key: "left", w: "@w" } },
            { Box: { key: "div", w: 6, cursor: "resize-x", drag: { path: "@w", min: 50, max: 150, delta: 1 }, on: { click: "reset" } } },
            { Box: { key: "right", flex: 1, on: { click: "other" } } },
        ] } } }"##,
    )
    .unwrap();
    let clicks = Rc::new(RefCell::new(Vec::new()));
    let c = clicks.clone();
    ui.on("reset", move |cx| {
        c.borrow_mut().push(cx.clicks);
        if cx.clicks == 2 {
            cx.data.set("w", 100);
        }
    });
    ui.on("other", |_| panic!("the right box must not get the release"));
    let f = frame(&mut ui);
    let r = hit_rect(&f, "div");
    let p = center(r);
    let far = Point::new(280.0, p.y);
    assert_eq!(uison_frame::cursor_at(&f, p), uison_core::Cursor::ResizeX);
    assert_eq!(uison_frame::hit_test(&f, far), Some("right"));
    // The press doesn't move it; the value follows the shift, not the point on the node.
    ui.on_event(&Event::MouseDown(p, MouseButton::Left), Some("div"), &Fake);
    assert_eq!(ui.data.num("w"), 100.0);
    ui.on_event(&Event::MouseMove(Point::new(p.x + 30.0, p.y)), Some("div"), &Fake);
    assert_eq!(ui.data.num("w"), 130.0);
    // Past the limit the divider stops, and the pointer ahead of it stays the divider's.
    ui.on_event(&Event::MouseMove(far), Some("right"), &Fake);
    assert_eq!(ui.data.num("w"), 150.0);
    let f = frame(&mut ui);
    assert_eq!(hit_rect(&f, "div").x, r.x + 50.0);
    assert_eq!(uison_frame::hit_test(&f, far), Some("div"));
    assert_eq!(uison_frame::cursor_at(&f, far), uison_core::Cursor::ResizeX);
    ui.on_event(&Event::MouseMove(Point::new(p.x - 70.0, p.y)), Some("div"), &Fake);
    assert_eq!(ui.data.num("w"), 50.0);
    // Released away from the divider: no click; the window is back to normal.
    ui.on_event(&Event::MouseUp(far, MouseButton::Left), Some("div"), &Fake);
    let f = frame(&mut ui);
    assert_eq!(uison_frame::hit_test(&f, far), Some("right"));
    assert_eq!(uison_frame::cursor_at(&f, far), uison_core::Cursor::Pointer);
    assert!(clicks.borrow().is_empty());
    // A double click — the default width.
    ui.on_event(&Event::MouseMove(Point::new(p.x + 30.0, p.y)), Some("div"), &Fake);
    click(&mut ui, "div");
    click(&mut ui, "div");
    assert_eq!(*clicks.borrow(), [1, 2]);
    assert_eq!(ui.data.num("w"), 100.0);
    // The other way: `delta: -1` — a panel to the right of the divider.
    let mut ui = Ui::new(
        r##"{ size: [300, 100], data: { w: 100 }, root: { Row: { w: "fill", h: "fill", cross: "stretch", children: [
            { Box: { flex: 1 } },
            { Box: { key: "div", w: 6, cursor: "resize-x", drag: { path: "@w", min: 50, max: 150, delta: -1 } } },
            { Box: { w: "@w" } },
        ] } } }"##,
    )
    .unwrap();
    let p = center(hit_rect(&frame(&mut ui), "div"));
    ui.on_event(&Event::MouseDown(p, MouseButton::Left), Some("div"), &Fake);
    ui.on_event(&Event::MouseMove(Point::new(p.x - 20.0, p.y)), Some("div"), &Fake);
    ui.on_event(&Event::MouseUp(Point::new(p.x - 20.0, p.y), MouseButton::Left), Some("div"), &Fake);
    assert_eq!(ui.data.num("w"), 120.0);
}

#[test]
fn virtualized_list_renders_only_visible_items() {
    let items: Vec<Json> = (0..10_000).map(|i| obj([("name", format!("Row {i}").into_json())])).collect();
    let mut ui = Ui::from_node(list("rows", "r", 20.0, text("@r.name")).key("list").fill_w().fill_h()).with_size(200.0, 100.0);
    ui.data.set("rows", items);
    let f = frame(&mut ui);
    let t = texts(&f);
    assert!(t.len() <= 7 && t[0] == "Row 0", "{t:?}");
    let p = Point::new(100.0, 50.0);
    ui.on_event(&Event::Scroll(p, 0.0, -20_000.0), Some("list"), &Fake);
    let t = texts(&frame(&mut ui));
    assert!(t.len() <= 7 && t[0] == "Row 1000", "{t:?}");
}

#[test]
fn dsl_closures_become_handlers() {
    // A closure on a button; one closure for all For rows — cx.key tells which row.
    let row = button("{@t}").key("row-{@t}").on_click(|cx| {
        let k = cx.key.to_string();
        cx.data.set("picked", k);
    });
    let screen = column().children([
        text("Clicked {@n} times"),
        button("+1").key("inc").on_click(|cx| cx.data.update("n", |n| n.as_f64().unwrap_or(0.0) + 1.0)),
        for_each("rows", "t", row),
    ]);
    assert!(screen.to_json().contains("~h"), "{}", screen.to_json());
    let mut ui = Ui::from_node(screen).with_size(300.0, 300.0);
    ui.data.set("rows", vec!["a", "b"]);
    click(&mut ui, "inc");
    click(&mut ui, "inc");
    assert_eq!(ui.data.num("n"), 2.0);
    click(&mut ui, "row-b");
    assert_eq!(ui.data.str("picked"), "row-b");
    assert!(texts(&frame(&mut ui)).contains(&"Clicked 2 times".to_string()));
}

#[test]
fn dsl_matches_json() {
    let from_dsl = column().gap(12).children([
        text("Sign up").size(22).bold(),
        text_field().label("Name").bind("form.name").key("name"),
        checkbox("I agree").bind("form.agree"),
        button("Submit").on("click", "submit").disabled("!@form.agree"),
    ]);
    let json = r#"{ Column: { gap: 12, children: [
        { Text: { text: "Sign up", size: 22, bold: true } },
        { TextField: { label: "Name", value: "@form.name", key: "name" } },
        { Checkbox: { label: "I agree", checked: "@form.agree" } },
        { Button: { text: "Submit", on: { click: "submit" }, disabled: "!@form.agree" } } ] } }"#;
    let from_json = uison_json::parse(json).unwrap().into_owned();
    assert_eq!(from_dsl.clone().into_json(), from_json);
    // And identical frames.
    let mut a = Ui::from_node(from_dsl.clone());
    let mut b = Ui::new(&format!("{{ root: {} }}", from_dsl.to_json())).unwrap();
    assert_eq!(a.render(&Fake), b.render(&Fake));
}

#[test]
fn code_can_edit_the_tree() {
    let mut ui = Ui::from_node(column().key("col").children([text("first").key("t1")]));
    ui.tree().find("t1").unwrap().set("text", "changed");
    ui.tree().find("col").unwrap().children().push(text("second").into_json());
    assert_eq!(texts(&frame(&mut ui)), ["changed", "second"]);
}

#[test]
fn standard_library_renders_in_both_themes() {
    let screen = column().gap(8).children([
        button("Primary"),
        button("Secondary").variant("secondary").icon("plus"),
        button("Danger").variant("danger").disabled(true),
        icon_button("trash"),
        checkbox("Checkbox").bind("a"),
        switch("Switch").bind("b"),
        radio_group().bind("r").options(&[("1", "One"), ("2", "Two")]),
        select().bind("s").options(&[("x", "Ex")]).label("Choice"),
        tabs().bind("t").tabs(&[("a", "A"), ("b", "B")]),
        text_field().label("Field").bind("f").error("Error"),
        slider().bind("v"),
        progress(0.4),
        divider(),
        card([text("in a card")]).title("Card"),
        list_tile("Row").subtitle("subtitle").leading(avatar("Anna Petrova")).trailing(badge("3")),
        section("Section", [icon("star")]),
    ]);
    for theme in ["light", "dark"] {
        let mut ui = Ui::from_node(screen.clone()).with_size(400.0, 900.0);
        ui.set_theme(theme);
        let src = ui.render(&Fake);
        let f = Frame::parse(&src, &Fake).unwrap();
        let t = texts(&f);
        assert!(!t.iter().any(|s| s.contains("Error in the UI description")), "{theme}: {t:?}");
        assert!(t.contains(&"AP".to_string()) && t.contains(&"Card".to_string()), "{t:?}");
    }
}

#[test]
fn timers_shortcuts_and_tokens() {
    let mut ui = Ui::from_node(column().children([text("{@n}"), container().key("b").w(10).h(10).fill("$theme.primary").on("click", "x")]));
    ui.data.set("n", 0);
    ui.on("bump", |cx| {
        cx.data.update("n", |v| v.as_f64().unwrap_or(0.0) + 1.0);
        cx.timer(0, "again");
    });
    ui.on("again", |cx| cx.data.update("n", |v| v.as_f64().unwrap_or(0.0) + 10.0));
    ui.on("accent", |cx| cx.set_token("primary", "#FF0000"));
    ui.shortcut("cmd+b", "bump").shortcut("cmd+shift+a", "accent");
    frame(&mut ui);
    let mut k = Key::char("b");
    k.mods.cmd = true;
    assert!(ui.on_event(&Event::Key(k.clone()), None, &Fake).is_some());
    assert_eq!(ui.data.num("n"), 1.0);
    // Without ⌘ it's not a shortcut.
    assert!(ui.on_event(&Event::Key(Key::char("b")), None, &Fake).is_none());
    // A 0 ms timer: a tick is requested right away and calls the action.
    assert_eq!(ui.wants_tick(), Some(std::time::Duration::ZERO));
    assert!(ui.on_event(&Event::Tick, None, &Fake).is_some());
    assert_eq!(ui.data.num("n"), 11.0);
    assert_eq!(ui.wants_tick(), None);
    // A theme token changes on the fly.
    let mut k = Key::char("A");
    k.mods.cmd = true;
    k.mods.shift = true;
    let src = ui.on_event(&Event::Key(k), None, &Fake).unwrap();
    assert!(src.contains("#FF0000"), "{src}");
}

#[test]
fn frame_timer_fires_on_early_frame() {
    // `timer(16)` animation on display frames: a frame arrives slightly before the deadline — the timer fires on it.
    let mut ui = Ui::from_node(column().children([text("{@n}")]));
    ui.data.set("n", 0);
    ui.on("step", |cx| {
        cx.data.update("n", |v| v.as_f64().unwrap_or(0.0) + 1.0);
        cx.timer(16, "step");
    });
    ui.on("start", |cx| cx.timer(16, "step"));
    ui.shortcut("cmd+s", "start");
    frame(&mut ui);
    let mut k = Key::char("s");
    k.mods.cmd = true;
    ui.on_event(&Event::Key(k), None, &Fake);
    let t = std::time::Instant::now();
    std::thread::sleep(std::time::Duration::from_millis(5));
    ui.on_event(&Event::Tick, None, &Fake);
    // A loaded machine (a VM during a parallel build) can oversleep past the tolerance — then "too early" proves nothing.
    if t.elapsed().as_millis() < 12 {
        assert_eq!(ui.data.num("n"), 0.0, "too early");
    }
    std::thread::sleep(std::time::Duration::from_millis(9));
    ui.on_event(&Event::Tick, None, &Fake);
    assert_eq!(ui.data.num("n"), 1.0, "frame 2 ms before the deadline");
}

#[test]
fn clicking_empty_space_does_not_highlight_buttons() {
    // A screen component with a scroll and buttons: clicking and hovering empty space
    // inside the scroll must not change how the buttons look.
    let mut ui = Ui::new(
        r#"{ size: [300, 300],
             components: { Screen: { body: { Scroll: { w: "fill", h: "fill",
               child: { Column: { children: [ { Button: { key: "b1", text: "One" } }, { IconButton: { key: "b2", icon: "plus" } } ] } } } } } },
             root: { Screen: {} } }"#,
    )
    .unwrap();
    let before = ui.render(&Fake);
    let p = Point::new(150.0, 250.0); // below the buttons — empty space inside the Scroll
    let f = Frame::parse(&before, &Fake).unwrap();
    let hit = uison_frame::hit_test(&f, p).map(str::to_string);
    assert_eq!(hit.as_deref(), Some("root"), "empty space — the Scroll hit area");
    for e in [Event::MouseMove(p), Event::MouseDown(p, MouseButton::Left)] {
        if let Some(src) = ui.on_event(&e, hit.as_deref(), &Fake) {
            assert_eq!(src, before, "{e:?} changed the frame");
        }
    }
}

#[test]
fn touch_tap_clicks_but_swipe_scrolls() {
    let rows: String = (0..30).map(|i| format!(r#"{{ Button: {{ key: "b{i}", text: "Button {i}", on: {{ click: "tap" }} }} }},"#)).collect();
    let src = format!(
        r#"{{ size: [300, 300], data: {{ taps: 0 }},
             root: {{ Scroll: {{ key: "s", w: "fill", h: "fill", child: {{ Column: {{ children: [{rows}] }} }} }} }} }}"#
    );
    let mut ui = Ui::new(&src).unwrap();
    ui.on("tap", |cx| {
        let n = cx.data.num("taps");
        cx.data.set("taps", n + 1.0);
    });
    let touch = MouseButton::Touch;
    let f = frame(&mut ui);
    let p = center(hit_rect(&f, "b6"));
    let hit = uison_frame::hit_test(&f, p).map(str::to_string);
    let send = |ui: &mut Ui, e: Event| {
        ui.on_event(&e, hit.as_deref(), &Fake);
    };

    // Tap: press and click, no hover.
    send(&mut ui, Event::MouseDown(p, touch));
    send(&mut ui, Event::MouseMove(Point::new(p.x, p.y + 3.0))); // finger jitter below the threshold
    send(&mut ui, Event::MouseUp(Point::new(p.x, p.y + 3.0), touch));
    assert_eq!(ui.data.num("taps"), 1.0);

    // Swipe up: the content follows the finger, no click.
    send(&mut ui, Event::MouseDown(p, touch));
    for k in 1..=10 {
        std::thread::sleep(std::time::Duration::from_millis(2));
        send(&mut ui, Event::MouseMove(Point::new(p.x, p.y - 10.0 * k as f32)));
    }
    send(&mut ui, Event::MouseUp(Point::new(p.x, p.y - 100.0), touch));
    assert_eq!(ui.data.num("taps"), 1.0, "swipe must not click");
    let after = frame(&mut ui);
    let scrolled = center(hit_rect(&after, "b6")).y;
    assert_eq!(scrolled, p.y - 100.0 + crate::gesture::SLOP, "content follows the finger minus slop");

    // Inertia: ticks continue the scroll.
    assert!(ui.wants_tick().is_some());
    std::thread::sleep(std::time::Duration::from_millis(20));
    ui.on_event(&Event::Tick, None, &Fake);
    assert!(center(hit_rect(&frame(&mut ui), "b6")).y < scrolled, "fling continues");

    // A touch during inertia stops it and presses nothing.
    send(&mut ui, Event::MouseDown(p, touch));
    send(&mut ui, Event::MouseUp(p, touch));
    assert!(ui.wants_tick().is_none());
    assert_eq!(ui.data.num("taps"), 1.0);
}

#[test]
fn touch_overscroll_springs_back() {
    let rows: String = (0..30).map(|i| format!(r#"{{ Text: {{ text: "Row {i}", h: 30 }} }},"#)).collect();
    let src = format!(r#"{{ size: [300, 300], root: {{ Scroll: {{ key: "s", w: "fill", h: "fill", child: {{ Column: {{ children: [{rows}] }} }} }} }} }}"#);
    let mut ui = Ui::new(&src).unwrap();
    let top = |ui: &mut Ui| {
        let f = frame(ui);
        let t = texts(&f);
        assert_eq!(t[0], "Row 0");
        // y of the first row: find its rectangle via the text primitive.
        fn first_text_y(items: &[uison_frame::Item]) -> Option<f32> {
            items.iter().find_map(|it| match &it.shape {
                Shape::Text { at, .. } => Some(at.y),
                Shape::Clip { items, .. } => first_text_y(items),
                _ => None,
            })
        }
        first_text_y(&f.items).unwrap()
    };
    let y0 = top(&mut ui);
    let p = Point::new(150.0, 100.0);
    let touch = MouseButton::Touch;
    ui.on_event(&Event::MouseDown(p, touch), Some("s"), &Fake);
    // Pull down 200 pt at the very start — the content moves down less than the finger.
    for k in 1..=20 {
        ui.on_event(&Event::MouseMove(Point::new(p.x, p.y + 10.0 * k as f32)), Some("s"), &Fake);
    }
    let pulled = top(&mut ui) - y0;
    assert!(pulled > 30.0 && pulled < 200.0 - crate::gesture::SLOP, "rubber band: {pulled}");
    std::thread::sleep(std::time::Duration::from_millis(30));
    ui.on_event(&Event::MouseUp(Point::new(p.x, p.y + 200.0), touch), Some("s"), &Fake);
    // The spring returns to the edge in ~0.5 s.
    assert!(ui.wants_tick().is_some());
    for _ in 0..80 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        ui.on_event(&Event::Tick, None, &Fake);
        if ui.wants_tick().is_none() {
            break;
        }
    }
    assert!(ui.wants_tick().is_none(), "spring finished");
    assert_eq!(top(&mut ui), y0);
}

#[test]
fn keys_from_component_params_are_unique() {
    // Regression: key "write-{@c.id}" inside a component gave all buttons the same key "write-".
    let mut ui = Ui::new(
        r#"{ size: [400, 200], data: { people: [{ id: 1 }, { id: 2 }] },
             components: { Card: { params: { c: null }, body: { Button: { text: "Write", key: "write-{$c.id}", on: { click: "write" } } } } },
             root: { Column: { children: { For: { each: "@people", as: "c", do: { Card: { c: "@c" } } } } } } }"#,
    )
    .unwrap();
    let hit = Rc::new(RefCell::new(String::new()));
    let h = hit.clone();
    ui.on("write", move |cx| *h.borrow_mut() = cx.key.to_string());
    let f = frame(&mut ui);
    hit_rect(&f, "write-1");
    hit_rect(&f, "write-2");
    click(&mut ui, "write-2");
    assert_eq!(*hit.borrow(), "write-2");
}

/// Platform for tests: `Fake` metrics, records requests, Shift as the test wishes.
#[derive(Default)]
struct Rec {
    clip: RefCell<Vec<String>>,
    pastes: std::cell::Cell<u32>,
    shift: std::cell::Cell<bool>,
}

impl TextMeasure for Rec {
    fn measure(&self, t: &str, f: &Font) -> TextMetrics {
        Fake.measure(t, f)
    }
}

impl Host for Rec {
    fn mods(&self) -> uison_core::Mods {
        uison_core::Mods { shift: self.shift.get(), ..Default::default() }
    }
    fn set_clipboard(&self, t: &str) {
        self.clip.borrow_mut().push(t.to_string());
    }
    fn request_paste(&self) {
        self.pastes.set(self.pastes.get() + 1);
    }
    fn open_url(&self, url: &str) -> bool {
        self.clip.borrow_mut().push(format!("open {url}"));
        true
    }
}

fn field_ui(value: &str, apple: bool) -> Ui {
    let mut ui = Ui::new(&format!(
        r#"{{ size: [400, 100], data: {{ name: "{value}" }}, root: {{ Column: {{ children: [
            {{ TextField: {{ key: "name", value: "@name" }} }} ] }} }} }}"#
    ))
    .unwrap();
    ui.update(|d| d.set("env.mod", if apple { "⌘" } else { "Ctrl+" }));
    ui
}

fn key(code: KeyCode, f: impl FnOnce(&mut uison_core::Mods)) -> Event {
    let mut k = Key::new(code);
    f(&mut k.mods);
    Event::Key(k)
}

fn cmd(ch: &str) -> Event {
    let mut k = Key::char(ch);
    k.mods.cmd = true;
    Event::Key(k)
}

/// Mouse in the `name` field at character `n` (Fake: 7 pt per character, text from the field's left edge).
fn field_point(ui: &mut Ui, n: usize) -> Point {
    let r = hit_rect(&frame(ui), "name/1/0");
    Point::new(r.x + n as f32 * 7.0 + 1.0, r.y + r.h / 2.0)
}

/// Selection rectangles in the frame (the light theme's `textSelection` color).
fn selection_rects(ui: &mut Ui) -> Vec<uison_core::Rect> {
    let sel = uison_core::Color::from_hex("#B4D5FE").unwrap();
    fn walk(items: &[uison_frame::Item], sel: uison_core::Color, out: &mut Vec<uison_core::Rect>) {
        for it in items {
            match &it.shape {
                Shape::Rect { rect, paint, .. } if paint.fill == Some(sel) => out.push(*rect),
                Shape::Clip { items, .. } => walk(items, sel, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(&frame(ui).items, sel, &mut out);
    out
}

#[test]
fn selection_keys_and_clipboard() {
    let mut ui = field_ui("привет мир", true); // Cyrillic on purpose: multibyte chars vs byte-index caret and selection
    let h = Rec::default();
    let send = |ui: &mut Ui, e: Event| {
        ui.on_event(&e, None, &h);
    };
    click(&mut ui, "name/1/0");
    send(&mut ui, key(KeyCode::End, |_| {}));
    for _ in 0..3 {
        send(&mut ui, key(KeyCode::Left, |m| m.shift = true));
    }
    let r = selection_rects(&mut ui);
    assert_eq!(r.len(), 1, "one selection rect");
    assert_eq!(r[0].w, 21.0, "three characters of 7 pt");
    send(&mut ui, cmd("c"));
    assert_eq!(*h.clip.borrow(), ["мир"]);
    // Typing replaces the selection.
    send(&mut ui, Event::Key(Key::char("дом")));
    assert_eq!(ui.data.str("name"), "привет дом");
    assert!(selection_rects(&mut ui).is_empty());
    // Alt+Shift+← on Apple — a whole word; an arrow without Shift collapses to the start.
    send(&mut ui, key(KeyCode::Left, |m| (m.alt, m.shift) = (true, true)));
    send(&mut ui, cmd("x"));
    assert_eq!(ui.data.str("name"), "привет ");
    assert_eq!(h.clip.borrow().last().unwrap(), "дом");
    // ⌘A, ⌘V → a request to the platform; the reply is Event::Paste, line breaks become spaces.
    send(&mut ui, cmd("a"));
    send(&mut ui, cmd("v"));
    assert_eq!(h.pastes.get(), 1);
    send(&mut ui, Event::Paste("раз\nдва".into()));
    assert_eq!(ui.data.str("name"), "раз два");
    // Alt+← — to the start of the word, ⌘⌫ — delete to the start of the line.
    send(&mut ui, key(KeyCode::Left, |m| m.alt = true));
    send(&mut ui, key(KeyCode::Backspace, |m| m.cmd = true));
    assert_eq!(ui.data.str("name"), "два");
}

#[test]
fn selection_keys_windows_style() {
    let mut ui = field_ui("один два три", false); // Cyrillic on purpose: multibyte word boundaries
    let h = Rec::default();
    click(&mut ui, "name/1/0");
    for e in [key(KeyCode::Home, |_| {}), key(KeyCode::Right, |m| m.cmd = true), key(KeyCode::Right, |m| (m.cmd, m.shift) = (true, true))] {
        ui.on_event(&e, None, &h);
    }
    // Ctrl+→ — to the start of the next word: "два " is selected.
    ui.on_event(&cmd("c"), None, &h);
    assert_eq!(*h.clip.borrow(), ["два "]);
    ui.on_event(&key(KeyCode::Backspace, |m| m.cmd = true), None, &h);
    assert_eq!(ui.data.str("name"), "один три");
    ui.on_event(&key(KeyCode::Backspace, |m| m.cmd = true), None, &h);
    assert_eq!(ui.data.str("name"), "три");
}

#[test]
fn selection_with_mouse() {
    let mut ui = field_ui("hello big world", true);
    let h = Rec::default();
    let hit = Some("name/1/0");
    let at = |ui: &mut Ui, n| field_point(ui, n);
    // Dragging: from the 6th character to the 9th — "big".
    let (a, b) = (at(&mut ui, 6), at(&mut ui, 9));
    for e in [Event::MouseDown(a, MouseButton::Left), Event::MouseMove(b), Event::MouseUp(b, MouseButton::Left)] {
        ui.on_event(&e, hit, &h);
    }
    ui.on_event(&cmd("c"), None, &h);
    assert_eq!(h.clip.borrow().last().unwrap(), "big");
    // Shift+click extends from the anchor.
    h.shift.set(true);
    let c = at(&mut ui, 15);
    ui.on_event(&Event::MouseDown(c, MouseButton::Left), hit, &h);
    ui.on_event(&Event::MouseUp(c, MouseButton::Left), hit, &h);
    h.shift.set(false);
    ui.on_event(&cmd("c"), None, &h);
    assert_eq!(h.clip.borrow().last().unwrap(), "big world");
    // Double click — a word, triple — everything.
    std::thread::sleep(std::time::Duration::from_millis(600));
    let p = at(&mut ui, 1);
    for n in 0..3 {
        ui.on_event(&Event::MouseDown(p, MouseButton::Left), hit, &h);
        ui.on_event(&Event::MouseUp(p, MouseButton::Left), hit, &h);
        ui.on_event(&cmd("c"), None, &h);
        let want = ["", "hello", "hello big world"][n];
        if n > 0 {
            assert_eq!(h.clip.borrow().last().unwrap(), want);
        }
    }
    // The mouse moves without a press — the selection doesn't change.
    let p = at(&mut ui, 3);
    ui.on_event(&Event::MouseMove(p), hit, &h);
    assert_eq!(selection_rects(&mut ui)[0].w, 15.0 * 7.0);
}

#[test]
fn long_text_does_not_jump_when_caret_blinks() {
    let long = "a".repeat(200);
    let mut ui = field_ui(&long, true);
    click(&mut ui, "name/1/0");
    ui.on_event(&key(KeyCode::End, |_| {}), None, &Fake);
    let x = |ui: &mut Ui| {
        let f = frame(ui);
        fn walk(items: &[uison_frame::Item]) -> Option<f32> {
            items.iter().find_map(|it| match &it.shape {
                Shape::Text { at, text, .. } if text.len() == 200 => Some(at.x),
                Shape::Clip { items, .. } => walk(items),
                _ => None,
            })
        }
        walk(&f.items).unwrap()
    };
    let x0 = x(&mut ui);
    assert!(x0 < 0.0, "long text shifted left");
    std::thread::sleep(std::time::Duration::from_millis(560));
    ui.on_event(&Event::Tick, None, &Fake);
    assert_eq!(x(&mut ui), x0, "caret blinked, text stays in place");
    // Caret left within the visible part — the text doesn't move.
    ui.on_event(&key(KeyCode::Left, |_| {}), None, &Fake);
    assert_eq!(x(&mut ui), x0);
}

#[test]
fn long_press_selects_word() {
    let mut ui = field_ui("hello world", true);
    let h = Rec::default();
    let p = field_point(&mut ui, 8);
    ui.on_event(&Event::MouseDown(p, MouseButton::Touch), Some("name/1/0"), &h);
    let wait = ui.wants_tick().expect("hold timer");
    assert!(wait <= std::time::Duration::from_millis(500));
    std::thread::sleep(wait);
    ui.on_event(&Event::Tick, None, &h);
    ui.on_event(&Event::MouseUp(p, MouseButton::Touch), Some("name/1/0"), &h);
    // The word is selected, the field menu above it: "Copy".
    let t = texts(&frame(&mut ui));
    assert!(t.contains(&"Copy".to_string()) && !t.contains(&"⌘C".to_string()), "by finger — no shortcuts");
    click_with(&mut ui, "~menu/1", &h);
    assert_eq!(*h.clip.borrow(), ["world"]);
}

fn popup_ui() -> (Ui, Rc<RefCell<Vec<String>>>) {
    let mut ui = Ui::new(
        r##"{ size: [400, 300], data: { name: "text" }, root: { Column: { gap: 10, children: [
            { Button: { key: "save", text: "Save", tooltip: "Save the file", on: { click: "save" } } },
            { Box: { key: "row", w: 200, h: 30, fill: "#EEE",
                     contextMenu: [{ label: "Open", action: "open" }, "-", { label: "Delete", action: "del", shortcut: "⌫" }, { label: "Unavailable", action: "x", disabled: true }] } },
            { TextField: { key: "name", value: "@name" } },
            { Link: { key: "site", text: "Site", href: "https://example.com" } },
        ] } } }"##,
    )
    .unwrap();
    ui.update(|d| d.set("env.mod", "⌘"));
    let log = Rc::new(RefCell::new(Vec::new()));
    for a in ["open", "del", "save"] {
        let l = log.clone();
        ui.on(a, move |cx| l.borrow_mut().push(format!("{} {} {}", cx.action, cx.event, cx.key)));
    }
    (ui, log)
}

#[test]
fn tooltip_after_delay_and_hidden_on_press() {
    let (mut ui, _) = popup_ui();
    let p = center(hit_rect(&frame(&mut ui), "save"));
    ui.on_event(&Event::MouseMove(p), Some("save"), &Fake);
    assert!(!texts(&frame(&mut ui)).contains(&"Save the file".to_string()));
    let wait = ui.wants_tick().expect("tooltip timer");
    assert!(wait <= std::time::Duration::from_millis(500));
    std::thread::sleep(wait);
    ui.on_event(&Event::Tick, None, &Fake);
    assert!(texts(&frame(&mut ui)).contains(&"Save the file".to_string()));
    // Moving inside the node — the tooltip stays (the frame doesn't change).
    assert!(ui.on_event(&Event::MouseMove(Point::new(p.x + 3.0, p.y)), Some("save"), &Fake).is_none());
    ui.on_event(&Event::MouseDown(p, MouseButton::Left), Some("save"), &Fake);
    assert!(!texts(&frame(&mut ui)).contains(&"Save the file".to_string()));
}

#[test]
fn context_menu_right_click_keys_and_release() {
    let (mut ui, log) = popup_ui();
    let p = center(hit_rect(&frame(&mut ui), "row"));
    ui.on_event(&Event::MouseDown(p, MouseButton::Right), Some("row"), &Fake);
    ui.on_event(&Event::MouseUp(p, MouseButton::Right), Some("row"), &Fake);
    let f = frame(&mut ui);
    assert!(texts(&f).contains(&"Delete".to_string()), "menu open");
    // The menu is at the cursor and inside the window.
    let r = hit_rect(&f, "~menu");
    assert!(r.x >= p.x - 0.5 && r.right() <= 400.0 && r.bottom() <= 300.0, "{r:?} at {p:?}");
    // A click on an item — the action with the node's key.
    click(&mut ui, "~menu/2");
    assert_eq!(*log.borrow(), ["del menu row"]);
    assert!(!texts(&frame(&mut ui)).contains(&"Delete".to_string()), "closed");
    // Keyboard: down (first enabled), down again — past the separator, Enter.
    ui.on_event(&Event::MouseDown(p, MouseButton::Right), Some("row"), &Fake);
    for k in [KeyCode::Down, KeyCode::Down, KeyCode::Down] {
        ui.on_event(&Event::Key(Key::new(k)), None, &Fake);
    }
    ui.on_event(&Event::Key(Key::new(KeyCode::Enter)), None, &Fake);
    assert_eq!(log.borrow().last().unwrap(), "open menu row", "wraps around, skipping the separator and the disabled item");
    // First letter and Escape.
    ui.on_event(&Event::MouseDown(p, MouseButton::Right), Some("row"), &Fake);
    ui.on_event(&Event::Key(Key::char("d")), None, &Fake);
    ui.on_event(&Event::Key(Key::new(KeyCode::Escape)), None, &Fake);
    assert!(!texts(&frame(&mut ui)).contains(&"Delete".to_string()));
    assert_eq!(log.borrow().len(), 2);
    // Press the right button, move to an item, release — select it (as in macOS).
    ui.on_event(&Event::MouseDown(p, MouseButton::Right), Some("row"), &Fake);
    let q = center(hit_rect(&frame(&mut ui), "~menu/0"));
    ui.on_event(&Event::MouseMove(q), Some("~menu/0"), &Fake);
    ui.on_event(&Event::MouseUp(q, MouseButton::Right), Some("~menu/0"), &Fake);
    assert_eq!(log.borrow().last().unwrap(), "open menu row");
    // A click outside closes it and presses nothing.
    ui.on_event(&Event::MouseDown(p, MouseButton::Right), Some("row"), &Fake);
    click(&mut ui, "save");
    assert!(!texts(&frame(&mut ui)).contains(&"Delete".to_string()));
    assert_eq!(log.borrow().len(), 3, "save not pressed — the click closed the menu");
}

#[test]
fn field_menu_and_link() {
    let (mut ui, _) = popup_ui();
    let h = Rec::default();
    click(&mut ui, "name/1/0");
    ui.on_event(&cmd("a"), None, &h);
    let p = center(hit_rect(&frame(&mut ui), "name/1/0"));
    ui.on_event(&Event::MouseDown(p, MouseButton::Right), Some("name/1/0"), &h);
    let t = texts(&frame(&mut ui));
    assert!(t.contains(&"Copy".to_string()) && t.contains(&"⌘C".to_string()));
    click_with(&mut ui, "~menu/1", &h);
    assert_eq!(*h.clip.borrow(), ["text"], "Copy — the selection goes to the clipboard");
    // Shift+F10 — the focused field's menu; "Paste" requests the clipboard.
    let mut k = Key::new(KeyCode::F10);
    k.mods.shift = true;
    ui.on_event(&Event::Key(k), None, &h);
    click_with(&mut ui, "~menu/2", &h);
    assert_eq!(h.pastes.get(), 1);
    // The link opens in the system.
    let f = frame(&mut ui);
    let p = center(hit_rect(&f, "site"));
    for e in [Event::MouseDown(p, MouseButton::Left), Event::MouseUp(p, MouseButton::Left)] {
        ui.on_event(&e, Some("site"), &h);
    }
    assert_eq!(h.clip.borrow().last().unwrap(), "open https://example.com");
}

/// A PNG header of the required size — nothing else is needed for the size from the header.
fn png_header(w: u32, h: u32) -> Vec<u8> {
    let mut b = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    b.extend_from_slice(&w.to_be_bytes());
    b.extend_from_slice(&h.to_be_bytes());
    b
}

#[test]
fn images_natural_size_and_avatar() {
    let mut ui = Ui::new(
        r#"{ size: [400, 400], root: { Column: { children: [
            { Image: { src: "mem:t-photo" } },
            { Image: { src: "mem:t-photo", w: 100 } },
            { Image: { src: "mem:t-wide", w: 50, h: 50, fit: "contain", radius: 8 } },
            { Avatar: { name: "Anna", src: "mem:t-photo", size: 40 } },
            { Avatar: { name: "Boris" } },
        ] } } }"#,
    )
    .unwrap();
    assert_eq!(ui.image("t-photo", png_header(200, 150)), "mem:t-photo");
    ui.image("t-wide", png_header(800, 100));
    let f = frame(&mut ui);
    let imgs: Vec<(uison_core::Rect, String, f32, uison_core::Fit)> = f
        .items
        .iter()
        .filter_map(|it| match &it.shape {
            Shape::Image { rect, src, radius, fit } => Some((*rect, src.to_string(), *radius, *fit)),
            _ => None,
        })
        .collect();
    assert_eq!(imgs.len(), 4, "three Images and the avatar photo");
    assert_eq!((imgs[0].0.w, imgs[0].0.h), (200.0, 150.0), "natural size");
    assert_eq!((imgs[1].0.w, imgs[1].0.h), (100.0, 75.0), "width set — height by aspect ratio");
    assert_eq!((imgs[2].0.w, imgs[2].0.h, imgs[2].2, imgs[2].3), (50.0, 50.0, 8.0, uison_core::Fit::Contain));
    assert_eq!((imgs[3].0.w, imgs[3].0.h), (40.0, 40.0));
    let t = texts(&f);
    assert!(t.contains(&"B".to_string()) && !t.contains(&"A".to_string()), "initials — only without a photo: {t:?}");
}

#[test]
fn post_from_other_thread() {
    use std::sync::atomic::{AtomicU32, Ordering};
    let mut ui = Ui::new(r#"{ data: { progress: 0 }, root: { ProgressBar: { value: "@progress" } } }"#).unwrap();
    let h = ui.handle();
    // Before the window starts: waits for the waker.
    std::thread::spawn(move || h.post(|cx| cx.data.set("progress", 0.5))).join().unwrap();
    let woke = Arc::new(AtomicU32::new(0));
    let w = woke.clone();
    ui.set_waker(Arc::new(move || {
        w.fetch_add(1, Ordering::SeqCst);
    }));
    assert_eq!(woke.load(Ordering::SeqCst), 1, "queue not empty — wake right away");
    assert!(ui.on_event(&Event::Wake, None, &Fake).is_some(), "ran — frame rebuilt");
    assert_eq!(ui.data.num("progress"), 0.5);
    // After the start: every post wakes.
    let h = ui.handle();
    std::thread::spawn(move || h.post(|cx| cx.data.set("progress", 1.0))).join().unwrap();
    assert_eq!(woke.load(Ordering::SeqCst), 2);
    ui.on_event(&Event::Wake, None, &Fake);
    assert_eq!(ui.data.num("progress"), 1.0);
    assert!(ui.on_event(&Event::Wake, None, &Fake).is_none(), "empty queue — same frame");
}

/// Text fragments of the frame with their `at` coordinates.
fn text_items(f: &Frame) -> Vec<(String, Point)> {
    f.items
        .iter()
        .filter_map(|it| match &it.shape {
            Shape::Text { text, at, .. } => Some((text.to_string(), *at)),
            _ => None,
        })
        .collect()
}

#[test]
fn flow_wraps_across_spans_and_link_hits_every_line() {
    // 7 points per character: 28 characters fit into 200 points.
    let mut ui = Ui::new(
        r##"{ size: [200, 200], root: { Flow: { w: "fill", children: [
            { Text: { text: "aaaa bbbb cccc " } },
            { Text: { key: "link", text: "dddd eeee ffff", decoration: "underline", on: { click: "go" } } },
        ] } } }"##,
    )
    .unwrap();
    let clicks = Rc::new(RefCell::new(0));
    let c = clicks.clone();
    ui.on("go", move |_| *c.borrow_mut() += 1);
    let f = frame(&mut ui);
    let t = text_items(&f);
    assert_eq!(t.iter().map(|(s, _)| s.as_str()).collect::<Vec<_>>(), ["aaaa bbbb cccc ", "dddd eeee", "ffff"]);
    assert_eq!((t[1].1.x, t[2].1.x), (105.0, 0.0));
    assert!(t[2].1.y > t[1].1.y, "the link wraps to the second line");
    // Underline under every fragment of the link.
    let lines = f.items.iter().filter(|it| matches!(it.shape, Shape::Line { .. })).count();
    assert_eq!(lines, 2);
    // A press on the second line of the link — the same link.
    let p = Point::new(t[2].1.x + 5.0, t[2].1.y + 5.0);
    assert_eq!(uison_frame::hit_test(&f, p), Some("link"));
    let hit = Some("link");
    for e in [Event::MouseMove(p), Event::MouseDown(p, MouseButton::Left), Event::MouseUp(p, MouseButton::Left)] {
        ui.on_event(&e, hit, &Fake);
    }
    assert_eq!(*clicks.borrow(), 1);
}

#[test]
fn flow_keeps_glued_words_and_aligns_baselines() {
    // No space between "foo" and "bar" — they wrap together.
    let mut ui = Ui::new(
        r##"{ size: [50, 200], root: { Flow: { w: "fill", children: [
            { Text: { text: "xx foo" } },
            { Text: { text: "bar", bold: true } },
        ] } } }"##,
    )
    .unwrap();
    let t = text_items(&frame(&mut ui));
    assert_eq!(t.iter().map(|(s, _)| s.as_str()).collect::<Vec<_>>(), ["xx", "foo", "bar"]);
    assert_eq!((t[1].1.y, t[2].1.y, t[2].1.x), (t[2].1.y, t[1].1.y, 21.0));

    // A larger run lowers the line's baseline: small text moves down.
    let mut ui = Ui::new(
        r##"{ size: [300, 200], root: { Flow: { w: "fill", children: [
            { Text: { text: "small ", size: 10 } },
            { Text: { text: "BIG", size: 20 } },
            { Box: { w: 30, h: 30, fill: "#F00" } },
        ] } } }"##,
    )
    .unwrap();
    let f = frame(&mut ui);
    let t = text_items(&f);
    // The baseline follows the 30-point block: the block's bottom sits on it.
    let base = 30.0;
    assert_eq!(t[0].1.y, base - 9.0);
    assert_eq!(t[1].1.y, base - 18.0);
    let rect = f.items.iter().find_map(|it| match it.shape {
        Shape::Rect { rect, .. } => Some(rect),
        _ => None,
    });
    assert_eq!(rect.map(|r| (r.x, r.y)), Some((6.0 * 7.0 + 3.0 * 7.0, 0.0)));
}

#[test]
fn text_without_wrap_keeps_spaces() {
    let mut ui = Ui::new(r##"{ size: [60, 200], root: { Text: { text: "a  b c d e f\n  x", wrap: false } } }"##).unwrap();
    assert_eq!(texts(&frame(&mut ui)), ["a  b c d e f", "  x"]);
}

#[test]
fn italic_and_generic_family_reach_the_frame() {
    let mut ui = Ui::new(r##"{ size: [200, 100], root: { Text: { text: "x", italic: true, font: "monospace" } } }"##).unwrap();
    let src = ui.render(&Fake);
    assert!(src.contains("italic:true") && src.contains(r#"font:"monospace""#), "{src}");
    let f = Frame::parse(&src, &Fake).unwrap();
    let font = f.items.iter().find_map(|it| match &it.shape {
        Shape::Text { font, .. } => Some(font.clone()),
        _ => None,
    });
    assert!(font.is_some_and(|f| f.italic && f.family.as_deref() == Some("monospace")));
}

#[test]
fn reference_examples_render() {
    // Examples from docs/screens/bindings.md: components with parameters and @~, For with index, switch, functions.
    let src = r##"{
      size: [400, 600],
      data: { stats: { total: 5 }, filter: "all", t: { priority: "high" }, user: { name: "Anna Petrova" } },
      components: {
        Stat: {
          params: { label: "", value: 0 },
          body: { Column: { children: [
            { Text: { text: "$label", size: 12 } },
            { Text: { text: "{$value}", size: 22, bold: true } },
          ] } },
        },
        Collapse: {
          params: { title: "", children: [] },
          body: { Column: { children: [
            { Row: { key: "head", toggle: "@~open", children: [ { Text: { text: "$title" } } ] } },
            { Box: { visible: "@~open", child: { Column: { children: "$children" } } } },
          ] } },
        },
      },
      root: { Column: { children: [
        { Stat: { label: "Tasks", value: "@stats.total" } },
        { For: { each: ["Morning", "Afternoon"], as: "p", index: "i", do: { Text: { text: "{@i}: {@p}" } } } },
        { Text: { text: "x", color: { switch: "@t.priority", cases: { high: "$theme.danger" }, default: "$theme.text" } } },
        { Text: { text: "{@user.name} → ", visible: { lt: [300, 600] } } },
        { Avatar: { name: "@user.name" } },
        { Text: { text: "len {@stats.total} {{braces}}" } },
        { Collapse: { key: "c", title: "More", children: [ { Text: { text: "inside" } } ] } },
      ] } },
    }"##;
    let mut ui = Ui::new(src).unwrap();
    let t = texts(&frame(&mut ui));
    for want in ["Tasks", "5", "0: Morning", "1: Afternoon", "len 5 {braces}", "More", "AP"] {
        assert!(t.iter().any(|s| s == want), "{want} in {t:?}");
    }
    assert!(!t.iter().any(|s| s == "inside"), "collapsed");
    // A key inside a component — as written (without the instance prefix).
    click(&mut ui, "head");
    assert!(texts(&frame(&mut ui)).iter().any(|s| s == "inside"), "@~open of the instance");
}

#[test]
fn canvas_draws_and_reports_pointer() {
    let src = r#"{ size: [200, 100], root: { Row: { children: [
        { Box: { w: 50, h: 100 } },
        { Canvas: { key: "chart", draw: "chart", flex: 1, h: "fill", on: { click: "pick", hover: "look" } } },
    ] } } }"#;
    let mut ui = Ui::new(src).unwrap();
    let rects = Rc::new(RefCell::new(Vec::new()));
    let r = rects.clone();
    ui.draw("chart", move |d| {
        r.borrow_mut().push(d.rect);
        let c = Point::new(d.rect.x + 10.0, d.rect.y + 10.0);
        d.w.poly(&[c, Point::new(c.x + 20.0, c.y), Point::new(c.x, c.y + 20.0)]).fill(uison_core::Color::BLACK);
    });
    let log = Rc::new(RefCell::new(Vec::new()));
    let l = log.clone();
    ui.on("look", move |cx| l.borrow_mut().push(format!("hover {:?}", cx.pos.map(|p| (p.x, p.y)))));
    let l = log.clone();
    ui.on("pick", move |cx| l.borrow_mut().push(format!("click {:?} x{}", cx.pos.map(|p| (p.x, p.y)), cx.clicks)));

    let f = frame(&mut ui);
    assert_eq!(rects.borrow().last().copied(), Some(uison_core::Rect::new(50.0, 0.0, 150.0, 100.0)));
    // The drawing is clipped to the node: the polygon is inside the clip.
    let clip = f.items.iter().find_map(|it| match &it.shape {
        Shape::Clip { items, .. } => Some(items),
        _ => None,
    });
    assert!(matches!(clip.unwrap()[0].shape, Shape::Poly { .. }));

    let p = Point::new(60.0, 30.0);
    ui.on_event(&Event::MouseMove(p), Some("chart"), &Fake);
    for _ in 0..2 {
        ui.on_event(&Event::MouseDown(p, MouseButton::Left), Some("chart"), &Fake);
        ui.on_event(&Event::MouseUp(p, MouseButton::Left), Some("chart"), &Fake);
    }
    ui.on_event(&Event::MouseMove(Point::new(10.0, 10.0)), None, &Fake);
    assert_eq!(
        *log.borrow(),
        ["hover Some((10.0, 30.0))", "click Some((10.0, 30.0)) x1", "click Some((10.0, 30.0)) x2", "hover None"]
    );
}
