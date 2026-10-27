//! The page's JSON in the same style people write uison screens by hand: unquoted keys,
//! short objects and arrays on one line, long ones on indented lines.

use uison::ui::Json;

const WIDTH: usize = 100;

pub fn pretty(v: &Json) -> String {
    let mut out = String::new();
    write(v, 0, 0, &mut out);
    out
}

/// `level` — nesting (line indent), `col` — where the value starts on the line.
fn write(v: &Json, level: usize, col: usize, out: &mut String) {
    let one = inline(v);
    if col + one.chars().count() <= WIDTH || !matches!(v, Json::Object(_) | Json::Array(_)) {
        out.push_str(&one);
        return;
    }
    // A node `{ Kind: { … } }` — the wrapper stays on the same line, no extra level.
    if let Json::Object(o) = v
        && let [(k, inner @ Json::Object(_))] = &o[..]
    {
        out.push_str("{ ");
        let start = out.len();
        key(k, out);
        out.push_str(": ");
        let used = out.len() - start;
        write(inner, level, col + 2 + used, out);
        out.push_str(" }");
        return;
    }
    let pad = "  ".repeat(level + 1);
    let (open, close) = if matches!(v, Json::Object(_)) { ('{', '}') } else { ('[', ']') };
    out.push(open);
    out.push('\n');
    match v {
        Json::Object(o) => {
            for (k, x) in o.iter() {
                out.push_str(&pad);
                let start = out.len();
                key(k, out);
                out.push_str(": ");
                let used = out.len() - start;
                write(x, level + 1, pad.len() + used, out);
                out.push_str(",\n");
            }
        }
        Json::Array(a) => {
            for x in a {
                out.push_str(&pad);
                write(x, level + 1, pad.len(), out);
                out.push_str(",\n");
            }
        }
        _ => {}
    }
    out.push_str(&"  ".repeat(level));
    out.push(close);
}

fn inline(v: &Json) -> String {
    match v {
        Json::Object(o) if o.is_empty() => "{}".into(),
        Json::Object(o) => {
            let parts: Vec<String> = o
                .iter()
                .map(|(k, x)| {
                    let mut s = String::new();
                    key(k, &mut s);
                    s.push_str(": ");
                    s.push_str(&inline(x));
                    s
                })
                .collect();
            format!("{{ {} }}", parts.join(", "))
        }
        Json::Array(a) => format!("[{}]", a.iter().map(inline).collect::<Vec<_>>().join(", ")),
        x => x.to_json(),
    }
}

fn key(k: &str, out: &mut String) {
    let ident = k.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_') && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if ident {
        out.push_str(k);
    } else {
        out.push_str(&Json::from(k.to_string()).to_json());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_inline_long_split_and_parses_back() {
        let src = r#"{"Column":{"padding":[0,8,0,8],"children":[{"Text":{"text":"Hello","size":16}},{"Text":{"text":"a much longer piece of text that does not fit on one line with the rest","bold":true}}]}}"#;
        let v = uison::json::parse(src).unwrap().into_owned();
        let p = pretty(&v);
        assert!(p.contains(r#"{ Text: { text: "Hello", size: 16 } },"#), "{p}");
        assert!(p.contains("padding: [0, 8, 0, 8],"), "{p}");
        // uison's extended JSON reads back into the same tree.
        assert_eq!(uison::json::parse(&p).unwrap().into_owned(), v);
    }
}
