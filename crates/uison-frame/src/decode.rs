//! Frame decoding straight from the streaming parser, with no intermediate tree.

use uison_core::{Color, Cursor, Fit, Font, Paint, Point, Rect, Size, TextMeasure};
use uison_json::{Error, Parser};

use crate::{Frame, Item, Shape};

const MAX_CLIP_DEPTH: u32 = 32;

pub(crate) fn parse(src: &str, m: &dyn TextMeasure) -> Result<Frame, Error> {
    let mut p = Parser::new(src);
    p.begin_object()?;
    let mut size = None;
    let mut background = Color::WHITE;
    let mut items = Vec::new();
    while let Some(key) = p.next_key()? {
        let at = p.key_pos();
        match &*key {
            "size" => {
                let [w, h] = nums(&mut p)?;
                size = Some(Size::new(w, h));
            }
            "background" => background = read_color(&mut p)?,
            "items" => items = read_items(&mut p, m, 0)?,
            _ => return Err(p.error_at(at, format!("unknown frame key '{key}'"))),
        }
    }
    p.finish()?;
    let size = size.ok_or_else(|| p.error_at(0, "frame needs 'size'"))?;
    Ok(Frame { size, background, items })
}

fn read_items(p: &mut Parser, m: &dyn TextMeasure, depth: u32) -> Result<Vec<Item>, Error> {
    if depth > MAX_CLIP_DEPTH {
        return Err(p.error("clip nesting too deep"));
    }
    let mut items = Vec::new();
    p.begin_array()?;
    while p.next_item()? {
        items.push(read_item(p, m, depth)?);
    }
    Ok(items)
}

/// Item keys as bits: checking "what is allowed for a shape" is then a single mask.
mod k {
    pub const FILL: u32 = 1 << 0;
    pub const STROKE: u32 = 1 << 1;
    pub const WIDTH: u32 = 1 << 2;
    pub const RADIUS: u32 = 1 << 3;
    pub const COLOR: u32 = 1 << 4;
    pub const AT: u32 = 1 << 5;
    pub const SIZE: u32 = 1 << 6;
    pub const FONT: u32 = 1 << 7;
    pub const BOLD: u32 = 1 << 8;
    pub const ITEMS: u32 = 1 << 9;
    pub const ID: u32 = 1 << 10;
    pub const CAP: u32 = 1 << 11;
    pub const CURSOR: u32 = 1 << 12;
    pub const SRC: u32 = 1 << 13;
    pub const FIT: u32 = 1 << 14;
    pub const ITALIC: u32 = 1 << 15;
    pub const NAMES: [&str; 16] =
        ["fill", "stroke", "width", "radius", "color", "at", "size", "font", "bold", "items", "id", "cap", "cursor", "src", "fit", "italic"];
}

enum Geo<'a> {
    Rect([f32; 4]),
    Line([f32; 4]),
    Circle([f32; 3]),
    Tri([f32; 6]),
    Poly(Vec<Point>),
    Text(std::borrow::Cow<'a, str>),
    Clip([f32; 4]),
    Hit([f32; 4]),
    Image([f32; 4]),
}

