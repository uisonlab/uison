//! Error-tolerant HTML parsing, as in browsers: void elements, implicit closing
//! (`<p>` until the next block, `<li>` until the next `<li>`), raw text of `script`/`style`,
//! entities. The tree lives in an arena: a node knows its parent (CSS selectors need it).

/// Document: node 0 is the root.
pub struct Dom {
    pub nodes: Vec<Node>,
    /// For an element: index among element siblings, their count, the previous element sibling —
    /// computed once (`:nth-child`, `+`, `~` selectors on long lists).
    pub sib: Vec<Sib>,
}

#[derive(Clone, Copy, Default)]
pub struct Sib {
    pub index: u32,
    pub count: u32,
    pub prev: Option<u32>,
}

pub struct Node {
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub data: Data,
}

pub enum Data {
    Root,
    Element { tag: String, attrs: Vec<(String, String)> },
    Text(String),
}

impl Node {
    pub fn tag(&self) -> Option<&str> {
        match &self.data {
            Data::Element { tag, .. } => Some(tag),
            _ => None,
        }
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        match &self.data {
            Data::Element { attrs, .. } => attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str()),
            _ => None,
        }
    }
}

/// Elements without content or a closing tag.
const VOID: [&str; 14] = ["area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source", "track", "wbr"];

/// Opening these elements closes an open `<p>`.
const CLOSES_P: [&str; 30] = [
    "address", "article", "aside", "blockquote", "center", "details", "dialog", "dir", "div", "dl", "fieldset", "figcaption", "figure",
    "footer", "form", "h1", "h2", "h3", "h4", "h5", "h6", "header", "hr", "main", "menu", "nav", "ol", "p", "pre", "section",
];

/// Content is raw text up to its own closing tag.
const RAW: [&str; 5] = ["script", "style", "textarea", "title", "xmp"];

impl Dom {
    pub fn parse(src: &str) -> Dom {
        let mut b = Builder { dom: Dom { nodes: vec![Node { parent: None, children: Vec::new(), data: Data::Root }], sib: Vec::new() }, stack: vec![0] };
        let bytes = src.as_bytes();
        let mut i = 0;
        let mut text_start = 0;
        while i < bytes.len() {
            if bytes[i] != b'<' {
                i += 1;
                continue;
            }
            let rest = &src[i..];
            let next = bytes.get(i + 1).copied().unwrap_or(0);
            // Comment, <!doctype>, <?xml?>.
            let skip_to = if let Some(body) = rest.strip_prefix("<!--") {
                Some(body.find("-->").map_or(src.len(), |e| i + 4 + e + 3))
            } else if next == b'!' || next == b'?' {
                Some(rest.find('>').map_or(src.len(), |e| i + e + 1))
            } else {
                None
            };
            if let Some(end) = skip_to {
                b.text(&src[text_start..i]);
                (i, text_start) = (end, end);
                continue;
            }
            let closing = next == b'/';
            let name_at = i + 1 + closing as usize;
            if !bytes.get(name_at).is_some_and(u8::is_ascii_alphabetic) {
                i += 1; // `<` in text: "a < b"
                continue;
            }
            b.text(&src[text_start..i]);
            let (tag, attrs, self_closing, end) = read_tag(src, name_at);
            i = end;
            if closing {
                b.close(&tag);
            } else {
                let raw = RAW.contains(&tag.as_str());
                let void = VOID.contains(&tag.as_str());
                b.open(tag.clone(), attrs);
                if raw {
                    // Raw text up to </tag (case-insensitive, without copying the rest of the document).
                    let e = find_close(&src[i..], &tag).map_or(src.len(), |e| i + e);
                    let body = &src[i..e];
                    if !body.is_empty() {
                        let text = if tag == "script" || tag == "style" { body.to_string() } else { decode(body) };
                        b.push(Data::Text(text));
                    }
                    b.close(&tag);
                    i = src[e..].find('>').map_or(src.len(), |g| e + g + 1);
                } else if void || self_closing {
                    b.close(&tag);
                }
            }
            text_start = i;
        }
        b.text(&src[text_start..]);
        let mut dom = b.dom;
        dom.sib = vec![Sib::default(); dom.nodes.len()];
        for n in &dom.nodes {
            let els: Vec<usize> = n.children.iter().copied().filter(|&c| dom.nodes[c].tag().is_some()).collect();
            for (i, &c) in els.iter().enumerate() {
                dom.sib[c] = Sib { index: i as u32, count: els.len() as u32, prev: i.checked_sub(1).map(|j| els[j] as u32) };
            }
        }
        dom
    }

