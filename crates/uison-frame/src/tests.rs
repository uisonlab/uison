use uison_core::{Color, Font, Point, Rect, Size, TextMetrics, TextMeasure};

use crate::*;

/// Deterministic "metrics": 7 points per character, height = font size.
pub struct FakeMeasure;

impl TextMeasure for FakeMeasure {
    fn measure(&self, text: &str, font: &Font) -> TextMetrics {
        TextMetrics {
            width: text.chars().count() as f32 * 7.0,
            ascent: font.size * 0.8,
            descent: font.size * 0.2,
            leading: 0.0,
        }
    }
}

impl uison_core::Host for FakeMeasure {}

fn load(src: &str) -> Frame {
    Frame::parse(src, &FakeMeasure).unwrap_or_else(|e| panic!("{e}"))
}

fn err(src: &str) -> String {
    Frame::parse(src, &FakeMeasure).expect_err("must fail").to_string()
}

const FORM: &str = r##"
// sample frame
{
  size: [400, 300],
  background: "#F5F5F7",
  items: [
    { rect: [20, 20, 360, 40], fill: "#FFF", stroke: "#C8C8CC", width: 1, radius: 6, id: "name" },
    { line: [20, 80, 380, 80], color: "#DDD" },
    { circle: [40, 120, 8], fill: "#3A7AFE", id: "radio" },
    { tri: [100, 100, 120, 100, 110, 115], fill: "#333" },
    { text: "Name", at: [28, 30], size: 14, color: "#1D1D1F", bold: true },
    { clip: [0, 200, 400, 100], items: [
      { rect: [0, 190, 400, 30], fill: "#EEE", id: "row0" },
      { rect: [0, 220, 400, 30], fill: "#E0E0E0", id: "row1" },
    ]},
  ],
}
"##;

#[test]
fn decode_form() {
    let f = load(FORM);
    assert_eq!(f.size, Size::new(400.0, 300.0));
    assert_eq!(f.background, Color::rgb(0xF5, 0xF5, 0xF7));
    assert_eq!(f.items.len(), 6);
    match &f.items[0].shape {
        Shape::Rect { rect, radius, paint } => {
            assert_eq!(*rect, Rect::new(20.0, 20.0, 360.0, 40.0));
            assert_eq!(*radius, 6.0);
            assert_eq!(paint.fill, Some(Color::WHITE));
            assert_eq!(paint.stroke, Some(Color::rgb(0xC8, 0xC8, 0xCC)));
        }
        s => panic!("{s:?}"),
    }
    assert_eq!(f.items[0].id.as_deref(), Some("name"));
    // Stroke 1 → +0.5, antialiasing +1.
    assert_eq!(f.items[0].bounds, Rect::new(18.5, 18.5, 363.0, 43.0));
    match &f.items[4].shape {
        Shape::Text { text, font, .. } => {
            assert_eq!(&**text, "Name");
            assert!(font.bold);
            assert_eq!(font.size, 14.0);
        }
        s => panic!("{s:?}"),
    }
}

#[test]
fn decode_errors() {
    assert!(err("{items: []}").contains("size"));
    assert!(err("{size: [1, 2], foo: 1}").contains("unknown frame key 'foo'"));
    assert!(err("{size: [1], items: []}").contains("expected 2 numbers"));
    assert!(err("{size: [1, 2], items: [{ rect: [0, 0, 1, 1] }]}").contains("needs 'fill' or 'stroke'"));
    assert!(err("{size: [1, 2], items: [{ rect: [0, 0, 1, 1], fill: \"#000\", at: [0, 0] }]}")
        .contains("'at' is not allowed for rect"));
    assert!(err("{size: [1, 2], items: [{ fill: \"#000\" }]}").contains("no shape"));
    assert!(err("{size: [1, 2], items: [{ rect: [0,0,1,1], circle: [0,0,1], fill: \"#000\" }]}")
        .contains("several shapes"));
    assert!(err("{size: [1, 2], items: [{ text: \"a\" }]}").contains("needs 'at'"));
    assert!(err("{size: [1, 2], items: [{ line: [0,0,1,1], colour: \"#000\" }]}").contains("'colour'"));
    assert!(err("{size: [1, 2], background: \"red\"}").contains("expected color"));
    assert!(err("{size: [1, 2], items: [{ line: [0,0,1,1], width: 1, width: 2 }]}").contains("duplicate"));
    // The error position points to the right line.
    let e = Frame::parse("{\n size: [1, 2],\n items: [\n  { rect: [0, 0, 1, 1] }\n ]\n}", &FakeMeasure).unwrap_err();
    assert_eq!(e.line, 4);
    // An error in a key points to the key itself.
    let e = Frame::parse("{size: [1, 2], items: [{ line: [0,0,1,1],  colour: \"#000\" }]}", &FakeMeasure).unwrap_err();
    assert_eq!((e.line, e.col), (1, 44), "{e}");
}

