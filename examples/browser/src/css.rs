//! CSS parsing: rules, selectors (with specificity), `@media`, values — colors and lengths.
//! Matching a selector against an element of the `html::Dom` tree.

use uison::Color;

use crate::html::Dom;

#[derive(Default)]
pub struct Sheet {
    pub rules: Vec<Rule>,
    /// Declaration blocks: a rule refers to its own (`a, b { … }` share one block).
    pub blocks: Vec<Vec<Decl>>,
    pub media: Vec<Vec<Query>>,
    /// Rules by the key of the rightmost part: `#id`, `.class`, `tag`; without a key — `universal`.
    /// An element is checked only against rules for its id, classes and tag (thousands of rules × thousands of elements).
    index: std::collections::HashMap<u64, Vec<u32>, KeyHash>,
    universal: Vec<u32>,
    /// Rules with `:hover` on the element itself.
    pub hover: Vec<u32>,
}

pub struct Rule {
    pub sel: Selector,
    pub block: usize,
    /// Index into `Sheet::media`.
    pub media: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Decl {
    pub name: String,
    pub value: String,
    pub important: bool,
    /// The value contains `var(` — substitute variables before parsing.
    pub has_var: bool,
}

/// An `@media` condition: window width and theme. Comma-separated queries mean "or".
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Query {
    pub never: bool,
    pub min_w: Option<f32>,
    pub max_w: Option<f32>,
    pub dark: Option<bool>,
}

impl Query {
    pub fn matches(&self, width: f32, dark: bool) -> bool {
        !self.never && self.min_w.is_none_or(|m| width >= m) && self.max_w.is_none_or(|m| width <= m) && self.dark.is_none_or(|d| d == dark)
    }
}

impl Sheet {
    pub fn parse(src: &str) -> Sheet {
        let mut sheet = Sheet::default();
        let src = strip_comments(src);
        sheet.parse_rules(&src, None);
        for (i, r) in sheet.rules.iter().enumerate() {
            let c = &r.sel.subject;
            let key = match (&c.id, c.classes.first(), &c.tag) {
                (Some(id), _, _) => key(b'#', id),
                (None, Some(cl), _) => key(b'.', cl),
                (None, None, Some(t)) => key(b't', t),
                _ => {
                    sheet.universal.push(i as u32);
                    continue;
                }
            };
            sheet.index.entry(key).or_default().push(i as u32);
            if r.sel.hover() {
                sheet.hover.push(i as u32);
            }
        }
        sheet
    }

    /// Indices of rules that may match the element (in table order).
    pub fn candidates(&self, dom: &Dom, id: usize, out: &mut Vec<u32>) {
        out.clear();
        let n = &dom.nodes[id];
        let mut keys = Vec::new();
        keys_of(n, &mut keys);
        for k in keys {
            if let Some(v) = self.index.get(&k) {
                out.extend_from_slice(v);
            }
        }
        out.extend_from_slice(&self.universal);
        out.sort_unstable();
        out.dedup();
    }

    fn parse_rules(&mut self, src: &str, media: Option<usize>) {
        let mut rest = src;
        loop {
            rest = rest.trim_start();
            if rest.is_empty() {
                return;
            }
            if let Some(at) = rest.strip_prefix('@') {
                let name_end = at.find(|c: char| !c.is_ascii_alphanumeric() && c != '-').unwrap_or(at.len());
                let name = at[..name_end].to_ascii_lowercase();
                // `@import …;`, `@charset …;` — up to the semicolon; the rest have a block.
                let semi = at.find(';');
                let brace = at.find('{');
                if semi.is_some_and(|s| brace.is_none_or(|b| s < b)) {
                    rest = &at[semi.unwrap() + 1..];
                    continue;
                }
                let Some(b) = brace else { return };
                let end = matching_brace(at, b);
                let body = &at[b + 1..end.min(at.len())];
                match name.as_str() {
                    "media" => {
                        let q = parse_media(&at[name_end..b]);
                        self.media.push(q);
                        let m = self.media.len() - 1;
                        self.parse_rules(body, Some(m));
                    }
                    // A block with rules inside: treat the condition as satisfied.
                    "supports" | "layer" | "document" | "container" => self.parse_rules(body, media),
                    _ => {} // @font-face, @keyframes, @page…
                }
                rest = at.get(end + 1..).unwrap_or("");
                continue;
            }
            let Some(b) = rest.find('{') else { return };
            let end = matching_brace(rest, b);
            let sels = &rest[..b];
            let body = &rest[b + 1..end.min(rest.len())];
            let decls = parse_decls(body);
            if !decls.is_empty() {
                let block = self.blocks.len();
                let mut any = false;
                for s in split_top(sels, ',') {
                    if let Some(sel) = Selector::parse(s.trim()) {
                        self.rules.push(Rule { sel, block, media });
                        any = true;
                    }
                }
                if any {
                    self.blocks.push(decls);
                }
            }
            rest = rest.get(end + 1..).unwrap_or("");
        }
    }
}

/// Index of the `}` closing the `{` at `open` (accounting for nesting and strings).
fn matching_brace(s: &str, open: usize) -> usize {
    let mut depth = 0;
    let mut quote = None;
    for (i, c) in s[open..].char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '{') => depth += 1,
            (None, '}') => {
                depth -= 1;
                if depth == 0 {
                    return open + i;
                }
            }
            _ => {}
        }
    }
    s.len()
}

fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;
    while let Some(p) = rest.find("/*") {
        out.push_str(&rest[..p]);
        rest = rest[p + 2..].find("*/").map_or("", |e| &rest[p + 2 + e + 2..]);
    }
    out.push_str(rest);
    out
}

/// Split on `sep` outside parentheses and quotes.
pub fn split_top(s: &str, sep: char) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut depth, mut quote, mut start) = (0i32, None, 0);
    for (i, c) in s.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '(' | '[') => depth += 1,
            (None, ')' | ']') => depth -= 1,
            (None, c) if c == sep && depth == 0 => {
                out.push(&s[start..i]);
                start = i + c.len_utf8();
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

/// Declarations `a: b; c: d !important` (a rule body or a `style` attribute).
pub fn parse_decls(body: &str) -> Vec<Decl> {
    split_top(body, ';')
        .into_iter()
        .filter_map(|d| {
            let (name, value) = d.split_once(':')?;
            let name = name.trim().to_ascii_lowercase();
            let mut value = value.trim();
            let important = value.to_ascii_lowercase().ends_with("!important");
            if important {
                value = value[..value.len() - "!important".len()].trim_end();
            }
            (!name.is_empty() && !value.is_empty()).then(|| Decl { name, has_var: value.contains("var("), value: value.to_string(), important })
        })
        .collect()
}

/// The `media` attribute of `<link>`/`<style>`: whether it matches the window (empty matches).
pub fn media_matches(q: &str, width: f32, dark: bool) -> bool {
    q.trim().is_empty() || parse_media(q).iter().any(|m| m.matches(width, dark))
}

fn parse_media(q: &str) -> Vec<Query> {
    split_top(q, ',')
        .into_iter()
        .map(|part| {
            let mut out = Query::default();
            let lower = part.to_ascii_lowercase();
            for word in lower.split(|c: char| c.is_whitespace() || c == '(' || c == ')').filter(|w| !w.is_empty()) {
                if matches!(word, "print" | "speech" | "not") {
                    out.never = true;
                }
            }
            let mut rest = lower.as_str();
            while let Some(p) = rest.find('(') {
                // Nested parentheses: `(max-width: calc(640px - 1px))`.
                let e = matching_paren(rest, p).unwrap_or(rest.len());
                if let Some((k, v)) = rest[p + 1..e].split_once(':') {
                    let px = || match parse_len(v.trim()) {
                        Some(Len::Px(x)) => Some(x),
                        Some(Len::Em(x) | Len::Rem(x)) => Some(x * 16.0),
                        Some(Len::Lin { px, em, .. }) => Some(px + em * 16.0),
                        _ => None,
                    };
                    match k.trim() {
                        "min-width" => out.min_w = px(),
                        "max-width" => out.max_w = px(),
                        "prefers-color-scheme" => out.dark = Some(v.trim() == "dark"),
                        "orientation" | "hover" | "pointer" | "prefers-reduced-motion" | "min-height" | "max-height" => {}
                        _ => out.never = true,
                    }
                }
                rest = &rest[(e + 1).min(rest.len())..];
            }
            out
        })
        .collect()
}

// ------------------------------------------------------------------ selectors

#[derive(Clone, Debug, PartialEq)]
pub enum Comb {
    Descendant,
    Child,
    /// `+`
    Next,
    /// `~`
    Sibling,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AttrOp {
    Exists,
    Eq(String),
    /// `~=` — a word in a space-separated list.
    Word(String),
    Prefix(String),
    Suffix(String),
    Contains(String),
    /// `|=` — the value or `value-…`.
    Dash(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Pseudo {
    Hover,
    Link,
    Root,
    Empty,
    FirstChild,
    LastChild,
    OnlyChild,
    /// `:nth-child(an+b)`; `last` — `:nth-last-child`.
    Nth { a: i32, b: i32, last: bool },
    /// `:not(a, b)` — none of them; `:is()`/`:where()` — at least one (simple selectors).
    Not(Vec<Compound>),
    Is(Vec<Compound>, bool),
    FirstOfType,
    LastOfType,
    Checked,
    Disabled,
    Lang(String),
    /// A pseudo-element or something we never have (`:focus`, `:visited`).
    Never,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Compound {
    pub tag: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attrs: Vec<(String, AttrOp)>,
    pub pseudos: Vec<Pseudo>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Selector {
    /// Right to left: `subject`, then (combinator, compound selector to the left).
    pub subject: Compound,
    pub chain: Vec<(Comb, Compound)>,
    /// (ids, classes/attributes/pseudo-classes, tags) packed into one number.
    pub spec: u32,
    /// Keys the ancestors must have (tags, ids, classes of parts joined by descendant/`>`):
    /// if any is missing among the ancestors, the selector doesn't match and the tree walk can be skipped.
    pub anc: Vec<u64>,
}

/// A hasher for keys that are already hashes (FNV): no second SipHash.
#[derive(Default, Clone, Copy)]
pub struct KeyHash;

impl std::hash::BuildHasher for KeyHash {
    type Hasher = KeyHasher;
    fn build_hasher(&self) -> KeyHasher {
        KeyHasher(0)
    }
}

pub struct KeyHasher(u64);

impl std::hash::Hasher for KeyHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, b: &[u8]) {
        for &x in b {
            self.0 = (self.0 ^ x as u64).wrapping_mul(0x0100_0000_01b3);
        }
    }
    fn write_u64(&mut self, v: u64) {
        self.0 = v;
    }
}

/// A key for the rule index and the ancestor filter: tag, `#id` or `.class`.
pub fn key(kind: u8, s: &str) -> u64 {
    // FNV-1a
    let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ kind as u64;
    for b in s.bytes() {
        h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// An element's keys: its tag, id and classes.
pub fn keys_of(n: &crate::html::Node, out: &mut Vec<u64>) {
    if let Some(t) = n.tag() {
        out.push(key(b't', t));
    }
    if let Some(i) = n.attr("id") {
        out.push(key(b'#', i));
    }
    for c in n.attr("class").unwrap_or("").split_ascii_whitespace() {
        out.push(key(b'.', c));
    }
}

impl Selector {
    pub fn parse(s: &str) -> Option<Selector> {
        let mut parts: Vec<(Option<Comb>, Compound)> = Vec::new();
        let mut p = SelParser { s, i: 0 };
        let mut comb = None;
        loop {
            let had_space = p.skip_ws();
            if p.done() {
                break;
            }
            match p.peek() {
                '>' | '+' | '~' => {
                    comb = Some(match p.bump() {
                        '>' => Comb::Child,
                        '+' => Comb::Next,
                        _ => Comb::Sibling,
                    });
                    continue;
                }
                _ => {}
            }
            if !parts.is_empty() && comb.is_none() && had_space {
                comb = Some(Comb::Descendant);
            }
            let c = p.compound()?;
            parts.push((comb.take(), c));
        }
        // Each part has the combinator to its left; the chain goes right to left.
        let (mut link, subject) = parts.pop()?;
        let mut chain = Vec::new();
        while let Some((c, comp)) = parts.pop() {
            chain.push((link.take().unwrap_or(Comb::Descendant), comp));
            link = c;
        }
        let spec = subject.spec() + chain.iter().map(|(_, c)| c.spec()).sum::<u32>();
        // Parts further left joined by descendant/`>` are ancestors; after `+`/`~` the chain walks siblings.
        let mut anc = Vec::new();
        for (comb, c) in &chain {
            if matches!(comb, Comb::Next | Comb::Sibling) {
                break;
            }
            if let Some(t) = &c.tag {
                anc.push(key(b't', t));
            }
            if let Some(i) = &c.id {
                anc.push(key(b'#', i));
            }
            anc.extend(c.classes.iter().map(|cl| key(b'.', cl)));
        }
        Some(Selector { subject, chain, spec, anc })
    }

    /// Whether the element itself has `:hover` (in the rightmost part).
    pub fn hover(&self) -> bool {
        self.subject.pseudos.contains(&Pseudo::Hover)
    }

    /// Whether element `id` matches; `hover` is the element under the mouse.
    pub fn matches(&self, dom: &Dom, id: usize, hover: Option<usize>) -> bool {
        if !self.subject.matches(dom, id, hover) {
            return false;
        }
        match_chain(dom, id, &self.chain, hover)
    }
}

fn match_chain(dom: &Dom, id: usize, chain: &[(Comb, Compound)], hover: Option<usize>) -> bool {
    let Some(((comb, c), rest)) = chain.split_first() else { return true };
    match comb {
        Comb::Child => parent_el(dom, id).is_some_and(|p| c.matches(dom, p, hover) && match_chain(dom, p, rest, hover)),
        Comb::Descendant => {
            let mut cur = parent_el(dom, id);
            while let Some(p) = cur {
                if c.matches(dom, p, hover) && match_chain(dom, p, rest, hover) {
                    return true;
                }
                cur = parent_el(dom, p);
            }
            false
        }
        Comb::Next => prev_el(dom, id).is_some_and(|p| c.matches(dom, p, hover) && match_chain(dom, p, rest, hover)),
        Comb::Sibling => {
            let mut cur = prev_el(dom, id);
            while let Some(p) = cur {
                if c.matches(dom, p, hover) && match_chain(dom, p, rest, hover) {
                    return true;
                }
                cur = prev_el(dom, p);
            }
            false
        }
    }
}

fn parent_el(dom: &Dom, id: usize) -> Option<usize> {
    dom.nodes[id].parent.filter(|&p| dom.nodes[p].tag().is_some())
}

/// Sibling elements (without text).
fn siblings(dom: &Dom, id: usize) -> Vec<usize> {
    match dom.nodes[id].parent {
        Some(p) => dom.nodes[p].children.iter().copied().filter(|&c| dom.nodes[c].tag().is_some()).collect(),
        None => vec![id],
    }
}

fn prev_el(dom: &Dom, id: usize) -> Option<usize> {
    dom.sib[id].prev.map(|p| p as usize)
}

impl Compound {
    fn spec(&self) -> u32 {
        let mut s = 0;
        if self.id.is_some() {
            s += 1 << 16;
        }
        s += ((self.classes.len() + self.attrs.len()) as u32) << 8;
        for p in &self.pseudos {
            s += match p {
                Pseudo::Not(list) | Pseudo::Is(list, true) => list.iter().map(Compound::spec).max().unwrap_or(0),
                Pseudo::Is(_, false) => 0,
                _ => 1 << 8,
            };
        }
        if self.tag.is_some() {
            s += 1;
        }
        s
    }

    pub fn matches(&self, dom: &Dom, id: usize, hover: Option<usize>) -> bool {
        let n = &dom.nodes[id];
        let Some(tag) = n.tag() else { return false };
        if self.tag.as_deref().is_some_and(|t| t != tag) {
            return false;
        }
        if self.id.as_deref().is_some_and(|i| n.attr("id") != Some(i)) {
            return false;
        }
        if !self.classes.is_empty() {
            let cls = n.attr("class").unwrap_or("");
            if !self.classes.iter().all(|c| cls.split_ascii_whitespace().any(|x| x == c)) {
                return false;
            }
        }
        for (name, op) in &self.attrs {
            let Some(v) = n.attr(name) else { return false };
            let ok = match op {
                AttrOp::Exists => true,
                AttrOp::Eq(x) => v == x,
                AttrOp::Word(x) => v.split_ascii_whitespace().any(|w| w == x),
                AttrOp::Prefix(x) => v.starts_with(x.as_str()),
                AttrOp::Suffix(x) => v.ends_with(x.as_str()),
                AttrOp::Contains(x) => v.contains(x.as_str()),
                AttrOp::Dash(x) => v == x || v.strip_prefix(x.as_str()).is_some_and(|r| r.starts_with('-')),
            };
            if !ok {
                return false;
            }
        }
        self.pseudos.iter().all(|p| match p {
            Pseudo::Hover => hover == Some(id),
            Pseudo::Link => tag == "a" && n.attr("href").is_some(),
            Pseudo::Root => parent_el(dom, id).is_none(),
            Pseudo::Empty => n.children.is_empty(),
            Pseudo::FirstChild => dom.sib[id].index == 0,
            Pseudo::LastChild => dom.sib[id].index + 1 == dom.sib[id].count,
            Pseudo::OnlyChild => dom.sib[id].count == 1,
            Pseudo::Nth { a, b, last } => {
                let sb = dom.sib[id];
                let pos = if *last { sb.count - sb.index } else { sb.index + 1 } as i32;
                // pos = a·n + b for some n ≥ 0.
                if *a == 0 { pos == *b } else { (pos - b) % a == 0 && (pos - b) / a >= 0 }
            }
            Pseudo::Not(list) => !list.iter().any(|c| c.matches(dom, id, hover)),
            Pseudo::Is(list, _) => list.iter().any(|c| c.matches(dom, id, hover)),
            Pseudo::FirstOfType => siblings(dom, id).into_iter().find(|&s| dom.nodes[s].tag() == Some(tag)) == Some(id),
            Pseudo::LastOfType => siblings(dom, id).into_iter().rev().find(|&s| dom.nodes[s].tag() == Some(tag)) == Some(id),
            Pseudo::Checked => n.attr("checked").is_some() || n.attr("selected").is_some(),
            Pseudo::Disabled => n.attr("disabled").is_some(),
            Pseudo::Lang(l) => {
                let mut cur = Some(id);
                while let Some(c) = cur {
                    if let Some(v) = dom.nodes[c].attr("lang") {
                        return v.eq_ignore_ascii_case(l) || v.to_ascii_lowercase().starts_with(&format!("{l}-"));
                    }
                    cur = dom.nodes[c].parent;
                }
                false
            }
            Pseudo::Never => false,
        })
    }
}

struct SelParser<'a> {
    s: &'a str,
    i: usize,
}

impl SelParser<'_> {
    fn done(&self) -> bool {
        self.i >= self.s.len()
    }

    fn peek(&self) -> char {
        self.s[self.i..].chars().next().unwrap_or('\0')
    }

    fn bump(&mut self) -> char {
        let c = self.peek();
        self.i += c.len_utf8();
        c
    }

    fn skip_ws(&mut self) -> bool {
        let start = self.i;
        while !self.done() && self.peek().is_whitespace() {
            self.bump();
        }
        self.i > start
    }

    fn ident(&mut self) -> String {
        let start = self.i;
        while !self.done() {
            let c = self.peek();
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '\\' || !c.is_ascii() {
                self.bump();
            } else {
                break;
            }
        }
        self.s[start..self.i].replace('\\', "")
    }

    fn compound(&mut self) -> Option<Compound> {
        let mut c = Compound::default();
        let start = self.i;
        if self.peek() == '*' {
            self.bump();
        } else if self.peek().is_alphabetic() {
            c.tag = Some(self.ident().to_ascii_lowercase());
        }
        loop {
            match self.peek() {
                '#' => {
                    self.bump();
                    c.id = Some(self.ident());
                }
                '.' => {
                    self.bump();
                    c.classes.push(self.ident());
                }
                '[' => {
                    self.bump();
                    let end = self.s[self.i..].find(']')? + self.i;
                    let body = &self.s[self.i..end];
                    self.i = end + 1;
                    c.attrs.push(parse_attr(body)?);
                }
                ':' => {
                    self.bump();
                    if self.peek() == ':' {
                        self.bump();
                        self.ident();
                        c.pseudos.push(Pseudo::Never);
                        continue;
                    }
                    let name = self.ident().to_ascii_lowercase();
                    let arg = if self.peek() == '(' {
                        let open = self.i;
                        let close = matching_paren(self.s, open)?;
                        self.i = close + 1;
                        Some(&self.s[open + 1..close])
                    } else {
                        None
                    };
                    c.pseudos.push(match (name.as_str(), arg) {
                        ("hover", None) => Pseudo::Hover,
                        ("link" | "any-link", None) => Pseudo::Link,
                        ("root", None) => Pseudo::Root,
                        ("empty", None) => Pseudo::Empty,
                        ("first-child", None) => Pseudo::FirstChild,
                        ("last-child", None) => Pseudo::LastChild,
                        ("only-child", None) => Pseudo::OnlyChild,
                        ("nth-child", Some(a)) => nth(a, false)?,
                        ("nth-last-child", Some(a)) => nth(a, true)?,
                        ("not", Some(a)) => Pseudo::Not(compound_list(a)?),
                        ("is" | "matches" | "any" | "-webkit-any", Some(a)) => Pseudo::Is(compound_list(a)?, true),
                        ("where", Some(a)) => Pseudo::Is(compound_list(a)?, false),
                        ("first-of-type", None) => Pseudo::FirstOfType,
                        ("last-of-type", None) => Pseudo::LastOfType,
                        ("checked", None) => Pseudo::Checked,
                        ("disabled", None) => Pseudo::Disabled,
                        ("enabled", None) => Pseudo::Not(vec![Compound { pseudos: vec![Pseudo::Disabled], ..Compound::default() }]),
                        ("lang", Some(a)) => Pseudo::Lang(a.trim().trim_matches(['"', '\'']).to_ascii_lowercase()),
                        // Without focus and visited-link states, such rules don't apply.
                        _ => Pseudo::Never,
                    });
                }
                _ => break,
            }
        }
        (self.i > start).then_some(c)
    }
}

/// `a, .b, #c` inside `:not()`/`:is()` — only simple (compound) selectors.
fn compound_list(arg: &str) -> Option<Vec<Compound>> {
    split_top(arg, ',')
        .into_iter()
        .map(|part| {
            let mut p = SelParser { s: part.trim(), i: 0 };
            let c = p.compound()?;
            p.done().then_some(c)
        })
        .collect()
}

fn matching_paren(s: &str, open: usize) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s[open..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_attr(body: &str) -> Option<(String, AttrOp)> {
    let ops = ["~=", "^=", "$=", "*=", "|=", "="];
    for op in ops {
        if let Some((k, v)) = body.split_once(op) {
            // Case-insensitive flag: `[type="a" i]` (the flag itself is ignored).
            let v = v.trim();
            let v = v.strip_suffix(" i").or_else(|| v.strip_suffix(" s")).unwrap_or(v);
            let v = v.trim().trim_matches(['"', '\'']).to_string();
            let k = k.trim().to_ascii_lowercase();
            return Some((
                k,
                match op {
                    "~=" => AttrOp::Word(v),
                    "^=" => AttrOp::Prefix(v),
                    "$=" => AttrOp::Suffix(v),
                    "*=" => AttrOp::Contains(v),
                    "|=" => AttrOp::Dash(v),
                    _ => AttrOp::Eq(v),
                },
            ));
        }
    }
    let k = body.trim().to_ascii_lowercase();
    (!k.is_empty()).then_some((k, AttrOp::Exists))
}

/// `odd`, `even`, `3`, `2n+1`, `-n+3`.
fn nth(arg: &str, last: bool) -> Option<Pseudo> {
    let s: String = arg.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_ascii_lowercase();
    let (a, b) = match s.as_str() {
        "odd" => (2, 1),
        "even" => (2, 0),
        _ => match s.split_once('n') {
            Some((a, b)) => {
                let a = match a {
                    "" | "+" => 1,
                    "-" => -1,
                    a => a.parse().ok()?,
                };
                (a, if b.is_empty() { 0 } else { b.trim_start_matches('+').parse().ok()? })
            }
            None => (0, s.parse().ok()?),
        },
    };
    Some(Pseudo::Nth { a, b, last })
}

// ------------------------------------------------------------------ values

/// A CSS length before computation (em from the font size, % from the parent's width).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Len {
    Px(f32),
    Em(f32),
    Rem(f32),
    Pct(f32),
    Vw(f32),
    Vh(f32),
    Auto,
    /// `calc()`: a sum of points, em (rem as 16 points), percent and vw.
    Lin { px: f32, em: f32, pct: f32, vw: f32 },
}

pub fn parse_len(s: &str) -> Option<Len> {
    let s = s.trim().to_ascii_lowercase();
    if s == "auto" {
        return Some(Len::Auto);
    }
    if s.ends_with(')') && ["calc(", "min(", "max(", "clamp("].iter().any(|f| s.starts_with(f)) {
        let mut p = Calc { s: s.as_bytes(), i: 0 };
        let v = p.factor()?;
        return Some(Len::Lin { px: v[0], em: v[1], pct: v[2], vw: v[3] });
    }
    let num_end = s.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+')).unwrap_or(s.len());
    let (num, unit) = s.split_at(num_end);
    let n: f32 = num.parse().ok()?;
    Some(match unit {
        "px" => Len::Px(n),
        "" => Len::Px(n), // 0 and unitless numbers (in old pages)
        "em" => Len::Em(n),
        "rem" => Len::Rem(n),
        "%" => Len::Pct(n),
        "pt" => Len::Px(n * 4.0 / 3.0),
        "pc" => Len::Px(n * 16.0),
        "in" => Len::Px(n * 96.0),
        "cm" => Len::Px(n * 96.0 / 2.54),
        "mm" => Len::Px(n * 96.0 / 25.4),
        "ex" | "ch" => Len::Em(n * 0.5),
        "vw" => Len::Vw(n),
        "vh" => Len::Vh(n),
        "vmin" | "vmax" => Len::Vw(n),
        _ => return None,
    })
}

/// Parsing `calc()`: the value is coefficients of px, em, %, vw (linear: you can only multiply and divide
/// by a number). `min`/`max` compare points, `clamp(a, b, c)` is `b` within the bounds.
struct Calc<'a> {
    s: &'a [u8],
    i: usize,
}

type V = [f32; 4];

impl Calc<'_> {
    fn ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn eat(&mut self, c: u8) -> bool {
        self.ws();
        let ok = self.s.get(self.i) == Some(&c);
        self.i += ok as usize;
        ok
    }

    fn expr(&mut self) -> Option<V> {
        let mut v = self.term()?;
        loop {
            self.ws();
            match self.s.get(self.i) {
                Some(b'+') | Some(b'-') => {
                    let neg = self.s[self.i] == b'-';
                    self.i += 1;
                    let t = self.term()?;
                    for k in 0..4 {
                        v[k] += if neg { -t[k] } else { t[k] };
                    }
                }
                _ => return Some(v),
            }
        }
    }

    fn term(&mut self) -> Option<V> {
        let mut v = self.factor()?;
        loop {
            if self.eat(b'*') {
                let f = self.factor()?;
                // One side is a number (unitless: only v[0] = px as the "number").
                v = if f[1..].iter().all(|&x| x == 0.0) && is_num(&f) { v.map(|x| x * f[0]) } else { f.map(|x| x * v[0]) };
            } else if self.eat(b'/') {
                let f = self.factor()?;
                if f[0] == 0.0 {
                    return None;
                }
                v = v.map(|x| x / f[0]);
            } else {
                return Some(v);
            }
        }
    }

    fn factor(&mut self) -> Option<V> {
        self.ws();
        let rest = &self.s[self.i..];
        for (name, f) in [("calc(", 0), ("min(", 1), ("max(", 2), ("clamp(", 3)] {
            if rest.starts_with(name.as_bytes()) {
                self.i += name.len();
                let mut args = vec![self.expr()?];
                while self.eat(b',') {
                    args.push(self.expr()?);
                }
                if !self.eat(b')') {
                    return None;
                }
                let px = |v: &V| v[0] + v[1] * 16.0;
                return Some(match f {
                    0 => args[0],
                    1 => *args.iter().min_by(|a, b| px(a).total_cmp(&px(b)))?,
                    2 => *args.iter().max_by(|a, b| px(a).total_cmp(&px(b)))?,
                    _ if args.len() == 3 => {
                        if px(&args[1]) < px(&args[0]) {
                            args[0]
                        } else if px(&args[1]) > px(&args[2]) {
                            args[2]
                        } else {
                            args[1]
                        }
                    }
                    _ => return None,
                });
            }
        }
        if self.eat(b'(') {
            let v = self.expr()?;
            return self.eat(b')').then_some(v);
        }
        let start = self.i;
        if matches!(self.s.get(self.i), Some(b'-' | b'+')) {
            self.i += 1;
        }
        while self.i < self.s.len() && (self.s[self.i].is_ascii_digit() || self.s[self.i] == b'.') {
            self.i += 1;
        }
        let n: f32 = std::str::from_utf8(&self.s[start..self.i]).ok()?.parse().ok()?;
        let u0 = self.i;
        while self.i < self.s.len() && (self.s[self.i].is_ascii_alphabetic() || self.s[self.i] == b'%') {
            self.i += 1;
        }
        Some(match &self.s[u0..self.i] {
            b"" | b"px" => [n, 0.0, 0.0, 0.0],
            b"em" => [0.0, n, 0.0, 0.0],
            b"rem" => [n * 16.0, 0.0, 0.0, 0.0],
            b"%" => [0.0, 0.0, n, 0.0],
            b"vw" | b"vmin" | b"vmax" => [0.0, 0.0, 0.0, n],
            b"pt" => [n * 4.0 / 3.0, 0.0, 0.0, 0.0],
            _ => return None,
        })
    }
}

/// A unitless number: it can be multiplied by.
fn is_num(v: &V) -> bool {
    v[1] == 0.0 && v[2] == 0.0 && v[3] == 0.0
}

/// A CSS color: `#abc`, `#aabbcc(dd)`, `rgb()/rgba()`, `hsl()/hsla()`, names, `transparent`.
pub fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim().to_ascii_lowercase();
    if let Some(h) = s.strip_prefix('#') {
        return match h.len() {
            3 | 6 | 8 => Color::from_hex(&s),
            4 => {
                let x: Vec<char> = h.chars().collect();
                Color::from_hex(&format!("#{0}{0}{1}{1}{2}{2}{3}{3}", x[0], x[1], x[2], x[3]))
            }
            _ => None,
        };
    }
    if s == "transparent" {
        return Some(Color::rgba(0, 0, 0, 0));
    }
    if let Some(open) = s.find('(') {
        let f = &s[..open];
        let args: Vec<&str> = s[open + 1..s.rfind(')')?].split(|c: char| c == ',' || c == '/' || c.is_whitespace()).filter(|a| !a.is_empty()).collect();
        let alpha = |i: usize| -> u8 {
            args.get(i).map_or(255, |a| {
                let v = a.strip_suffix('%').map_or_else(|| a.parse().unwrap_or(1.0), |p| p.parse::<f32>().unwrap_or(100.0) / 100.0);
                (v.clamp(0.0, 1.0) * 255.0).round() as u8
            })
        };
        if args.len() < 3 {
            return None;
        }
        return match f {
            "rgb" | "rgba" => {
                let ch = |a: &str| -> Option<u8> {
                    let v = match a.strip_suffix('%') {
                        Some(p) => p.parse::<f32>().ok()? * 2.55,
                        None => a.parse::<f32>().ok()?,
                    };
                    Some(v.clamp(0.0, 255.0).round() as u8)
                };
                Some(Color::rgba(ch(args[0])?, ch(args[1])?, ch(args[2])?, alpha(3)))
            }
            "hsl" | "hsla" => {
                let h = args[0].trim_end_matches("deg").parse::<f32>().ok()? / 360.0;
                let sat = args[1].trim_end_matches('%').parse::<f32>().ok()? / 100.0;
                let l = args[2].trim_end_matches('%').parse::<f32>().ok()? / 100.0;
                let (r, g, b) = hsl(h.rem_euclid(1.0), sat.clamp(0.0, 1.0), l.clamp(0.0, 1.0));
                Some(Color::rgba(r, g, b, alpha(3)))
            }
            _ => None,
        };
    }
    NAMED.split(',').find_map(|e| {
        let (name, hex) = e.split_once(':')?;
        (name == s).then(|| Color::from_hex(&format!("#{hex}"))).flatten()
    })
}

fn hsl(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let f = |t: f32| {
        let t = t.rem_euclid(1.0);
        let v = if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        };
        (v * 255.0).round() as u8
    };
    (f(h + 1.0 / 3.0), f(h), f(h - 1.0 / 3.0))
}

