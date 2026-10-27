//! Word wrapping using the backend's metrics.

use uison_core::{Font, TextMeasure};

use crate::el::TextLine;

/// How lines are measured while wrapping (experiment 3 of Task2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WrapMode {
    /// Sum of word and space widths — each word is measured once (CTLine cache per word).
    WordSum,
    /// Each candidate line is measured whole — exact, with kerning between words.
    Exact,
}

/// Split text into lines no wider than `max_w`. Paragraphs split on `\n`, words on spaces;
/// a word longer than a line is cut by characters. More than `max_lines` lines —
/// the last one is truncated with an ellipsis.
pub fn wrap(text: &str, font: &Font, max_w: f32, max_lines: usize, mode: WrapMode, m: &dyn TextMeasure) -> Vec<TextLine> {
    let width = |s: &str| m.measure(s, font).width;
    let mut lines: Vec<String> = Vec::new();
    if !max_w.is_finite() {
        lines.extend(text.split('\n').map(str::to_string));
    } else {
        let space = width(" ");
        for para in text.split('\n') {
            let mut cur = String::new();
            let mut cur_w = 0.0;
            for word in para.split(' ').filter(|w| !w.is_empty()) {
                let ww = width(word);
                if !cur.is_empty() {
                    let fits = match mode {
                        WrapMode::WordSum => cur_w + space + ww <= max_w,
                        WrapMode::Exact => width(&format!("{cur} {word}")) <= max_w,
                    };
                    if fits {
                        cur.push(' ');
                        cur.push_str(word);
                        cur_w += space + ww;
                        continue;
                    }
                    lines.push(std::mem::take(&mut cur));
                }
                // A new line starts with this word; a long word is cut.
                let mut rest = word;
                while width(rest) > max_w {
                    let cut = fit_prefix(rest, max_w, &width);
                    lines.push(rest[..cut].to_string());
                    rest = &rest[cut..];
                }
                cur.push_str(rest);
                cur_w = width(rest);
            }
            lines.push(cur);
        }
    }
    if lines.len() > max_lines {
        lines.truncate(max_lines);
        let last = lines.last_mut().unwrap();
        *last = ellipsize(last, max_w, &width);
    }
    lines.into_iter().map(|t| TextLine { width: width(&t), text: t }).collect()
}

/// Length (in bytes) of the longest prefix no wider than `max_w`; at least one character.
pub(crate) fn fit_prefix(s: &str, max_w: f32, width: &dyn Fn(&str) -> f32) -> usize {
    let bounds: Vec<usize> = s.char_indices().map(|(i, _)| i).skip(1).chain([s.len()]).collect();
    // Binary search over character boundaries.
    let (mut lo, mut hi) = (0, bounds.len());
    while lo < hi {
        let mid = (lo + hi) / 2;
        if width(&s[..bounds[mid]]) <= max_w { lo = mid + 1 } else { hi = mid }
    }
    bounds[lo.saturating_sub(1)]
}

/// Truncate the string so that together with "…" it fits in `max_w`.
fn ellipsize(s: &str, max_w: f32, width: &dyn Fn(&str) -> f32) -> String {
    let mut t = s.trim_end().to_string();
    loop {
        let candidate = format!("{t}…");
        if t.is_empty() || !max_w.is_finite() || width(&candidate) <= max_w {
            return candidate;
        }
        t.pop();
        let trimmed = t.trim_end().len();
        t.truncate(trimmed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uison_core::TextMetrics;

    /// 10 points per character (chars, not bytes: the Cyrillic samples below check that).
    struct Mono;

    impl TextMeasure for Mono {
        fn measure(&self, t: &str, _: &Font) -> TextMetrics {
            TextMetrics { width: t.chars().count() as f32 * 10.0, ascent: 8.0, descent: 2.0, leading: 0.0 }
        }
    }

    fn lines(text: &str, max_w: f32, max_lines: usize) -> Vec<String> {
        wrap(text, &Font::default(), max_w, max_lines, WrapMode::WordSum, &Mono).into_iter().map(|l| l.text).collect()
    }

    #[test]
    fn wraps_words_and_paragraphs() {
        assert_eq!(lines("один два три четыре", 90.0, 99), ["один два", "три", "четыре"]);
        assert_eq!(lines("а\nб в", 100.0, 99), ["а", "б в"]);
        assert_eq!(lines("", 100.0, 99), [""]);
        assert_eq!(lines("без переноса", f32::INFINITY, 99), ["без переноса"]);
    }

    #[test]
    fn breaks_long_words_and_ellipsizes() {
        assert_eq!(lines("абвгдежзик", 40.0, 99), ["абвг", "дежз", "ик"]);
        assert_eq!(lines("один два три четыре", 90.0, 2), ["один два", "три…"]);
        assert_eq!(lines("очень длинное слово", 60.0, 1), ["очень…"]);
    }
}
