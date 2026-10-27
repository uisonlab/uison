//! What the screen compiler understands — for wrapper generators (`uison-gen`: C++ widgets).
//! The same tables `Ui` uses to check node properties, so the wrappers never drift from it.

use crate::expand;
pub use crate::library::BUILTINS;

/// Properties of every node and component: layout (`w`, `flex`…), `key`, `visible`, `on`, tooltip…
pub const COMMON: &[&str] = &expand::COMMON;

/// Behaviors of built-in nodes: `toggle`, `select`, `drag`, `interactive`…
pub const BEHAVIOR: &[&str] = &expand::BEHAVIOR;

/// A built-in node's own properties (without common ones and behaviors): `Text` → `text`, `size`, `bold`…
pub fn builtin_props(kind: &str) -> Option<&'static [&'static str]> {
    if kind == "For" {
        return Some(&expand::FOR);
    }
    expand::allowed(kind)
}

/// Standard library components: (file, source JSON), in file order.
pub fn standard_sources() -> &'static [(&'static str, &'static str)] {
    &crate::library::STD_SOURCES
}

/// Description of a built-in node's property or a common property — for the reference (`elements.md`),
/// C++ widget and Rust DSL comments. `kind` is the node: some properties mean something else there (`align` on `Text`).
pub fn doc(kind: &str, prop: &str) -> Option<&'static str> {
    let own = match (kind, prop) {
        ("Text", "align") | ("Flow", "align") => "line alignment: left, center, right",
        ("Text", "fill") => "fill behind the text (highlight), color",
        ("Text", "size") | ("Input", "size") => "font size, pt",
        ("Text", "color") | ("Input", "color") => "text color",
        ("Line", "color") | ("Vector", "color") => "line color",
        ("Line", "width") => "line width",
        ("Vector", "weight") => "icon line width",
        ("Wrap", "cross") => "alignment within a row: start, center, end",
        ("Image", "radius") => "image corner radius",
        ("For", "key") => "key of each repetition: \"row-{@t.id}\"",
        ("For", "index") | ("List", "index") => "item index name: index: \"i\" → @i",
        ("For", "as") | ("List", "as") => "item name inside the template: as: \"t\" → @t.title",
        ("Icon", "name") => "icon from the set (list below)",
        ("Avatar", "name") => "name: initials and circle color",
        ("Avatar", "src") => "photo instead of initials",
        ("Radio", "value") => "this button's value (selected when the group's value equals it)",
        ("Radio", "group") | ("Tab", "group") => "binding of the group's selected value: \"@plan\"",
        ("Tab", "value") => "tab value",
        ("Slider", "value") | ("ProgressBar", "value") => "value: number or binding \"@volume\"",
        ("Badge", "text") => "number or short text",
        ("Badge", "textColor") => "text color",
        ("Link", "href") => "address: opens in the browser",
        _ => "",
    };
    if !own.is_empty() {
        return Some(own);
    }
    Some(match prop {
        // Common
        "key" => "node name: events (cx.key), focus, hit areas, test scenarios",
        "visible" => "whether the node is shown: bool or binding \"@path\" / \"!@path\"",
        "disabled" => "disabled: grayed out, not clickable; bool or binding",
        "w" => "width: number (pt), \"fill\" — all available space, \"hug\" — fit content, \"50%\"",
        "h" => "height: number, \"fill\", \"hug\", \"50%\"",
        "minW" | "minH" => "minimum size, pt",
        "maxW" | "maxH" => "maximum size, pt",
        "flex" => "share of free space in Row/Column (1, 2…)",
        "margin" => "outer spacing: number, [vert, horiz] or [top, right, bottom, left]",
        "place" => "position in Stack: center, top, bottomRight…",
        "pos" => "position in Stack: [x, y]",
        "overlay" => "drawn above siblings, does not affect the parent's size (popups)",
        "on" => "events → actions: { click: \"add\", change: …, submit: …, hover: … }",
        "cursor" => "cursor over the node: arrow, pointer, text",
        "tooltip" => "tooltip on hover (long press)",
        "contextMenu" => "right-click menu: [{ label, action }, \"-\", …]",
        // Behaviors
        "toggle" => "a press toggles the bool at the path: \"@form.agree\"",
        "select" => "a press writes the value: { path: \"@tab\", value: \"inbox\" }",
        "drag" => "dragging sets a number: { path: \"@volume\", min, max, step }",
        "interactive" => "hover/pressed states for { state: … } selectors",
        "focusable" => "receives keyboard focus (Tab)",
        "dismiss" => "a press outside the node writes false to the path (closes a popup)",
        "href" => "link: a press opens the address in the browser",
        // Containers
        "padding" => "inner spacing: number, [vert, horiz] or [top, right, bottom, left]",
        "fill" => "fill: color \"#RRGGBB\" or token \"$theme.card\"",
        "stroke" => "border color",
        "width" => "border width",
        "radius" => "corner radius",
        "clip" => "clip children to own bounds",
        "align" => "child position: center, top, left, bottomRight…",
        "child" => "single child",
        "children" => "children",
        "gap" => "spacing between children",
        "main" => "distribution along the main axis: start, center, end, between, around, evenly",
        "cross" => "alignment along the cross axis: start, center, end, stretch",
        "stackBelow" => "narrower than this width — children stack in a column (phone)",
        "runGap" => "spacing between wrapped rows",
        "lineHeight" => "line spacing (multiplier)",
        // List
        "items" => "data array: \"@rows\"",
        "itemKey" => "row key from data: \"{@m.id}\"",
        "itemHeight" => "row height (virtualization)",
        "item" => "row template",
        "each" => "data array: \"@todos\"",
        "do" => "repeated template",
        // Text
        "text" => "text; {@path} — data substitution",
        "size" => "size",
        "bold" => "bold",
        "italic" => "italic",
        "color" => "color",
        "font" => "font: a name or monospace, serif, sans-serif",
        "maxLines" => "at most this many lines (then …)",
        "wrap" => "false — no line wrapping",
        "decoration" => "line: none, underline, lineThrough",
        // Input
        "value" => "value — binding \"@form.email\"",
        "placeholder" => "hint in an empty field",
        "placeholderColor" => "placeholder color",
        "caretColor" => "caret color",
        "password" => "hide characters",
        "keyboard" => "on-screen keyboard: text, email, number, decimal, phone, url, password",
        // Graphics
        "src" => "image: file path, \"mem:name\" (from bytes), in the browser also a URL",
        "fit" => "fit: cover, contain, fill",
        "at" => "position: [x, y]",
        "r" => "radius",
        "from" => "start: [x, y]",
        "to" => "end: [x, y]",
        "cap" => "line caps: round, butt",
        "points" => "vertices: [x1, y1, x2, y2, x3, y3]",
        "view" => "icon coordinate system: [w, h]",
        "weight" => "line width",
        "draw" => "the program's draw function: ui.draw(\"chart\", |d| …) — primitives into d.w",
        // Common parameters of the element library
        "icon" => "icon: a name from the Icon set (plus, trash, check…)",
        "label" => "label",
        "title" => "title",
        "subtitle" => "line below the title",
        "checked" => "checked: binding \"@form.agree\"",
        "options" => "options: [{ value, label }]",
        "tabs" => "tabs: [{ value, label }]",
        "min" => "minimum value",
        "max" => "maximum value",
        "step" => "step (0 — continuous)",
        "error" => "error text below the field (null — no error)",
        "leading" => "node on the left (icon, avatar)",
        "trailing" => "node on the right",
        "selected" => "selected",
        "direction" => "layout: column or row",
        "vertical" => "vertical",
        _ => return None,
    })
}
