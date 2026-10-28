//! MCP server (JSON-RPC 2.0, one message per line on stdin/stdout): the agent commands as tools.
//! The screen goes to the model as text, one node per line (`tree::to_text`): fewer tokens than JSON.

use uison_ui::Json;

use crate::Agent;
use crate::tree::{diff_len, diff_text, flatten, to_text, view};

/// Protocol versions we speak; the client's is echoed back if we know it.
const VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

/// Above this many changed nodes the answer is the whole screen: a new screen reads better than a diff.
const WHOLE: usize = 40;

const TARGET: &str = r#""key": { "type": "string", "description": "Node key from the snapshot (after #)" },
  "name": { "type": "string", "description": "Or the node's name (exact, else a part of it), when there is one such node" },
  "role": { "type": "string", "description": "With name: only nodes of this role (button, option, textfield…)" }"#;

fn tools() -> Json {
    let src = format!(
        r#"[
{{ "name": "snapshot", "description": "The program's screen: one node per line — role, \"name\", #key, state (checked, selected, disabled, focused, offscreen, value=…). Indented nodes are inside the one above. Call it first and whenever unsure; actions return only what changed.",
  "inputSchema": {{ "type": "object", "properties": {{
    "under": {{ "type": "string", "description": "Only this node's subtree (a key)" }},
    "rects": {{ "type": "boolean", "description": "Add rectangles @x,y,w,h (points)" }} }} }} }},
{{ "name": "click", "description": "Press and release on a node, as a person would with the mouse. Fails if the node is disabled or covered (e.g. by a dialog); scrolls it into view if needed.",
  "inputSchema": {{ "type": "object", "properties": {{ {TARGET},
    "clicks": {{ "type": "integer", "description": "2 — double click" }},
    "button": {{ "type": "string", "enum": ["left", "right"] }} }} }} }},
{{ "name": "type", "description": "Type text into a textfield (focuses it; replaces its text unless append). Without key/name — into the focused field.",
  "inputSchema": {{ "type": "object", "properties": {{ {TARGET},
    "text": {{ "type": "string" }},
    "append": {{ "type": "boolean", "description": "Add to the end instead of replacing" }},
    "submit": {{ "type": "boolean", "description": "Press Enter after typing" }} }}, "required": ["text"] }} }},
{{ "name": "press_key", "description": "A key to the focused node: enter, escape, tab, backspace, delete, up, down, left, right, home, end, a letter; with modifiers: cmd+a, shift+tab (cmd is the primary modifier: Ctrl on Windows and Linux).",
  "inputSchema": {{ "type": "object", "properties": {{ "code": {{ "type": "string" }} }}, "required": ["code"] }} }},
{{ "name": "hover", "description": "Rest the pointer on a node (shows its tooltip, hover handlers).",
  "inputSchema": {{ "type": "object", "properties": {{ {TARGET} }} }} }},
{{ "name": "scroll", "description": "Scroll a list or scroll view: to an item number (lists), to a node key, by points (positive — down) or to an edge. Given any other node — scrolls it into view.",
  "inputSchema": {{ "type": "object", "properties": {{ {TARGET},
    "to": {{ "description": "Item number in a list, or a key to bring into view" }},
    "by": {{ "type": "number" }},
    "edge": {{ "type": "string", "enum": ["top", "bottom"] }} }} }} }},
{{ "name": "set_value", "description": "Set a control to a value: slider (number), checkbox (true/false), textfield (text).",
  "inputSchema": {{ "type": "object", "properties": {{ {TARGET}, "value": {{}} }}, "required": ["value"] }} }},
{{ "name": "choose", "description": "Choose an option by its name: in a dropdown (key of the dropdown, it is opened first), among tabs or radio buttons.",
  "inputSchema": {{ "type": "object", "properties": {{ {TARGET}, "option": {{ "type": "string" }} }}, "required": ["option"] }} }},
{{ "name": "context_menu", "description": "Open the context menu of a node (right click) and choose an item by name; without item — only open it and see the items.",
  "inputSchema": {{ "type": "object", "properties": {{ {TARGET}, "item": {{ "type": "string" }} }} }} }},
{{ "name": "wait", "description": "Let the program work (background tasks) for up to ms milliseconds, or until a node with this name appears / data at a path has a value.",
  "inputSchema": {{ "type": "object", "properties": {{
    "ms": {{ "type": "integer" }},
    "until_name": {{ "type": "string" }},
    "until_path": {{ "type": "string" }},
    "until_value": {{}} }} }} }},
{{ "name": "read_data", "description": "The program's data model as JSON (what the screen is built from): the whole of it or at a dotted path. Useful for lists longer than the screen.",
  "inputSchema": {{ "type": "object", "properties": {{ "path": {{ "type": "string" }} }} }} }},
{{ "name": "list_actions", "description": "Names of the program's action handlers (for call_action).",
  "inputSchema": {{ "type": "object", "properties": {{}} }} }},
{{ "name": "call_action", "description": "Call a program action by name directly, bypassing the screen (no checks for disabled or hidden). Prefer click.",
  "inputSchema": {{ "type": "object", "properties": {{ "action": {{ "type": "string" }}, "key": {{ "type": "string" }} }}, "required": ["action"] }} }},
{{ "name": "resize", "description": "Change the window size in points (layouts differ: under 700 wide many programs switch to a phone layout).",
  "inputSchema": {{ "type": "object", "properties": {{ "w": {{ "type": "number" }}, "h": {{ "type": "number" }} }}, "required": ["w", "h"] }} }}
]"#
    );
    uison_json::parse(&src).map(|v| v.into_owned()).unwrap_or_else(|e| panic!("uison-agent: tools: {e}"))
}

