//! Expanded tree: only built-in nodes, all values computed.
//! `layout` lays it out and `emit` turns it into primitives.

use uison_core::{Color, Cursor, Fit, Font, Point, Size};

use crate::Json;

/// Size along an axis.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Dim {
    /// Fit the content.
    #[default]
    Hug,
    /// All available space.
    Fill,
    Px(f32),
    /// Fraction of the available space, 0..=1.
    Pct(f32),
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Insets {
    pub t: f32,
    pub r: f32,
    pub b: f32,
    pub l: f32,
}

impl Insets {
    pub fn h(&self) -> f32 {
        self.l + self.r
    }

    pub fn v(&self) -> f32 {
        self.t + self.b
    }
}

/// Alignment inside a rectangle: fractions per axis (0 — start, 0.5 — center, 1 — end).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Align {
    pub x: f32,
    pub y: f32,
}

impl Align {
    pub const TOP_LEFT: Align = Align { x: 0.0, y: 0.0 };

    pub fn parse(s: &str) -> Option<Align> {
        let (x, y) = match s {
            "topLeft" => (0.0, 0.0),
            "top" => (0.5, 0.0),
            "topRight" => (1.0, 0.0),
            "left" => (0.0, 0.5),
            "center" => (0.5, 0.5),
            "right" => (1.0, 0.5),
            "bottomLeft" => (0.0, 1.0),
            "bottom" => (0.5, 1.0),
            "bottomRight" => (1.0, 1.0),
            _ => return None,
        };
        Some(Align { x, y })
    }
}

/// Distribution along the main axis of Row/Column.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Main {
    #[default]
    Start,
    Center,
    End,
    Between,
    Around,
    Evenly,
}

/// Alignment along the cross axis of Row/Column.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Cross {
    #[default]
    Start,
    Center,
    End,
    Stretch,
}

/// Common layout properties of any node.
#[derive(Clone, Debug, PartialEq)]
pub struct Lay {
    pub w: Dim,
    pub h: Dim,
    pub min_w: f32,
    pub max_w: f32,
    pub min_h: f32,
    pub max_h: f32,
    pub flex: Option<f32>,
    pub margin: Insets,
    /// Padding inside a container (Box, Row, Column, Stack, Wrap).
    pub padding: Insets,
    /// Position of a Stack child: alignment…
    pub place: Option<Align>,
    /// …or explicit coordinates.
    pub pos: Option<Point>,
    /// Drawn on top of everything (after the rest of the tree), doesn't affect the Stack size.
    pub overlay: bool,
}

