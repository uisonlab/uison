//! Values of complex JSON: parameters `$x`, bindings `@path`, interpolation
//! `"{@path}"`, `{ state }`/`{ switch }` selectors and value functions.
//!
//! Two steps:
//! - `subst` — substitution in the **caller's** context: `$param` → the parameter's
//!   value, `@item.x` (a For alias) → the absolute path `@todos.3.x`.
//!   Bindings stay bindings: that is how behaviors (toggle, input) get the path.
//! - `eval` — the final value: bindings are read from the model, selectors
//!   pick by state, theme tokens `$theme.name` are substituted.
//!
//! Parameters and theme tokens are separate namespaces: a button has a parameter
//! `text`, and the theme has a color `text`; `"$text"` is always the parameter.

use std::rc::Rc;

use uison_core::{Color, Point};

use crate::data::{Data, segments, step};
use crate::el::{Align, Dim, Insets};
use crate::library::Library;
use crate::state::{Flags, UiState};
use crate::{Error, Json};

#[derive(Clone, Debug)]
pub(crate) enum Alias {
    /// Alias of a model path (an array element).
    Path(String),
    /// Alias of a value (an element of a literal array, `index`).
    Value(Json),
}

#[derive(Clone)]
pub(crate) struct Scope {
    /// Key of the component instance — for local state `@~name`.
    pub inst: Rc<str>,
    pub params: Rc<Vec<(String, Json)>>,
    pub aliases: Rc<Vec<(String, Alias)>>,
    pub flags: Flags,
    pub disabled: bool,
}

impl Scope {
    pub fn root() -> Scope {
        Scope {
            inst: Rc::from("root"),
            params: Rc::new(Vec::new()),
            aliases: Rc::new(Vec::new()),
            flags: Flags::default(),
            disabled: false,
        }
    }

    pub fn param(&self, name: &str) -> Option<&Json> {
        self.params.iter().find(|(k, _)| k == name).map(|(_, v)| v)
    }

    pub fn with_alias(&self, name: &str, alias: Alias) -> Scope {
        let mut aliases = (*self.aliases).clone();
        aliases.push((name.to_string(), alias));
        Scope { aliases: Rc::new(aliases), ..self.clone() }
    }
}

/// Theme tokens by name.
pub(crate) type Theme = std::collections::HashMap<String, Json>;

pub(crate) struct Env<'a> {
    pub lib: &'a Library,
    pub theme: &'a Theme,
    pub data: &'a Data,
    pub st: &'a UiState,
}

enum Bound {
    Path(String),
    Value(Json),
}

