//! Shared types and the backend contract. Nothing platform-specific.

mod geom;
pub mod icon;
pub mod image;
pub mod time;

pub use geom::{Insets, Point, Rect, Size};
pub use icon::set_icon;
pub use image::Fit;

/// sRGB color, 8 bits per channel, alpha not premultiplied.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const WHITE: Color = Color::rgb(255, 255, 255);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
        Color { r, g, b, a }
    }

    /// `#RGB`, `#RRGGBB`, `#RRGGBBAA`.
    pub fn from_hex(s: &str) -> Option<Color> {
        let h = s.strip_prefix('#')?.as_bytes();
        let d = |i: usize| -> Option<u8> {
            match h[i] {
                c @ b'0'..=b'9' => Some(c - b'0'),
                c @ b'a'..=b'f' => Some(c - b'a' + 10),
                c @ b'A'..=b'F' => Some(c - b'A' + 10),
                _ => None,
            }
        };
        let byte = |i: usize| Some(d(i)? << 4 | d(i + 1)?);
        match h.len() {
            3 => Some(Color::rgb(d(0)? * 17, d(1)? * 17, d(2)? * 17)),
            6 => Some(Color::rgb(byte(0)?, byte(2)?, byte(4)?)),
            8 => Some(Color::rgba(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
            _ => None,
        }
    }

    pub fn is_opaque(self) -> bool {
        self.a == 255
    }
}

/// Font of a text line. `family: None` — the system font.
#[derive(Clone, Debug, PartialEq)]
pub struct Font {
    /// Family; the backend maps the generic names `monospace`, `serif`, `sans-serif` to its own.
    pub family: Option<Box<str>>,
    pub size: f32,
    pub bold: bool,
    pub italic: bool,
}

impl Default for Font {
    fn default() -> Self {
        Font { family: None, size: 13.0, bold: false, italic: false }
    }
}

/// Size of single-line text: width and line height (ascent + descent + leading).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct TextMetrics {
    pub width: f32,
    pub ascent: f32,
    pub descent: f32,
    pub leading: f32,
}

impl TextMetrics {
    pub fn height(&self) -> f32 {
        self.ascent + self.descent + self.leading
    }
}

/// Fill and/or stroke of a shape. The stroke is centered on the outline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paint {
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub width: f32,
}

impl Paint {
    pub const fn fill(c: Color) -> Paint {
        Paint { fill: Some(c), stroke: None, width: 1.0 }
    }

    pub const fn stroke(c: Color, width: f32) -> Paint {
        Paint { fill: None, stroke: Some(c), width }
    }

    /// How far the stroke extends beyond the shape's outline.
    pub fn outset(&self) -> f32 {
        if self.stroke.is_some() { self.width / 2.0 } else { 0.0 }
    }
}

/// Everything a backend must be able to draw. Coordinates are logical points,
/// origin at the top left, Y down. Fill first, then stroke.
pub trait Canvas {
    fn rect(&mut self, r: Rect, radius: f32, paint: Paint);
    fn circle(&mut self, c: Point, r: f32, paint: Paint);
    fn triangle(&mut self, p: [Point; 3], paint: Paint) {
        self.poly(&p, paint);
    }
    /// Closed polygon (chart sectors, arrows, custom shapes); filled by the
    /// nonzero winding rule, stroke joins are rounded.
    fn poly(&mut self, p: &[Point], paint: Paint);
    /// `round` — rounded caps (icons); otherwise cut flush at the points.
    fn line(&mut self, a: Point, b: Point, width: f32, color: Color, round: bool);
    /// `at` — top-left corner of the line (not the baseline).
    fn text(&mut self, at: Point, text: &str, font: &Font, color: Color);
    fn push_clip(&mut self, r: Rect);
    fn pop_clip(&mut self);
    /// Image `src` (see [`image`]) fitted into `r`, with rounded corners. A backend without
    /// image support draws nothing.
    fn image(&mut self, _r: Rect, _src: &str, _radius: f32, _fit: Fit) {}
}

pub trait TextMeasure {
    fn measure(&self, text: &str, font: &Font) -> TextMetrics;
}

/// The platform as seen by the app: text measurement and requests to the system.
/// Requests only in response to a user action (browsers and mobile systems require it).
pub trait Host: TextMeasure {
    /// Modifiers held right now (Shift+click extends the selection).
    fn mods(&self) -> Mods {
        Mods::default()
    }
    fn set_clipboard(&self, _text: &str) {}
    /// Read the clipboard; the answer is [`Event::Paste`] (right after the event or later: Wayland, X11, browser).
    fn request_paste(&self) {}
    /// Open a link in the system; `false` — failed.
    fn open_url(&self, _url: &str) -> bool {
        false
    }
    /// Let the person choose a file with the system's dialog. The app gets its own copy (in a folder only
    /// it can read) as [`Event::Message`] `{"action": action, "key": path}`, an empty `key` — cancelled.
    /// `false` — there is no such dialog here (a phone has one; on a computer a path is typed).
    fn pick_file(&self, _action: &str) -> bool {
        false
    }
    /// Natural size of an image in points; `None` — still loading or doesn't exist.
    /// By default, from the PNG/JPEG header in memory or a file ([`image::natural_size`]).
    fn image_size(&self, src: &str) -> Option<Size> {
        image::natural_size(src)
    }
}

/// Mouse cursor shape over an area.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Cursor {
    #[default]
    Arrow,
    /// Hand — a clickable element.
    Pointer,
    /// I-beam — a text input field.
    Text,
}

