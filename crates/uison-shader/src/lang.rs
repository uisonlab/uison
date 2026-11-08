//! The language: source text → a typed tree. Every back end (SPIR-V now; MSL, HLSL, AGSL, a CPU
//! interpreter later) reads the same tree, so a program means the same thing everywhere.
//!
//! Types are checked here, and the places where GLSL converts silently are made explicit:
//! a float next to a vector becomes `Splat` (`v * 2.0`, `mix(a, b, 0.5)`).

use std::fmt;

/// `Float(1)` is `float`, `Float(2..=4)` is `vec2`..`vec4`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ty {
    Bool,
    Float(u8),
}

impl Ty {
    pub const FLOAT: Ty = Ty::Float(1);

    /// Components of a float type (0 for bool).
    pub fn n(self) -> u8 {
        match self {
            Ty::Float(n) => n,
            Ty::Bool => 0,
        }
    }
}

impl fmt::Display for Ty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Ty::Bool => f.write_str("bool"),
            Ty::Float(1) => f.write_str("float"),
            Ty::Float(n) => write!(f, "vec{n}"),
        }
    }
}

/// What the program reads about the pixel and the item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    /// vec2: 0..1 across the item's box, (0, 0) at the top left.
    Uv,
    /// vec2: points from the box's top left.
    Pos,
    /// vec2: the box's size in points.
    Size,
    /// float: the item's `time`, seconds.
    Time,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmp {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
}

/// Built-in functions, as in GLSL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Func {
    Abs,
    Sign,
    Floor,
    Ceil,
    Fract,
    Round,
    Trunc,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    /// `atan(y, x)`.
    Atan2,
    Sinh,
    Cosh,
    Tanh,
    Pow,
    Exp,
    Log,
    Exp2,
    Log2,
    Sqrt,
    InverseSqrt,
    Radians,
    Degrees,
    Mod,
    Min,
    Max,
    Clamp,
    Mix,
    Step,
    SmoothStep,
    Length,
    Distance,
    Dot,
    Cross,
    Normalize,
    Reflect,
}

/// How a function's arguments and result are typed.
#[derive(Clone, Copy)]
enum Shape {
    /// N arguments of one float type (floats among vectors are splatted), the result of that type.
    Each(usize),
    /// N arguments of one float type, a float result.
    ToFloat(usize),
    /// `cross(vec3, vec3)`.
    Cross,
}

const FUNCS: &[(&str, Func, Shape)] = &[
    ("abs", Func::Abs, Shape::Each(1)),
    ("sign", Func::Sign, Shape::Each(1)),
    ("floor", Func::Floor, Shape::Each(1)),
    ("ceil", Func::Ceil, Shape::Each(1)),
    ("fract", Func::Fract, Shape::Each(1)),
    ("round", Func::Round, Shape::Each(1)),
    ("trunc", Func::Trunc, Shape::Each(1)),
    ("sin", Func::Sin, Shape::Each(1)),
    ("cos", Func::Cos, Shape::Each(1)),
    ("tan", Func::Tan, Shape::Each(1)),
    ("asin", Func::Asin, Shape::Each(1)),
    ("acos", Func::Acos, Shape::Each(1)),
    ("atan", Func::Atan, Shape::Each(1)),
    ("sinh", Func::Sinh, Shape::Each(1)),
    ("cosh", Func::Cosh, Shape::Each(1)),
    ("tanh", Func::Tanh, Shape::Each(1)),
    ("pow", Func::Pow, Shape::Each(2)),
    ("exp", Func::Exp, Shape::Each(1)),
    ("log", Func::Log, Shape::Each(1)),
    ("exp2", Func::Exp2, Shape::Each(1)),
    ("log2", Func::Log2, Shape::Each(1)),
    ("sqrt", Func::Sqrt, Shape::Each(1)),
    ("inversesqrt", Func::InverseSqrt, Shape::Each(1)),
    ("radians", Func::Radians, Shape::Each(1)),
    ("degrees", Func::Degrees, Shape::Each(1)),
    ("mod", Func::Mod, Shape::Each(2)),
    ("min", Func::Min, Shape::Each(2)),
    ("max", Func::Max, Shape::Each(2)),
    ("clamp", Func::Clamp, Shape::Each(3)),
    ("mix", Func::Mix, Shape::Each(3)),
    ("step", Func::Step, Shape::Each(2)),
    ("smoothstep", Func::SmoothStep, Shape::Each(3)),
    ("length", Func::Length, Shape::ToFloat(1)),
    ("distance", Func::Distance, Shape::ToFloat(2)),
    ("dot", Func::Dot, Shape::ToFloat(2)),
    ("cross", Func::Cross, Shape::Cross),
    ("normalize", Func::Normalize, Shape::Each(1)),
    ("reflect", Func::Reflect, Shape::Each(2)),
];

