//! C++ widgets (`uison_widgets.hpp`) from the screen schema: built-in nodes and components as
//! Flutter-style functions taking property structs (C++20 designated initializers):
//!
//! ```cpp
//! Column({.gap = 12, .children = {Text("Hello", {.size = 20}), Button("+1", {.on_click = inc})}})
//! ```
//!
//! `uison-gen` — standard widgets (built-in nodes and `lib/*.json`) → `crates/uison-c/include/uison_widgets.hpp`;
//! `uison-gen [--namespace app] components.json…` — your own components into a separate header.
//! `uison-gen --rust` — the Rust DSL (`crates/uison-ui/src/widgets.rs`, module [`rust`]).
//! `uison-gen --md` — the element reference (`docs/screens/elements.md`, module [`md`]).
//!
//! As a library — for `build.rs`: [`standard_header`], [`components_header`], [`rust::standard`], [`rust::components`].

pub mod md;
pub mod rust;

use uison_json::Value;
use uison_ui::schema;

pub(crate) type Json = Value<'static>;

/// Library components that `Ui` draws itself (tooltip, menu): not exposed as widgets.
pub(crate) const INTERNAL: [&str; 2] = ["Tooltip", "ContextMenu"];

/// The main property goes first, unnamed: `Text("Hello")`, `Button("+1", {…})`.
pub(crate) const POSITIONAL: [(&str, &str); 11] = [
    ("Text", "text"),
    ("Button", "text"),
    ("Icon", "name"),
    ("IconButton", "icon"),
    ("Badge", "text"),
    ("Link", "text"),
    ("Avatar", "name"),
    ("Image", "src"),
    ("ListTile", "title"),
    ("Checkbox", "label"),
    ("Switch", "label"),
];

/// Input elements of the standard library: they have `on_change`.
const CHANGE: [&str; 9] = ["Input", "Checkbox", "Radio", "RadioGroup", "Switch", "Slider", "Select", "Tabs", "TextField"];

/// Built-in containers: they get the behavior fields (`toggle`, `select`, `drag`…); custom elements are made from them.
const CONTAINERS: [&str; 5] = ["Box", "Row", "Column", "Stack", "Wrap"];

/// Node-valued properties written last (like `child:` in Flutter); `do` of `For` becomes `item`.
const SLOTS: [&str; 4] = ["child", "item", "do", "children"];

const KEYWORDS: [&str; 30] = [
    "auto", "bool", "break", "case", "catch", "char", "class", "const", "default", "delete", "do", "double", "else",
    "enum", "explicit", "float", "for", "if", "int", "long", "namespace", "new", "operator", "private", "public",
    "register", "return", "switch", "template", "this",
];

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Kind {
    Value,
    Children,
}

pub(crate) struct Field {
    pub json: String,
    pub cpp: String,
    pub kind: Kind,
    pub doc: Option<String>,
}

/// Widget: node name; fields, events and slots (child, children) — in this order.
pub(crate) struct Widget {
    pub name: String,
    pub fields: Vec<Field>,
    pub events: Vec<&'static str>,
    pub slots: Vec<Field>,
}

/// `maxLines` → `max_lines`; `do` → `item`; a C++ keyword gets a trailing `_`.
fn cpp_name(json: &str) -> String {
    if json == "do" {
        return "item".into();
    }
    let mut s = String::new();
    for c in json.chars() {
        if c.is_ascii_uppercase() {
            s.push('_');
            s.push(c.to_ascii_lowercase());
        } else {
            s.push(c);
        }
    }
    if KEYWORDS.contains(&s.as_str()) {
        s.push('_');
    }
    s
}

fn field(json: &str, doc: Option<String>) -> Field {
    let kind = if json == "children" { Kind::Children } else { Kind::Value };
    Field { json: json.into(), cpp: cpp_name(json), kind, doc }
}

/// A parameter's default value — as a comment on the field.
fn default_doc(v: &Json) -> Option<String> {
    match v {
        Value::Null => None,
        Value::Array(a) if a.is_empty() => None,
        Value::Object(_) => Some("default computed (variant, state)".into()),
        v => Some(format!("default {}", v.to_json())),
    }
}

/// Layout properties from the common ones (`schema::COMMON`) come first, the other common ones after them.
const LAYOUT: [&str; 11] = ["w", "h", "minW", "maxW", "minH", "maxH", "flex", "margin", "place", "pos", "overlay"];