impl Cursor {
    pub fn parse(s: &str) -> Option<Cursor> {
        match s {
            "arrow" => Some(Cursor::Arrow),
            "pointer" => Some(Cursor::Pointer),
            "text" => Some(Cursor::Text),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Cursor::Arrow => "arrow",
            Cursor::Pointer => "pointer",
            Cursor::Text => "text",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyCode {
    /// A regular key; the characters are in `Key::text`.
    Char,
    Backspace,
    Delete,
    Enter,
    Tab,
    Escape,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    /// The system Back key (Android).
    Back,
    /// The context menu key (on PC keyboards).
    Menu,
    /// F10: Shift+F10 opens the context menu.
    F10,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Mods {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub cmd: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Key {
    pub code: KeyCode,
    /// Typed text (with Shift and keyboard layout applied), without control characters.
    pub text: Option<Box<str>>,
    pub mods: Mods,
}

impl Key {
    pub fn new(code: KeyCode) -> Key {
        Key { code, text: None, mods: Mods::default() }
    }

    pub fn char(text: &str) -> Key {
        Key { code: KeyCode::Char, text: Some(text.into()), mods: Mods::default() }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Other,
    /// A finger (touch screen): no hover; a drag over a scroll view scrolls rather than presses.
    Touch,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    MouseDown(Point, MouseButton),
    MouseUp(Point, MouseButton),
    /// Mouse movement (with or without a button held).
    MouseMove(Point),
    /// The cursor left the window.
    MouseLeave,
    /// Scroll at a point: (dx, dy) in logical points, positive dy moves the content down.
    Scroll(Point, f32, f32),
    Resize(Size),
    /// The window's safe area changed (iOS: notch, status bar, home indicator).
    SafeArea(Insets),
    /// System theme: `true` — dark. The backend sends it at startup and on change.
    Appearance(bool),
    Key(Key),
    /// A timer requested via [`App::wants_tick`] fired.
    Tick,
    /// Clipboard text — the answer to [`Host::request_paste`] or a paste by system means (browser).
    Paste(Box<str>),
    /// An image finished loading — [`Host::image_size`] knows its size, the frame must be rebuilt.
    ImageLoaded,
    /// Woken from another thread (the waker from [`App::set_waker`]): run deferred work.
    Wake,
    /// A JSON message for the app from outside — an action or a data change being replayed
    /// (`{"action":…}`, `{"data":{…}}`), later the remote client. `uison-ui` understands it.
    Message(Box<str>),
}

impl Event {
    /// Short name for logs (no f32 formatting: it weighs ~16 KB).
    pub fn name(&self) -> &'static str {
        match self {
            Event::MouseDown(..) => "down",
            Event::MouseUp(..) => "up",
            Event::MouseMove(_) => "move",
            Event::MouseLeave => "leave",
            Event::Scroll(..) => "scroll",
            Event::Resize(_) => "resize",
            Event::SafeArea(_) => "safe-area",
            Event::Appearance(_) => "appearance",
            Event::Key(_) => "key",
            Event::Tick => "tick",
            Event::Paste(_) => "paste",
            Event::ImageLoaded => "image",
            Event::Wake => "wake",
            Event::Message(_) => "message",
        }
    }

    /// Cursor position for mouse and scroll events.
    pub fn pos(&self) -> Option<Point> {
        match self {
            Event::MouseDown(p, _) | Event::MouseUp(p, _) | Event::MouseMove(p) | Event::Scroll(p, ..) => Some(*p),
            _ => None,
        }
    }
}

/// On-screen keyboard for the focused field (phone).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Keyboard {
    #[default]
    Text,
    Email,
    /// Digits only.
    Number,
    /// Digits and the decimal separator.
    Decimal,
    Phone,
    Url,
    /// Text without suggestions or autocorrect learning.
    Password,
}

impl Keyboard {
    pub fn parse(s: &str) -> Option<Keyboard> {
        Some(match s {
            "text" => Keyboard::Text,
            "email" => Keyboard::Email,
            "number" => Keyboard::Number,
            "decimal" => Keyboard::Decimal,
            "phone" => Keyboard::Phone,
            "url" => Keyboard::Url,
            "password" => Keyboard::Password,
            _ => return None,
        })
    }
}

/// Event loop waker: may be called from any thread.
pub type Waker = std::sync::Arc<dyn Fn() + Send + Sync>;

/// The app: all logic in Rust, only frames as text go out.
/// `host` is the platform: text metrics for layout (centering, line wrapping)
/// and requests to the system (clipboard, links, image sizes).
pub trait App {
    /// The first frame.
    fn frame(&mut self, host: &dyn Host) -> String;
    /// Reaction to an event. `hit` is the `id` of the topmost element under the cursor.
    /// `None` — the frame didn't change.
    fn on_event(&mut self, event: &Event, hit: Option<&str>, host: &dyn Host) -> Option<String>;
    /// Send [`Event::Tick`] after this time (caret blinking, animations).
    /// Asked after every event; `None` — no timer needed.
    fn wants_tick(&self) -> Option<std::time::Duration> {
        None
    }
    /// The backend provides a waker: calling it from any thread leads to [`Event::Wake`] on the UI thread.
    fn set_waker(&mut self, _wake: Waker) {}
    /// Whether an on-screen keyboard is needed (an input field is focused) and which one.
    /// Asked after every event.
    fn wants_keyboard(&self) -> Option<Keyboard> {
        None
    }
}
