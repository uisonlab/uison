//! An interactive form: state in Rust, a new frame for every event.
//! `demo --static` shows the static frame from form.json (stage 3).
//!
//! Environment variables: UISON_STATS=1 for timings, UISON_FLASH=1 to highlight redraws.

use uison::{App, Color, Event, Host, MouseButton, Point, Rect, Size, TextMeasure, Writer};

const CARD_W: f32 = 432.0;
const CARD_H: f32 = 512.0;

const BLUE: Color = Color::rgb(0x00, 0x7A, 0xFF);
const BLUE_HOVER: Color = Color::rgb(0x1A, 0x88, 0xFF);
const BLUE_DOWN: Color = Color::rgb(0x00, 0x62, 0xCC);
const INK: Color = Color::rgb(0x1C, 0x1C, 0x1E);
const LABEL: Color = Color::rgb(0x3C, 0x3C, 0x43);
const MUTED: Color = Color::rgb(0x8E, 0x8E, 0x93);
const BORDER: Color = Color::rgb(0xC7, 0xC7, 0xCC);
const BORDER_HOVER: Color = Color::rgb(0xA0, 0xA0, 0xA8);

const COUNTRIES: [&str; 4] = ["Ukraine", "Poland", "Germany", "Spain"];

struct Form {
    size: Size,
    hover: Option<String>,
    pressed: Option<String>,
    focus: Option<String>,
    agree: bool,
    pro: bool,
    country: usize,
    /// The Country dropdown is open.
    open: bool,
    sent: u32,
}

impl Form {
    fn new() -> Form {
        Form {
            size: Size::new(480.0, 560.0),
            hover: None,
            pressed: None,
            focus: None,
            agree: true,
            pro: true,
            country: 0,
            open: false,
            sent: 0,
        }
    }

    /// All visible state, so the frame isn't rebuilt when nothing changed.
    #[allow(clippy::type_complexity)]
    fn snapshot(&self) -> (Option<String>, Option<String>, Option<String>, bool, bool, usize, bool, u32) {
        (self.hover.clone(), self.pressed.clone(), self.focus.clone(), self.agree, self.pro, self.country, self.open, self.sent)
    }

    fn is(&self, slot: &Option<String>, id: &str) -> bool {
        slot.as_deref() == Some(id)
    }

    fn click(&mut self, id: &str) {
        match id {
            "agree" => self.agree = !self.agree,
            "plan_basic" => self.pro = false,
            "plan_pro" => self.pro = true,
            "country" => {
                self.open = !self.open;
                self.focus = Some(id.into());
            }
            _ if id.starts_with("opt") => {
                self.country = id[3..].parse().unwrap_or(self.country);
                self.open = false;
            }
            "submit" if self.agree => self.sent += 1,
            "name" | "email" => self.focus = Some(id.into()),
            _ => self.focus = None,
        }
    }