fn is_name(s: &str) -> bool {
    s.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_') && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Path of instance-local state: `@~open` → `~.<instance key>.open`.
pub(crate) fn local_path(inst: &str, name: &str) -> String {
    format!("~.{inst}.{name}")
}

/// Resolve an alias at the start of a binding path (without `@`).
fn bind(expr: &str, sc: &Scope) -> Bound {
    // `~name` — instance-local state; `~.…` — an already resolved path.
    if let Some(name) = expr.strip_prefix('~')
        && !name.starts_with('.')
    {
        return Bound::Path(local_path(&sc.inst, name));
    }
    let (head, rest) = expr.split_once('.').unwrap_or((expr, ""));
    for (name, alias) in sc.aliases.iter().rev() {
        if name == head {
            return match alias {
                Alias::Path(p) if rest.is_empty() => Bound::Path(p.clone()),
                Alias::Path(p) => Bound::Path(format!("{p}.{rest}")),
                Alias::Value(v) => {
                    Bound::Value(segments(rest).try_fold(v, |v, s| step(v, s)).cloned().unwrap_or(Json::Null))
                }
            };
        }
    }
    Bound::Path(expr.to_string())
}

// ---------------------------------------------------------------- subst

pub(crate) fn subst(v: &Json, sc: &Scope) -> Json {
    subst_f(v, sc, true)
}

/// `freeze` — fix `state` selectors to the state of this scope: a value passed
/// as a parameter is written in the caller's context, and its state
/// (button hover) must apply inside the nested component (icon) as well.
/// Inside nodes (slots) we don't fix — they have their own state.
fn subst_f(v: &Json, sc: &Scope, freeze: bool) -> Json {
    match v {
        Json::String(s) => subst_str(s, sc),
        Json::Array(a) => Json::Array(a.iter().map(|x| subst_f(x, sc, freeze)).collect()),
        Json::Object(o) => {
            if freeze
                && let [(k, table)] = o.as_slice()
                && k == "state"
            {
                return subst_f(&pick_state(table, sc.flags).cloned().unwrap_or(Json::Null), sc, freeze);
            }
            let node = o.len() == 1 && o[0].0.starts_with(|c: char| c.is_ascii_uppercase());
            Json::Object(o.iter().map(|(k, x)| (k.clone(), subst_f(x, sc, freeze && !node))).collect())
        }
        _ => v.clone(),
    }
}

/// Branch of the `state` selector; priority disabled > pressed > focus > hover > normal.
fn pick_state(table: &Json, f: Flags) -> Option<&Json> {
    let pick = [("disabled", f.disabled), ("pressed", f.pressed), ("focus", f.focus), ("hover", f.hover)]
        .into_iter()
        .find(|(k, on)| *on && table.get(k).is_some())
        .map_or("normal", |(k, _)| k);
    table.get(pick)
}

/// `$name` or `$name.field.field` → (name, path to the field).
fn param_ref(s: &str) -> Option<(&str, Option<&str>)> {
    let rest = s.strip_prefix('$')?;
    let (name, path) = match rest.split_once('.') {
        Some((n, p)) => (n, Some(p)),
        None => (rest, None),
    };
    (is_name(name) && name != "theme").then_some((name, path))
}

/// Field of a parameter value: for a binding — a longer path, for a literal — the nested value.
fn param_field(p: &Json, path: Option<&str>) -> Json {
    let Some(path) = path else { return p.clone() };
    match p {
        Json::String(s) if s.starts_with('@') || s.starts_with("!@") => Json::from(format!("{s}.{path}")),
        _ => segments(path).try_fold(p, |v, seg| step(v, seg)).cloned().unwrap_or(Json::Null),
    }
}

/// `$$x`, `@@x`, `!!@x` — literal text starting with `$`, `@`, `!@`: the first character is
/// dropped when the value is evaluated (like `{{` for a brace).
fn escaped(s: &str) -> Option<&str> {
    (s.starts_with("$$") || s.starts_with("@@") || s.starts_with("!!@")).then(|| &s[1..])
}

/// Text from outside (a web page, user input) as a literal string value: braces are doubled and
/// a leading `$`, `@`, `!@` is escaped, so the text is never taken for a binding or a parameter.
pub fn literal(s: &str) -> String {
    let s = escape_braces(s);
    if s.starts_with(['$', '@']) || s.starts_with("!@") { format!("{}{s}", &s[..1]) } else { s }
}

fn subst_str(s: &str, sc: &Scope) -> Json {
    if escaped(s).is_some() {
        // Resolved in eval: here it must not be taken for `$param` or `@path`.
        return Json::from(s.to_string());
    }
    if let Some((name, path)) = param_ref(s)
        && let Some(p) = sc.param(name)
    {
        return param_field(p, path);
    }
    if let Some(e) = s.strip_prefix("!@") {
        return match bind(e, sc) {
            Bound::Path(p) => Json::from(format!("!@{p}")),
            Bound::Value(v) => Json::Bool(!v.truthy()),
        };
    }
    if let Some(e) = s.strip_prefix('@') {
        return match bind(e, sc) {
            Bound::Path(p) => Json::from(format!("@{p}")),
            Bound::Value(v) => v,
        };
    }
    if s.contains('{') {
        // Escaped braces stay escaped: eval unescapes them (once).
        return Json::from(interpolate_keep(s, true, |expr| {
            Some(match subst_str(expr, sc) {
                // A binding or a token stays an expression — evaluated in eval.
                Json::String(r) if r.starts_with(['@', '$', '!']) => format!("{{{r}}}"),
                v => escape_braces(&to_text(&v)),
            })
        }));
    }
    Json::from(s.to_string())
}

fn escape_braces(s: &str) -> String {
    // Literal text inside interpolation must not become an expression again.
    s.replace('{', "{{").replace('}', "}}")
}

/// Replace `{expr}` (expr starts with `@`, `$`, `!@`) via `f`; `{{`/`}}` are literal braces.
fn interpolate(s: &str, f: impl FnMut(&str) -> Option<String>) -> String {
    interpolate_keep(s, false, f)
}

/// `keep`: leave `{{`/`}}` as they are (substituting parameters before eval).
fn interpolate_keep(s: &str, keep: bool, mut f: impl FnMut(&str) -> Option<String>) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find(['{', '}']) {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        if tail.starts_with("{{") || tail.starts_with("}}") {
            out.push_str(&tail[..if keep { 2 } else { 1 }]);
            rest = &tail[2..];
            continue;
        }
        if tail.starts_with('{')
            && let Some(end) = tail.find('}')
        {
            let expr = &tail[1..end];
            if expr.starts_with(['@', '$', '!'])
                && let Some(r) = f(expr)
            {
                out.push_str(&r);
                rest = &tail[end + 1..];
                continue;
            }
        }
        out.push_str(&tail[..1]);
        rest = &tail[1..];
    }
    out.push_str(rest);
    out
}