    /// The first element with the tag (in document order).
    pub fn find(&self, tag: &str) -> Option<usize> {
        (0..self.nodes.len()).find(|&i| self.nodes[i].tag() == Some(tag))
    }

    /// Text of the subtree (without markup).
    pub fn text(&self, id: usize) -> String {
        let mut out = String::new();
        self.collect_text(id, &mut out);
        out
    }

    fn collect_text(&self, id: usize, out: &mut String) {
        match &self.nodes[id].data {
            Data::Text(t) => out.push_str(t),
            _ => self.nodes[id].children.iter().for_each(|&c| self.collect_text(c, out)),
        }
    }
}

struct Builder {
    dom: Dom,
    /// Open elements: root … current.
    stack: Vec<usize>,
}

impl Builder {
    fn cur(&self) -> usize {
        *self.stack.last().unwrap()
    }

    fn tag_at(&self, depth: usize) -> &str {
        self.dom.nodes[self.stack[depth]].tag().unwrap_or("")
    }

    fn push(&mut self, data: Data) -> usize {
        let id = self.dom.nodes.len();
        let parent = self.cur();
        self.dom.nodes.push(Node { parent: Some(parent), children: Vec::new(), data });
        self.dom.nodes[parent].children.push(id);
        id
    }

    fn text(&mut self, s: &str) {
        if s.is_empty() {
            return;
        }
        let t = decode(s);
        // Adjacent text is merged (after a skipped comment).
        let cur = self.cur();
        if let Some(&last) = self.dom.nodes[cur].children.last()
            && let Data::Text(prev) = &mut self.dom.nodes[last].data
        {
            prev.push_str(&t);
            return;
        }
        self.push(Data::Text(t));
    }

    /// Depth of the nearest open `tag`, not past a boundary (`stop` — boundary tags).
    fn in_scope(&self, tag: &str, stop: &[&str]) -> Option<usize> {
        for d in (1..self.stack.len()).rev() {
            let t = self.tag_at(d);
            if t == tag {
                return Some(d);
            }
            if stop.contains(&t) {
                return None;
            }
        }
        None
    }

    fn pop_to(&mut self, depth: usize) {
        self.stack.truncate(depth);
    }

    fn open(&mut self, tag: String, attrs: Vec<(String, String)>) {
        const SCOPE: [&str; 6] = ["table", "td", "th", "button", "html", "body"];
        let implied: &[(&str, &[&str])] = match tag.as_str() {
            t if CLOSES_P.contains(&t) => &[("p", &SCOPE)],
            "li" => &[("li", &["ul", "ol", "menu", "table"])],
            "dt" | "dd" => &[("dt", &["dl"]), ("dd", &["dl"])],
            "tr" => &[("tr", &["table"])],
            "td" | "th" => &[("td", &["tr", "table"]), ("th", &["tr", "table"])],
            "thead" | "tbody" | "tfoot" => &[("thead", &["table"]), ("tbody", &["table"]), ("tfoot", &["table"])],
            "option" => &[("option", &["select"])],
            _ => &[],
        };
        for (t, stop) in implied {
            if let Some(d) = self.in_scope(t, stop) {
                self.pop_to(d);
            }
        }
        // A heading doesn't nest inside a heading.
        if tag.len() == 2 && tag.starts_with('h') && tag.as_bytes()[1].is_ascii_digit() {
            let d = (1..self.stack.len()).rev().find(|&d| {
                let t = self.tag_at(d);
                t.len() == 2 && t.starts_with('h') && t.as_bytes()[1].is_ascii_digit()
            });
            if let Some(d) = d {
                self.pop_to(d);
            }
        }
        let id = self.push(Data::Element { tag, attrs });
        self.stack.push(id);
    }

