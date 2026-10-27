//! Extended JSON (unquoted keys, trailing commas, comments, any whitespace) → strict JSON on one line.
//! Strings and numbers are copied as written: no float formatting, no re-escaping. The input is
//! assumed valid (a frame from `Writer`, a parsed file); garbage in gives garbage out, not a panic.

/// Strict, single-line JSON of `src`: `{size:[1, 2],}` → `{"size":[1,2]}`.
pub fn compact(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    // A comma is written only before the next value: a trailing one before `]` or `}` disappears.
    let mut comma = false;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b' ' | b'\t' | b'\n' | b'\r' => i += 1,
            b'/' if matches!(b.get(i + 1), Some(b'/' | b'*')) => i = skip_comment(b, i),
            b',' => {
                comma = true;
                i += 1;
            }
            c @ (b']' | b'}') => {
                comma = false;
                out.push(c as char);
                i += 1;
            }
            b'"' => {
                put_comma(&mut out, &mut comma);
                let end = string_end(b, i);
                out.push_str(&src[i..end]);
                i = end;
            }
            c if c.is_ascii_alphabetic() || c == b'_' || c == b'$' => {
                put_comma(&mut out, &mut comma);
                let start = i;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'$') {
                    i += 1;
                }
                // A bare word before `:` is a key; otherwise true, false, null.
                if b.get(skip_space(b, i)) == Some(&b':') {
                    out.push('"');
                    out.push_str(&src[start..i]);
                    out.push('"');
                } else {
                    out.push_str(&src[start..i]);
                }
            }
            _ => {
                // Numbers, `:`, `{`, `[`: a number runs to the next delimiter and is copied whole.
                let c = b[i];
                if c != b':' {
                    put_comma(&mut out, &mut comma);
                }
                let start = i;
                i += 1;
                if !matches!(c, b':' | b'{' | b'[') {
                    while i < b.len() && !matches!(b[i], b',' | b']' | b'}' | b' ' | b'\t' | b'\n' | b'\r' | b'/' | b':') {
                        i += 1;
                    }
                }
                out.push_str(&src[start..i]);
            }
        }
    }
    out
}

fn put_comma(out: &mut String, comma: &mut bool) {
    if std::mem::take(comma) {
        out.push(',');
    }
}

/// Index after the closing quote of the string starting at `i`.
fn string_end(b: &[u8], mut i: usize) -> usize {
    i += 1;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'"' => return i + 1,
            _ => i += 1,
        }
    }
    b.len()
}

fn skip_comment(b: &[u8], i: usize) -> usize {
    if b[i + 1] == b'/' {
        b[i..].iter().position(|&c| c == b'\n').map_or(b.len(), |n| i + n + 1)
    } else {
        b[i + 2..].windows(2).position(|w| w == b"*/").map_or(b.len(), |n| i + 2 + n + 2)
    }
}

fn skip_space(b: &[u8], mut i: usize) -> usize {
    loop {
        match b.get(i) {
            Some(b' ' | b'\t' | b'\n' | b'\r') => i += 1,
            Some(b'/') if matches!(b.get(i + 1), Some(b'/' | b'*')) => i = skip_comment(b, i),
            _ => return i,
        }
    }
}