// ---------------------------------------------------------------- eval

const MAX_DEPTH: u32 = 32;

pub(crate) fn eval(v: &Json, sc: &Scope, env: &Env) -> Result<Json, Error> {
    eval_d(v, sc, env, 0)
}

/// Truthiness of a value without copying bound data (`visible: "!@inbox.view"`
/// must not copy an array of 2000 mails just to check it for emptiness).
pub(crate) fn truthy(v: &Json, sc: &Scope, env: &Env) -> Result<bool, Error> {
    if let Json::String(s) = v
        && escaped(s).is_none()
    {
        let lookup_truthy = |e: &str| match bind(e, sc) {
            Bound::Path(p) => env.data.get(&p).is_some_and(Json::truthy),
            Bound::Value(v) => v.truthy(),
        };
        if let Some(e) = s.strip_prefix("!@") {
            return Ok(!lookup_truthy(e));
        }
        if let Some(e) = s.strip_prefix('@') {
            return Ok(lookup_truthy(e));
        }
        if let Some((name, path)) = param_ref(s)
            && let Some(p) = sc.param(name)
        {
            return truthy(&param_field(p, path), sc, env);
        }
    }
    Ok(eval(v, sc, env)?.truthy())
}

fn lookup(expr: &str, sc: &Scope, env: &Env) -> Json {
    match bind(expr, sc) {
        Bound::Path(p) => env.data.get(&p).cloned().unwrap_or(Json::Null),
        Bound::Value(v) => v,
    }
}

fn eval_d(v: &Json, sc: &Scope, env: &Env, depth: u32) -> Result<Json, Error> {
    if depth > MAX_DEPTH {
        return Err(Error::new("value nesting too deep (cyclic $parameter or theme token?)"));
    }
    match v {
        Json::String(s) => eval_str(s, sc, env, depth),
        Json::Array(a) => a.iter().map(|x| eval_d(x, sc, env, depth + 1)).collect::<Result<_, _>>().map(Json::Array),
        Json::Object(o) => {
            let Some((first, _)) = o.first() else { return Ok(v.clone()) };
            match &**first {
                "state" | "switch" | "ratio" | "eq" | "lt" | "gt" | "not" | "initials" | "pick" | "len" => {
                    eval_fn(v, sc, env, depth)
                }
                _ => o
                    .iter()
                    .map(|(k, x)| Ok((k.clone(), eval_d(x, sc, env, depth + 1)?)))
                    .collect::<Result<_, Error>>()
                    .map(Json::Object),
            }
        }
        _ => Ok(v.clone()),
    }
}

fn eval_str(s: &str, sc: &Scope, env: &Env, depth: u32) -> Result<Json, Error> {
    if let Some(text) = escaped(s) {
        return Ok(Json::from(if text.contains(['{', '}']) { interpolate(text, |_| None) } else { text.to_string() }));
    }
    if let Some(token) = s.strip_prefix("$theme.") {
        return match env.theme.get(token) {
            Some(t) => eval_d(t, sc, env, depth + 1),
            None => Err(Error::new(format!("unknown theme token $theme.{token}"))),
        };
    }
    if let Some((name, path)) = param_ref(s) {
        return match sc.param(name) {
            Some(p) => eval_d(&param_field(p, path), sc, env, depth + 1),
            None => Err(Error::new(format!("unknown parameter ${name}"))),
        };
    }
    if let Some(e) = s.strip_prefix("!@") {
        return Ok(Json::Bool(!lookup(e, sc, env).truthy()));
    }
    if let Some(e) = s.strip_prefix('@') {
        return Ok(lookup(e, sc, env));
    }
    if s.contains(['{', '}']) {
        let mut err = None;
        let out = interpolate(s, |expr| match eval_str(expr, sc, env, depth + 1) {
            Ok(v) => Some(to_text(&v)),
            Err(e) => {
                err = Some(e);
                None
            }
        });
        return match err {
            Some(e) => Err(e),
            None => Ok(Json::from(out)),
        };
    }
    Ok(Json::from(s.to_string()))
}