/// The names a program can read; `pi` is a constant.
const INPUTS: &[(&str, Input, Ty)] =
    &[("uv", Input::Uv, Ty::Float(2)), ("pos", Input::Pos, Ty::Float(2)), ("size", Input::Size, Ty::Float(2)), ("time", Input::Time, Ty::FLOAT)];

#[derive(Clone, Debug, PartialEq)]
pub struct Expr {
    pub kind: Kind,
    pub ty: Ty,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Num(f32),
    /// A vector constant (a color literal).
    Const(Vec<f32>),
    Input(Input),
    /// The value of the program's `let` number `i`.
    Let(usize),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    /// A float repeated into the vector type `ty`.
    Splat(Box<Expr>),
    /// Operands of the same float type.
    Arith(Op, Box<Expr>, Box<Expr>),
    /// Floats → bool.
    Compare(Cmp, Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    /// `c ? a : b`: a bool and two values of type `ty`.
    Select(Box<Expr>, Box<Expr>, Box<Expr>),
    /// Components (0..4) of a vector, two or more.
    Swizzle(Box<Expr>, Vec<u8>),
    /// One component of a vector.
    Component(Box<Expr>, u8),
    /// `vecN(…)`: floats and vectors whose components add up to N.
    Construct(Vec<Expr>),
    /// Arguments already of the types the function takes.
    Call(Func, Vec<Expr>),
}

/// `let` values in order, then the result: `vec4` (straight alpha), `vec3` (opaque) or `float` (gray).
#[derive(Clone, Debug, PartialEq)]
pub struct Program {
    pub lets: Vec<Expr>,
    pub result: Expr,
}

/// A mistake in the program: `at` is the character it was found at (from 0).
#[derive(Clone, Debug, PartialEq)]
pub struct Error {
    pub at: usize,
    pub msg: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "col {}: {}", self.at + 1, self.msg)
    }
}

impl std::error::Error for Error {}

/// Longest program: shaders are one-liners or a few lines, not files.
const MAX_LEN: usize = 16 * 1024;
/// Deepest nesting of parentheses and operators.
const MAX_DEPTH: u32 = 64;

