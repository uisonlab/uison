//! The tree from `Ui::semantics`: finding nodes, changes between two snapshots, a compact text form.

use uison_ui::Json;

fn s<'a>(n: &'a Json, k: &str) -> Option<&'a str> {
    n.get(k).and_then(Json::as_str)
}

fn children(n: &Json) -> &[Json] {
    n.get("children").and_then(Json::as_array).unwrap_or(&[])
}

fn nodes(tree: &Json) -> &[Json] {
    tree.as_array().unwrap_or(&[])
}

/// Nodes in reading order (depth first).
pub fn flatten(tree: &Json) -> Vec<&Json> {
    fn walk<'a>(ns: &'a [Json], out: &mut Vec<&'a Json>) {
        for n in ns {
            out.push(n);
            walk(children(n), out);
        }
    }
    let mut out = Vec::new();
    walk(nodes(tree), &mut out);
    out
}

/// The node with `key` and its ancestors (nearest last).
pub fn find<'a>(tree: &'a Json, key: &str) -> Option<(&'a Json, Vec<&'a Json>)> {
    fn walk<'a>(ns: &'a [Json], key: &str, path: &mut Vec<&'a Json>) -> Option<&'a Json> {
        for n in ns {
            if s(n, "key") == Some(key) {
                return Some(n);
            }
            path.push(n);
            if let Some(f) = walk(children(n), key, path) {
                return Some(f);
            }
            path.pop();
        }
        None
    }
    let mut path = Vec::new();
    walk(nodes(tree), key, &mut path).map(|n| (n, path))
}

/// A node with a key by its name, best match first: the whole name (ignoring case), the start of it
/// ("Form" → "Form ⌘5"), a whole word in it, a part of it. Several equally good — an error listing
/// them, so the agent can pick a key.
pub fn by_name(tree: &Json, name: &str, role: Option<&str>) -> Result<String, String> {
    let want = name.to_lowercase();
    // A field is found by its placeholder too ("Search mail").
    let named = |n: &Json| s(n, "name").or_else(|| s(n, "placeholder")).map(str::to_lowercase);
    let word = |c: Option<char>| c.is_none_or(|c| !c.is_alphanumeric());
    let rank = |x: &str| -> Option<u8> {
        if x == want {
            return Some(0);
        }
        if x.starts_with(&want) && word(x[want.len()..].chars().next()) {
            return Some(1);
        }
        let mut best = None;
        for (i, _) in x.match_indices(want.as_str()) {
            let whole = word(x[..i].chars().next_back()) && word(x[i + want.len()..].chars().next());
            best = best.min(Some(if whole { 2 } else { 3 })).or(Some(if whole { 2 } else { 3 }));
        }
        best
    };
    let mut found: Vec<(u8, &Json)> = flatten(tree)
        .into_iter()
        .filter(|n| n.get("key").is_some() && role.is_none_or(|r| s(n, "role") == Some(r)))
        .filter_map(|n| Some((rank(&named(n)?)?, n)))
        .collect();
    let best = found.iter().map(|(r, _)| *r).min();
    found.retain(|(r, _)| Some(*r) == best);
    match found.as_slice() {
        [(_, n)] => Ok(s(n, "key").unwrap_or("").to_string()),
        [] => Err(format!("nothing named '{name}'")),
        many => {
            let keys: Vec<String> = many.iter().take(8).map(|(_, n)| format!("{} ({})", s(n, "key").unwrap_or(""), s(n, "name").unwrap_or(""))).collect();
            Err(format!("'{name}' matches {} nodes: {}", many.len(), keys.join(", ")))
        }
    }
}

/// The tree without `rect` (the agent doesn't need geometry), or only the subtree of `under`.
pub fn view(tree: &Json, rects: bool, under: Option<&str>) -> Json {
    fn clean(n: &Json, rects: bool) -> Json {
        let mut n = n.clone();
        if !rects {
            n.remove("rect");
        }
        if let Some(kids) = n.get("children").and_then(Json::as_array).map(|k| k.iter().map(|c| clean(c, rects)).collect::<Vec<_>>()) {
            n.set("children", kids);
        }
        n
    }
    let roots: Vec<Json> = match under.and_then(|k| find(tree, k)) {
        Some((n, _)) => vec![clean(n, rects)],
        None => nodes(tree).iter().map(|n| clean(n, rects)).collect(),
    };
    Json::Array(roots)
}

