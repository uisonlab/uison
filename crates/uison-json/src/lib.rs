//! Extended JSON without dependencies.
//!
//! Differences from strict JSON: `//` and `/* */` comments, trailing commas,
//! unquoted identifier keys (`{ size: 14 }`).
//!
//! The main API is the streaming [`Parser`]: the consumer drives parsing itself
//! (`begin_object` / `next_key` / `number` …) with no intermediate tree.
//! [`Value`] is built on top of it for when the structure isn't known in advance.

use std::borrow::Cow;
use std::fmt;

mod compact;
mod value;
mod write;

pub use compact::compact;
pub use value::{Value, parse};
pub use write::write_str;

#[derive(Debug, Clone, PartialEq)]
pub struct Error {
    pub line: u32,
    pub col: u32,
    pub msg: Cow<'static, str>,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.col, self.msg)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// Start of the UTF-8 character that byte `i` falls into.
fn char_start(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && i > 0 && b[i] & 0xC0 == 0x80 {
        i -= 1;
    }
    i
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Null,
    Bool,
    Number,
    String,
    Array,
    Object,
}

const MAX_DEPTH: u32 = 128;

/// Exact powers of ten (up to 10^22 all are representable in f64 exactly).
const POW10: [f64; 23] = [
    1e0, 1e1, 1e2, 1e3, 1e4, 1e5, 1e6, 1e7, 1e8, 1e9, 1e10, 1e11, 1e12, 1e13, 1e14, 1e15, 1e16,
    1e17, 1e18, 1e19, 1e20, 1e21, 1e22,
];

pub struct Parser<'a> {
    src: &'a str,
    b: &'a [u8],
    pos: usize,
    depth: u32,
    /// The current container has no elements yet (no comma needed).
    first: bool,
    /// Start of the last key read, for error messages.
    key_pos: usize,
}

impl<'a> Parser<'a> {
    pub fn new(src: &'a str) -> Self {
        Parser { src, b: src.as_bytes(), pos: 0, depth: 0, first: false, key_pos: 0 }
    }

    /// Current position in bytes.
    pub fn pos(&self) -> usize {
        self.pos
    }

    /// Error at position `pos`. Line and column are computed only here,
    /// so a successful parse doesn't spend time counting.
    /// Start position of the last key returned by [`Parser::next_key`].
    pub fn key_pos(&self) -> usize {
        self.key_pos
    }

    pub fn error_at(&self, pos: usize, msg: impl Into<Cow<'static, str>>) -> Error {
        let pos = pos.min(self.b.len());
        let before = &self.b[..pos];
        let line = before.iter().filter(|&&c| c == b'\n').count() as u32 + 1;
        let line_start = before.iter().rposition(|&c| c == b'\n').map_or(0, |i| i + 1);
        // Column in characters, not bytes: UTF-8 continuation bytes aren't counted.
        let col = before[line_start..].iter().filter(|&&c| c & 0xC0 != 0x80).count() as u32 + 1;
        let mut msg = msg.into();
        // A long line (single-line JSON: UISON_JSON in C, a string in code): line:col says little,
        // so add a fragment around the spot.
        let line_end = self.b[pos..].iter().position(|&c| c == b'\n').map_or(self.b.len(), |i| pos + i);
        if line_end - line_start > 80 {
            let from = char_start(self.b, pos.saturating_sub(40).max(line_start));
            let to = char_start(self.b, (pos + 24).min(line_end));
            let text = |a: usize, b: usize| String::from_utf8_lossy(&self.b[a..b]).split_whitespace().collect::<Vec<_>>().join(" ");
            let (l, r) = (if from > line_start { "…" } else { "" }, if to < line_end { "…" } else { "" });
            msg = format!("{msg} near \"{l}{} <here> {}{r}\"", text(from, pos), text(pos, to)).into();
        }
        Error { line, col, msg }
    }

    pub fn error(&self, msg: impl Into<Cow<'static, str>>) -> Error {
        self.error_at(self.pos, msg)
    }

    #[inline]
    fn skip_ws(&mut self) -> Result<()> {
        loop {
            match self.b.get(self.pos) {
                Some(b' ' | b'\n' | b'\r' | b'\t') => self.pos += 1,
                Some(b'/') => self.skip_comment()?,
                _ => return Ok(()),
            }
        }
    }