pub fn parse(src: &str) -> Result<Program, Error> {
    if src.len() > MAX_LEN {
        return Err(Error { at: 0, msg: format!("the program is longer than {MAX_LEN} bytes") });
    }
    let toks = lex(src)?;
    let mut p = Parser { toks, i: 0, lets: Vec::new(), names: Vec::new(), depth: 0 };
    while p.peek_ident() == Some("let") {
        p.i += 1;
        let at = p.at();
        let name = match p.next() {
            Tok::Ident(n) => n,
            _ => return Err(Error { at, msg: "expected a name after 'let'".into() }),
        };
        if INPUTS.iter().any(|i| i.0 == name) || name == "pi" || name == "let" {
            return Err(Error { at, msg: format!("'{name}' is taken: choose another name") });
        }
        p.expect("=")?;
        let value = p.expr()?;
        p.expect(";")?;
        p.names.push((name, p.lets.len(), value.ty));
        p.lets.push(value);
    }
    let at = p.at();
    let result = p.expr()?;
    if p.toks[p.i].0 != Tok::End {
        return Err(Error { at: p.at(), msg: "expected the end of the program (did you miss an operator or a ';'?)".into() });
    }
    if !matches!(result.ty, Ty::Float(1 | 3 | 4)) {
        return Err(Error { at, msg: format!("the program gives {}: it must give a color — vec4 (rgba), vec3 (rgb) or float (gray)", result.ty) });
    }
    Ok(Program { lets: p.lets, result })
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Num(f32),
    Color([f32; 4]),
    Ident(String),
    Punct(&'static str),
    End,
}

const PUNCT2: [&str; 6] = ["<=", ">=", "==", "!=", "&&", "||"];
const PUNCT1: [&str; 15] = ["+", "-", "*", "/", "(", ")", ",", ".", ";", "=", "?", ":", "<", ">", "!"];

fn lex(src: &str) -> Result<Vec<(Tok, usize)>, Error> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let start = i;
        if c.is_whitespace() {
            i += 1;
        } else if c == '/' && chars.get(i + 1) == Some(&'/') {
            // A comment to the end of the line.
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit)) {
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
                let mut j = i + 1;
                if j < chars.len() && (chars[j] == '+' || chars[j] == '-') {
                    j += 1;
                }
                if j < chars.len() && chars[j].is_ascii_digit() {
                    i = j;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            let text: String = chars[start..i].iter().collect();
            let v = text.parse::<f32>().map_err(|_| Error { at: start, msg: format!("'{text}' is not a number") })?;
            out.push((Tok::Num(v), start));
        } else if c.is_ascii_alphabetic() || c == '_' {
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push((Tok::Ident(chars[start..i].iter().collect()), start));
        } else if c == '#' {
            i += 1;
            while i < chars.len() && chars[i].is_ascii_hexdigit() {
                i += 1;
            }
            let hex: Vec<f32> = chars[start + 1..i].iter().map(|c| c.to_digit(16).unwrap() as f32).collect();
            let rgba = match hex.len() {
                3 | 4 => {
                    let mut v = [1.0; 4];
                    for (k, d) in hex.iter().enumerate() {
                        v[k] = d * 17.0 / 255.0;
                    }
                    v
                }
                6 | 8 => {
                    let mut v = [1.0; 4];
                    for (k, d) in hex.chunks(2).enumerate() {
                        v[k] = (d[0] * 16.0 + d[1]) / 255.0;
                    }
                    v
                }
                _ => return Err(Error { at: start, msg: "a color is #RGB, #RGBA, #RRGGBB or #RRGGBBAA".into() }),
            };
            out.push((Tok::Color(rgba), start));
        } else if let Some(p) = PUNCT2.iter().find(|p| chars[i..].starts_with(&p.chars().collect::<Vec<_>>())) {
            out.push((Tok::Punct(p), start));
            i += 2;
        } else if let Some(p) = PUNCT1.iter().find(|p| p.starts_with(c)) {
            out.push((Tok::Punct(p), start));
            i += 1;
        } else {
            return Err(Error { at: start, msg: format!("unexpected '{c}'") });
        }
    }
    out.push((Tok::End, chars.len()));
    Ok(out)
}

struct Parser {
    toks: Vec<(Tok, usize)>,
    i: usize,
    lets: Vec<Expr>,
    /// `let` names in scope: (name, index, type); later ones shadow earlier ones.
    names: Vec<(String, usize, Ty)>,
    depth: u32,
}

fn bx(e: Expr) -> Box<Expr> {
    Box::new(e)
}

impl Parser {
    fn at(&self) -> usize {
        self.toks[self.i].1
    }

    fn next(&mut self) -> Tok {
        let t = self.toks[self.i].0.clone();
        if t != Tok::End {
            self.i += 1;
        }
        t
    }