/// Value functions and selectors. This is not code: only picking from a table
/// and simple data transformations for display.
fn eval_fn(v: &Json, sc: &Scope, env: &Env, depth: u32) -> Result<Json, Error> {
    let arg = |k: &str| -> Result<Json, Error> {
        v.get(k).map_or(Ok(Json::Null), |x| eval_d(x, sc, env, depth + 1))
    };
    let (name, _) = v.as_object().unwrap().first().unwrap();
    Ok(match &**name {
        // Value by state; priority disabled > pressed > focus > hover > normal.
        "state" => match pick_state(v.get("state").unwrap(), sc.flags) {
            Some(x) => eval_d(x, sc, env, depth + 1)?,
            None => Json::Null,
        },
        "switch" => {
            let key = to_text(&arg("switch")?);
            match v.get("cases").and_then(|c| c.get(&key)).or_else(|| v.get("default")) {
                Some(x) => eval_d(x, sc, env, depth + 1)?,
                None => Json::Null,
            }
        }
        // Fraction of the value within a range, 0..=1 (Slider, ProgressBar fill).
        "ratio" => {
            let x = arg("ratio")?.as_f64().unwrap_or(0.0);
            let min = arg("min")?.as_f64().unwrap_or(0.0);
            let max = arg("max")?.as_f64().unwrap_or(1.0);
            let r = if max > min { ((x - min) / (max - min)).clamp(0.0, 1.0) } else { 0.0 };
            Json::from(if arg("invert")?.truthy() { 1.0 - r } else { r })
        }
        "eq" => {
            let pair = arg("eq")?;
            let a = pair.as_array().filter(|a| a.len() == 2).ok_or_else(|| Error::new("eq expects [a, b]"))?;
            Json::Bool(to_text(&a[0]) == to_text(&a[1]))
        }
        // Number comparison: { lt: ["@env.width", 600] } — narrow screen.
        "lt" | "gt" => {
            let pair = arg(name)?;
            let a = pair.as_array().filter(|a| a.len() == 2).ok_or_else(|| Error::new(format!("{name} expects [a, b]")))?;
            let (x, y) = (a[0].as_f64().unwrap_or(0.0), a[1].as_f64().unwrap_or(0.0));
            Json::Bool(if &**name == "lt" { x < y } else { x > y })
        }
        "not" => Json::Bool(!arg("not")?.truthy()),
        // Initials: first letters of the first two words.
        "initials" => Json::from(
            to_text(&arg("initials")?)
                .split_whitespace()
                .take(2)
                .filter_map(|w| w.chars().next())
                .flat_map(char::to_uppercase)
                .collect::<String>(),
        ),
        // A list item picked by the string's hash (avatar color).
        "pick" => {
            let key = to_text(&arg("pick")?);
            let from = arg("from")?;
            let list = from.as_array().filter(|a| !a.is_empty()).ok_or_else(|| Error::new("pick needs non-empty 'from'"))?;
            let h = key.bytes().fold(5381u32, |h, b| h.wrapping_mul(33) ^ b as u32);
            list[h as usize % list.len()].clone()
        }
        "len" => Json::from(match arg("len")? {
            Json::Array(a) => a.len(),
            Json::String(s) => s.chars().count(),
            Json::Object(o) => o.len(),
            _ => 0,
        }),
        _ => unreachable!(),
    })
}

/// For nodes: the `state`/`switch` selector only picks a branch without evaluating
/// its content (it may contain For and bindings that are evaluated later).
pub(crate) fn select_branch(v: &Json, sc: &Scope, env: &Env) -> Result<Json, Error> {
    let Some([(k, x)]) = v.as_object().map(|o| &o[..1.min(o.len())]) else { return Ok(v.clone()) };
    let chosen = match &**k {
        "state" => pick_state(x, sc.flags).cloned().unwrap_or(Json::Null),
        "switch" => {
            let key = to_text(&eval(x, sc, env)?);
            v.get("cases").and_then(|c| c.get(&key)).or_else(|| v.get("default")).cloned().unwrap_or(Json::Null)
        }
        _ => return Ok(v.clone()),
    };
    select_branch(&chosen, sc, env)
}

