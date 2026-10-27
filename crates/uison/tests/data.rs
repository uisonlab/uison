//! Structs ↔ data: `#[derive(Data)]`.
//! Cyrillic sample values are intentional: they check that non-ASCII strings round-trip unescaped.

use std::collections::HashMap;

use uison::prelude::*;

#[derive(Data, Default, Debug, PartialEq, Clone)]
struct Point {
    x: f64,
    y: f64,
}

#[derive(Data, Default)]
struct Todo {
    /// Field documentation doesn't get in the way.
    pub id: i32,
    title: String,
    #[data(rename = "isDone")]
    done: bool,
    note: Option<String>,
    tags: Vec<String>,
    at: Point,
    scores: HashMap<String, f32>,
    #[data(skip)]
    cache: Option<fn() -> i32>,
}

#[derive(Data, Default, Debug, PartialEq)]
pub struct Form {
    pub name: String,
    pub age: u32,
    pub agree: bool,
}

#[test]
fn derive_round_trip() {
    let mut ui = Ui::from_node(column());
    let t = Todo {
        id: 7,
        title: "Купить \"молоко\"".into(),
        done: true,
        note: None,
        tags: vec!["дом".into()],
        at: Point { x: 1.5, y: -2.0 },
        scores: HashMap::from([("b".into(), 2.0), ("a".into(), 1.0)]),
        cache: Some(|| 1),
    };
    ui.data.set("todo", t);
    assert_eq!(ui.data.str("todo.title"), "Купить \"молоко\"");
    assert!(ui.data.bool("todo.isDone"));
    assert!(ui.data.get("todo.cache").is_none(), "skip: not in data");
    assert_eq!(ui.data.get("todo.scores").unwrap().to_json(), r#"{"a":1,"b":2}"#, "HashMap keys are sorted");
    let back: Todo = ui.data.read("todo").unwrap();
    assert_eq!((back.id, back.done, back.at.x, back.tags.len(), back.note.is_none()), (7, true, 1.5, 1, true));
    assert_eq!(back.scores["b"], 2.0);
    assert!(back.cache.is_none(), "skip: from Default");

    // Missing in data comes from Default; not an object gives None.
    ui.data.set("partial", obj([("title", "x".into_json())]));
    let p: Todo = ui.data.read("partial").unwrap();
    assert_eq!((p.id, p.title.as_str(), p.done), (0, "x", false));
    assert!(ui.data.read::<Todo>("todo.title").is_none());

    // edit: read, change, write back.
    ui.data.edit("todo", |t: &mut Todo| t.tags.push("срочно".into()));
    assert_eq!(ui.data.len("todo.tags"), 2);
    let list: Vec<Point> = vec![Point { x: 1.0, y: 2.0 }, Point::default()];
    ui.data.set("points", list.clone());
    assert_eq!(ui.data.read::<Vec<Point>>("points"), Some(list));
}

#[test]
fn field_order_and_edit() {
    let mut ui = Ui::from_node(column());
    ui.data.set("form", Form { name: "Анна".into(), age: 30, agree: true });
    assert_eq!(ui.data.get("form").unwrap().to_json(), r#"{"name":"Анна","age":30,"agree":true}"#);
    ui.data.edit("form", |f: &mut Form| f.age += 1);
    assert_eq!(ui.data.read::<Form>("form"), Some(Form { name: "Анна".into(), age: 31, agree: true }));
}