    fn peek_punct(&self) -> Option<&'static str> {
        match self.toks[self.i].0 {
            Tok::Punct(p) => Some(p),
            _ => None,
        }
    }

    fn peek_ident(&self) -> Option<&str> {
        match &self.toks[self.i].0 {
            Tok::Ident(n) => Some(n),
            _ => None,
        }
    }

    fn eat(&mut self, p: &str) -> bool {
        if self.peek_punct() == Some(p) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, p: &str) -> Result<(), Error> {
        if self.eat(p) { Ok(()) } else { Err(Error { at: self.at(), msg: format!("expected '{p}'") }) }
    }

    fn expr(&mut self) -> Result<Expr, Error> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(Error { at: self.at(), msg: "too deeply nested".into() });
        }
        let e = self.ternary();
        self.depth -= 1;
        e
    }

    fn ternary(&mut self) -> Result<Expr, Error> {
        let at = self.at();
        let c = self.or()?;
        if !self.eat("?") {
            return Ok(c);
        }
        if c.ty != Ty::Bool {
            return Err(Error { at, msg: format!("the condition before '?' is {}, not bool (a comparison)", c.ty) });
        }
        let at_a = self.at();
        let a = self.expr()?;
        self.expect(":")?;
        let b = self.expr()?;
        let (a, b) = unify(a, b).ok_or_else(|| Error { at: at_a, msg: "both sides of ':' must have the same type".into() })?;
        let ty = a.ty;
        Ok(Expr { kind: Kind::Select(bx(c), bx(a), bx(b)), ty })
    }

    fn or(&mut self) -> Result<Expr, Error> {
        let mut l = self.and()?;
        while self.peek_punct() == Some("||") {
            let at = self.at();
            self.i += 1;
            let r = self.and()?;
            logic_operands(&l, &r, "||", at)?;
            l = Expr { kind: Kind::Or(bx(l), bx(r)), ty: Ty::Bool };
        }
        Ok(l)
    }

    fn and(&mut self) -> Result<Expr, Error> {
        let mut l = self.compare()?;
        while self.peek_punct() == Some("&&") {
            let at = self.at();
            self.i += 1;
            let r = self.compare()?;
            logic_operands(&l, &r, "&&", at)?;
            l = Expr { kind: Kind::And(bx(l), bx(r)), ty: Ty::Bool };
        }
        Ok(l)
    }

    fn compare(&mut self) -> Result<Expr, Error> {
        let l = self.add()?;
        let op = match self.peek_punct() {
            Some("<") => Cmp::Lt,
            Some("<=") => Cmp::Le,
            Some(">") => Cmp::Gt,
            Some(">=") => Cmp::Ge,
            Some("==") => Cmp::Eq,
            Some("!=") => Cmp::Ne,
            _ => return Ok(l),
        };
        let at = self.at();
        self.i += 1;
        let r = self.add()?;
        if l.ty != Ty::FLOAT || r.ty != Ty::FLOAT {
            return Err(Error { at, msg: format!("comparisons take floats, not {} and {} (compare components: v.x < 0.5)", l.ty, r.ty) });
        }
        Ok(Expr { kind: Kind::Compare(op, bx(l), bx(r)), ty: Ty::Bool })
    }

    fn add(&mut self) -> Result<Expr, Error> {
        let mut l = self.mul()?;
        loop {
            let op = match self.peek_punct() {
                Some("+") => Op::Add,
                Some("-") => Op::Sub,
                _ => return Ok(l),
            };
            let at = self.at();
            self.i += 1;
            let r = self.mul()?;
            l = arith(op, l, r, at)?;
        }
    }

    fn mul(&mut self) -> Result<Expr, Error> {
        let mut l = self.unary()?;
        loop {
            let op = match self.peek_punct() {
                Some("*") => Op::Mul,
                Some("/") => Op::Div,
                _ => return Ok(l),
            };
            let at = self.at();
            self.i += 1;
            let r = self.unary()?;
            l = arith(op, l, r, at)?;
        }
    }

    fn unary(&mut self) -> Result<Expr, Error> {
        let at = self.at();
        if self.eat("-") {
            let e = self.unary()?;
            if e.ty == Ty::Bool {
                return Err(Error { at, msg: "'-' takes a number, not bool".into() });
            }
            let ty = e.ty;
            return Ok(Expr { kind: Kind::Neg(bx(e)), ty });
        }
        if self.eat("!") {
            let e = self.unary()?;
            if e.ty != Ty::Bool {
                return Err(Error { at, msg: format!("'!' takes bool, not {}", e.ty) });
            }
            return Ok(Expr { kind: Kind::Not(bx(e)), ty: Ty::Bool });
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Expr, Error> {
        let mut e = self.primary()?;
        while self.eat(".") {
            let at = self.at();
            let Tok::Ident(name) = self.next() else {
                return Err(Error { at, msg: "expected components after '.' (x, xy, rgb…)".into() });
            };
            e = swizzle(e, &name, at)?;
        }
        Ok(e)
    }

    fn primary(&mut self) -> Result<Expr, Error> {
        let at = self.at();
        match self.next() {
            Tok::Num(v) => Ok(Expr { kind: Kind::Num(v), ty: Ty::FLOAT }),
            Tok::Color(c) => Ok(Expr { kind: Kind::Const(c.to_vec()), ty: Ty::Float(4) }),
            Tok::Punct("(") => {
                let e = self.expr()?;
                self.expect(")")?;
                Ok(e)
            }
            Tok::Ident(name) => {
                if self.peek_punct() == Some("(") {
                    self.i += 1;
                    let mut args = Vec::new();
                    if !self.eat(")") {
                        loop {
                            args.push((self.at(), self.expr()?));
                            if self.eat(")") {
                                break;
                            }
                            self.expect(",")?;
                        }
                    }
                    return call(&name, args, at);
                }
                if let Some(&(_, i, ty)) = self.names.iter().rev().find(|n| n.0 == name) {
                    return Ok(Expr { kind: Kind::Let(i), ty });
                }
                if let Some(&(_, input, ty)) = INPUTS.iter().find(|i| i.0 == name) {
                    return Ok(Expr { kind: Kind::Input(input), ty });
                }
                if name == "pi" {
                    return Ok(Expr { kind: Kind::Num(std::f32::consts::PI), ty: Ty::FLOAT });
                }
                Err(Error { at, msg: format!("unknown name '{name}' (the inputs are uv, pos, size, time)") })
            }
            Tok::End => Err(Error { at, msg: "unexpected end of the program".into() }),
            Tok::Punct(p) => Err(Error { at, msg: format!("unexpected '{p}'") }),
        }
    }
}

fn logic_operands(l: &Expr, r: &Expr, op: &str, at: usize) -> Result<(), Error> {
    if l.ty != Ty::Bool || r.ty != Ty::Bool {
        return Err(Error { at, msg: format!("'{op}' takes bool, not {} and {}", l.ty, r.ty) });
    }
    Ok(())
}

/// Two values of one type: a float next to a vector is splatted.
fn unify(a: Expr, b: Expr) -> Option<(Expr, Expr)> {
    match (a.ty, b.ty) {
        (x, y) if x == y => Some((a, b)),
        (Ty::Float(1), Ty::Float(n)) => Some((splat(a, n), b)),
        (Ty::Float(n), Ty::Float(1)) => Some((a, splat(b, n))),
        _ => None,
    }
}

/// A float as `vecN`; anything already of that size stays as it is.
fn splat(e: Expr, n: u8) -> Expr {
    if n == 1 || e.ty == Ty::Float(n) { e } else { Expr { kind: Kind::Splat(bx(e)), ty: Ty::Float(n) } }
}

fn arith(op: Op, l: Expr, r: Expr, at: usize) -> Result<Expr, Error> {
    let sym = ["+", "-", "*", "/"][op as usize];
    if l.ty == Ty::Bool || r.ty == Ty::Bool {
        return Err(Error { at, msg: format!("'{sym}' takes numbers, not {} and {}", l.ty, r.ty) });
    }
    let (lt, rt) = (l.ty, r.ty);
    let (l, r) = unify(l, r).ok_or_else(|| Error { at, msg: format!("'{sym}' cannot combine {lt} and {rt}") })?;
    let ty = l.ty;
    Ok(Expr { kind: Kind::Arith(op, bx(l), bx(r)), ty })
}

fn swizzle(e: Expr, name: &str, at: usize) -> Result<Expr, Error> {
    let n = match e.ty {
        Ty::Float(n) => n,
        Ty::Bool => return Err(Error { at, msg: "bool has no components".into() }),
    };
    let set = |c: char| "xyzw".find(c).map(|i| (0, i)).or_else(|| "rgba".find(c).map(|i| (1, i)));
    let mut idx = Vec::new();
    let mut sets = Vec::new();
    for c in name.chars() {
        let (s, i) = set(c).ok_or_else(|| Error { at, msg: format!("'{name}': components are x y z w or r g b a") })?;
        if i as u8 >= n {
            return Err(Error { at, msg: format!("'{name}': {} has no component '{c}'", e.ty) });
        }
        sets.push(s);
        idx.push(i as u8);
    }
    if idx.is_empty() || idx.len() > 4 || sets.iter().any(|&s| s != sets[0]) {
        return Err(Error { at, msg: format!("'{name}': one to four of x y z w, or of r g b a") });
    }
    let ty = Ty::Float(idx.len() as u8);
    Ok(match (n, idx.len()) {
        // A float's `.x` is the float, `.xxx` repeats it.
        (1, k) => splat(e, k as u8),
        (_, 1) => Expr { kind: Kind::Component(bx(e), idx[0]), ty },
        _ => Expr { kind: Kind::Swizzle(bx(e), idx), ty },
    })
}

fn call(name: &str, args: Vec<(usize, Expr)>, at: usize) -> Result<Expr, Error> {
    if let Some(n) = name.strip_prefix("vec").and_then(|n| n.parse::<u8>().ok()).filter(|n| (2..=4).contains(n)) {
        return construct(n, args, at);
    }
    let Some(&(_, func, shape)) = FUNCS.iter().find(|f| f.0 == name) else {
        return Err(Error { at, msg: format!("unknown function '{name}'") });
    };
    // atan with two arguments is atan2.
    let (func, shape) = if func == Func::Atan && args.len() == 2 { (Func::Atan2, Shape::Each(2)) } else { (func, shape) };
    let want = match shape {
        Shape::Each(k) | Shape::ToFloat(k) => k,
        Shape::Cross => 2,
    };
    if args.len() != want {
        let s = if want == 1 { "" } else { "s" };
        return Err(Error { at, msg: format!("{name} takes {want} argument{s}, not {}", args.len()) });
    }
    for (a_at, a) in &args {
        if a.ty == Ty::Bool {
            return Err(Error { at: *a_at, msg: format!("{name} takes numbers, not bool") });
        }
    }
    // All arguments to one type: the widest; floats are splatted, other vectors must match.
    let n = args.iter().map(|a| a.1.ty.n()).max().unwrap();
    let mut out = Vec::with_capacity(args.len());
    for (a_at, a) in args {
        let k = a.ty.n();
        if k != 1 && k != n {
            return Err(Error { at: a_at, msg: format!("{name}: {} next to vec{n}", a.ty) });
        }
        out.push(splat(a, n));
    }
    let ty = match shape {
        Shape::Each(_) => Ty::Float(n),
        Shape::ToFloat(_) => Ty::FLOAT,
        Shape::Cross if n == 3 => Ty::Float(3),
        Shape::Cross => return Err(Error { at, msg: "cross takes vec3 and vec3".into() }),
    };
    Ok(Expr { kind: Kind::Call(func, out), ty })
}

fn construct(n: u8, args: Vec<(usize, Expr)>, at: usize) -> Result<Expr, Error> {
    let ty = Ty::Float(n);
    if args.len() == 1 && args[0].1.ty == Ty::FLOAT {
        return Ok(splat(args.into_iter().next().unwrap().1, n));
    }
    let mut total = 0;
    for (a_at, a) in &args {
        if a.ty == Ty::Bool {
            return Err(Error { at: *a_at, msg: format!("vec{n} takes numbers, not bool") });
        }
        total += a.ty.n();
    }
    if total != n {
        return Err(Error { at, msg: format!("vec{n} needs {n} components, these give {total}") });
    }
    Ok(Expr { kind: Kind::Construct(args.into_iter().map(|a| a.1).collect()), ty })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err(src: &str) -> String {
        parse(src).unwrap_err().to_string()
    }

    #[test]
    fn types_and_broadcasting() {
        let p = parse("let d = distance(uv, vec2(0.5)); mix(#3b5bdb, #9c36b5, smoothstep(0.2, 0.3, d))").unwrap();
        assert_eq!(p.lets[0].ty, Ty::FLOAT);
        assert_eq!(p.result.ty, Ty::Float(4));
        // The float `t` and the edges are splatted to vec4.
        let Kind::Call(Func::Mix, args) = &p.result.kind else { panic!() };
        assert!(matches!(args[2].kind, Kind::Splat(_)));
        assert_eq!(parse("vec3(uv * 2.0 - 1.0, 0.5)").unwrap().result.ty, Ty::Float(3));
        assert_eq!(parse("sin(time) * 0.5 + 0.5").unwrap().result.ty, Ty::FLOAT);
        assert_eq!(parse("uv.x > 0.5 && uv.y < 0.5 ? #fff : #000").unwrap().result.ty, Ty::Float(4));
        assert_eq!(parse("vec4(uv.yx, uv.x.xx).abgr").unwrap().result.ty, Ty::Float(4));
        assert_eq!(parse("atan(uv.y, uv.x) / pi").unwrap().result.kind, parse("atan(uv.y, uv.x) / pi").unwrap().result.kind);
    }

    #[test]
    fn colors() {
        let p = parse("#f80").unwrap();
        assert_eq!(p.result.kind, Kind::Const(vec![1.0, 136.0 / 255.0, 0.0, 1.0]));
        let p = parse("#00000080").unwrap();
        assert_eq!(p.result.kind, Kind::Const(vec![0.0, 0.0, 0.0, 128.0 / 255.0]));
    }

    #[test]
    fn mistakes_point_at_the_place() {
        assert_eq!(err("uv"), "col 1: the program gives vec2: it must give a color — vec4 (rgba), vec3 (rgb) or float (gray)");
        assert_eq!(err("vec3(uv)"), "col 1: vec3 needs 3 components, these give 2");
        assert_eq!(err("uv + vec3(1.0)"), "col 4: '+' cannot combine vec2 and vec3");
        assert_eq!(err("uv.z"), "col 4: 'z': vec2 has no component 'z'");
        assert_eq!(err("uv.xg"), "col 4: 'xg': one to four of x y z w, or of r g b a");
        assert_eq!(err("foo(1.0)"), "col 1: unknown function 'foo'");
        assert_eq!(err("mix(1.0, 2.0)"), "col 1: mix takes 3 arguments, not 2");
        assert_eq!(err("uv < 0.5 ? 1.0 : 0.0"), "col 4: comparisons take floats, not vec2 and float (compare components: v.x < 0.5)");
        assert_eq!(err("1.0 ? 1.0 : 0.0"), "col 1: the condition before '?' is float, not bool (a comparison)");
        assert_eq!(err("let uv = 1.0; uv"), "col 5: 'uv' is taken: choose another name");
        assert_eq!(err("sin(time) 1.0"), "col 11: expected the end of the program (did you miss an operator or a ';'?)");
        assert_eq!(err("#12345"), "col 1: a color is #RGB, #RGBA, #RRGGBB or #RRGGBBAA");
        assert_eq!(err("zz"), "col 1: unknown name 'zz' (the inputs are uv, pos, size, time)");
        assert!(err(&"(".repeat(100)).contains("too deeply nested"));
    }

    #[test]
    fn lets_shadow_earlier_lets() {
        let p = parse("let a = 1.0; let a = vec3(a); a").unwrap();
        assert_eq!(p.result, Expr { kind: Kind::Let(1), ty: Ty::Float(3) });
    }
}