/// All 148 CSS color names.
const NAMED: &str = "aliceblue:f0f8ff,antiquewhite:faebd7,aqua:00ffff,aquamarine:7fffd4,azure:f0ffff,beige:f5f5dc,bisque:ffe4c4,\
black:000000,blanchedalmond:ffebcd,blue:0000ff,blueviolet:8a2be2,brown:a52a2a,burlywood:deb887,cadetblue:5f9ea0,chartreuse:7fff00,\
chocolate:d2691e,coral:ff7f50,cornflowerblue:6495ed,cornsilk:fff8dc,crimson:dc143c,cyan:00ffff,darkblue:00008b,darkcyan:008b8b,\
darkgoldenrod:b8860b,darkgray:a9a9a9,darkgreen:006400,darkgrey:a9a9a9,darkkhaki:bdb76b,darkmagenta:8b008b,darkolivegreen:556b2f,\
darkorange:ff8c00,darkorchid:9932cc,darkred:8b0000,darksalmon:e9967a,darkseagreen:8fbc8f,darkslateblue:483d8b,darkslategray:2f4f4f,\
darkslategrey:2f4f4f,darkturquoise:00ced1,darkviolet:9400d3,deeppink:ff1493,deepskyblue:00bfff,dimgray:696969,dimgrey:696969,\
dodgerblue:1e90ff,firebrick:b22222,floralwhite:fffaf0,forestgreen:228b22,fuchsia:ff00ff,gainsboro:dcdcdc,ghostwhite:f8f8ff,gold:ffd700,\
goldenrod:daa520,gray:808080,green:008000,greenyellow:adff2f,grey:808080,honeydew:f0fff0,hotpink:ff69b4,indianred:cd5c5c,indigo:4b0082,\
ivory:fffff0,khaki:f0e68c,lavender:e6e6fa,lavenderblush:fff0f5,lawngreen:7cfc00,lemonchiffon:fffacd,lightblue:add8e6,lightcoral:f08080,\
lightcyan:e0ffff,lightgoldenrodyellow:fafad2,lightgray:d3d3d3,lightgreen:90ee90,lightgrey:d3d3d3,lightpink:ffb6c1,lightsalmon:ffa07a,\
lightseagreen:20b2aa,lightskyblue:87cefa,lightslategray:778899,lightslategrey:778899,lightsteelblue:b0c4de,lightyellow:ffffe0,lime:00ff00,\
limegreen:32cd32,linen:faf0e6,magenta:ff00ff,maroon:800000,mediumaquamarine:66cdaa,mediumblue:0000cd,mediumorchid:ba55d3,\
mediumpurple:9370db,mediumseagreen:3cb371,mediumslateblue:7b68ee,mediumspringgreen:00fa9a,mediumturquoise:48d1cc,\
mediumvioletred:c71585,midnightblue:191970,mintcream:f5fffa,mistyrose:ffe4e1,moccasin:ffe4b5,navajowhite:ffdead,navy:000080,\
oldlace:fdf5e6,olive:808000,olivedrab:6b8e23,orange:ffa500,orangered:ff4500,orchid:da70d6,palegoldenrod:eee8aa,palegreen:98fb98,\
paleturquoise:afeeee,palevioletred:db7093,papayawhip:ffefd5,peachpuff:ffdab9,peru:cd853f,pink:ffc0cb,plum:dda0dd,powderblue:b0e0e6,\
purple:800080,rebeccapurple:663399,red:ff0000,rosybrown:bc8f8f,royalblue:4169e1,saddlebrown:8b4513,salmon:fa8072,sandybrown:f4a460,\
seagreen:2e8b57,seashell:fff5ee,sienna:a0522d,silver:c0c0c0,skyblue:87ceeb,slateblue:6a5acd,slategray:708090,slategrey:708090,\
snow:fffafa,springgreen:00ff7f,steelblue:4682b4,tan:d2b48c,teal:008080,thistle:d8bfd8,tomato:ff6347,turquoise:40e0d0,violet:ee82ee,\
wheat:f5deb3,white:ffffff,whitesmoke:f5f5f5,yellow:ffff00,yellowgreen:9acd32";