    fn close(&mut self, tag: &str) {
        // Close up to the matching one; a stray closing tag is skipped.
        if let Some(d) = (1..self.stack.len()).rev().find(|&d| self.tag_at(d) == tag) {
            self.pop_to(d);
        }
    }
}

/// Where `</tag` starts in `s` (tag letter case doesn't matter).
fn find_close(s: &str, tag: &str) -> Option<usize> {
    let b = s.as_bytes();
    let mut from = 0;
    while let Some(p) = s[from..].find("</") {
        let at = from + p;
        let name = &b[at + 2..(at + 2 + tag.len()).min(b.len())];
        if name.eq_ignore_ascii_case(tag.as_bytes()) {
            return Some(at);
        }
        from = at + 2;
    }
    None
}

/// Tag at `name_at` (after `<` or `</`): name, attributes, `/>`, end (after `>`).
fn read_tag(src: &str, name_at: usize) -> (String, Vec<(String, String)>, bool, usize) {
    let b = src.as_bytes();
    let mut i = name_at;
    while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'>' && b[i] != b'/' {
        i += 1;
    }
    let tag = src[name_at..i].to_ascii_lowercase();
    let mut attrs = Vec::new();
    let mut self_closing = false;
    loop {
        while i < b.len() && (b[i].is_ascii_whitespace() || b[i] == b'/') {
            self_closing = b[i] == b'/';
            i += 1;
        }
        if i >= b.len() {
            return (tag, attrs, self_closing, i);
        }
        if b[i] == b'>' {
            return (tag, attrs, self_closing, i + 1);
        }
        self_closing = false;
        let start = i;
        while i < b.len() && !b[i].is_ascii_whitespace() && !matches!(b[i], b'>' | b'=' | b'/') {
            i += 1;
        }
        let name = src[start..i].to_ascii_lowercase();
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        let mut value = String::new();
        if i < b.len() && b[i] == b'=' {
            i += 1;
            while i < b.len() && b[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < b.len() && (b[i] == b'"' || b[i] == b'\'') {
                let q = b[i];
                let e = src[i + 1..].find(q as char).map_or(src.len(), |e| i + 1 + e);
                value = decode(&src[i + 1..e]);
                i = (e + 1).min(src.len());
            } else {
                let s = i;
                while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'>' {
                    i += 1;
                }
                value = decode(&src[s..i]);
            }
        }
        if !name.is_empty() && !attrs.iter().any(|(k, _)| *k == name) {
            attrs.push((name, value));
        }
    }
}

/// Named entities: the most common ones.
const ENTITIES: [(&str, &str); 52] = [
    ("amp", "&"), ("lt", "<"), ("gt", ">"), ("quot", "\""), ("apos", "'"), ("nbsp", "\u{a0}"), ("copy", "©"), ("reg", "®"),
    ("trade", "™"), ("hellip", "…"), ("mdash", "—"), ("ndash", "–"), ("laquo", "«"), ("raquo", "»"), ("ldquo", "“"),
    ("rdquo", "”"), ("lsquo", "‘"), ("rsquo", "’"), ("sbquo", "‚"), ("bdquo", "„"), ("bull", "•"), ("middot", "·"),
    ("times", "×"), ("divide", "÷"), ("deg", "°"), ("plusmn", "±"), ("euro", "€"), ("pound", "£"), ("yen", "¥"), ("cent", "¢"),
    ("sect", "§"), ("para", "¶"), ("shy", "\u{ad}"), ("rarr", "→"), ("larr", "←"), ("uarr", "↑"), ("darr", "↓"), ("harr", "↔"),
    ("rArr", "⇒"), ("hearts", "♥"), ("check", "✓"), ("thinsp", "\u{2009}"), ("ensp", "\u{2002}"), ("emsp", "\u{2003}"),
    ("zwj", "\u{200d}"), ("zwnj", "\u{200c}"), ("iexcl", "¡"), ("iquest", "¿"), ("frac12", "½"), ("sup2", "²"), ("micro", "µ"),
    ("prime", "′"),
];

/// Expand `&amp;`, `&#169;`, `&#xA9;`; unknown ones stay as is.
pub fn decode(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(p) = rest.find('&') {
        out.push_str(&rest[..p]);
        rest = &rest[p..];
        let end = rest[1..].find(|c: char| !c.is_ascii_alphanumeric() && c != '#').map_or(rest.len(), |e| e + 1);
        let name = &rest[1..end];
        let ch = if let Some(num) = name.strip_prefix('#') {
            let n = match num.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok(),
                None => num.parse().ok(),
            };
            n.map(|n| char::from_u32(n).unwrap_or('\u{fffd}').to_string())
        } else {
            ENTITIES.iter().find(|(k, _)| *k == name).map(|(_, v)| v.to_string())
        };
        match ch {
            Some(c) if end > 1 => {
                out.push_str(&c);
                rest = &rest[end..];
                rest = rest.strip_prefix(';').unwrap_or(rest);
            }
            _ => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tree on one line: `div(p("a") p("b"))`.
    fn dump(dom: &Dom, id: usize) -> String {
        let n = &dom.nodes[id];
        match &n.data {
            Data::Text(t) => format!("{t:?}"),
            _ => {
                let kids: Vec<String> = n.children.iter().map(|&c| dump(dom, c)).collect();
                format!("{}({})", n.tag().unwrap_or("#"), kids.join(" "))
            }
        }
    }

    fn tree(src: &str) -> String {
        dump(&Dom::parse(src), 0)
    }

    #[test]
    fn implied_closing_and_void_elements() {
        assert_eq!(tree("<p>a<p>b<div>c</div>"), r#"#(p("a") p("b") div("c"))"#);
        assert_eq!(tree("<ul><li>a<li>b<ul><li>c</ul></ul>"), r#"#(ul(li("a") li("b" ul(li("c")))))"#);
        assert_eq!(tree("a<br>b<img src=x.png>c<hr/>"), r#"#("a" br() "b" img() "c" hr())"#);
        assert_eq!(tree("<table><tr><td>1<td>2<tr><td>3</table>"), r#"#(table(tr(td("1") td("2")) tr(td("3"))))"#);
        assert_eq!(tree("<b>x</i>y</b>"), r#"#(b("xy"))"#);
    }

    #[test]
    fn raw_text_comments_and_entities() {
        assert_eq!(tree("<style>p > a { x: '<b>' }</style>"), r#"#(style("p > a { x: '<b>' }"))"#);
        assert_eq!(tree("a<!-- <p> -->b<!doctype html>c"), r#"#("abc")"#);
        assert_eq!(tree("&lt;&amp;&#169;&#x41;&nbsp;&unknown; &copy"), format!("#({:?})", "<&©A\u{a0}&unknown; ©"));
        assert_eq!(tree("1 < 2"), r#"#("1 < 2")"#);
    }

    #[test]
    fn attributes() {
        let dom = Dom::parse(r#"<A HREF="/x?a=1&amp;b=2" class='c d' data-x=5 checked>t</a>"#);
        let a = &dom.nodes[dom.find("a").unwrap()];
        assert_eq!(a.attr("href"), Some("/x?a=1&b=2"));
        assert_eq!(a.attr("class"), Some("c d"));
        assert_eq!(a.attr("data-x"), Some("5"));
        assert_eq!(a.attr("checked"), Some(""));
        assert_eq!(dom.text(0), "t");
    }
}
