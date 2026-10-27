#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Point {
        Point { x, y }
    }
}

/// Insets from the edges (the screen's safe area: notch, status bar, home indicator).
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Insets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Size {
    pub w: f32,
    pub h: f32,
}

impl Size {
    pub const fn new(w: f32, h: f32) -> Size {
        Size { w, h }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect { x, y, w, h }
    }

    pub fn from_size(s: Size) -> Rect {
        Rect::new(0.0, 0.0, s.w, s.h)
    }

    /// Bounding rectangle of a set of points.
    pub fn bounding(points: &[Point]) -> Rect {
        let (mut x0, mut y0) = (f32::INFINITY, f32::INFINITY);
        let (mut x1, mut y1) = (f32::NEG_INFINITY, f32::NEG_INFINITY);
        for p in points {
            x0 = x0.min(p.x);
            y0 = y0.min(p.y);
            x1 = x1.max(p.x);
            y1 = y1.max(p.y);
        }
        Rect::new(x0, y0, x1 - x0, y1 - y0)
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn is_empty(&self) -> bool {
        !(self.w > 0.0 && self.h > 0.0)
    }

    pub fn area(&self) -> f32 {
        if self.is_empty() { 0.0 } else { self.w * self.h }
    }

    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.y >= self.y && p.x < self.right() && p.y < self.bottom()
    }

    /// `o` lies entirely inside.
    pub fn covers(&self, o: &Rect) -> bool {
        o.x >= self.x && o.y >= self.y && o.right() <= self.right() && o.bottom() <= self.bottom()
    }

    pub fn intersects(&self, o: &Rect) -> bool {
        self.x < o.right() && o.x < self.right() && self.y < o.bottom() && o.y < self.bottom()
    }

    /// Intersection; empty if the rectangles don't intersect.
    pub fn intersect(&self, o: &Rect) -> Rect {
        let x = self.x.max(o.x);
        let y = self.y.max(o.y);
        let r = self.right().min(o.right());
        let b = self.bottom().min(o.bottom());
        if r > x && b > y { Rect::new(x, y, r - x, b - y) } else { Rect::default() }
    }

    /// Union. Empty rectangles are ignored.
    pub fn union(&self, o: &Rect) -> Rect {
        if self.is_empty() {
            return *o;
        }
        if o.is_empty() {
            return *self;
        }
        let x = self.x.min(o.x);
        let y = self.y.min(o.y);
        Rect::new(x, y, self.right().max(o.right()) - x, self.bottom().max(o.bottom()) - y)
    }

    pub fn inflate(&self, d: f32) -> Rect {
        Rect::new(self.x - d, self.y - d, self.w + 2.0 * d, self.h + 2.0 * d)
    }

    /// Expand to integer coordinates.
    pub fn round_out(&self) -> Rect {
        let x = self.x.floor();
        let y = self.y.floor();
        Rect::new(x, y, self.right().ceil() - x, self.bottom().ceil() - y)
    }
}