    fn skip_comment(&mut self) -> Result<()> {
        let start = self.pos;
        match self.b.get(self.pos + 1) {
            Some(b'/') => {
                self.pos += 2;
                while let Some(&c) = self.b.get(self.pos) {
                    self.pos += 1;
                    if c == b'\n' {
                        break;
                    }
                }
                Ok(())
            }
            Some(b'*') => {
                self.pos += 2;
                loop {
                    match self.b.get(self.pos) {
                        None => return Err(self.error_at(start, "unterminated comment")),
                        Some(b'*') if self.b.get(self.pos + 1) == Some(&b'/') => {
                            self.pos += 2;
                            return Ok(());
                        }
                        _ => self.pos += 1,
                    }
                }
            }
            _ => Err(self.error("unexpected '/'")),
        }
    }

    /// Kind of the next value (without consuming it).
    pub fn peek(&mut self) -> Result<Kind> {
        self.skip_ws()?;
        match self.b.get(self.pos) {
            Some(b'{') => Ok(Kind::Object),
            Some(b'[') => Ok(Kind::Array),
            Some(b'"') => Ok(Kind::String),
            Some(b'-' | b'0'..=b'9') => Ok(Kind::Number),
            Some(b't' | b'f') => Ok(Kind::Bool),
            Some(b'n') => Ok(Kind::Null),
            None => Err(self.error("unexpected end of input")),
            Some(_) => Err(self.error("unexpected character")),
        }
    }

    pub fn begin_object(&mut self) -> Result<()> {
        self.open(b'{', "expected '{'")
    }

    pub fn begin_array(&mut self) -> Result<()> {
        self.open(b'[', "expected '['")
    }