pub struct Server {
    title: String,
    tools: Json,
}

impl Server {
    pub fn new(title: &str) -> Server {
        Server { title: title.to_string(), tools: tools() }
    }

    /// One JSON-RPC message → the answer, `None` for notifications.
    pub fn handle(&mut self, agent: &mut Agent, line: &str) -> Option<String> {
        let msg = match uison_json::parse(line) {
            Ok(v) => v.into_owned(),
            Err(e) => return Some(rpc_error(Json::Null, -32700, &format!("parse error: {e}"))),
        };
        let id = msg.get("id").cloned()?;
        let method = msg.get("method").and_then(Json::as_str).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or_else(Json::object);
        let result = match method {
            "initialize" => {
                let asked = params.get("protocolVersion").and_then(Json::as_str).unwrap_or("");
                let version = VERSIONS.iter().find(|v| **v == asked).unwrap_or(&VERSIONS[0]);
                let mut info = Json::object();
                info.set("name", format!("uison-agent: {}", self.title));
                info.set("version", env!("CARGO_PKG_VERSION"));
                let mut caps = Json::object();
                caps.set("tools", Json::object());
                let mut r = Json::object();
                r.set("protocolVersion", *version);
                r.set("capabilities", caps);
                r.set("serverInfo", info);
                r.set(
                    "instructions",
                    format!(
                        "The program \"{}\" without a screen. Call snapshot to see it as text, then act by #key: click, type, choose, set_value, scroll, context_menu. Every action answers with what changed on the screen.",
                        self.title
                    ),
                );
                r
            }
            "ping" => Json::object(),
            "tools/list" => {
                let mut r = Json::object();
                r.set("tools", self.tools.clone());
                r
            }
            "tools/call" => {
                let name = params.get("name").and_then(Json::as_str).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or_else(Json::object);
                let (text, error) = match call(agent, name, &args) {
                    Ok(t) => (t, false),
                    Err(e) => (e, true),
                };
                let mut item = Json::object();
                item.set("type", "text");
                item.set("text", text);
                let mut r = Json::object();
                r.set("content", vec![item]);
                r.set("isError", error);
                r
            }
            _ => return Some(rpc_error(id, -32601, &format!("unknown method '{method}'"))),
        };
        let mut out = Json::object();
        out.set("jsonrpc", "2.0");
        out.set("id", id);
        out.set("result", result);
        Some(out.to_json())
    }
}

