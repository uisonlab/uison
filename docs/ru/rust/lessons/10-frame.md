# Урок 10. Свой кадр

[English](../../../rust/lessons/10-frame.md) · **Русский**

**Что будет:** поле, на котором щелчок ставит синюю точку, счётчик точек и кнопка «Clear» — без экранов и элементов: кадр из примитивов программа строит сама.

![Урок 10](../../../rust/img/10-frame.png)

<!-- code: crates/uison/examples/10-frame.rs -->
```rust
//! Lesson 10: your own frame, without Ui and screens. The program builds the frame from primitives (rectangles,
//! circles, text, hit areas) and handles events itself: the `App` trait and the `Writer` builder.
//! A click on the field places a dot, the "Clear" button removes them all.
use uison::{App, Color, Cursor, Event, Font, Host, MouseButton, Point, Rect, Size, Writer};

const BACKGROUND: Color = Color::rgb(0xF5, 0xF5, 0xF7);
const BORDER: Color = Color::rgb(0xD2, 0xD2, 0xD7);
const MUTED: Color = Color::rgb(0x86, 0x86, 0x8B);
const RED: Color = Color::rgb(0xFF, 0x3B, 0x30);
const BLUE: Color = Color::rgb(0x00, 0x7A, 0xFF);

#[derive(Default)]
struct Dots {
    dots: Vec<Point>,
}

impl Dots {
    fn render(&self, host: &dyn Host) -> String {
        let mut w = Writer::new(Size::new(400.0, 300.0), BACKGROUND);
        w.rect(Rect::new(10.0, 10.0, 380.0, 230.0)).radius(10.0).fill(Color::WHITE).stroke(BORDER, 1.0);
        w.hit(Rect::new(10.0, 10.0, 380.0, 230.0), "field", Cursor::Pointer);
        // A centered label: the platform measures the text width.
        let label = format!("Dots: {}", self.dots.len());
        let width = host.measure(&label, &Font { size: 13.0, ..Font::default() }).width;
        w.text(&label, Point::new(200.0 - width / 2.0, 258.0)).size(13.0).color(MUTED);
        w.rect(Rect::new(290.0, 250.0, 100.0, 32.0)).radius(8.0).fill(RED);
        w.text("Clear", Point::new(323.0, 257.0)).size(13.0).bold().color(Color::WHITE);
        w.hit(Rect::new(290.0, 250.0, 100.0, 32.0), "clear", Cursor::Pointer);
        for p in &self.dots {
            w.circle(*p, 8.0).fill(BLUE);
        }
        w.finish()
    }
}

impl App for Dots {
    fn frame(&mut self, host: &dyn Host) -> String {
        self.render(host)
    }

    /// `hit` is the id of the hit area under the cursor. `None` when the frame didn't change.
    fn on_event(&mut self, e: &Event, hit: Option<&str>, host: &dyn Host) -> Option<String> {
        match (e, hit) {
            (Event::MouseDown(_, MouseButton::Left), Some("clear")) => self.dots.clear(),
            (Event::MouseDown(p, MouseButton::Left), Some("field")) if self.dots.len() < 64 => self.dots.push(*p),
            _ => return None,
        }
        Some(self.render(host))
    }
}

fn main() {
    uison::run(Dots::default(), "Lesson 10 — your own frame");
}
```

## Как это работает

1. **Нижний уровень.** `Ui` — сам приложение: он реализует трейт `App`, а под ним — кадр примитивов. Свой `App` берёт у uison только окно, отрисовку, перерисовку изменившегося и hit-тест.
2. **Кадр.** `Writer` строит его цепочкой: `rect`, `circle`, `line`, `tri`, `poly` (многоугольник), `text`, `image`, `begin_clip`/`end_clip`, у фигур — `fill`, `stroke`, `radius`; `hit(rect, id, cursor)` — невидимая область мыши. `finish()` — готовая строка кадра.
3. **События.** `on_event` получает событие (`Event::MouseDown(point, button)`, `Key`, `Scroll`, `Resize`…) и `hit` — id области под курсором. Вернуть новый кадр — `Some(…)`, кадр не изменился — `None`. Перерисуется только то, что изменилось: кадры сравниваются.
4. **Текст.** Ширину строки меряет платформа: `host.measure(text, &Font { size, .. })`.
5. **Когда это нужно.** Графики, редакторы, игры, свои элементы с нестандартной отрисовкой. Остальное удобнее экраном — у него есть состояния, фокус, прокрутка, ввод текста.

## Попробуйте

- Соединить точки линиями: `w.line(a, b).color(…).width(2.0)`.
- Перетаскивание точки: запомнить её на `MouseDown`, двигать на `MouseMove`, отпустить на `MouseUp`.

Дальше — [урок 11: везде](11-everywhere.md).