#[test]
fn hit() {
    let f = load(FORM);
    assert_eq!(hit_test(&f, Point::new(30.0, 30.0)), Some("name")); // text without id is transparent
    assert_eq!(hit_test(&f, Point::new(40.0, 120.0)), Some("radio"));
    assert_eq!(hit_test(&f, Point::new(47.0, 127.0)), None); // bounds corner, but outside the circle
    assert_eq!(hit_test(&f, Point::new(10.0, 195.0)), None); // row0 outside the clip
    assert_eq!(hit_test(&f, Point::new(10.0, 205.0)), Some("row0"));
    assert_eq!(hit_test(&f, Point::new(10.0, 225.0)), Some("row1"));
    assert_eq!(hit_test(&f, Point::new(110.0, 104.0)), None); // triangle without id
}

#[test]
fn diff_single_change() {
    let a = load(FORM);
    let b = load(&FORM.replace("#3A7AFE", "#FF0000"));
    assert_eq!(diff(&a, &b), Damage::Rects(vec![Rect::new(31.0, 111.0, 18.0, 18.0)]));
    assert_eq!(diff(&a, &a), Damage::None);
    assert_eq!(diff(&a, &load(&FORM.replace("#F5F5F7", "#000"))), Damage::Full);
}

#[test]
fn diff_inside_clip_is_clipped() {
    let a = load(FORM);
    let b = load(&FORM.replace("#EEE", "#ABC"));
    // row0 = [0,190,400,30] + 1 → clipped at the top to y=200.
    assert_eq!(diff(&a, &b), Damage::Rects(vec![Rect::new(0.0, 200.0, 400.0, 21.0)]));
}

#[test]
fn diff_insert_keeps_rest_clean() {
    let mut w = Writer::new(Size::new(1000.0, 1000.0), Color::WHITE);
    for i in 0..20 {
        w.rect(Rect::new(i as f32 * 50.0, 0.0, 40.0, 40.0)).fill(Color::BLACK);
    }
    let a = load(&w.finish());
    let mut w = Writer::new(Size::new(1000.0, 1000.0), Color::WHITE);
    for i in 0..20 {
        if i == 10 {
            w.circle(Point::new(500.0, 500.0), 10.0).fill(Color::BLACK);
        }
        w.rect(Rect::new(i as f32 * 50.0, 0.0, 40.0, 40.0)).fill(Color::BLACK);
    }
    let b = load(&w.finish());
    // Prefix/suffix match: only the inserted circle is dirty.
    assert_eq!(diff(&a, &b), Damage::Rects(vec![Rect::new(489.0, 489.0, 22.0, 22.0)]));
}

#[test]
fn writer_numbers() {
    let mut w = Writer::new(Size::new(1.0, 1.0), Color::WHITE);
    for v in [0.0, -0.0, 1.0, -2.5, 0.05, 0.001, 0.0004, 12.3456, -0.125, 1e6, 123.1] {
        w.circle(Point::new(v, 0.0), 1.0).fill(Color::BLACK);
    }
    let src = w.finish();
    assert!(src.contains("[0,0,1]") && src.contains("[-2.5,0,1]") && src.contains("[0.05,0,1]"), "{src}");
    assert!(src.contains("[0.001,0,1]") && src.contains("[12.346,0,1]") && src.contains("[-0.125,0,1]"), "{src}");
    assert!(src.contains("[1000000,0,1]") && src.contains("[123.1,0,1]") && !src.contains("-0,"), "{src}");
}

