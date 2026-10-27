//! Single-line editing: caret, selection (anchor + caret), words.
//! Indices are byte offsets, always on a character boundary.

/// A word is letters, digits and `_`; everything else (spaces, punctuation) separates words.
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub fn prev_char(s: &str, i: usize) -> usize {
    s[..i].char_indices().next_back().map_or(0, |(j, _)| j)
}

pub fn next_char(s: &str, i: usize) -> usize {
    s[i..].chars().next().map_or(i, |c| i + c.len_utf8())
}

/// Start of the word left of `i` (like Alt+← on macOS, Ctrl+← on Windows).
pub fn word_left(s: &str, i: usize) -> usize {
    let mut start = 0;
    for (j, c) in s[..i].char_indices().rev().skip_while(|&(_, c)| !is_word(c)) {
        if !is_word(c) {
            break;
        }
        start = j;
    }
    start
}

/// Start of the next word (Ctrl+→ on Windows and Linux).
pub fn next_word(s: &str, i: usize) -> usize {
    let mut seen_gap = false;
    for (j, c) in s[i..].char_indices() {
        if !is_word(c) {
            seen_gap = true;
        } else if seen_gap {
            return i + j;
        }
    }
    s.len()
}

/// End of the word right of `i` (Alt+→ on macOS).
pub fn word_right(s: &str, i: usize) -> usize {
    let mut end = i;
    let mut seen = false;
    for (j, c) in s[i..].char_indices() {
        if is_word(c) {
            seen = true;
        } else if seen {
            return i + j;
        }
        end = i + j + c.len_utf8();
    }
    end
}

/// The word (or the gap between words) at the index — for a double click.
pub fn word_at(s: &str, i: usize) -> (usize, usize) {
    // A click past the last character counts as on the last character.
    let i = if i >= s.len() { prev_char(s, s.len()) } else { i };
    let Some(c) = s[i..].chars().next() else { return (0, 0) };
    let kind = is_word(c);
    let start = s[..i].char_indices().rev().take_while(|&(_, c)| is_word(c) == kind).last().map_or(i, |(j, _)| j);
    let end = s[i..].char_indices().find(|&(_, c)| is_word(c) != kind).map_or(s.len(), |(j, _)| i + j);
    (start, end)
}

/// Field: text, caret and selection anchor (`anchor == caret` — no selection).
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub text: String,
    pub caret: usize,
    pub anchor: usize,
}

impl Field {
    pub fn new(text: &str, caret: usize, anchor: usize) -> Field {
        let fix = |mut i: usize| {
            i = i.min(text.len());
            while !text.is_char_boundary(i) {
                i -= 1;
            }
            i
        };
        Field { text: text.to_string(), caret: fix(caret), anchor: fix(anchor) }
    }

    /// The selected range, if not empty.
    pub fn selection(&self) -> Option<(usize, usize)> {
        (self.caret != self.anchor).then(|| (self.caret.min(self.anchor), self.caret.max(self.anchor)))
    }

    pub fn selected(&self) -> &str {
        self.selection().map_or("", |(a, b)| &self.text[a..b])
    }

    /// Caret to `i`; `extend` — extend the selection from the anchor (Shift).
    pub fn move_to(&mut self, i: usize, extend: bool) {
        self.caret = i;
        if !extend {
            self.anchor = i;
        }
    }

    pub fn select(&mut self, a: usize, b: usize) {
        self.anchor = a;
        self.caret = b;
    }

    /// Replace the selection (or insert at the caret) with text.
    pub fn insert(&mut self, t: &str) {
        let (a, b) = self.selection().unwrap_or((self.caret, self.caret));
        self.text.replace_range(a..b, t);
        self.move_to(a + t.len(), false);
    }

    /// Delete the selection or, if there is none, from the caret to `to`.
    pub fn delete_to(&mut self, to: usize) {
        let (a, b) = self.selection().unwrap_or((self.caret.min(to), self.caret.max(to)));
        self.text.replace_range(a..b, "");
        self.move_to(a, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words() {
        // Cyrillic on purpose: byte indices must land on multi-byte character boundaries.
        let s = "Привет, мир  rust_ui!";
        let at = |w: &str| s.find(w).unwrap();
        assert_eq!(word_left(s, s.len()), at("rust_ui"));
        assert_eq!(word_left(s, at("rust_ui")), at("мир"));
        assert_eq!(word_left(s, at("мир") + 2), at("мир"));
        assert_eq!(word_left(s, at("мир")), 0);
        assert_eq!(word_right(s, 0), at(","));
        assert_eq!(word_right(s, at(",")), at("  "));
        assert_eq!(next_word(s, 0), at("мир"));
        assert_eq!(next_word(s, at("rust_ui")), s.len());
        assert_eq!(word_at(s, at("ир")), (at("мир"), at("  ")));
        assert_eq!(word_at(s, at("  ") + 1), (at("  "), at("rust_ui")));
        assert_eq!(word_at(s, s.len()), (at("!"), s.len()));
        assert_eq!(word_at("", 0), (0, 0));
    }

    #[test]
    fn insert_and_delete_replace_selection() {
        let mut f = Field::new("hello world", 0, 5);
        assert_eq!(f.selected(), "hello");
        f.insert("bye");
        assert_eq!((f.text.as_str(), f.caret, f.selection()), ("bye world", 3, None));
        f.move_to(f.text.len(), true);
        f.delete_to(0);
        assert_eq!((f.text.as_str(), f.caret), ("bye", 3));
        f.delete_to(prev_char(&f.text, f.caret));
        assert_eq!(f.text, "by");
        // Indices past the end and inside a character are aligned.
        let f = Field::new("ёж", 1, usize::MAX);
        assert_eq!((f.caret, f.anchor), (0, 4));
    }
}
