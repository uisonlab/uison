//! Sizes and numbers for people, in the interface language. Integer-only: std's `f32` formatting weighs ~16 KB.

use crate::i18n::Lang;

/// Finder counts in decimal units, Explorer and Linux in binary.
#[cfg(target_vendor = "apple")]
const BASE: u64 = 1000;
#[cfg(not(target_vendor = "apple"))]
const BASE: u64 = 1024;

pub fn bytes(n: u64, lang: Lang) -> String {
    bytes_with_base(n, BASE, lang)
}

fn bytes_with_base(n: u64, base: u64, lang: Lang) -> String {
    let units = lang.units();
    let (mut unit, mut div) = (0, 1u128);
    while n as u128 >= div * base as u128 && unit < units.len() - 1 {
        div *= base as u128;
        unit += 1;
    }
    if unit == 0 {
        return format!("{n} {}", units[0]);
    }
    // Tenths, rounded; from 100 on, whole numbers.
    let tenths = (n as u128 * 10 + div / 2) / div;
    if tenths >= 1000 {
        format!("{} {}", (n as u128 + div / 2) / div, units[unit])
    } else {
        format!("{}{}{} {}", tenths / 10, lang.decimal(), tenths % 10, units[unit])
    }
}

/// `1234567` → `1 234 567` (a narrow space reads fine in any locale).
pub fn count(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 * 3);
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('\u{2009}');
        }
        out.push(ch);
    }
    out
}

pub fn percent(part: u64, whole: u64, lang: Lang) -> String {
    if whole == 0 {
        return "—".to_owned();
    }
    let (part, whole) = (part as u128, whole as u128);
    if part * 10 >= whole {
        format!("{}%", (part * 100 + whole / 2) / whole)
    } else if part * 1000 >= whole {
        let tenths = (part * 1000 + whole / 2) / whole;
        format!("{}{}{}%", tenths / 10, lang.decimal(), tenths % 10)
    } else {
        format!("<0{}1%", lang.decimal())
    }
}

/// Duration in seconds with tenths: "27.9 s".
pub fn seconds(d: std::time::Duration, lang: Lang) -> String {
    let tenths = (d.as_millis() + 50) / 100;
    format!("{}{}{} {}", tenths / 10, lang.decimal(), tenths % 10, lang.seconds_unit())
}

/// Shorten to `max` characters, keeping both ends.
pub fn ellipsize(text: &str, max: usize) -> String {
    let count = text.chars().count();
    if count <= max {
        return text.to_owned();
    }
    let head = (max - 1) / 2;
    let tail = max - 1 - head;
    let start: String = text.chars().take(head).collect();
    let end: String = text.chars().skip(count - tail).collect();
    format!("{start}…{end}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_bytes() {
        let en = Lang::En;
        assert_eq!(bytes_with_base(0, 1000, en), "0 B");
        assert_eq!(bytes_with_base(999, 1000, en), "999 B");
        assert_eq!(bytes_with_base(1500, 1000, en), "1.5 KB");
        assert_eq!(bytes_with_base(474_300_000_000, 1000, en), "474 GB");
        assert_eq!(bytes_with_base(47_430_000_000, 1000, en), "47.4 GB");
        assert_eq!(bytes_with_base(1024 * 1024, 1024, en), "1.0 MB");
        assert_eq!(bytes_with_base(999, 1000, Lang::Ru), "999 Б");
        assert_eq!(bytes_with_base(47_430_000_000, 1000, Lang::Ru), "47,4 ГБ");
    }

    #[test]
    fn formats_counts_and_percent() {
        let en = Lang::En;
        assert_eq!(count(1234567), "1\u{2009}234\u{2009}567");
        assert_eq!(percent(1, 0, en), "—");
        assert_eq!(percent(50, 100, en), "50%");
        assert_eq!(percent(5, 100, en), "5.0%");
        assert_eq!(percent(1, 100_000, en), "<0.1%");
        assert_eq!(percent(5, 100, Lang::Ru), "5,0%");
        assert_eq!(percent(1, 100_000, Lang::Ru), "<0,1%");
        assert_eq!(seconds(std::time::Duration::from_millis(27_940), en), "27.9 s");
        assert_eq!(seconds(std::time::Duration::from_millis(27_940), Lang::Ru), "27,9 с");
        assert_eq!(ellipsize("abcdefghijkl", 7), "abc…jkl");
    }
}