#[test]
fn writer_roundtrip() {
    let mut w = Writer::new(Size::new(200.0, 100.0), Color::rgba(1, 2, 3, 4));
    w.rect(Rect::new(1.0, 2.5, 3.0, 4.0)).fill(Color::WHITE).stroke(Color::BLACK, 2.0).radius(3.0).id("r");
    w.line(Point::new(0.0, 0.0), Point::new(10.0, 10.0)).color(Color::BLACK).width(0.5);
    w.begin_clip(Rect::new(0.0, 0.0, 50.0, 50.0));
    w.circle(Point::new(5.0, 5.0), 2.0).stroke(Color::BLACK, 1.0);
    w.text("a \"q\"\n", Point::new(1.0, 1.0)).size(12.0).font("Menlo").bold().color(Color::BLACK);
    w.end_clip();
    w.tri([Point::new(0.0, 0.0), Point::new(1.0, 0.0), Point::new(0.0, 1.0)]).fill(Color::BLACK);
    let src = w.finish();
    let f = load(&src);
    assert_eq!(f.background, Color::rgba(1, 2, 3, 4));
    assert_eq!(f.items.len(), 4);
    let Shape::Clip { items, .. } = &f.items[2].shape else { panic!() };
    let Shape::Text { text, font, .. } = &items[1].shape else { panic!() };
    assert_eq!(&**text, "a \"q\"\n");
    assert_eq!(font.family.as_deref(), Some("Menlo"));
}

