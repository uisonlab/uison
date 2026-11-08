//! Places rectangles in the atlas texture on shelves: rows whose height is set by the first
//! entry. Nothing is freed one by one: when the atlas is full it starts over (`reset`).

pub struct Shelves {
    size: u32,
    /// (y, height, next free x) per shelf.
    shelves: Vec<(u32, u32, u32)>,
    bottom: u32,
}

impl Shelves {
    pub fn new(size: u32) -> Shelves {
        Shelves { size, shelves: Vec::new(), bottom: 0 }
    }

    pub fn reset(&mut self) {
        self.shelves.clear();
        self.bottom = 0;
    }

    /// The top-left corner for a `w`×`h` rectangle; `None` — no room left.
    pub fn place(&mut self, w: u32, h: u32) -> Option<(u32, u32)> {
        if w > self.size || h > self.size {
            return None;
        }
        // The lowest shelf that fits without wasting more than a third of its height.
        let best = self
            .shelves
            .iter_mut()
            .filter(|s| s.1 >= h && s.1 <= h + h / 2 + 2 && self.size - s.2 >= w)
            .min_by_key(|s| s.1);
        if let Some(s) = best {
            let x = s.2;
            s.2 += w;
            return Some((x, s.0));
        }
        if self.size - self.bottom < h {
            return None;
        }
        let y = self.bottom;
        self.shelves.push((y, h, w));
        self.bottom += h;
        Some((0, y))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_shelves_then_runs_out() {
        let mut a = Shelves::new(100);
        assert_eq!(a.place(60, 10), Some((0, 0)));
        assert_eq!(a.place(40, 10), Some((60, 0)));
        // The first shelf is full: a new one below.
        assert_eq!(a.place(10, 12), Some((0, 10)));
        assert_eq!(a.place(10, 30), Some((0, 22)));
        assert_eq!(a.place(10, 60), None);
        a.reset();
        assert_eq!(a.place(100, 100), Some((0, 0)));
    }
}