/// Value as text (for interpolation and `switch` keys).
pub(crate) fn to_text(v: &Json) -> String {
    match v {
        Json::Null => String::new(),
        Json::Bool(b) => b.to_string(),
        Json::Number(n) if n.fract() == 0.0 && n.abs() < 1e15 => (*n as i64).to_string(),
        Json::Number(n) => n.to_string(),
        Json::String(s) => s.to_string(),
        _ => v.to_json(),
    }
}

// ---------------------------------------------------------------- typed values

fn bad(prop: &str, want: &str, v: &Json) -> Error {
    Error::new(format!("'{prop}' must be {want}, got {}", v.to_json()))
}

pub(crate) fn num(v: &Json, prop: &str) -> Result<f32, Error> {
    v.as_f64().map(|n| n as f32).ok_or_else(|| bad(prop, "a number", v))
}

pub(crate) fn opt_num(v: &Json, prop: &str) -> Result<Option<f32>, Error> {
    if v.is_null() { Ok(None) } else { num(v, prop).map(Some) }
}

/// Color; `null` — no color. Fully transparent counts as absent.
pub(crate) fn color(v: &Json, prop: &str) -> Result<Option<Color>, Error> {
    match v {
        Json::Null => Ok(None),
        Json::String(s) => {
            let c = Color::from_hex(s).ok_or_else(|| bad(prop, "a color #RGB, #RRGGBB or #RRGGBBAA", v))?;
            Ok((c.a != 0).then_some(c))
        }
        _ => Err(bad(prop, "a color", v)),
    }
}

pub(crate) fn insets(v: &Json, prop: &str) -> Result<Insets, Error> {
    let n = |x: &Json| num(x, prop);
    match v {
        Json::Null => Ok(Insets::default()),
        Json::Number(_) => {
            let a = n(v)?;
            Ok(Insets { t: a, r: a, b: a, l: a })
        }
        Json::Array(a) if a.len() == 2 => {
            let (y, x) = (n(&a[0])?, n(&a[1])?);
            Ok(Insets { t: y, r: x, b: y, l: x })
        }
        Json::Array(a) if a.len() == 4 => Ok(Insets { t: n(&a[0])?, r: n(&a[1])?, b: n(&a[2])?, l: n(&a[3])? }),
        _ => Err(bad(prop, "a number, [vertical, horizontal] or [top, right, bottom, left]", v)),
    }
}

pub(crate) fn dim(v: &Json, prop: &str) -> Result<Dim, Error> {
    match v {
        Json::Null => Ok(Dim::Hug),
        Json::Number(n) => Ok(Dim::Px(*n as f32)),
        Json::String(s) => match &**s {
            "fill" => Ok(Dim::Fill),
            "hug" => Ok(Dim::Hug),
            s => s
                .strip_suffix('%')
                .and_then(|p| p.trim().parse::<f32>().ok())
                .map(|p| Dim::Pct(p / 100.0))
                .ok_or_else(|| bad(prop, "a number, \"fill\", \"hug\" or \"N%\"", v)),
        },
        _ => Err(bad(prop, "a size", v)),
    }
}

pub(crate) fn align(v: &Json, prop: &str) -> Result<Align, Error> {
    v.as_str().and_then(Align::parse).ok_or_else(|| {
        bad(prop, "one of topLeft, top, topRight, left, center, right, bottomLeft, bottom, bottomRight", v)
    })
}

pub(crate) fn point(v: &Json, prop: &str) -> Result<Point, Error> {
    match v.as_array() {
        Some([x, y]) => Ok(Point::new(num(x, prop)?, num(y, prop)?)),
        _ => Err(bad(prop, "[x, y]", v)),
    }
}