#[test]
fn hit_areas() {
    let src = |cursor: &str, x: f32| {
        format!(
            r##"{{ size: [100, 100], items: [
                {{ hit: [{x}, 0, 50, 50], id: "btn", cursor: "{cursor}" }},
                {{ text: "OK", at: [5, 5] }},
                {{ line: [0, 90, 50, 90], cap: "round", width: 2 }},
            ] }}"##
        )
    };
    let a = load(&src("pointer", 0.0));
    // Text without id is transparent — we hit the hit area under it.
    assert_eq!(hit_test(&a, Point::new(10.0, 10.0)), Some("btn"));
    assert_eq!(cursor_at(&a, Point::new(10.0, 10.0)), uison_core::Cursor::Pointer);
    assert_eq!(cursor_at(&a, Point::new(70.0, 70.0)), uison_core::Cursor::Arrow);
    // Moving a hit area or changing its cursor doesn't touch pixels.
    assert_eq!(diff(&a, &load(&src("text", 30.0))), Damage::None);
    assert!(err(r#"{size: [1, 1], items: [{ hit: [0, 0, 1, 1] }]}"#).contains("hit needs 'id'"));
    assert!(err(r##"{size: [1, 1], items: [{ hit: [0, 0, 1, 1], id: "x", fill: "#000" }]}"##).contains("not allowed"));
    let Shape::Line { round, .. } = a.items[2].shape else { panic!() };
    assert!(round);
    // Writer supports hit and round caps.
    let mut w = Writer::new(Size::new(10.0, 10.0), Color::WHITE);
    w.hit(Rect::new(0.0, 0.0, 5.0, 5.0), "h", uison_core::Cursor::Text);
    w.line(Point::new(0.0, 0.0), Point::new(5.0, 5.0)).round();
    let f = load(&w.finish());
    assert_eq!(cursor_at(&f, Point::new(1.0, 1.0)), uison_core::Cursor::Text);
}

#[test]
fn diff_appended_overlay_keeps_clip_contents_clean() {
    // Main content in a clip, one rect inside it changes; a panel is appended at the end.
    let frame = |open: bool| {
        let mut w = Writer::new(Size::new(400.0, 400.0), Color::WHITE);
        w.begin_clip(Rect::new(0.0, 0.0, 400.0, 400.0));
        for i in 0..10 {
            w.rect(Rect::new(10.0, i as f32 * 30.0, 100.0, 20.0)).fill(Color::BLACK);
        }
        w.rect(Rect::new(10.0, 320.0, 200.0, 30.0)).stroke(if open { Color::rgb(0, 0, 255) } else { Color::BLACK }, 1.0);
        w.end_clip();
        if open {
            w.rect(Rect::new(10.0, 360.0, 200.0, 30.0)).fill(Color::WHITE);
        }
        load(&w.finish())
    };
    let Damage::Rects(rs) = diff(&frame(false), &frame(true)) else { panic!("must not be full") };
    let area: f32 = rs.iter().map(Rect::area).sum();
    assert!(area < 400.0 * 400.0 * 0.15, "{rs:?}");
}

#[test]
fn image_primitive() {
    let mut w = crate::Writer::new(Size::new(100.0, 100.0), Color::WHITE);
    let _ = w.image(Rect::new(10.0, 10.0, 40.0, 40.0), "mem:a").radius(20.0).fit(uison_core::Fit::Contain);
    let src = w.finish();
    let f = load(&src);
    assert_eq!(
        f.items[0].shape,
        Shape::Image { rect: Rect::new(10.0, 10.0, 40.0, 40.0), src: "mem:a".into(), radius: 20.0, fit: uison_core::Fit::Contain }
    );
    // A different source: damage covers the image rectangle.
    let g = load(&src.replace("mem:a", "mem:b"));
    let d = crate::diff(&f, &g);
    assert!(d.rects(f.rect()).all(|r| r.intersect(&Rect::new(10.0, 10.0, 40.0, 40.0)).area() > 0.0));
    assert!(err(r#"{ size: [1, 1], items: [{ image: [0, 0, 1, 1] }] }"#).contains("image needs 'src'"));
    assert!(err(r#"{ size: [1, 1], items: [{ image: [0, 0, 1, 1], src: "a", fit: "tile" }] }"#).contains("fit must be"));
}

#[test]
fn poly_roundtrip_and_hit() {
    // L-shaped figure: the inner corner is not hit.
    let pts = [(0.0, 0.0), (40.0, 0.0), (40.0, 10.0), (10.0, 10.0), (10.0, 40.0), (0.0, 40.0)];
    let pts: Vec<Point> = pts.iter().map(|&(x, y)| Point::new(x, y)).collect();
    let mut w = Writer::new(Size::new(100.0, 100.0), Color::WHITE);
    w.poly(&pts).fill(Color::BLACK).id("L");
    let f = load(&w.finish());
    let Shape::Poly { points, .. } = &f.items[0].shape else { panic!() };
    assert_eq!(points.len(), 6);
    assert_eq!(f.items[0].bounds, Rect::new(-1.0, -1.0, 42.0, 42.0));
    assert_eq!(hit_test(&f, Point::new(5.0, 30.0)), Some("L"));
    assert_eq!(hit_test(&f, Point::new(30.0, 5.0)), Some("L"));
    assert_eq!(hit_test(&f, Point::new(30.0, 30.0)), None);
    assert!(err("{ size: [1, 1], items: [{ poly: [0, 0, 1, 1], fill: \"#000\" }] }").contains("at least 3 points"));
    assert!(err("{ size: [1, 1], items: [{ poly: [0, 0, 1, 1, 2], fill: \"#000\" }] }").contains("at least 3 points"));
}

#[test]
fn event_json_round_trip() {
    use crate::event;
    use uison_core::{Event, Insets, Key, KeyCode, Mods, MouseButton};
    let p = Point::new(120.5, -3.25);
    let mut all = vec![
        Event::MouseDown(p, MouseButton::Left),
        Event::MouseUp(p, MouseButton::Touch),
        Event::MouseDown(p, MouseButton::Right),
        Event::MouseDown(p, MouseButton::Other),
        Event::MouseMove(Point::new(0.001, 1e4)),
        Event::MouseLeave,
        Event::Scroll(p, -0.5, 40.0),
        Event::Resize(Size::new(1280.0, 800.0)),
        Event::SafeArea(Insets { top: 47.0, right: 0.0, bottom: 34.0, left: 0.0 }),
        Event::Appearance(true),
        Event::Appearance(false),
        Event::Paste("a\"b\\c\nd ж 😀".into()),
        Event::Tick,
        Event::ImageLoaded,
        Event::Wake,
    ];
    for code in [
        KeyCode::Char, KeyCode::Backspace, KeyCode::Delete, KeyCode::Enter, KeyCode::Tab, KeyCode::Escape, KeyCode::Left,
        KeyCode::Right, KeyCode::Up, KeyCode::Down, KeyCode::Home, KeyCode::End, KeyCode::PageUp, KeyCode::PageDown,
        KeyCode::Back, KeyCode::Menu, KeyCode::F10,
    ] {
        let mods = Mods { shift: true, ctrl: false, alt: true, cmd: false };
        all.push(Event::Key(Key { code, text: (code == KeyCode::Char).then(|| "ж".into()), mods }));
    }
    for e in all {
        let s = event::to_json(&e);
        assert_eq!(event::parse(&s).unwrap(), e, "{s}");
    }
    assert_eq!(
        event::to_json(&Event::MouseDown(Point::new(10.0, 20.5), MouseButton::Left)),
        r#"{"type":"down","x":10,"y":20.5,"button":"left"}"#
    );
    // Extra keys are skipped, missing modifiers are false.
    assert_eq!(event::parse(r#"{t: 1532, type: "move", x: 1, y: 2, hit: "row/3"}"#).unwrap(), Event::MouseMove(Point::new(1.0, 2.0)));
    assert_eq!(event::parse(r#"{"type":"key","code":"enter"}"#).unwrap(), Event::Key(Key::new(KeyCode::Enter)));
    assert!(event::parse(r#"{"type":"fly"}"#).is_err());
}

/// `scenarios/events.jsonl` — shared with C++ (`cpp-test`): every line is the canonical form.
#[test]
fn event_json_shared_vectors() {
    let src = include_str!("../../../scenarios/events.jsonl");
    let mut types = std::collections::HashSet::new();
    for line in src.lines() {
        let e = crate::event::parse(line).unwrap_or_else(|err| panic!("{err}: {line}"));
        assert_eq!(crate::event::to_json(&e), line);
        types.insert(e.name());
    }
    // Every Event variant is covered.
    assert_eq!(types.len(), 14, "{types:?}");
}


#[test]
fn writer_embeds_a_frame() {
    use crate::{Writer, hit_test};
    let inner = load(
        r##"{ size: [100, 50], background: "#102030", items: [
            { rect: [0, 0, 10, 10], fill: "#f00", stroke: "#0f0", width: 2, radius: 3 },
            { line: [0, 0, 5, 5], color: "#00f", width: 1.5, cap: "round" },
            { circle: [5, 5, 2], fill: "#fff" }, { tri: [0, 0, 1, 0, 0, 1], fill: "#111" },
            { poly: [0, 0, 4, 0, 4, 4], stroke: "#222" },
            { text: "Hi", at: [1, 2], size: 13, font: "monospace", bold: true, italic: true, color: "#333" },
            { image: [0, 0, 8, 8], src: "mem:a", radius: 4, fit: "contain" },
            { clip: [0, 0, 50, 50], items: [ { hit: [10, 10, 20, 20], id: "btn", cursor: "pointer" } ] },
        ] }"##,
    );
    let mut w = Writer::new(Size::new(300.0, 200.0), Color::WHITE);
    w.frame(&inner, Point::new(10.0, 20.0), 1.0, "r/");
    let outer = load(&w.finish());
    // Everything is inside one clip of the inner frame's size; the background comes first.
    let Shape::Clip { rect, items } = &outer.items[0].shape else { panic!("{:?}", outer.items[0]) };
    assert_eq!(*rect, Rect::new(10.0, 20.0, 100.0, 50.0));
    assert_eq!(items.len(), 1 + inner.items.len());
    for (a, b) in items[1..].iter().zip(&inner.items) {
        assert_eq!(std::mem::discriminant(&a.shape), std::mem::discriminant(&b.shape));
    }
    let Shape::Text { at, font, .. } = &items[6].shape else { panic!() };
    assert_eq!((*at, font.bold, font.italic, font.family.as_deref()), (Point::new(11.0, 22.0), true, true, Some("monospace")));
    assert_eq!(hit_test(&outer, Point::new(25.0, 35.0)), Some("r/btn"));
    assert_eq!(crate::cursor_at(&outer, Point::new(25.0, 35.0)), uison_core::Cursor::Pointer);
    // Half size: positions, sizes and fonts halve around the origin.
    let mut w = Writer::new(Size::new(300.0, 200.0), Color::WHITE);
    w.frame(&inner, Point::new(10.0, 20.0), 0.5, "r/");
    let half = load(&w.finish());
    let Shape::Clip { rect, items } = &half.items[0].shape else { panic!() };
    assert_eq!(*rect, Rect::new(10.0, 20.0, 50.0, 25.0));
    let Shape::Text { at, font, .. } = &items[6].shape else { panic!() };
    assert_eq!((*at, font.size), (Point::new(10.5, 21.0), 6.5));
    assert_eq!(hit_test(&half, Point::new(17.0, 27.0)), Some("r/btn"));
}

/// Console mode in memory: hello, frames as strict JSON lines, a resize, a host request,
/// a wake from another thread, garbage ignored, the end of input ends the loop.
#[test]
fn console_serves_frames_and_events() {
    use std::io::{BufReader, Cursor, Read};
    use std::sync::{Arc, Mutex};
    use uison_core::{App, Event, Host, Waker};

    struct T {
        size: Size,
        waker: Option<Waker>,
        woke: bool,
    }
    impl App for T {
        fn frame(&mut self, _: &dyn Host) -> String {
            let mut w = crate::Writer::new(self.size, Color::WHITE);
            w.hit(Rect::new(0.0, 0.0, 10.0, 10.0), "b", uison_core::Cursor::Pointer);
            w.text(if self.woke { "woke" } else { "waiting" }, Point::new(0.0, 20.0));
            w.finish()
        }
        fn on_event(&mut self, e: &Event, hit: Option<&str>, host: &dyn Host) -> Option<String> {
            match e {
                Event::Resize(s) => self.size = *s,
                Event::MouseDown(..) => {
                    assert_eq!(hit, Some("b"));
                    host.set_clipboard("copied \"x\"");
                    let w = self.waker.clone().unwrap();
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                        w();
                    });
                    return None;
                }
                Event::Wake => self.woke = true,
                _ => return None,
            }
            Some(self.frame(host))
        }
        fn set_waker(&mut self, w: Waker) {
            self.waker = Some(w);
        }
    }
    /// Input that ends only after a while (the wake must come before the end).
    struct Later;
    impl Read for Later {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            std::thread::sleep(std::time::Duration::from_millis(200));
            Ok(0)
        }
    }
    #[derive(Clone)]
    struct Sink(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for Sink {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let input = "{\"type\":\"resize\",\"w\":300,\"h\":200}\nnot json\n\n{\"type\":\"down\",\"x\":5,\"y\":5,\"button\":\"left\"}\n";
    let sink = Sink(Arc::default());
    let app = T { size: Size::new(100.0, 50.0), waker: None, woke: false };
    let code = crate::console::serve(Box::new(app), &FakeMeasure, "Test", BufReader::new(Cursor::new(input).chain(Later)), sink.clone());
    assert_eq!(code, 0);
    let out = String::from_utf8(sink.0.lock().unwrap().clone()).unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[0], r#"{"uison":0,"title":"Test"}"#);
    for l in &lines {
        uison_json::parse(l).unwrap_or_else(|e| panic!("{e}: {l}"));
    }
    assert!(lines[1].starts_with(r#"{"frame":{"size":[100,50],"#), "{}", lines[1]);
    assert!(lines[2].starts_with(r#"{"frame":{"size":[300,200],"#), "{}", lines[2]);
    assert_eq!(lines[3], r#"{"host":"copy","text":"copied \"x\""}"#);
    assert!(lines[4].contains(r#""text":"woke""#), "{}", lines[4]);
    assert_eq!(lines.len(), 5);
}

#[test]
fn scroll_under_resting_mouse_rehovers() {
    // Two rows scroll under a still mouse: the app hears a move to the same point with the new row.
    use std::{cell::RefCell, rc::Rc};
    use uison_core::{App, Event, Host, MouseButton};
    struct List {
        top: f32,
        moves: Rc<RefCell<Vec<Option<String>>>>,
    }
    impl App for List {
        fn frame(&mut self, _: &dyn Host) -> String {
            let (a, b) = (self.top, self.top + 50.0);
            format!(r#"{{size:[100,100],items:[{{hit:[0,{a},100,50],id:"a"}},{{hit:[0,{b},100,50],id:"b"}}]}}"#)
        }
        fn on_event(&mut self, e: &Event, hit: Option<&str>, host: &dyn Host) -> Option<String> {
            match e {
                Event::Scroll(_, _, dy) => self.top += dy,
                Event::MouseMove(_) => self.moves.borrow_mut().push(hit.map(str::to_string)),
                _ => return None,
            }
            Some(self.frame(host))
        }
    }
    let moves = Rc::new(RefCell::new(vec![]));
    let mut rt = Runtime::new(Box::new(List { top: 0.0, moves: moves.clone() }), &FakeMeasure).unwrap();
    let p = Point::new(50.0, 70.0);
    let mut send = |e| drop(rt.dispatch(&e, &FakeMeasure).unwrap());
    send(Event::MouseMove(p));
    send(Event::Scroll(p, 0.0, -10.0)); // still "b"
    send(Event::Scroll(p, 0.0, 40.0)); // "a" slides under the mouse
    send(Event::MouseDown(p, MouseButton::Touch));
    send(Event::Scroll(p, 0.0, 50.0)); // a finger hovers nothing
    assert_eq!(*moves.borrow(), [Some("b".into()), Some("a".into())]);
}