/// Widget fields: own → sizes → common → behaviors (built-ins) → events → slots (child, children).
fn widget(name: &str, own: &[(String, Option<String>)], builtin: bool, std: bool) -> Widget {
    let mut fields: Vec<Field> = Vec::new();
    let mut slots: Vec<Field> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let mut add = |f: Field, to_slots: bool, fields: &mut Vec<Field>, slots: &mut Vec<Field>| {
        if seen.contains(&f.json) || f.json == "on" {
            return;
        }
        seen.push(f.json.clone());
        if to_slots { slots.push(f) } else { fields.push(f) }
    };
    for (p, doc) in own {
        let doc = doc.clone().or_else(|| schema::doc(name, p).map(String::from));
        add(field(p, doc), SLOTS.contains(&p.as_str()), &mut fields, &mut slots);
    }
    let is_for = name == "For";
    if !is_for {
        let common = schema::COMMON.iter().filter(|p| LAYOUT.contains(p)).chain(schema::COMMON.iter().filter(|p| !LAYOUT.contains(p)));
        for p in common {
            add(field(p, schema::doc(name, p).map(String::from)), false, &mut fields, &mut slots);
        }
        // Behaviors (toggle, select, drag…) — on containers; on the rest — via a .prop() chain.
        if builtin && CONTAINERS.contains(&name) {
            for p in schema::BEHAVIOR {
                add(field(p, schema::doc(name, p).map(String::from)), false, &mut fields, &mut slots);
            }
        }
    }
    // `For` — only its own properties (key is its own, not the common one).
    if is_for {
        let key = fields.iter().position(|f| f.json == "key");
        if let Some(i) = key {
            let k = fields.remove(i);
            fields.push(k);
        }
    }
    // Events: click — on all; change and submit — on input elements; custom components — all three
    // (the component's body decides which of them actually arrive).
    let mut events = Vec::new();
    if !is_for {
        events.push("click");
        if !std || CHANGE.contains(&name) {
            events.push("change");
        }
        if !std || matches!(name, "TextField" | "Input") {
            events.push("submit");
        }
        if name == "Canvas" {
            events.push("hover");
        }
    }
    Widget { name: name.into(), fields, events, slots }
}

pub(crate) fn standard() -> Vec<Widget> {
    let mut out = Vec::new();
    for kind in schema::BUILTINS {
        let props = schema::builtin_props(kind).unwrap_or_else(|| panic!("{kind}: no schema"));
        let own: Vec<_> = props.iter().map(|p| (p.to_string(), None)).collect();
        out.push(widget(kind, &own, true, true));
    }
    for (file, src) in schema::standard_sources() {
        let w = components(src, true).unwrap_or_else(|e| panic!("{file}:{e}"));
        out.extend(w.into_iter().filter(|w| !INTERNAL.contains(&w.name.as_str())));
    }
    out
}