#[cfg(test)]
mod tests {
    use super::*;

    fn el(dom: &Dom, tag: &str, n: usize) -> usize {
        (0..dom.nodes.len()).filter(|&i| dom.nodes[i].tag() == Some(tag)).nth(n).unwrap()
    }

    #[test]
    fn rules_media_and_declarations() {
        let s = Sheet::parse(
            "/* c */ @charset 'x'; h1, .a > p { color: red; margin: 0 auto !important }
             @media (max-width: 600px) { p { color: blue } } @font-face { src: x } @media print { p { color: green } }",
        );
        assert_eq!(s.rules.len(), 4);
        assert_eq!(s.rules[0].block, s.rules[1].block);
        assert_eq!(s.blocks[0][1], Decl { name: "margin".into(), value: "0 auto".into(), important: true, has_var: false });
        let m = s.rules[2].media.unwrap();
        assert!(s.media[m][0].matches(500.0, false) && !s.media[m][0].matches(700.0, false));
        let m = s.rules[3].media.unwrap();
        assert!(!s.media[m][0].matches(500.0, false));
        let s = Sheet::parse("@media all and (max-width:calc(640px - 1px)) { p { x: y } } a[lang=hi] { x: y }");
        let m = s.rules[0].media.unwrap();
        assert!(s.media[m][0].matches(600.0, false) && !s.media[m][0].matches(1100.0, false));
        assert_eq!(s.rules[1].sel.subject.attrs[0].1, AttrOp::Eq("hi".into()));
    }

