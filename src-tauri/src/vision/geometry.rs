use serde::Serialize;

/// An axis-aligned rectangle in source-image pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn from_edges(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self::new(left, top, (right - left).max(0.0), (bottom - top).max(0.0))
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn center_y(&self) -> f32 {
        self.y + self.h / 2.0
    }

    pub fn union(&self, other: &Rect) -> Rect {
        Rect::from_edges(
            self.x.min(other.x),
            self.y.min(other.y),
            self.right().max(other.right()),
            self.bottom().max(other.bottom()),
        )
    }

    /// Height of the band where both rectangles overlap vertically (0 if none).
    pub fn vertical_overlap(&self, other: &Rect) -> f32 {
        (self.bottom().min(other.bottom()) - self.y.max(other.y)).max(0.0)
    }

    pub fn scaled(&self, factor: f32) -> Rect {
        Rect::new(self.x * factor, self.y * factor, self.w * factor, self.h * factor)
    }

    pub fn translated(&self, dx: f32, dy: f32) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.w, self.h)
    }

    /// Clamps to a `width` x `height` image and returns whole-pixel `(x, y, w, h)`.
    pub fn to_pixels(&self, width: u32, height: u32) -> (u32, u32, u32, u32) {
        let left = self.x.floor().clamp(0.0, width as f32) as u32;
        let top = self.y.floor().clamp(0.0, height as f32) as u32;
        let right = self.right().ceil().clamp(0.0, width as f32) as u32;
        let bottom = self.bottom().ceil().clamp(0.0, height as f32) as u32;
        (left, top, right - left, bottom - top)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_and_overlap() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(5.0, 6.0, 10.0, 10.0);
        assert_eq!(a.union(&b), Rect::new(0.0, 0.0, 15.0, 16.0));
        assert_eq!(a.vertical_overlap(&b), 4.0);
        assert_eq!(a.vertical_overlap(&Rect::new(0.0, 20.0, 1.0, 1.0)), 0.0);
    }

    #[test]
    fn to_pixels_clamps() {
        let r = Rect::new(-5.0, 2.5, 20.0, 100.0);
        assert_eq!(r.to_pixels(10, 50), (0, 2, 10, 48));
    }
}