    fn open(&mut self, c: u8, msg: &'static str) -> Result<()> {
        self.skip_ws()?;
        if self.b.get(self.pos) != Some(&c) {
            return Err(self.error(msg));
        }
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(self.error("nesting too deep"));
        }
        self.pos += 1;
        self.first = true;
        Ok(())
    }

    fn close(&mut self) {
        self.pos += 1;
        self.depth -= 1;
        self.first = false;
    }

    /// Next object key, or `None` if the object has ended.
    /// After a key (and the colon) the caller must read the value.
    pub fn next_key(&mut self) -> Result<Option<Cow<'a, str>>> {
        self.skip_ws()?;
        if !self.first {
            match self.b.get(self.pos) {
                Some(b',') => {
                    self.pos += 1;
                    self.skip_ws()?;
                }
                Some(b'}') => {
                    self.close();
                    return Ok(None);
                }
                _ => return Err(self.error("expected ',' or '}'")),
            }
        }
        // Empty object or trailing comma.
        if self.b.get(self.pos) == Some(&b'}') {
            self.close();
            return Ok(None);
        }
        self.first = false;
        self.key_pos = self.pos;
        let key = match self.b.get(self.pos) {
            Some(b'"') => self.string_body()?,
            Some(&c) if is_ident_start(c) => {
                let start = self.pos;
                self.pos += 1;
                while self.b.get(self.pos).is_some_and(|&c| is_ident(c)) {
                    self.pos += 1;
                }
                Cow::Borrowed(&self.src[start..self.pos])
            }
            _ => return Err(self.error("expected key")),
        };
        self.skip_ws()?;
        if self.b.get(self.pos) != Some(&b':') {
            return Err(self.error("expected ':'"));
        }
        self.pos += 1;
        Ok(Some(key))
    }

    /// Whether there is a next array element. If `true`, the caller reads the value.
    pub fn next_item(&mut self) -> Result<bool> {
        self.skip_ws()?;
        if !self.first {
            match self.b.get(self.pos) {
                Some(b',') => {
                    self.pos += 1;
                    self.skip_ws()?;
                }
                Some(b']') => {
                    self.close();
                    return Ok(false);
                }
                _ => return Err(self.error("expected ',' or ']'")),
            }
        }
        if self.b.get(self.pos) == Some(&b']') {
            self.close();
            return Ok(false);
        }
        self.first = false;
        Ok(true)
    }

    pub fn string(&mut self) -> Result<Cow<'a, str>> {
        self.skip_ws()?;
        if self.b.get(self.pos) != Some(&b'"') {
            return Err(self.error("expected string"));
        }
        self.string_body()
    }

    /// Parse a string at the current position (on the opening quote).
    /// Without escape sequences, returns a slice of the input without copying.
    fn string_body(&mut self) -> Result<Cow<'a, str>> {
        let start = self.pos + 1;
        let mut i = start;
        loop {
            match self.b.get(i) {
                None => return Err(self.error_at(start - 1, "unterminated string")),
                Some(b'"') => {
                    self.pos = i + 1;
                    return Ok(Cow::Borrowed(&self.src[start..i]));
                }
                Some(b'\\') => return self.string_escaped(start, i),
                Some(&c) if c < 0x20 => return Err(self.error_at(i, "control character in string")),
                _ => i += 1,
            }
        }
    }

    fn string_escaped(&mut self, start: usize, mut i: usize) -> Result<Cow<'a, str>> {
        let mut out = String::with_capacity(i - start + 16);
        let mut run = start;
        loop {
            match self.b.get(i) {
                None => return Err(self.error_at(start - 1, "unterminated string")),
                Some(b'"') => {
                    out.push_str(&self.src[run..i]);
                    self.pos = i + 1;
                    return Ok(Cow::Owned(out));
                }
                Some(b'\\') => {
                    out.push_str(&self.src[run..i]);
                    let esc = i;
                    let c = match self.b.get(i + 1) {
                        Some(b'"') => '"',
                        Some(b'\\') => '\\',
                        Some(b'/') => '/',
                        Some(b'b') => '\u{8}',
                        Some(b'f') => '\u{c}',
                        Some(b'n') => '\n',
                        Some(b'r') => '\r',
                        Some(b't') => '\t',
                        Some(b'u') => {
                            let hi = self.hex4(i + 2)?;
                            i += 4;
                            let code = if (0xD800..0xDC00).contains(&hi) {
                                if self.b.get(i + 2) != Some(&b'\\') || self.b.get(i + 3) != Some(&b'u') {
                                    return Err(self.error_at(esc, "lone surrogate"));
                                }
                                let lo = self.hex4(i + 4)?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return Err(self.error_at(esc, "invalid surrogate pair"));
                                }
                                i += 6;
                                0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                            } else {
                                hi
                            };
                            char::from_u32(code).ok_or_else(|| self.error_at(esc, "invalid unicode escape"))?
                        }
                        _ => return Err(self.error_at(esc, "invalid escape")),
                    };
                    out.push(c);
                    i += 2;
                    run = i;
                }
                Some(&c) if c < 0x20 => return Err(self.error_at(i, "control character in string")),
                _ => i += 1,
            }
        }
    }

    fn hex4(&self, at: usize) -> Result<u32> {
        let mut v = 0u32;
        for k in 0..4 {
            let d = match self.b.get(at + k) {
                Some(&c @ b'0'..=b'9') => c - b'0',
                Some(&c @ b'a'..=b'f') => c - b'a' + 10,
                Some(&c @ b'A'..=b'F') => c - b'A' + 10,
                _ => return Err(self.error_at(at + k, "invalid hex digit")),
            };
            v = v * 16 + d as u32;
        }
        Ok(v)
    }

    /// A number. Correctly rounded if the significant digits fit
    /// in 2^53 and the exponent is within ±22 — practically every UI number.
    /// Otherwise the error is a few ULP (~1e-15 relative): this way we avoid
    /// `str::parse::<f64>` from std, which weighs ~16 KB.
    pub fn number(&mut self) -> Result<f64> {
        self.skip_ws()?;
        let b = self.b;
        let start = self.pos;
        let mut i = start;
        let neg = b.get(i) == Some(&b'-');
        if neg {
            i += 1;
        }
        // Significant digits (up to 19, fit in u64) and the decimal exponent.
        let mut mant: u64 = 0;
        let mut sig = 0u32;
        let mut exp10: i32 = 0;
        let mut digit = |d: u8, frac: bool| {
            if mant == 0 && d == 0 {
                exp10 -= frac as i32; // leading zero
            } else if sig < 19 {
                mant = mant * 10 + d as u64;
                sig += 1;
                exp10 -= frac as i32;
            } else {
                exp10 += !frac as i32; // dropped digit of the integer part
            }
        };
        let int_start = i;
        while let Some(&c @ b'0'..=b'9') = b.get(i) {
            digit(c - b'0', false);
            i += 1;
        }
        if i == int_start {
            return Err(self.error_at(start, "expected number"));
        }
        if b[int_start] == b'0' && i - int_start > 1 {
            return Err(self.error_at(int_start, "leading zero"));
        }
        if b.get(i) == Some(&b'.') {
            i += 1;
            let frac_start = i;
            while let Some(&c @ b'0'..=b'9') = b.get(i) {
                digit(c - b'0', true);
                i += 1;
            }
            if i == frac_start {
                return Err(self.error_at(i, "expected digit after '.'"));
            }
        }
        if matches!(b.get(i), Some(b'e' | b'E')) {
            i += 1;
            let exp_neg = b.get(i) == Some(&b'-');
            if matches!(b.get(i), Some(b'+' | b'-')) {
                i += 1;
            }
            let exp_start = i;
            let mut e: i32 = 0;
            while let Some(&c @ b'0'..=b'9') = b.get(i) {
                e = (e * 10 + (c - b'0') as i32).min(100_000);
                i += 1;
            }
            if i == exp_start {
                return Err(self.error_at(i, "expected exponent digits"));
            }
            exp10 += if exp_neg { -e } else { e };
        }
        self.pos = i;
        let v = if mant == 0 {
            0.0
        } else if mant < 1 << 53 && exp10.abs() <= 22 {
            // Clinger: both operands are exact, one IEEE operation gives correct rounding.
            let m = mant as f64;
            if exp10 >= 0 { m * POW10[exp10 as usize] } else { m / POW10[-exp10 as usize] }
        } else {
            // Beyond ±400 the result is 0 or infinity anyway.
            let mut e = exp10.clamp(-400, 400);
            let mut v = mant as f64;
            while e > 22 {
                v *= 1e22;
                e -= 22;
            }
            while e < -22 {
                v /= 1e22;
                e += 22;
            }
            if e >= 0 { v * POW10[e as usize] } else { v / POW10[-e as usize] }
        };
        Ok(if neg { -v } else { v })
    }

    pub fn boolean(&mut self) -> Result<bool> {
        self.skip_ws()?;
        if self.literal(b"true") {
            Ok(true)
        } else if self.literal(b"false") {
            Ok(false)
        } else {
            Err(self.error("expected boolean"))
        }
    }

    pub fn null(&mut self) -> Result<()> {
        self.skip_ws()?;
        if self.literal(b"null") { Ok(()) } else { Err(self.error("expected null")) }
    }

    fn literal(&mut self, word: &[u8]) -> bool {
        let end = self.pos + word.len();
        if self.b.get(self.pos..end) == Some(word) && !self.b.get(end).is_some_and(|&c| is_ident(c)) {
            self.pos = end;
            true
        } else {
            false
        }
    }

    /// Skip a value of any kind.
    pub fn skip_value(&mut self) -> Result<()> {
        match self.peek()? {
            Kind::Object => {
                self.begin_object()?;
                while self.next_key()?.is_some() {
                    self.skip_value()?;
                }
            }
            Kind::Array => {
                self.begin_array()?;
                while self.next_item()? {
                    self.skip_value()?;
                }
            }
            Kind::String => {
                self.string()?;
            }
            Kind::Number => {
                self.number()?;
            }
            Kind::Bool => {
                self.boolean()?;
            }
            Kind::Null => self.null()?,
        }
        Ok(())
    }

    /// Check that only whitespace and comments remain after the value.
    pub fn finish(&mut self) -> Result<()> {
        self.skip_ws()?;
        if self.pos != self.b.len() {
            return Err(self.error("unexpected trailing characters"));
        }
        Ok(())
    }
}

#[inline]
fn is_ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_' || c == b'$'
}

#[inline]
fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$'
}

#[cfg(test)]
mod tests;