/// A node without children and geometry — what is compared between snapshots.
fn bare(n: &Json) -> Json {
    let mut n = n.clone();
    n.remove("children");
    n.remove("rect");
    n
}

/// Changes from `old` to `new`: nodes with a key are matched by key, texts by their content.
/// `{"added":[…],"removed":[…],"changed":[{"key":…,<new fields>}]}`, `None` — nothing changed.
pub fn diff(old: &Json, new: &Json) -> Option<Json> {
    let (o, n) = (flatten(old), flatten(new));
    let keyed = |v: &[&Json]| -> Vec<(String, Json)> { v.iter().filter_map(|x| Some((s(x, "key")?.to_string(), bare(x)))).collect() };
    let texts = |v: &[&Json]| -> Vec<Json> { v.iter().filter(|x| x.get("key").is_none()).map(|x| bare(x)).collect() };
    let (ok, nk) = (keyed(&o), keyed(&n));
    let (mut ot, mut nt) = (texts(&o), texts(&n));
    // Texts present in both (as many times as in both) are not changes.
    ot.retain(|t| match nt.iter().position(|x| x == t) {
        Some(i) => {
            nt.remove(i);
            false
        }
        None => true,
    });
    let mut added: Vec<Json> = Vec::new();
    let mut changed: Vec<Json> = Vec::new();
    for (k, node) in &nk {
        match ok.iter().find(|(ko, _)| ko == k) {
            None => added.push(node.clone()),
            Some((_, prev)) if prev != node => {
                let mut c = Json::object();
                c.set("key", k.clone());
                for (f, v) in node.as_object().unwrap_or(&[]) {
                    if prev.get(f) != Some(v) {
                        c.set(f, v.clone());
                    }
                }
                for (f, _) in prev.as_object().unwrap_or(&[]) {
                    if node.get(f).is_none() {
                        c.set(f, Json::Null);
                    }
                }
                changed.push(c);
            }
            _ => {}
        }
    }
    added.extend(nt);
    // A removed node is told by role, name and key: "- checkbox \"Buy bread\" #1/0".
    let short = |n: &Json| {
        let mut m = Json::object();
        for f in ["role", "name", "key"] {
            if let Some(v) = n.get(f) {
                m.set(f, v.clone());
            }
        }
        m
    };
    let mut removed: Vec<Json> = ok.iter().filter(|(k, _)| !nk.iter().any(|(kn, _)| kn == k)).map(|(_, n)| short(n)).collect();
    removed.extend(ot);
    if added.is_empty() && removed.is_empty() && changed.is_empty() {
        return None;
    }
    let mut d = Json::object();
    for (name, list) in [("added", added), ("removed", removed), ("changed", changed)] {
        if !list.is_empty() {
            d.set(name, list);
        }
    }
    Some(d)
}

/// Number of entries in a diff.
pub fn diff_len(d: &Json) -> usize {
    ["added", "removed", "changed"].iter().map(|k| d.get(k).and_then(Json::as_array).map_or(0, <[Json]>::len)).sum()
}

/// One line per node, indented by depth — fewer tokens than JSON for a model:
/// `checkbox "Buy bread" #1/0 checked`, `textfield "Email" #email value="" placeholder="you@…"`.
pub fn to_text(tree: &Json) -> String {
    let mut out = String::new();
    fn walk(ns: &[Json], depth: usize, out: &mut String) {
        for n in ns {
            line(n, depth, out);
            walk(children(n), depth + 1, out);
        }
    }
    walk(nodes(tree), 0, &mut out);
    out
}