    fn render(&self, m: &dyn TextMeasure) -> String {
        let mut w = Writer::new(self.size, Color::rgb(0xF2, 0xF2, 0xF7));
        // The card is centered in the window.
        let ox = ((self.size.w - CARD_W) / 2.0).max(0.0).round();
        let oy = ((self.size.h - CARD_H) / 2.0).max(0.0).round();
        let at = |x: f32, y: f32| Point::new(ox + x, oy + y);
        let rect = |x: f32, y: f32, w: f32, h: f32| Rect::new(ox + x, oy + y, w, h);

        w.rect(rect(0.0, 0.0, CARD_W, CARD_H)).fill(Color::WHITE).stroke(Color::rgb(0xE0, 0xE0, 0xE5), 1.0).radius(12.0);
        w.text("Sign up", at(24.0, 20.0)).size(22.0).bold().color(INK);
        w.text("Fill in the form to continue", at(24.0, 52.0)).size(13.0).color(MUTED);

        let fields: [(&str, &str, &str, f32); 3] = [
            ("name", "Name", "John Smith", 90.0),
            ("email", "Email", "john@example.com", 158.0),
            ("country", "Country", COUNTRIES[self.country], 226.0),
        ];
        for (id, label, value, y) in fields {
            w.text(label, at(24.0, y)).size(12.0).color(LABEL);
            let (border, width) = if self.is(&self.focus, id) {
                (BLUE, 2.0)
            } else if self.is(&self.hover, id) {
                (BORDER_HOVER, 1.0)
            } else {
                (BORDER, 1.0)
            };
            w.rect(rect(24.0, y + 20.0, 384.0, 36.0)).fill(Color::WHITE).stroke(border, width).radius(8.0).id(id);
            let color = if id == "name" { Color::rgb(0xAE, 0xAE, 0xB2) } else { INK };
            w.text(value, at(36.0, y + 29.0)).size(14.0).color(color);
        }
        // Select arrow: down when closed, up when open.
        let arrow = if self.open {
            [at(380.0, 268.0), at(392.0, 268.0), at(386.0, 261.0)]
        } else {
            [at(380.0, 261.0), at(392.0, 261.0), at(386.0, 268.0)]
        };
        w.tri(arrow).fill(MUTED);
        w.line(at(24.0, 302.0), at(408.0, 302.0)).color(Color::rgb(0xE5, 0xE5, 0xEA));

        // Checkbox.
        let box_color = match (self.agree, self.is(&self.hover, "agree")) {
            (true, false) => BLUE,
            (true, true) => BLUE_HOVER,
            (false, _) => Color::WHITE,
        };
        let it = w.rect(rect(24.0, 318.0, 20.0, 20.0)).fill(box_color).radius(5.0).id("agree");
        if !self.agree {
            let border = if self.is(&self.hover, "agree") { BORDER_HOVER } else { BORDER };
            let _ = it.stroke(border, 1.5);
        } else {
            drop(it);
            w.line(at(29.0, 328.0), at(33.0, 332.0)).color(Color::WHITE).width(2.0);
            w.line(at(33.0, 332.0), at(39.0, 323.0)).color(Color::WHITE).width(2.0);
        }
        w.text("I agree to the terms", at(54.0, 319.0)).size(14.0).color(INK);

        // Radio buttons.
        w.text("Plan", at(24.0, 354.0)).size(12.0).color(LABEL);
        for (id, label, x, on) in [("plan_basic", "Basic", 34.0, !self.pro), ("plan_pro", "Pro", 154.0, self.pro)] {
            let c = at(x, 386.0);
            if on {
                w.circle(c, 9.0).fill(BLUE).id(id);
                w.circle(c, 3.5).fill(Color::WHITE);
            } else {
                let border = if self.is(&self.hover, id) { BORDER_HOVER } else { BORDER };
                w.circle(c, 9.0).fill(Color::WHITE).stroke(border, 1.0).id(id);
            }
            w.text(label, at(x + 16.0, 377.0)).size(14.0).color(INK);
        }

        // Button: darker and 1pt lower while pressed.
        let down = self.is(&self.pressed, "submit") && self.is(&self.hover, "submit");
        let fill = if !self.agree {
            Color::rgb(0xB0, 0xD4, 0xFF)
        } else if down {
            BLUE_DOWN
        } else if self.is(&self.hover, "submit") {
            BLUE_HOVER
        } else {
            BLUE
        };
        let dy = if down { 1.0 } else { 0.0 };
        w.rect(rect(24.0, 424.0 + dy, 384.0, 44.0)).fill(fill).radius(10.0).id("submit");
        let label = if self.sent == 0 { "Submit".to_string() } else { format!("Sent: {}", self.sent) };
        let font = uison::Font { family: None, size: 15.0, bold: true, italic: false };
        let tm = m.measure(&label, &font);
        let tx = (24.0 + (384.0 - tm.width) / 2.0).round();
        let ty = (424.0 + dy + (44.0 - tm.height()) / 2.0).round();
        w.text(&label, at(tx, ty)).size(15.0).bold().color(Color::WHITE);

        w.text("Have an account? Sign in", at(140.0, 482.0)).size(12.0).color(MUTED);

        // The dropdown goes last, so it's on top of everything and hit-test
        // finds its options before the elements beneath it.
        if self.open {
            let top = 286.0;
            let h = COUNTRIES.len() as f32 * 32.0 + 8.0;
            w.rect(rect(26.0, top + 3.0, 384.0, h)).fill(Color::rgba(0, 0, 0, 22)).radius(10.0); // shadow
            w.rect(rect(24.0, top, 384.0, h)).fill(Color::WHITE).stroke(BORDER, 1.0).radius(8.0);
            for (i, name) in COUNTRIES.iter().enumerate() {
                let id = format!("opt{i}");
                let y = top + 4.0 + i as f32 * 32.0;
                let bg = if self.is(&self.hover, &id) { Color::rgb(0xE8, 0xF0, 0xFF) } else { Color::WHITE };
                w.rect(rect(28.0, y, 376.0, 32.0)).fill(bg).radius(6.0).id(&id);
                let selected = i == self.country;
                if selected {
                    w.line(at(38.0, y + 16.0), at(42.0, y + 20.0)).color(BLUE).width(2.0);
                    w.line(at(42.0, y + 20.0), at(49.0, y + 11.0)).color(BLUE).width(2.0);
                }
                w.text(name, at(60.0, y + 8.0)).size(14.0).color(if selected { BLUE } else { INK });
            }
        }
        w.finish()
    }
}

/// ids that react to hover.
fn interactive(id: Option<&str>) -> Option<String> {
    id.filter(|id| {
        matches!(*id, "name" | "email" | "country" | "agree" | "plan_basic" | "plan_pro" | "submit") || id.starts_with("opt")
    })
    .map(str::to_owned)
}