fn read_item(p: &mut Parser, m: &dyn TextMeasure, depth: u32) -> Result<Item, Error> {
    let start = p.pos();
    p.begin_object()?;
    let mut geo: Option<Geo> = None;
    let mut used = 0u32;
    let (mut fill, mut stroke, mut color) = (None, None, None);
    let (mut width, mut radius, mut size) = (None, None, None);
    let mut at = None;
    let mut font: Option<Box<str>> = None;
    let mut bold = false;
    let mut italic = false;
    let mut round = false;
    let mut cursor = Cursor::Arrow;
    let mut id: Option<Box<str>> = None;
    let mut items = Vec::new();
    let mut src: Option<Box<str>> = None;
    let mut fit = Fit::Cover;

    while let Some(key) = p.next_key()? {
        let key_at = p.key_pos();
        let bit = match &*key {
            "rect" | "line" | "circle" | "tri" | "poly" | "text" | "clip" | "hit" | "image" => {
                if geo.is_some() {
                    return Err(p.error_at(key_at, "item has several shapes"));
                }
                geo = Some(match &*key {
                    "rect" => Geo::Rect(nums(p)?),
                    "line" => Geo::Line(nums(p)?),
                    "circle" => Geo::Circle(nums(p)?),
                    "tri" => Geo::Tri(nums(p)?),
                    "poly" => Geo::Poly(points(p)?),
                    "text" => Geo::Text(p.string()?),
                    "hit" => Geo::Hit(nums(p)?),
                    "image" => Geo::Image(nums(p)?),
                    _ => Geo::Clip(nums(p)?),
                });
                0
            }
            "fill" => {
                fill = Some(read_color(p)?);
                k::FILL
            }
            "stroke" => {
                stroke = Some(read_color(p)?);
                k::STROKE
            }
            "color" => {
                color = Some(read_color(p)?);
                k::COLOR
            }
            "width" => {
                width = Some(non_negative(p)?);
                k::WIDTH
            }
            "radius" => {
                radius = Some(non_negative(p)?);
                k::RADIUS
            }
            "size" => {
                size = Some(non_negative(p)?);
                k::SIZE
            }
            "at" => {
                let [x, y] = nums(p)?;
                at = Some(Point::new(x, y));
                k::AT
            }
            "font" => {
                font = Some(p.string()?.into());
                k::FONT
            }
            "bold" => {
                bold = p.boolean()?;
                k::BOLD
            }
            "italic" => {
                italic = p.boolean()?;
                k::ITALIC
            }
            "id" => {
                id = Some(p.string()?.into());
                k::ID
            }
            "cap" => {
                let at = p.pos();
                round = match &*p.string()? {
                    "round" => true,
                    "butt" => false,
                    _ => return Err(p.error_at(at, "cap must be 'round' or 'butt'")),
                };
                k::CAP
            }
            "cursor" => {
                let at = p.pos();
                let name = p.string()?;
                cursor = Cursor::parse(&name)
                    .ok_or_else(|| p.error_at(at, "cursor must be 'arrow', 'pointer', 'text', 'resize-x' or 'resize-y'"))?;
                k::CURSOR
            }
            "items" => {
                items = read_items(p, m, depth + 1)?;
                k::ITEMS
            }
            "src" => {
                src = Some(p.string()?.into());
                k::SRC
            }
            "fit" => {
                let at = p.pos();
                fit = Fit::parse(&p.string()?).ok_or_else(|| p.error_at(at, "fit must be 'cover', 'contain' or 'fill'"))?;
                k::FIT
            }
            _ => return Err(p.error_at(key_at, format!("unknown item key '{key}'"))),
        };
        if used & bit != 0 {
            return Err(p.error_at(key_at, format!("duplicate key '{key}'")));
        }
        used |= bit;
    }

    let Some(geo) = geo else {
        return Err(p.error_at(start, "item has no shape (rect, line, circle, tri, poly, text, clip, hit, image)"));
    };
    let (allowed, name) = match geo {
        Geo::Rect(_) => (k::FILL | k::STROKE | k::WIDTH | k::RADIUS | k::ID, "rect"),
        Geo::Line(_) => (k::COLOR | k::WIDTH | k::CAP | k::ID, "line"),
        Geo::Circle(_) => (k::FILL | k::STROKE | k::WIDTH | k::ID, "circle"),
        Geo::Tri(_) => (k::FILL | k::STROKE | k::WIDTH | k::ID, "tri"),
        Geo::Poly(_) => (k::FILL | k::STROKE | k::WIDTH | k::ID, "poly"),
        Geo::Text(_) => (k::AT | k::SIZE | k::COLOR | k::FONT | k::BOLD | k::ITALIC | k::ID, "text"),
        Geo::Clip(_) => (k::ITEMS | k::ID, "clip"),
        Geo::Hit(_) => (k::CURSOR | k::ID, "hit"),
        Geo::Image(_) => (k::SRC | k::FIT | k::RADIUS | k::ID, "image"),
    };
    let extra = used & !allowed;
    if extra != 0 {
        let bad = k::NAMES[extra.trailing_zeros() as usize];
        return Err(p.error_at(start, format!("'{bad}' is not allowed for {name}")));
    }

    let paint = Paint { fill, stroke, width: width.unwrap_or(1.0) };
    let needs_paint = |p: &Parser| {
        if fill.is_none() && stroke.is_none() {
            Err(p.error_at(start, format!("{name} needs 'fill' or 'stroke'")))
        } else {
            Ok(())
        }
    };
    let shape = match geo {
        Geo::Rect([x, y, w, h]) => {
            needs_paint(p)?;
            Shape::Rect { rect: Rect::new(x, y, w, h), radius: radius.unwrap_or(0.0), paint }
        }
        Geo::Line([x1, y1, x2, y2]) => Shape::Line {
            a: Point::new(x1, y1),
            b: Point::new(x2, y2),
            width: paint.width,
            color: color.unwrap_or(Color::BLACK),
            round,
        },
        Geo::Circle([x, y, r]) => {
            needs_paint(p)?;
            Shape::Circle { center: Point::new(x, y), radius: r, paint }
        }
        Geo::Tri([x1, y1, x2, y2, x3, y3]) => {
            needs_paint(p)?;
            Shape::Tri { points: [Point::new(x1, y1), Point::new(x2, y2), Point::new(x3, y3)], paint }
        }
        Geo::Poly(points) => {
            needs_paint(p)?;
            Shape::Poly { points: points.into(), paint }
        }
        Geo::Text(text) => {
            let at = at.ok_or_else(|| p.error_at(start, "text needs 'at'"))?;
            let font = Font { family: font, size: size.unwrap_or(Font::default().size), bold, italic };
            Shape::Text { at, text: text.into(), font, color: color.unwrap_or(Color::BLACK) }
        }
        Geo::Clip([x, y, w, h]) => Shape::Clip { rect: Rect::new(x, y, w, h), items },
        Geo::Image([x, y, w, h]) => {
            let src = src.ok_or_else(|| p.error_at(start, "image needs 'src'"))?;
            Shape::Image { rect: Rect::new(x, y, w, h), src, radius: radius.unwrap_or(0.0), fit }
        }
        Geo::Hit([x, y, w, h]) => {
            if id.is_none() {
                return Err(p.error_at(start, "hit needs 'id'"));
            }
            Shape::Hit { rect: Rect::new(x, y, w, h), cursor }
        }
    };
    let bounds = shape.bounds(m);
    Ok(Item { shape, id, bounds })
}