impl Default for Lay {
    fn default() -> Self {
        Lay {
            w: Dim::Hug,
            h: Dim::Hug,
            min_w: 0.0,
            max_w: f32::INFINITY,
            min_h: 0.0,
            max_h: f32::INFINITY,
            flex: None,
            margin: Insets::default(),
            padding: Insets::default(),
            place: None,
            pos: None,
            overlay: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Paint {
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub width: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Behavior {
    /// A click toggles the bool at the path.
    Toggle(String),
    /// A click writes values at paths (selection + closing the list).
    Select(Vec<(String, Json)>),
    /// Dragging along X changes the number at the path.
    Drag { path: String, min: f32, max: f32, step: f32, inset: f32 },
}

impl Act {
    /// Whether the node is pressable (button, field, switch) rather than just catching the mouse
    /// (scrolling, the scrim under a modal).
    pub fn pressable(&self) -> bool {
        !self.disabled
            && (!self.on.is_empty() || self.behavior.is_some() || self.href.is_some() || self.focusable || self.cursor != uison_core::Cursor::Arrow)
    }
}

/// Everything about node interaction. Only interactive nodes have it.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Act {
    /// Event → action name.
    pub on: Vec<(String, String)>,
    pub behavior: Option<Behavior>,
    pub cursor: Cursor,
    pub focusable: bool,
    pub disabled: bool,
    /// Overlay: a click outside it (or Escape) writes `false` at this path.
    pub dismiss: Option<String>,
    /// Tooltip on hover.
    pub tooltip: Option<String>,
    /// Context menu: `[{ label, action, shortcut?, disabled? }, "-", …]`.
    pub menu: Option<Json>,
    /// Link: a click opens it in the system (`Link`).
    pub href: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextLine {
    pub text: String,
    pub width: f32,
}

/// A line along text: `decoration` of `Text`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Deco {
    #[default]
    None,
    Underline,
    LineThrough,
}

/// Piece of a `Text` run within one `Flow` line; coordinates are relative to the `Text` node's box.
#[derive(Clone, Debug, PartialEq)]
pub struct Frag {
    pub text: String,
    pub x: f32,
    /// Top of the glyph line (like `at` of the `text` primitive).
    pub y: f32,
    pub w: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextEl {
    pub text: String,
    pub font: Font,
    pub color: Color,
    /// 0 — left, 0.5 — center, 1 — right.
    pub align: f32,
    pub max_lines: usize,
    pub line_height: f32,
    /// `false` — lines only at `\n`, spaces as is (`<pre>`).
    pub wrap: bool,
    pub deco: Deco,
    /// Background under the text (`<code>`, `<mark>`).
    pub fill: Option<Color>,
    /// After layout.
    pub lines: Vec<TextLine>,
    pub line_h: f32,
    pub glyph_h: f32,
    pub ascent: f32,
    /// A `Flow` run: fragments are drawn, not `lines`.
    pub in_flow: bool,
    pub frags: Vec<Frag>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InputEl {
    /// Path in the data model.
    pub path: String,
    pub value: String,
    pub placeholder: String,
    pub font: Font,
    pub color: Color,
    pub placeholder_color: Color,
    pub caret_color: Color,
    pub password: bool,
    /// On-screen keyboard (`keyboard: "email"`).
    pub keyboard: uison_core::Keyboard,
    /// Caret (byte index) — only for the focused field; `caret_on` — visible (blinking).
    pub caret: Option<usize>,
    pub caret_on: bool,
    /// Selected range (bytes of the value), non-empty.
    pub sel: Option<(usize, usize)>,
    pub sel_color: Color,
    /// Text shift from the previous frame: the caret moves within the visible part without jerking the text.
    pub shift: f32,
    /// After layout.
    pub line_h: f32,
}

impl InputEl {
    /// What is drawn: the value, or dots instead of it for a password.
    pub fn shown(&self) -> String {
        if self.password { "•".repeat(self.value.chars().count()) } else { self.value.clone() }
    }

    /// Caret index in the displayed string.
    pub fn shown_index(&self, i: usize) -> usize {
        if self.password { self.value[..i].chars().count() * '•'.len_utf8() } else { i }
    }
}

/// A primitive in node coordinates (or, for Vector, in its own coordinate system).
#[derive(Clone, Debug, PartialEq)]
pub enum Prim {
    /// Fills the node size.
    Rect { paint: Paint, radius: f32 },
    Circle { r: f32, paint: Paint },
    Line { from: Point, to: Point, width: Option<f32>, color: Option<Color>, round: bool },
    Tri { points: [Point; 3], paint: Paint },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Kind {
    Box { paint: Paint, radius: f32, clip: bool, align: Align },
    /// `stack_below`: a row narrower than this becomes a column (narrow screen).
    Row { gap: f32, main: Main, cross: Cross, stack_below: f32 },
    Column { gap: f32, main: Main, cross: Cross },
    Stack,
    /// `center` — line items centered on the line height.
    Wrap { gap: f32, run_gap: f32, center: bool },
    /// Inline flow: `Text` children are runs of one text (wrapping across their boundaries),
    /// other children are unbreakable blocks on the baseline.
    Flow { align: f32, line_height: f32 },
    Spacer,
    /// Vertical scrolling. `offset` and `content` — after layout;
    /// `over` — overscroll past the edge (bounce when scrolling by finger), on top of `offset`.
    Scroll { offset: f32, content: f32, over: f32 },
    /// Virtualized list: only the visible items `first..` are expanded.
    List { item_h: f32, count: usize, first: usize, offset: f32, over: f32 },
    Text(Box<TextEl>),
    Input(Box<InputEl>),
    /// Image: source (`mem:name` or a file), fit, corner radius.
    Image { src: String, fit: Fit, radius: f32 },
    Prim(Prim),
    /// Drawing in a `view` × `view` coordinate system, scaled to the node size.
    Vector { view: f32, color: Color, weight: f32 },
    /// Custom drawing: the program's `draw` function (`Ui::draw`) writes primitives into the frame.
    Canvas { draw: String },
}

#[derive(Clone, Debug, PartialEq)]
pub struct El {
    pub key: String,
    pub kind: Kind,
    pub lay: Lay,
    pub children: Vec<El>,
    pub act: Option<Box<Act>>,
    /// Node box size (without margin) — after layout.
    pub size: Size,
    /// Position of the outer rectangle (with margin) relative to the parent's box.
    pub pos: Point,
}

impl El {
    pub fn new(key: String, kind: Kind) -> El {
        El { key, kind, lay: Lay::default(), children: Vec::new(), act: None, size: Size::default(), pos: Point::default() }
    }

    /// Number of nodes in the subtree (for statistics).
    pub fn count(&self) -> usize {
        1 + self.children.iter().map(El::count).sum::<usize>()
    }
}
