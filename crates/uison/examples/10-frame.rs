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
