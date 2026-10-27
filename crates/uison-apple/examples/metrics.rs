//! String widths in CoreText, to compare with the browser (experiment 4 of Task3).
//! The samples (Cyrillic, Latin, emoji) match web/scenarios/metrics.json on purpose.
use uison_core::{Font, TextMeasure};
use uison_apple::Text;

pub const SAMPLES: [(&str, f32, bool); 10] = [
    ("Регистрация", 22.0, true),
    ("Заполните форму, чтобы продолжить", 13.0, false),
    ("Входящие", 20.0, true),
    ("Скинул макеты, посмотри, когда будет время", 12.0, false),
    ("Отправить", 13.0, true),
    ("ivan@example.com", 14.0, false),
    ("Нажатий: 0", 12.0, false),
    ("The quick brown fox jumps over the lazy dog", 13.0, false),
    ("1 234 567,89 ₴", 15.0, true),
    ("Отлично, спасибо 👍", 13.0, false),
];

fn main() {
    let t = Text::new();
    for (s, size, bold) in SAMPLES {
        let m = t.measure(s, &Font { family: None, size, bold, italic: false });
        println!("{}\t{:.2}\t{:.2}\t{:.2}", s, m.width, m.ascent, m.descent);
    }
}