/// Parameter comments in the components' source text: `variant: "primary", // primary | secondary`
/// or a `// …` line above the parameter. (component, parameter) → text.
pub(crate) fn param_comments(src: &str) -> Vec<(String, String, String)> {
    let mut out = Vec::new();
    let (mut depth, mut comp, mut params_depth) = (0i32, String::new(), 0i32);
    let mut above: Vec<String> = Vec::new();
    for line in src.lines() {
        let t = line.trim();
        if let Some(c) = t.strip_prefix("//") {
            above.push(c.trim().to_string());
            continue;
        }
        // The name before the colon (unquoted key) and the comment at the end of the line.
        let key = t.split(':').next().filter(|k| !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
        let tail = code_comment(t);
        if let Some(k) = key {
            if depth == 1 {
                comp = k.to_string();
                params_depth = 0;
            } else if depth == 2 && k == "params" {
                params_depth = 3;
            } else if params_depth > 0 && depth == params_depth {
                let doc = tail.map(str::to_string).or_else(|| (!above.is_empty()).then(|| above.join(" ")));
                if let Some(d) = doc {
                    out.push((comp.clone(), k.to_string(), d));
                }
            }
        }
        above.clear();
        // Bracket depth outside strings and comments.
        let (mut in_str, mut prev) = (false, ' ');
        for c in t.chars() {
            if in_str {
                if c == '"' && prev != '\\' {
                    in_str = false;
                }
            } else if c == '"' {
                in_str = true;
            } else if c == '/' && prev == '/' {
                break;
            } else if c == '{' {
                depth += 1;
            } else if c == '}' {
                depth -= 1;
                if params_depth > 0 && depth < params_depth {
                    params_depth = 0;
                }
            }
            prev = c;
        }
    }
    out
}

/// A `// …` comment at the end of a code line (not inside a JSON string).
fn code_comment(t: &str) -> Option<&str> {
    let (mut in_str, mut prev) = (false, ' ');
    for (i, c) in t.char_indices() {
        if in_str {
            if c == '"' && prev != '\\' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
        } else if c == '/' && prev == '/' {
            return Some(t[i + 1..].trim()).filter(|s| !s.is_empty());
        }
        prev = c;
    }
    None
}

/// Components from the text `{ Name: { params: {…}, body } }` — in file order; a parameter's description is
/// its comment in the text plus its default value.
pub(crate) fn components(src: &str, std: bool) -> Result<Vec<Widget>, uison_json::Error> {
    let defs = uison_json::parse(src)?.into_owned();
    let notes = param_comments(src);
    let mut out = Vec::new();
    for (name, def) in defs.as_object().unwrap_or(&[]) {
        let params = def.get("params").and_then(Json::as_object).unwrap_or(&[]);
        let own: Vec<_> = params
            .iter()
            .map(|(p, v)| {
                let note = notes
                    .iter()
                    .find(|(c, k, _)| c == &**name && k == &**p)
                    .map(|(_, _, d)| d.trim_end_matches('.').to_string())
                    .or_else(|| schema::doc(name, p).map(String::from));
                let doc = match (note, default_doc(v)) {
                    (Some(n), Some(d)) => Some(format!("{n}; {d}")),
                    (n, d) => n.or(d),
                };
                (p.to_string(), doc)
            })
            .collect();
        out.push(widget(name, &own, false, std));
    }
    Ok(out)
}

fn emit(out: &mut String, w: &Widget) {
    let props = format!("{}Props", w.name);
    out.push_str(&format!("struct {props} {{\n"));
    let field_line = |out: &mut String, f: &Field| {
        let ty = match f.kind {
            Kind::Value => "Prop",
            Kind::Children => "std::vector<Node>",
        };
        out.push_str(&format!("    {ty} {} = {{}};", f.cpp));
        if let Some(d) = &f.doc {
            out.push_str(&format!(" ///< {d}"));
        }
        out.push('\n');
    };
    for f in &w.fields {
        field_line(out, f);
    }
    for e in &w.events {
        out.push_str(&format!("    Action on_{e} = {{}};\n"));
    }
    for f in &w.slots {
        field_line(out, f);
    }
    out.push_str("};\n");
    // Value properties — as a table (field pointer, JSON name) and a shared loop: the code doesn't grow with the number of fields.
    let values: Vec<&Field> = w.fields.iter().chain(&w.slots).filter(|f| f.kind == Kind::Value).collect();
    out.push_str(&format!("inline UISON_NOINLINE Node {}({props} p) {{\n", w.name));
    // Comma-separated list, lines up to ~110 characters.
    let list = |f: &dyn Fn(&Field) -> String| {
        let mut s = String::new();
        let mut line = 0;
        for (i, v) in values.iter().enumerate() {
            let item = f(v);
            if i > 0 {
                s.push(',');
                if line + item.len() > 100 {
                    s.push_str("\n        ");
                    line = 0;
                } else {
                    s.push(' ');
                }
            }
            line += item.len() + 2;
            s.push_str(&item);
        }
        s
    };
    out.push_str(&format!("    static constexpr Prop {props}::*f[] = {{{}}};\n", list(&|v| format!("&{props}::{}", v.cpp))));
    out.push_str(&format!("    static constexpr const char* k[] = {{{}}};\n", list(&|v| format!("\"{}\"", v.json))));
    out.push_str(&format!("    Node n = uison::detail::widget(\"{}\", p, f, k);\n", w.name));
    for f in w.fields.iter().chain(&w.slots).filter(|f| f.kind == Kind::Children) {
        out.push_str(&format!("    if (!p.{0}.empty()) n.children(std::move(p.{0}));\n", f.cpp));
    }
    for e in &w.events {
        out.push_str(&format!("    p.on_{e}.apply(n, \"{e}\");\n"));
    }
    out.push_str("    return n;\n}\n");
    if let Some((_, main)) = POSITIONAL.iter().find(|(n, _)| *n == w.name)
        && let Some(f) = w.fields.iter().find(|f| f.json == *main)
    {
        // A template, not `Prop`: a braced list can't be deduced into it, so `Checkbox({.checked = …})` doesn't
        // become ambiguous (otherwise GCC 11–12 tries it for the main argument too).
        out.push_str(&format!(
            "template <class T, std::enable_if_t<std::is_constructible_v<Prop, T&&>, int> = 0>\n\
             inline Node {0}(T&& {1}, {props} p = {{}}) {{\n    p.{1} = Prop(std::forward<T>({1}));\n    return {0}(std::move(p));\n}}\n",
            w.name, f.cpp
        ));
    }
    out.push('\n');
}

/// The whole header: `guard` — the include guard, `include` — what to include.
fn header(widgets: &[Widget], ns: &str, guard: &str, include: &str, what: &str) -> String {
    let mut out = format!(
        "// {what}: Flutter-style C++ widgets — functions taking property structs (C++20).\n\
         // Generated by uison-gen (crates/uison-gen) — don't edit by hand.\n\
         //\n\
         //   Column({{.gap = 12, .children = {{Text(\"Hello\", {{.size = 20}}), Button(\"+1\", {{.on_click = inc}})}}}})\n\
         //\n\
         // Fields go in declaration order (otherwise GCC and MSVC won't compile it): own properties → sizes\n\
         // (w, h, flex, margin…) → common (key, visible, disabled, tooltip…) → events → child/children.\n\
         // Any other property — via a chain: Text(\"…\").prop(\"name\", value).\n\
         #ifndef {guard}\n#define {guard}\n\n#include {include}\n\n\
         #if (defined(_MSVC_LANG) ? _MSVC_LANG : __cplusplus) < 202002L\n\
         #error \"uison widgets need C++20 (designated initializers)\"\n#endif\n\n\
         namespace {ns} {{\n\n"
    );
    if ns != "uison" && !ns.starts_with("uison::") {
        out.push_str("using uison::Action;\nusing uison::Node;\nusing uison::Prop;\n\n");
    }
    for w in widgets {
        emit(&mut out, w);
    }
    out.push_str(&format!("}} // namespace {ns}\n\n#endif // {guard}\n"));
    out
}

/// The standard `uison_widgets.hpp`.
pub fn standard_header() -> String {
    header(&standard(), "uison::widgets", "UISON_WIDGETS_HPP", "\"uison.hpp\"", "uison")
}

/// Widgets for your own components (`components.json` texts: `{ Name: { params, body } }`) in the
/// namespace `ns`; `what` — where they come from (for the header comment).
pub fn components_header(srcs: &[&str], ns: &str, what: &str) -> Result<String, uison_json::Error> {
    let mut widgets = Vec::new();
    for s in srcs {
        widgets.extend(components(s, false)?);
    }
    let guard = format!("{}_WIDGETS_HPP", ns.replace("::", "_").to_uppercase());
    Ok(header(&widgets, ns, &guard, "<uison_widgets.hpp>", what))
}

#[cfg(test)]
mod tests {
    /// The header in the repository matches the generator's output (the schema hasn't drifted).
    #[test]
    fn widgets_header_up_to_date() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../uison-c/include/uison_widgets.hpp");
        let now = super::standard_header();
        let saved = std::fs::read_to_string(path).unwrap_or_default();
        assert!(saved == now, "uison_widgets.hpp is stale: cargo run -p uison-gen > crates/uison-c/include/uison_widgets.hpp");
    }

    /// The Rust DSL in the repository matches the generator's output.
    #[test]
    fn rust_widgets_up_to_date() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../uison-ui/src/widgets.rs");
        let now = super::rust::standard();
        let saved = std::fs::read_to_string(path).unwrap_or_default();
        assert!(saved == now, "widgets.rs is stale: cargo run -p uison-gen -- --rust -o crates/uison-ui/src/widgets.rs");
    }

    /// The element reference matches the generator's output.
    #[test]
    fn elements_md_up_to_date() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/screens/elements.md");
        let now = super::md::elements();
        let saved = std::fs::read_to_string(path).unwrap_or_default();
        assert!(saved == now, "elements.md is stale: cargo run -p uison-gen -- --md -o docs/screens/elements.md");
    }
}
