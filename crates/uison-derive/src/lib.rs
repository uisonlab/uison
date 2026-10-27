//! `#[derive(Data)]`: struct ↔ `uison` data — `IntoJson` and `FromJson` for a struct
//! with named fields. No `syn` or `quote`: tokens are parsed by hand, code is built as a string.
//!
//! ```ignore
//! #[derive(Data, Default)]
//! struct Todo {
//!     id: i32,
//!     title: String,
//!     #[data(rename = "isDone")]
//!     done: bool,
//!     #[data(skip)]
//!     cache: Vec<u8>,
//! }
//! ```
//!
//! A field missing from the data (or of the wrong type) is taken from the struct's `Default`.

use proc_macro::{Delimiter, TokenStream, TokenTree};

#[proc_macro_derive(Data, attributes(data))]
pub fn derive_data(input: TokenStream) -> TokenStream {
    match derive(input) {
        Ok(code) => code.parse().expect("uison-derive: generated code"),
        Err(msg) => format!("compile_error!({msg:?});").parse().unwrap(),
    }
}

struct Field {
    name: String,
    json: String,
    skip: bool,
}

fn derive(input: TokenStream) -> Result<String, String> {
    let mut it = input.into_iter().peekable();
    let mut name = None;
    while let Some(t) = it.next() {
        match t {
            // Struct attributes (#[derive], ///) and visibility are skipped.
            TokenTree::Punct(p) if p.as_char() == '#' => {
                it.next();
            }
            TokenTree::Ident(i) if i.to_string() == "struct" => {
                name = match it.next() {
                    Some(TokenTree::Ident(n)) => Some(n.to_string()),
                    _ => return Err("Data: expected struct name".into()),
                };
                break;
            }
            TokenTree::Ident(i) if i.to_string() == "enum" || i.to_string() == "union" => {
                return Err("Data: only structs with named fields".into());
            }
            _ => {}
        }
    }
    let name = name.ok_or("Data: expected a struct")?;
    let body = match it.next() {
        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => g.stream(),
        Some(TokenTree::Punct(p)) if p.as_char() == '<' => return Err("Data: generic structs are not supported".into()),
        _ => return Err("Data: only structs with named fields".into()),
    };
    let fields = fields(body)?;
    let path = "::uison::ui";
    let mut into = String::new();
    let mut from = String::new();
    for f in fields.iter().filter(|f| !f.skip) {
        into.push_str(&format!("o.set({:?}, {path}::IntoJson::into_json(self.{}));\n", f.json, f.name));
        from.push_str(&format!(
            "if let Some(x) = v.get({:?}).and_then({path}::FromJson::from_json) {{ out.{} = x; }}\n",
            f.json, f.name
        ));
    }
    Ok(format!(
        "impl {path}::IntoJson for {name} {{\n\
             fn into_json(self) -> {path}::Json {{\n let mut o = {path}::Json::object();\n {into} o\n }}\n\
         }}\n\
         impl {path}::FromJson for {name} {{\n\
             fn from_json(v: &{path}::Json) -> ::core::option::Option<Self> {{\n\
                 if !matches!(v, {path}::Json::Object(_)) {{ return None; }}\n\
                 let mut out = <Self as ::core::default::Default>::default();\n {from} Some(out)\n }}\n\
         }}\n"
    ))
}

/// Fields `{ #[attr] pub name: Type, … }`: commas inside a type's `<…>` aren't separators.
fn fields(body: TokenStream) -> Result<Vec<Field>, String> {
    let mut out = Vec::new();
    let mut chunk: Vec<TokenTree> = Vec::new();
    let mut depth = 0i32;
    let mut arrow = false; // `->` in a type (`fn() -> T`) is not a closing bracket
    for t in body.into_iter().chain(std::iter::once(TokenTree::Punct(proc_macro::Punct::new(',', proc_macro::Spacing::Alone)))) {
        if let TokenTree::Punct(p) = &t {
            let after_dash = std::mem::replace(&mut arrow, p.as_char() == '-' && p.spacing() == proc_macro::Spacing::Joint);
            match p.as_char() {
                '<' => depth += 1,
                '>' if !after_dash => depth -= 1,
                ',' if depth == 0 => {
                    if !chunk.is_empty() {
                        out.push(field(std::mem::take(&mut chunk))?);
                    }
                    continue;
                }
                _ => {}
            }
        }
        chunk.push(t);
    }
    Ok(out)
}

fn field(tokens: Vec<TokenTree>) -> Result<Field, String> {
    let (mut json, mut skip) = (None, false);
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            TokenTree::Punct(p) if p.as_char() == '#' => {
                if let Some(TokenTree::Group(g)) = tokens.get(i + 1) {
                    attr(g.stream(), &mut json, &mut skip)?;
                }
                i += 2;
            }
            TokenTree::Ident(id) if id.to_string() == "pub" => {
                i += 1;
                if let Some(TokenTree::Group(g)) = tokens.get(i)
                    && g.delimiter() == Delimiter::Parenthesis
                {
                    i += 1;
                }
            }
            TokenTree::Ident(id) => {
                let name = id.to_string();
                let name = name.strip_prefix("r#").unwrap_or(&name).to_string();
                return Ok(Field { json: json.unwrap_or_else(|| name.clone()), name: id.to_string(), skip });
            }
            _ => return Err("Data: only structs with named fields".into()),
        }
    }
    Err("Data: field without a name".into())
}

/// `#[data(rename = "x")]`, `#[data(skip)]`; other attributes (`///`, `#[allow]`) are ignored.
fn attr(s: TokenStream, json: &mut Option<String>, skip: &mut bool) -> Result<(), String> {
    let mut it = s.into_iter();
    match it.next() {
        Some(TokenTree::Ident(i)) if i.to_string() == "data" => {}
        _ => return Ok(()),
    }
    let Some(TokenTree::Group(g)) = it.next() else { return Err("Data: #[data(rename = \"…\")] or #[data(skip)]".into()) };
    let t: Vec<TokenTree> = g.stream().into_iter().collect();
    match t.as_slice() {
        [TokenTree::Ident(i)] if i.to_string() == "skip" => *skip = true,
        [TokenTree::Ident(i), TokenTree::Punct(eq), TokenTree::Literal(l)] if i.to_string() == "rename" && eq.as_char() == '=' => {
            let s = l.to_string();
            *json = Some(s.trim_matches('"').to_string());
        }
        _ => return Err("Data: #[data(rename = \"…\")] or #[data(skip)]".into()),
    }
    Ok(())
}
