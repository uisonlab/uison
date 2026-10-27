//! Cursors from an Xcursor theme: when the compositor lacks `cursor-shape-v1` (GNOME 46, Weston)
//! and on X11. The format is simple: a table of contents and ARGB images (premultiplied) in several sizes.

use std::path::PathBuf;

use uison_core::Cursor;

pub struct Image {
    pub w: i32,
    pub h: i32,
    pub hot_x: i32,
    pub hot_y: i32,
    /// How many times the image is larger than nominal (for `set_buffer_scale`).
    pub scale: i32,
    pub pixels: Vec<u32>,
}

pub struct CursorTheme {
    dirs: Vec<PathBuf>,
    themes: Vec<String>,
    size: i32,
    cache: Vec<(&'static str, i32, Option<Image>)>,
}

impl CursorTheme {
    /// `theme`, `size` come from desktop settings (portal); `XCURSOR_THEME`/`XCURSOR_SIZE` take precedence.
    pub fn load(theme: Option<&str>, size: Option<i32>) -> CursorTheme {
        let home = std::env::var("HOME").unwrap_or_default();
        let dirs: Vec<PathBuf> = match std::env::var("XCURSOR_PATH") {
            Ok(p) => p.split(':').map(|d| PathBuf::from(d.replace('~', &home))).collect(),
            Err(_) => {
                let data = std::env::var("XDG_DATA_HOME").unwrap_or_else(|_| format!("{home}/.local/share"));
                [format!("{data}/icons"), format!("{home}/.icons"), "/usr/share/icons".into(), "/usr/share/pixmaps".into()]
                    .into_iter()
                    .map(PathBuf::from)
                    .collect()
            }
        };
        let first = std::env::var("XCURSOR_THEME")
            .ok()
            .filter(|t| !t.is_empty())
            .or_else(|| theme.map(str::to_string))
            .unwrap_or_else(|| "default".into());
        let size = std::env::var("XCURSOR_SIZE").ok().and_then(|s| s.parse().ok()).or(size).unwrap_or(24);
        let mut t = CursorTheme { dirs, themes: Vec::new(), size, cache: Vec::new() };
        t.add_theme(&first, 0);
        // Last resort: common themes.
        for fallback in ["Adwaita", "DMZ-White", "breeze_cursors", "hicolor"] {
            t.add_theme(fallback, 0);
        }
        t
    }

    /// The theme and those it inherits from (`Inherits=` in index.theme).
    fn add_theme(&mut self, name: &str, depth: u32) {
        if depth > 8 || self.themes.iter().any(|t| t == name) {
            return;
        }
        self.themes.push(name.to_string());
        let mut parents = Vec::new();
        for d in &self.dirs {
            for file in ["index.theme", "cursor.theme"] {
                if let Ok(s) = std::fs::read_to_string(d.join(name).join(file)) {
                    for line in s.lines() {
                        if let Some(v) = line.trim().strip_prefix("Inherits") {
                            let v = v.trim_start().trim_start_matches('=');
                            parents.extend(v.split([',', ';']).map(|p| p.trim().to_string()).filter(|p| !p.is_empty()));
                        }
                    }
                }
            }
        }
        for p in parents {
            self.add_theme(&p, depth + 1);
        }
    }

    /// Cursor image: the first name found from the list (CSS names and old X11 ones).
    pub fn image(&mut self, names: &[&'static str], scale: i32) -> Option<&Image> {
        let key = names[0];
        if let Some(i) = self.cache.iter().position(|(k, s, _)| *k == key && *s == scale) {
            return self.cache[i].2.as_ref();
        }
        let want = self.size * scale;
        let img = self.find(names, want).map(|mut img| {
            // An image larger than nominal is shown with a buffer scale.
            img.scale = if img.w >= self.size * scale && scale > 1 { scale } else { 1 };
            img
        });
        self.cache.push((key, scale, img));
        self.cache.last().unwrap().2.as_ref()
    }

    fn find(&self, names: &[&str], want: i32) -> Option<Image> {
        for theme in &self.themes {
            for d in &self.dirs {
                for n in names {
                    if let Ok(data) = std::fs::read(d.join(theme).join("cursors").join(n))
                        && let Some(img) = parse(&data, want)
                    {
                        return Some(img);
                    }
                }
            }
        }
        None
    }
}

/// The image whose nominal size is closest to `want`.
fn parse(d: &[u8], want: i32) -> Option<Image> {
    let u = |i: usize| d.get(i..i + 4).map(|s| u32::from_le_bytes(s.try_into().unwrap()));
    if d.get(..4)? != b"Xcur" {
        return None;
    }
    let ntoc = u(12)? as usize;
    const IMAGE: u32 = 0xfffd_0002;
    let mut best: Option<(u32, u32)> = None; // (nominal, position)
    for i in 0..ntoc.min(1024) {
        let e = 16 + i * 12;
        if u(e)? != IMAGE {
            continue;
        }
        let (size, pos) = (u(e + 4)?, u(e + 8)?);
        let better = best.is_none_or(|(s, _)| (size as i32 - want).abs() < (s as i32 - want).abs());
        if better {
            best = Some((size, pos));
        }
    }
    let p = best?.1 as usize;
    let (w, h, hx, hy) = (u(p + 16)?, u(p + 20)?, u(p + 24)?, u(p + 28)?);
    if w == 0 || h == 0 || w > 1024 || h > 1024 {
        return None;
    }
    let start = p + 36;
    let n = (w * h) as usize;
    let bytes = d.get(start..start + n * 4)?;
    let pixels = bytes.as_chunks::<4>().0.iter().map(|c| u32::from_le_bytes(*c)).collect();
    Some(Image { w: w as i32, h: h as i32, hot_x: hx as i32, hot_y: hy as i32, scale: 1, pixels })
}

/// Cursor names in themes: CSS first (new themes), then X11 (old ones).
pub fn names(c: Shape) -> &'static [&'static str] {
    match c {
        Shape::App(Cursor::Arrow) => &["default", "left_ptr", "arrow"],
        Shape::App(Cursor::Pointer) => &["pointer", "hand2", "hand1", "pointing_hand"],
        Shape::App(Cursor::Text) => &["text", "xterm", "ibeam"],
        Shape::Edge(1) => &["n-resize", "top_side"],
        Shape::Edge(2) => &["s-resize", "bottom_side"],
        Shape::Edge(4) => &["w-resize", "left_side"],
        Shape::Edge(8) => &["e-resize", "right_side"],
        Shape::Edge(5) => &["nw-resize", "top_left_corner"],
        Shape::Edge(9) => &["ne-resize", "top_right_corner"],
        Shape::Edge(6) => &["sw-resize", "bottom_left_corner"],
        Shape::Edge(10) => &["se-resize", "bottom_right_corner"],
        Shape::Edge(_) => &["default", "left_ptr"],
    }
}

/// Window cursor shape: from the app or an edge of our own frame.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    App(Cursor),
    Edge(u32),
}

impl Shape {
    /// Number in `cursor-shape-v1`.
    pub fn wp_shape(self) -> u32 {
        match self {
            Shape::App(Cursor::Arrow) => 1,
            Shape::App(Cursor::Pointer) => 4,
            Shape::App(Cursor::Text) => 9,
            Shape::Edge(1) => 19,
            Shape::Edge(2) => 22,
            Shape::Edge(4) => 25,
            Shape::Edge(8) => 18,
            Shape::Edge(5) => 21,
            Shape::Edge(9) => 20,
            Shape::Edge(6) => 24,
            Shape::Edge(10) => 23,
            Shape::Edge(_) => 1,
        }
    }
}