    #[test]
    fn selectors_match_and_specificity() {
        let dom = Dom::parse(r#"<div id=m class="x y"><p>1</p><p class=z>2</p><a href=/q>3</a></div><ul><li>a<li>b<li>c</ul>"#);
        let sel = |s: &str| Selector::parse(s).unwrap_or_else(|| panic!("{s}"));
        let p0 = el(&dom, "p", 0);
        let p1 = el(&dom, "p", 1);
        assert!(sel("div p").matches(&dom, p0, None));
        assert!(sel("#m > p:first-child").matches(&dom, p0, None));
        assert!(!sel("#m > p:first-child").matches(&dom, p1, None));
        assert!(sel("p + .z").matches(&dom, p1, None));
        assert!(sel("p ~ a[href^='/']").matches(&dom, el(&dom, "a", 0), None));
        assert!(sel(".x.y p:not(.z)").matches(&dom, p0, None));
        assert!(!sel(".x.y p:not(.z)").matches(&dom, p1, None));
        assert!(sel("li:nth-child(odd)").matches(&dom, el(&dom, "li", 2), None));
        assert!(!sel("li:nth-child(2n+1)").matches(&dom, el(&dom, "li", 1), None));
        assert!(sel("li:last-child").matches(&dom, el(&dom, "li", 2), None));
        assert!(!sel("a:hover").matches(&dom, el(&dom, "a", 0), None));
        assert!(sel("a:hover").matches(&dom, el(&dom, "a", 0), Some(el(&dom, "a", 0))));
        assert!(!sel("p::before").matches(&dom, p0, None));
        assert_eq!(sel("#m .x a:hover").spec, (1 << 16) + (2 << 8) + 1);
        assert_eq!(sel("ul li").spec, 2);
    }

    #[test]
    fn colors_and_lengths() {
        assert_eq!(parse_color("#f00"), Some(Color::rgb(255, 0, 0)));
        assert_eq!(parse_color("rgb(0, 128, 255)"), Some(Color::rgb(0, 128, 255)));
        assert_eq!(parse_color("rgba(0 0 0 / 50%)"), Some(Color::rgba(0, 0, 0, 128)));
        assert_eq!(parse_color("hsl(120, 100%, 25%)"), Some(Color::rgb(0, 128, 0)));
        assert_eq!(parse_color("RebeccaPurple"), Some(Color::rgb(0x66, 0x33, 0x99)));
        assert_eq!(parse_color("nonsense"), None);
        assert_eq!(parse_len("1.5em"), Some(Len::Em(1.5)));
        assert_eq!(parse_len("12pt"), Some(Len::Px(16.0)));
        assert_eq!(parse_len("50%"), Some(Len::Pct(50.0)));
        assert_eq!(parse_len("0"), Some(Len::Px(0.0)));
        assert_eq!(parse_len("calc(1120px - 1px)"), Some(Len::Lin { px: 1119.0, em: 0.0, pct: 0.0, vw: 0.0 }));
        assert_eq!(parse_len("calc(100% - 2 * 1.5em)"), Some(Len::Lin { px: 0.0, em: -3.0, pct: 100.0, vw: 0.0 }));
        assert_eq!(parse_len("clamp(10px, 50vw, 900px)").map(|_| ()), Some(()));
        assert_eq!(parse_len("max(8px, calc(1rem + 2px))"), Some(Len::Lin { px: 18.0, em: 0.0, pct: 0.0, vw: 0.0 }));
    }
}