/// A node as one line (without a newline at the end if `depth` is `usize::MAX`).
pub fn line(n: &Json, depth: usize, out: &mut String) {
    // `usize::MAX`: only the fields (a part of a change line).
    if depth != usize::MAX {
        for _ in 0..depth.min(32) {
            out.push_str("  ");
        }
        out.push_str(s(n, "role").unwrap_or("?"));
    }
    if let Some(name) = s(n, "name") {
        out.push(' ');
        quote(name, out);
    }
    if let Some(k) = s(n, "key") {
        out.push_str(" #");
        out.push_str(k);
    }
    for (f, v) in n.as_object().unwrap_or(&[]) {
        match (f.as_ref(), v) {
            ("role" | "name" | "key" | "children", _) => {}
            // A checkbox says both ways; the rest only when it's so.
            ("checked", Json::Bool(b)) => out.push_str(if *b { " checked" } else { " unchecked" }),
            ("expanded", Json::Bool(b)) => out.push_str(if *b { " expanded" } else { " collapsed" }),
            (_, Json::Bool(true)) => {
                out.push(' ');
                out.push_str(f);
            }
            (_, Json::Bool(false)) => {}
            ("rect", Json::Array(r)) => {
                out.push_str(" @");
                let parts: Vec<String> = r.iter().map(|x| num(x.as_f64().unwrap_or(0.0))).collect();
                out.push_str(&parts.join(","));
            }
            (_, Json::Array(items)) => {
                out.push(' ');
                out.push_str(f);
                out.push_str("=[");
                for (i, it) in items.iter().enumerate() {
                    if i > 0 {
                        out.push('|');
                    }
                    out.push_str(it.as_str().map_or_else(|| it.to_json(), str::to_string).as_str());
                }
                out.push(']');
            }
            (_, Json::String(t)) => {
                out.push(' ');
                out.push_str(f);
                out.push('=');
                quote(t, out);
            }
            (_, Json::Number(x)) => {
                out.push(' ');
                out.push_str(f);
                out.push('=');
                out.push_str(&num(*x));
            }
            (_, Json::Null) => {
                out.push(' ');
                out.push_str(f);
                out.push_str("=none");
            }
            (_, other) => {
                out.push(' ');
                out.push_str(f);
                out.push('=');
                out.push_str(&other.to_json());
            }
        }
    }
    out.push('\n');
}

fn quote(t: &str, out: &mut String) {
    uison_json::write_str(out, t);
}

fn num(x: f64) -> String {
    if x.fract() == 0.0 && x.abs() < 1e15 { (x as i64).to_string() } else { x.to_string() }
}

/// States shown as a word when true; a change to false or absent reads "not <flag>".
const FLAGS: [&str; 8] = ["selected", "focused", "disabled", "offscreen", "checked", "expanded", "menu", "password"];

/// A diff as text: `+ …` added, `- …` removed, `~ #key field=new` changed.
pub fn diff_text(d: &Json) -> String {
    let mut out = String::new();
    for n in d.get("added").and_then(Json::as_array).unwrap_or(&[]) {
        out.push_str("+ ");
        line(n, 0, &mut out);
    }
    for n in d.get("removed").and_then(Json::as_array).unwrap_or(&[]) {
        out.push_str("- ");
        line(n, 0, &mut out);
    }
    for n in d.get("changed").and_then(Json::as_array).unwrap_or(&[]) {
        out.push_str("~ ");
        out.push('#');
        out.push_str(s(n, "key").unwrap_or(""));
        // A change says both ways: "not selected", "not focused", "name=none".
        for (f, v) in n.as_object().unwrap_or(&[]) {
            match (f.as_ref(), v) {
                ("key", _) => continue,
                ("checked", Json::Bool(false)) => out.push_str(" unchecked"),
                ("expanded", Json::Bool(false)) => out.push_str(" collapsed"),
                (_, Json::Bool(false)) | (_, Json::Null) if FLAGS.contains(&f.as_ref()) => {
                    out.push_str(" not ");
                    out.push_str(f);
                }
                _ => {
                    let mut one = Json::object();
                    one.set(f, v.clone());
                    let mut part = String::new();
                    line(&one, usize::MAX, &mut part);
                    out.push_str(part.trim_end_matches('\n'));
                }
            }
        }
        out.push('\n');
    }
    out
}