/// An array of exactly N numbers.
fn nums<const N: usize>(p: &mut Parser) -> Result<[f32; N], Error> {
    let start = p.pos();
    let mut out = [0.0; N];
    p.begin_array()?;
    for v in out.iter_mut() {
        if !p.next_item()? {
            return Err(p.error_at(start, format!("expected {N} numbers")));
        }
        *v = p.number()? as f32;
    }
    if p.next_item()? {
        return Err(p.error_at(start, format!("expected {N} numbers")));
    }
    Ok(out)
}

/// A flat coordinate array [x1, y1, x2, y2, …]: at least three points.
fn points(p: &mut Parser) -> Result<Vec<Point>, Error> {
    let start = p.pos();
    let mut xy = Vec::new();
    p.begin_array()?;
    while p.next_item()? {
        xy.push(p.number()? as f32);
    }
    if xy.len() < 6 || xy.len() % 2 != 0 {
        return Err(p.error_at(start, "poly needs pairs of numbers, at least 3 points"));
    }
    Ok(xy.as_chunks::<2>().0.iter().map(|&[x, y]| Point::new(x, y)).collect())
}

fn non_negative(p: &mut Parser) -> Result<f32, Error> {
    let at = p.pos();
    let v = p.number()?;
    if v < 0.0 {
        return Err(p.error_at(at, "must not be negative"));
    }
    Ok(v as f32)
}

fn read_color(p: &mut Parser) -> Result<Color, Error> {
    let at = p.pos();
    let s = p.string()?;
    Color::from_hex(&s).ok_or_else(|| p.error_at(at, "expected color #RGB, #RRGGBB or #RRGGBBAA"))
}
