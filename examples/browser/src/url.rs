//! Addresses: parsing `scheme://host:port/path` and resolving relative links against the page address.

/// A parsed address `http://host:port/path?query`.
#[derive(Debug, PartialEq)]
pub struct Url<'a> {
    pub scheme: &'a str,
    pub host: &'a str,
    pub port: u16,
    /// Path with the query, starts with `/`.
    pub path: &'a str,
}

pub fn parse(url: &str) -> Option<Url<'_>> {
    let (scheme, rest) = url.split_once("://")?;
    let (hostport, path) = match rest.find(['/', '?', '#']) {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let path = path.split('#').next().unwrap_or("/");
    let path = if path.is_empty() { "/" } else { path };
    let default = if scheme == "https" { 443 } else { 80 };
    let (host, port) = match hostport.rsplit_once(':') {
        Some((h, p)) if p.chars().all(|c| c.is_ascii_digit()) && !p.is_empty() => (h, p.parse().ok()?),
        _ => (hostport, default),
    };
    Some(Url { scheme, host, port, path })
}

/// Whether the link has its own scheme (`http:`, `about:`, `mailto:`).
fn has_scheme(href: &str) -> bool {
    match href.find(':') {
        Some(i) => i > 0 && href[..i].chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')),
        None => false,
    }
}

/// Link `href` on page `base` → full address.
pub fn resolve(base: &str, href: &str) -> String {
    let href = href.trim();
    if has_scheme(href) {
        return href.to_string();
    }
    let Some(b) = parse(base) else {
        // about:, file — no host: the link as is.
        return href.to_string();
    };
    let origin = if b.port == if b.scheme == "https" { 443 } else { 80 } {
        format!("{}://{}", b.scheme, b.host)
    } else {
        format!("{}://{}:{}", b.scheme, b.host, b.port)
    };
    if let Some(rest) = href.strip_prefix("//") {
        return format!("{}://{rest}", b.scheme);
    }
    let base_path = b.path.split('?').next().unwrap_or("/");
    let joined = if href.starts_with('/') {
        href.to_string()
    } else if href.starts_with('?') {
        format!("{base_path}{href}")
    } else if href.starts_with('#') || href.is_empty() {
        b.path.to_string()
    } else {
        let dir = &base_path[..base_path.rfind('/').map_or(0, |i| i + 1)];
        format!("{dir}{href}")
    };
    format!("{origin}{}", normalize(&joined))
}

/// Remove `.` and `..` from the path (the query is left alone).
fn normalize(path: &str) -> String {
    let (p, q) = path.split_once('?').map_or((path, None), |(p, q)| (p, Some(q)));
    let mut out: Vec<&str> = Vec::new();
    for seg in p.split('/').skip(1) {
        match seg {
            "." => {}
            ".." => {
                out.pop();
            }
            s => out.push(s),
        }
    }
    // `a/./` and `a/..` leave the path a directory.
    if p.ends_with("/.") || p.ends_with("/..") {
        out.push("");
    }
    let mut s = format!("/{}", out.join("/"));
    if let Some(q) = q {
        s.push('?');
        s.push_str(q);
    }
    s
}

/// Address for display: `%D0%97…` → "З…" (if the bytes form UTF-8 text).
pub fn display(url: &str) -> String {
    let b = url.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        match (b[i], b.get(i + 1).and_then(|&c| hex(c)), b.get(i + 2).and_then(|&c| hex(c))) {
            // Only non-ASCII bytes: `%2F`, `%20` etc. stay as they are.
            (b'%', Some(h), Some(l)) if h >= 8 => {
                out.push((h * 16 + l) as u8);
                i += 3;
            }
            (c, ..) => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| url.to_string())
}

/// Path for the request line: non-ASCII and spaces become `%XX`.
pub fn encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for &c in path.as_bytes() {
        if c > 0x7E || c <= b' ' {
            out.push_str(&format!("%{c:02X}"));
        } else {
            out.push(c as char);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_resolves() {
        assert_eq!(parse("http://info.cern.ch"), Some(Url { scheme: "http", host: "info.cern.ch", port: 80, path: "/" }));
        assert_eq!(parse("http://a.b:8080/x/y?q=1#top").map(|u| (u.port, u.path)), Some((8080, "/x/y?q=1")));
        let base = "http://info.cern.ch/hypertext/WWW/TheProject.html";
        assert_eq!(resolve(base, "Summary.html"), "http://info.cern.ch/hypertext/WWW/Summary.html");
        assert_eq!(resolve(base, "../DataSources/Top.html"), "http://info.cern.ch/hypertext/DataSources/Top.html");
        assert_eq!(resolve(base, "/x"), "http://info.cern.ch/x");
        assert_eq!(resolve(base, "//example.com/a"), "http://example.com/a");
        assert_eq!(resolve(base, "?q=2"), "http://info.cern.ch/hypertext/WWW/TheProject.html?q=2");
        assert_eq!(resolve(base, "about:home"), "about:home");
        assert_eq!(resolve("http://h:81/a/b", "c"), "http://h:81/a/c");
        assert_eq!(resolve("about:home", "about:css"), "about:css");
        // Cyrillic on purpose: percent-decoding of non-ASCII UTF-8.
        let wiki = "https://ru.wikipedia.org/wiki/%D0%97%D0%B0%D0%B3%2F x";
        assert_eq!(display(wiki), "https://ru.wikipedia.org/wiki/Заг%2F x");
        assert_eq!(encode_path("/wiki/Заг x"), "/wiki/%D0%97%D0%B0%D0%B3%20x");
    }
}