pub(crate) fn string(v: &Json) -> String {
    to_text(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_eval(src: &str, sc: &Scope, data: &Data) -> Json {
        let lib = Library::default();
        let theme: Theme = [("primary".to_string(), Json::from("#007AFF")), ("alias".to_string(), Json::from("$theme.primary"))]
            .into_iter()
            .collect();
        let st = UiState::default();
        let env = Env { lib: &lib, theme: &theme, data, st: &st };
        eval(&uison_json::parse(src).unwrap().into_owned(), sc, &env).unwrap()
    }

    #[test]
    fn params_bindings_interpolation() {
        let data = Data::new(uison_json::parse(r#"{ user: { name: "Anna" }, todos: [{ t: "a" }, { t: "b" }] }"#).unwrap().into_owned());
        let caller = Scope::root().with_alias("item", Alias::Path("todos.1".into()));
        // subst at the caller: alias → absolute path, a binding stays a binding.
        let p = subst(&Json::from("@item.t"), &caller);
        assert_eq!(p, Json::from("@todos.1.t"));
        let body = Scope { params: Rc::new(vec![("text".into(), p), ("n".into(), Json::from(3))]), ..Scope::root() };
        assert_eq!(env_eval(r#""$text""#, &body, &data), Json::from("b"));
        assert_eq!(env_eval(r#""Hello, {@user.name}! {$n} pcs.""#, &body, &data), Json::from("Hello, Anna! 3 pcs."));
        assert_eq!(env_eval(r#""{{literal}}""#, &body, &data), Json::from("{literal}"));
        assert_eq!(env_eval(r#""$theme.alias""#, &body, &data), Json::from("#007AFF"));
        assert_eq!(env_eval(r#""!@user.missing""#, &body, &data), Json::Bool(true));
        // Parameter field: for a binding — a longer path, for a literal — the nested value.
        let sc = Scope {
            params: Rc::new(vec![
                ("todo".into(), Json::from("@todos.0")),
                ("lit".into(), uison_json::parse(r#"{ a: { b: 7 } }"#).unwrap().into_owned()),
            ]),
            ..Scope::root()
        };
        assert_eq!(subst(&Json::from("$todo.t"), &sc), Json::from("@todos.0.t"));
        assert_eq!(env_eval(r#""$todo.t""#, &sc, &data), Json::from("a"));
        assert_eq!(env_eval(r#""{$lit.a.b} pcs.""#, &sc, &data), Json::from("7 pcs."));
        // Interpolation at the caller with a literal containing braces.
        let sc = Scope { params: Rc::new(vec![("x".into(), Json::from("{a}"))]), ..Scope::root() };
        let s = subst(&Json::from("v={$x}"), &sc);
        assert_eq!(env_eval(&s.to_json(), &Scope::root(), &data), Json::from("v={a}"));
    }

    #[test]
    fn selectors_and_functions() {
        let data = Data::default();
        let mut sc = Scope::root();
        let src = r##"{ state: { normal: "a", hover: "b", pressed: "c", disabled: "d" } }"##;
        assert_eq!(env_eval(src, &sc, &data), Json::from("a"));
        sc.flags.hover = true;
        assert_eq!(env_eval(src, &sc, &data), Json::from("b"));
        sc.flags.pressed = true;
        assert_eq!(env_eval(src, &sc, &data), Json::from("c"));
        sc.flags.disabled = true;
        assert_eq!(env_eval(src, &sc, &data), Json::from("d"));
        let sc = Scope { params: Rc::new(vec![("v".into(), Json::from("primary"))]), ..Scope::root() };
        assert_eq!(env_eval(r#"{ switch: "$v", cases: { primary: 1 }, default: 2 }"#, &sc, &data), Json::from(1));
        assert_eq!(env_eval(r#"{ switch: true, cases: { true: "yes" } }"#, &sc, &data), Json::from("yes"));
        assert_eq!(env_eval(r#"{ ratio: 25, min: 0, max: 100 }"#, &sc, &data), Json::from(0.25));
        assert_eq!(env_eval(r#"{ ratio: 25, min: 0, max: 100, invert: true }"#, &sc, &data), Json::from(0.75));
        assert_eq!(env_eval(r#"{ eq: ["$v", "primary"] }"#, &sc, &data), Json::Bool(true));
        assert_eq!(env_eval(r#"{ initials: "анна петрова" }"#, &sc, &data), Json::from("АП")); // Cyrillic on purpose: multibyte first letters, non-ASCII uppercase
        assert_eq!(env_eval(r#"{ len: [1, 2, 3] }"#, &sc, &data), Json::from(3));
    }
}
