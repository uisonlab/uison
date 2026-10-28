use uison_frame::console::Estimate;
use uison_ui::{Json, Ui};

use crate::Agent;
use crate::tree::{by_name, find, flatten, to_text};

const SCREEN: &str = r##"{
  data: {
    n: 0, agree: false, name: "", sent: "", country: null, volume: 20, modal: false, filter: "all",
    countries: [{ value: "de", label: "Germany" }, { value: "fr", label: "France" }],
    tabs: [{ value: "all", label: "All" }, { value: "done", label: "Done" }],
    rows: [],
  },
  root: { Stack: { w: "fill", h: "fill", children: [
    { Column: { padding: 16, gap: 8, cross: "start", children: [
      { Text: { text: "Count {@n}" } },
      { Button: { key: "inc", text: "Add one", on: { click: "inc" } } },
      { Button: { key: "never", text: "Never", disabled: true, on: { click: "inc" } } },
      { Checkbox: { key: "agree", checked: "@agree", label: "I agree" } },
      { TextField: { key: "name", label: "Your name", value: "@name", placeholder: "John", on: { submit: "send" } } },
      { Text: { text: "Sent: {@sent}" } },
      { Select: { key: "country", label: "Country", value: "@country", options: "@countries" } },
      { Tabs: { key: "filter", value: "@filter", tabs: "@tabs" } },
      { Slider: { key: "volume", value: "@volume", min: 0, max: 100, step: 1 } },
      { Button: { key: "open", text: "Open dialog", on: { click: "open" } } },
      { List: { key: "rows", h: 200, w: 300, itemHeight: 20, items: "@rows", as: "r", item: { Button: { key: "row-{@r}", text: "Row {@r}", variant: "text", on: { click: "row" } } } } },
    ] } },
    { Box: { visible: "@modal", w: "fill", h: "fill", fill: "#00000044", align: "center", dismiss: "@modal",
      child: { Box: { padding: 20, fill: "#FFFFFF", child: { Button: { key: "close", text: "Close", on: { click: "close" } } } } } } },
  ] } },
}"##;

fn ui() -> Ui {
    let mut ui = Ui::new(SCREEN).unwrap_or_else(|e| panic!("{e}"));
    let rows: Vec<Json> = (0..500).map(|i| Json::from(i as f64)).collect();
    ui.data.set("rows", rows);
    ui.on("inc", |cx| {
        let n = cx.data.num("n");
        cx.data.set("n", n + 1.0);
    });
    ui.on("send", |cx| {
        let name = cx.data.str("name").to_string();
        cx.data.set("sent", name);
    });
    ui.on("open", |cx| cx.data.set("modal", true));
    ui.on("close", |cx| cx.data.set("modal", false));
    ui.on("row", |cx| {
        let k = cx.key.to_string();
        cx.data.set("sent", k);
    });
    ui
}

fn run(a: &mut Agent, cmd: &str) -> Json {
    let line = a.handle(cmd).expect("an answer");
    uison_json::parse(&line).unwrap().into_owned()
}

fn ok(a: &mut Agent, cmd: &str) -> Json {
    let r = run(a, cmd);
    assert!(r.get("error").is_none(), "{cmd} → {}", r.to_json());
    r
}

fn err(a: &mut Agent, cmd: &str) -> String {
    let r = run(a, cmd);
    r.get("error").and_then(Json::as_str).unwrap_or_else(|| panic!("{cmd} should fail: {}", r.to_json())).to_string()
}

fn node(a: &mut Agent, key: &str) -> Json {
    let t = a.tree();
    find(&t, key).unwrap_or_else(|| panic!("no '{key}' in\n{}", to_text(&t))).0.clone()
}

#[test]
fn tree_has_roles_names_and_state() {
    let m = Estimate;
    let mut a = Agent::new(ui(), &m);
    let t = a.tree();
    let role = |k: &str| find(&t, k).and_then(|(n, _)| n.get("role")?.as_str().map(str::to_string));
    assert_eq!(role("inc").as_deref(), Some("button"));
    assert_eq!(role("agree").as_deref(), Some("checkbox"));
    assert_eq!(role("volume").as_deref(), Some("slider"));
    assert_eq!(role("rows").as_deref(), Some("list"));
    assert_eq!(by_name(&t, "Your name", Some("textfield")).map(|k| k.starts_with("name")), Ok(true));
    let never = find(&t, "never").unwrap().0;
    assert_eq!(never.get("disabled").and_then(Json::as_bool), Some(true));
    let names: Vec<&str> = flatten(&t).iter().filter_map(|n| n.get("name")?.as_str()).collect();
    assert!(names.contains(&"Count 0") && names.contains(&"Add one") && names.contains(&"I agree"), "{names:?}");
    // Only the visible rows of a 500-item list are in the tree.
    let rows = find(&t, "rows").unwrap().0;
    assert_eq!(rows.get("count").and_then(Json::as_f64), Some(500.0));
    assert!(rows.get("children").and_then(Json::as_array).is_some_and(|c| c.len() < 30));
}