impl App for Form {
    fn frame(&mut self, m: &dyn Host) -> String {
        self.render(m)
    }

    fn on_event(&mut self, e: &Event, hit: Option<&str>, m: &dyn Host) -> Option<String> {
        let before = self.snapshot();
        match e {
            Event::MouseMove(_) => self.hover = interactive(hit),
            Event::MouseLeave => self.hover = None,
            // A click outside the open list only closes it.
            Event::MouseDown(_, MouseButton::Left)
                if self.open && !hit.is_some_and(|h| h == "country" || h.starts_with("opt")) =>
            {
                self.open = false;
                self.pressed = None;
            }
            Event::Key(k) if self.open && k.code == uison::KeyCode::Escape => self.open = false,
            Event::MouseDown(_, MouseButton::Left) => {
                self.hover = interactive(hit);
                self.pressed = interactive(hit);
                if self.pressed.is_none() {
                    self.focus = None;
                }
            }
            Event::MouseUp(_, MouseButton::Left) => {
                if let Some(id) = self.pressed.take()
                    && hit == Some(id.as_str())
                {
                    self.click(&id);
                }
            }
            Event::Resize(s) => {
                self.size = *s;
                return Some(self.render(m));
            }
            _ => {}
        }
        (before != self.snapshot()).then(|| self.render(m))
    }
}

struct Static;

impl App for Static {
    fn frame(&mut self, _: &dyn Host) -> String {
        include_str!("../form.json").to_owned()
    }

    fn on_event(&mut self, _: &Event, _: Option<&str>, _: &dyn Host) -> Option<String> {
        None
    }
}

/// Runs a headless scenario: a PNG of each step with the redrawn area highlighted.
#[cfg(target_vendor = "apple")]
fn simulate(dir: &str) {
    use uison::Headless;
    let p = |x: f32, y: f32| Point::new(x, y);
    let l = MouseButton::Left;
    let steps: Vec<(&str, Event)> = vec![
        ("hover_submit", Event::MouseMove(p(240.0, 470.0))),
        ("press_submit", Event::MouseDown(p(240.0, 470.0), l)),
        ("release_submit", Event::MouseUp(p(240.0, 470.0), l)),
        ("hover_agree", Event::MouseMove(p(58.0, 352.0))),
        ("press_agree", Event::MouseDown(p(58.0, 352.0), l)),
        ("release_agree", Event::MouseUp(p(58.0, 352.0), l)),
        ("hover_basic", Event::MouseMove(p(58.0, 410.0))),
        ("press_basic", Event::MouseDown(p(58.0, 410.0), l)),
        ("release_basic", Event::MouseUp(p(58.0, 410.0), l)),
        ("hover_name", Event::MouseMove(p(240.0, 152.0))),
        ("press_name", Event::MouseDown(p(240.0, 152.0), l)),
        ("release_name", Event::MouseUp(p(240.0, 152.0), l)),
        ("open_country", Event::MouseDown(p(240.0, 288.0), l)),
        ("open_country_up", Event::MouseUp(p(240.0, 288.0), l)),
        ("hover_option", Event::MouseMove(p(240.0, 390.0))),
        ("pick_option_down", Event::MouseDown(p(240.0, 390.0), l)),
        ("pick_option_up", Event::MouseUp(p(240.0, 390.0), l)),
        ("move_away", Event::MouseMove(p(5.0, 5.0))),
        ("move_nothing", Event::MouseMove(p(6.0, 6.0))),
        ("resize", Event::Resize(Size::new(600.0, 640.0))),
    ];
    let mut h = Headless::new(Form::new(), 2.0).expect("first frame");
    std::fs::create_dir_all(dir).unwrap();
    h.save_png(&format!("{dir}/00_start.png"), false).unwrap();
    for (i, (name, ev)) in steps.into_iter().enumerate() {
        let full = h.rt.frame.rect();
        match h.event(ev).expect("frame") {
            None => println!("{name:<16} no change"),
            Some((up, paint)) => println!(
                "{name:<16} app {:>4}us  load {:>4}us  diff {:>3}us  paint {:>5}us  damage {:>5.1}%  {:?}",
                up.app.as_micros(),
                up.load.as_micros(),
                up.diff.as_micros(),
                paint.as_micros(),
                100.0 * up.damage.ratio(full),
                up.damage.rects(full).map(|r| (r.x, r.y, r.w, r.h)).collect::<Vec<_>>(),
            ),
        }
        h.save_png(&format!("{dir}/{:02}_{name}.png", i + 1), true).unwrap();
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    #[cfg(target_vendor = "apple")]
    if let Some(i) = args.iter().position(|a| a == "--simulate") {
        return simulate(args.get(i + 1).map_or("demo_out", String::as_str));
    }
    if args.iter().any(|a| a == "--static") {
        uison::run(Static, "uison demo");
    } else {
        uison::run(Form::new(), "uison demo");
    }
}