fn rpc_error(id: Json, code: i32, msg: &str) -> String {
    let mut e = Json::object();
    e.set("code", code);
    e.set("message", msg.to_string());
    let mut out = Json::object();
    out.set("jsonrpc", "2.0");
    out.set("id", id);
    out.set("error", e);
    out.to_json()
}

/// A tool call → the agent command → text for the model.
fn call(agent: &mut Agent, tool: &str, args: &Json) -> Result<String, String> {
    let mut cmd = args.clone();
    if cmd.as_object().is_none() {
        cmd = Json::object();
    }
    let (q, act) = match tool {
        "snapshot" => {
            let tree = agent.tree();
            let rects = args.get("rects").and_then(Json::as_bool).unwrap_or(false);
            let v = view(&tree, rects, args.get("under").and_then(Json::as_str));
            agent.seen = tree;
            agent.seq += 1;
            return Ok(to_text(&v));
        }
        "read_data" => ("data", None),
        "list_actions" => ("actions", None),
        "click" => ("", Some("click")),
        "type" => ("", Some("type")),
        "press_key" => ("", Some("key")),
        "hover" => ("", Some("hover")),
        "scroll" => ("", Some("scroll")),
        "set_value" => ("", Some("set")),
        "choose" => {
            cmd.set("option", args.get("option").cloned().unwrap_or(Json::Null));
            ("", Some("pick"))
        }
        "context_menu" => ("", Some("menu")),
        "resize" => ("", Some("resize")),
        "wait" => {
            let mut until = Json::object();
            if let Some(n) = args.get("until_name") {
                until.set("name", n.clone());
            }
            if let Some(p) = args.get("until_path") {
                until.set("path", p.clone());
                if let Some(v) = args.get("until_value") {
                    until.set("value", v.clone());
                }
            }
            if until.as_object().is_some_and(|o| !o.is_empty()) {
                cmd.set("until", until);
            }
            ("", Some("wait"))
        }
        "call_action" => {
            let mut m = Json::object();
            m.set("action", args.get("action").cloned().unwrap_or(Json::Null));
            m.set("key", args.get("key").cloned().unwrap_or_else(|| Json::from("")));
            let before = flatten(&agent.seen).len();
            return agent.command(&m).map(|a| changes(agent, &a, before));
        }
        _ => return Err(format!("unknown tool '{tool}'")),
    };
    if let Some(a) = act {
        cmd.set("do", a);
        let before = flatten(&agent.seen).len();
        let answer = agent.command(&cmd)?;
        return Ok(changes(agent, &answer, before));
    }
    cmd.set("query", q);
    let a = agent.command(&cmd)?;
    Ok(match q {
        "data" => a.get("data").map_or_else(|| "null".into(), Json::to_json_pretty),
        _ => a.get("actions").and_then(Json::as_array).unwrap_or(&[]).iter().filter_map(Json::as_str).collect::<Vec<_>>().join("\n"),
    })
}

/// The answer to an action as text: what changed (or the whole screen if most of it did), and what
/// the program asked of the system.
fn changes(agent: &mut Agent, a: &Json, before: usize) -> String {
    let mut out = String::from("ok\n");
    match a.get("changes") {
        None => out.push_str("(nothing changed on screen)\n"),
        Some(d) => {
            let total = flatten(&agent.seen).len();
            // Not seen yet, or most of it is new: the whole screen in reading order reads better.
            if before == 0 || (diff_len(d) > WHOLE && diff_len(d) * 2 > total) || diff_len(d) * 3 > total * 2 {
                out.push_str("screen changed:\n");
                out.push_str(&to_text(&view(&agent.seen, false, None)));
            } else {
                out.push_str("changes (+ added, - removed, ~ changed):\n");
                out.push_str(&diff_text(d));
            }
        }
    }
    for n in a.get("host").and_then(Json::as_array).unwrap_or(&[]) {
        out.push_str("program asked the system: ");
        out.push_str(&n.to_json());
        out.push('\n');
    }
    out
}
