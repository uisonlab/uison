use std::fmt::Write as _;

use crate::Value;

/// Append a quoted, escaped string.
pub fn write_str(out: &mut String, s: &str) {
    out.push('"');
    let b = s.as_bytes();
    let mut run = 0;
    for (i, &c) in b.iter().enumerate() {
        let esc: &str = match c {
            b'"' => "\\\"",
            b'\\' => "\\\\",
            b'\n' => "\\n",
            b'\r' => "\\r",
            b'\t' => "\\t",
            0..=0x1F => "",
            _ => continue,
        };
        out.push_str(&s[run..i]);
        if esc.is_empty() {
            const HEX: &[u8; 16] = b"0123456789abcdef";
            out.push_str("\\u00");
            out.push(HEX[(c >> 4) as usize] as char);
            out.push(HEX[(c & 15) as usize] as char);
        } else {
            out.push_str(esc);
        }
        run = i + 1;
    }
    out.push_str(&s[run..]);
    out.push('"');
}

impl Value<'_> {
    /// Compact standard JSON.
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        write_value(&mut out, self, None, 0);
        out
    }

    /// Standard JSON indented by 2 spaces.
    pub fn to_json_pretty(&self) -> String {
        let mut out = String::new();
        write_value(&mut out, self, Some(2), 0);
        out
    }
}

fn write_value(out: &mut String, v: &Value, indent: Option<usize>, level: usize) {
    let newline = |out: &mut String, level: usize| {
        if let Some(n) = indent {
            out.push('\n');
            out.extend(std::iter::repeat_n(' ', n * level));
        }
    };
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => write_number(out, *n),
        Value::String(s) => write_str(out, s),
        Value::Array(a) if a.is_empty() => out.push_str("[]"),
        Value::Object(o) if o.is_empty() => out.push_str("{}"),
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                newline(out, level + 1);
                write_value(out, x, indent, level + 1);
            }
            newline(out, level);
            out.push(']');
        }
        Value::Object(o) => {
            out.push('{');
            for (i, (k, x)) in o.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                newline(out, level + 1);
                write_str(out, k);
                out.push_str(if indent.is_some() { ": " } else { ":" });
                write_value(out, x, indent, level + 1);
            }
            newline(out, level);
            out.push('}');
        }
    }
}

fn write_number(out: &mut String, n: f64) {
    if !n.is_finite() {
        out.push_str("null");
    } else if n.fract() == 0.0 && n.abs() < 1e15 {
        let _ = write!(out, "{}", n as i64);
    } else {
        let _ = write!(out, "{n}");
    }
}