#[test]
fn click_type_and_changes() {
    let m = Estimate;
    let mut a = Agent::new(ui(), &m);
    ok(&mut a, r#"{"id":1,"query":"tree"}"#);
    let r = ok(&mut a, r#"{"id":2,"do":"click","key":"inc"}"#);
    assert_eq!(a.ui().data.num("n"), 1.0);
    // Only what changed comes back: the counter text.
    let changes = r.get("changes").expect("changes").to_json();
    assert!(changes.contains("Count 1") && changes.contains("Count 0"), "{changes}");
    assert!(!changes.contains("Add one"), "{changes}");
    // Two clicks in a row are two clicks, not a double click.
    ok(&mut a, r#"{"do":"click","name":"Add one"}"#);
    assert_eq!(a.ui().data.num("n"), 2.0);

    ok(&mut a, r#"{"do":"type","name":"Your name","text":"Anna","submit":true}"#);
    assert_eq!(a.ui().data.str("sent"), "Anna");
    // Replacing is the default; append adds.
    ok(&mut a, r#"{"do":"type","name":"Your name","text":" Lee","append":true}"#);
    assert_eq!(a.ui().data.str("name"), "Anna Lee");
    ok(&mut a, r#"{"do":"set","name":"Your name","value":"Bo"}"#);
    assert_eq!(a.ui().data.str("name"), "Bo");
}

#[test]
fn a_person_can_not_do_it_neither_can_the_agent() {
    let m = Estimate;
    let mut a = Agent::new(ui(), &m);
    assert!(err(&mut a, r#"{"id":1,"do":"click","key":"never"}"#).contains("disabled"));
    assert_eq!(a.ui().data.num("n"), 0.0);
    assert!(err(&mut a, r#"{"do":"click","key":"nope"}"#).contains("no node"));
    // A modal covers the screen: the button under it can't be pressed.
    ok(&mut a, r#"{"do":"click","key":"open"}"#);
    assert!(err(&mut a, r#"{"do":"click","key":"inc"}"#).contains("covered"));
    assert_eq!(a.ui().data.num("n"), 0.0);
    ok(&mut a, r#"{"do":"click","key":"close"}"#);
    ok(&mut a, r#"{"do":"click","key":"inc"}"#);
    assert_eq!(a.ui().data.num("n"), 1.0);
}

#[test]
fn controls() {
    let m = Estimate;
    let mut a = Agent::new(ui(), &m);
    ok(&mut a, r#"{"do":"set","key":"agree","value":true}"#);
    assert!(a.ui().data.bool("agree"));
    ok(&mut a, r#"{"do":"set","key":"agree","value":true}"#);
    assert!(a.ui().data.bool("agree"), "set to the same value leaves it");
    assert_eq!(node(&mut a, "agree").get("checked").and_then(Json::as_bool), Some(true));

    ok(&mut a, r#"{"do":"set","key":"volume","value":73}"#);
    assert_eq!(a.ui().data.num("volume"), 73.0);

    let country = by_name(&a.tree(), "Country", None).unwrap();
    ok(&mut a, &format!(r#"{{"do":"pick","key":"{country}","option":"France"}}"#));
    assert_eq!(a.ui().data.str("country"), "fr");
    assert_eq!(node(&mut a, &country).get("expanded").and_then(Json::as_bool), Some(false), "the dropdown closed");

    ok(&mut a, r#"{"do":"pick","option":"Done"}"#);
    assert_eq!(a.ui().data.str("filter"), "done");
}

#[test]
fn lists_scroll_to_items() {
    let m = Estimate;
    let mut a = Agent::new(ui(), &m);
    ok(&mut a, r#"{"do":"scroll","key":"rows","to":300}"#);
    let rows = node(&mut a, "rows");
    assert_eq!(rows.get("first").and_then(Json::as_f64), Some(300.0), "{}", rows.to_json());
    ok(&mut a, r#"{"do":"click","key":"row-305"}"#);
    assert_eq!(a.ui().data.str("sent"), "row-305");
    ok(&mut a, r#"{"do":"scroll","key":"rows","edge":"top"}"#);
    assert_eq!(node(&mut a, "rows").get("first").and_then(Json::as_f64), Some(0.0));
}

#[test]
fn queries_and_console_messages() {
    let m = Estimate;
    let mut a = Agent::new(ui(), &m);
    let d = ok(&mut a, r#"{"id":1,"query":"data","path":"countries"}"#);
    assert!(d.get("data").unwrap().to_json().contains("Germany"));
    let acts = ok(&mut a, r#"{"id":2,"query":"actions"}"#).get("actions").unwrap().to_json();
    assert!(acts.contains("inc") && acts.contains("send"), "{acts}");
    // The console way still works: an action by name and a data change; no id — no answer.
    assert!(a.handle(r#"{"action":"inc"}"#).is_none());
    assert_eq!(a.ui().data.num("n"), 1.0);
    ok(&mut a, r#"{"id":3,"data":{"name":"Zed"}}"#);
    assert_eq!(a.ui().data.str("name"), "Zed");
    let t = ok(&mut a, r#"{"id":4,"query":"text"}"#).get("text").and_then(Json::as_str).unwrap().to_string();
    assert!(t.contains("button \"Add one\" #inc"), "{t}");
}

#[test]
fn mcp_round_trip() {
    let m = Estimate;
    let mut a = Agent::new(ui(), &m);
    let mut s = crate::mcp::Server::new("Test");
    let init = s.handle(&mut a, r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18"}}"#).unwrap();
    assert!(init.contains("\"protocolVersion\":\"2025-06-18\""), "{init}");
    assert!(s.handle(&mut a, r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#).is_none());
    let list = s.handle(&mut a, r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#).unwrap();
    assert!(list.contains("\"snapshot\"") && list.contains("\"click\""));
    let snap = s.handle(&mut a, r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"snapshot","arguments":{}}}"#).unwrap();
    assert!(snap.contains("Add one"), "{snap}");
    let click = s.handle(&mut a, r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"click","arguments":{"key":"inc"}}}"#).unwrap();
    assert!(click.contains("Count 1") && click.contains("\"isError\":false"), "{click}");
    let bad = s.handle(&mut a, r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"click","arguments":{"key":"never"}}}"#).unwrap();
    assert!(bad.contains("\"isError\":true") && bad.contains("disabled"), "{bad}");
}
