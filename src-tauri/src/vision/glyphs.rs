//! Reading fixed bitmap fonts by template matching.
//!
//! General OCR misreads small pixel fonts (9 as S, 953 as "ss.3"), but such a font draws each
//! character identically every time, so comparing glyph shapes is exact. The templates are
//! learned on the user's own screen during calibration: no game font data ships with the app.

use image::Rgba;
use serde::{Deserialize, Serialize};

use super::{Frame, Rect};

/// Minimum share of ink two glyphs must have in common to count as the same character.
pub const MIN_SIMILARITY: f32 = 0.8;

/// A glyph's ink pixels.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(into = "Vec<String>", try_from = "Vec<String>")]
pub struct Mask {
    width: u32,
    height: u32,
    ink: Vec<bool>,
}

impl Mask {
    /// The ink of columns `left..right` and rows `top..bottom` of `frame`.
    pub fn new(
        frame: &Frame,
        (left, right): (u32, u32),
        (top, bottom): (u32, u32),
        is_ink: impl Fn(&Rgba<u8>) -> bool,
    ) -> Self {
        let ink =
            (top..bottom).flat_map(|y| (left..right).map(move |x| (x, y))).map(|(x, y)| is_ink(frame.get_pixel(x, y)));
        Self { width: right - left, height: bottom - top, ink: ink.collect() }
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    fn get(&self, x: u32, y: u32) -> bool {
        x < self.width && y < self.height && self.ink[(y * self.width + x) as usize]
    }

    /// Shared ink over combined ink, with both masks aligned at the top left. Glyphs whose
    /// size differs by more than a pixel are never the same character.
    pub fn similarity(&self, other: &Mask) -> f32 {
        if self.width.abs_diff(other.width) > 1 || self.height.abs_diff(other.height) > 1 {
            return 0.0;
        }
        let (mut shared, mut combined) = (0u32, 0u32);
        for y in 0..self.height.max(other.height) {
            for x in 0..self.width.max(other.width) {
                let (a, b) = (self.get(x, y), other.get(x, y));
                shared += u32::from(a && b);
                combined += u32::from(a || b);
            }
        }
        if combined == 0 { 1.0 } else { shared as f32 / combined as f32 }
    }
}

impl From<Mask> for Vec<String> {
    fn from(mask: Mask) -> Self {
        mask.ink
            .chunks(mask.width.max(1) as usize)
            .map(|row| row.iter().map(|&i| if i { '#' } else { '.' }).collect())
            .collect()
    }
}

impl TryFrom<Vec<String>> for Mask {
    type Error = String;

    fn try_from(rows: Vec<String>) -> Result<Self, String> {
        let width = rows.first().map_or(0, |r| r.chars().count()) as u32;
        if rows.iter().any(|r| r.chars().count() as u32 != width) {
            return Err("glyph rows differ in length".into());
        }
        Ok(Self {
            width,
            height: rows.len() as u32,
            ink: rows.iter().flat_map(|r| r.chars().map(|c| c == '#')).collect(),
        })
    }
}

/// Runs of columns in `region` that contain ink, as `(left, right)` pixel ranges.
pub fn ink_columns(frame: &Frame, region: Rect, is_ink: impl Fn(&Rgba<u8>) -> bool) -> Vec<(u32, u32)> {
    let (x, y, w, h) = region.to_pixels(frame.width(), frame.height());
    let mut runs = Vec::new();
    let mut start = None;
    for col in x..=x + w {
        let inked = col < x + w && (y..y + h).any(|row| is_ink(frame.get_pixel(col, row)));
        match (inked, start) {
            (true, None) => start = Some(col),
            (false, Some(s)) => {
                runs.push((s, col));
                start = None;
            }
            _ => {}
        }
    }
    runs
}

/// The first and last rows of `region` that contain ink, as a `(top, bottom)` range.
pub fn ink_rows(frame: &Frame, region: Rect, is_ink: impl Fn(&Rgba<u8>) -> bool) -> Option<(u32, u32)> {
    let (x, y, w, h) = region.to_pixels(frame.width(), frame.height());
    let inked = |row: &u32| (x..x + w).any(|col| is_ink(frame.get_pixel(col, *row)));
    let top = (y..y + h).find(inked)?;
    let bottom = (y..y + h).rev().find(inked)? + 1;
    Some((top, bottom))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Glyph {
    char: char,
    mask: Mask,
}

/// The characters of one font at one or more sizes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlyphSet {
    glyphs: Vec<Glyph>,
}

impl GlyphSet {
    /// Records `masks` as the characters of `text`, one each, replacing any earlier template
    /// for the same character and size. Returns false, learning nothing, when the counts differ.
    pub fn learn(&mut self, masks: &[Mask], text: &str) -> bool {
        let chars: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
        if chars.len() != masks.len() {
            return false;
        }
        for (&char, mask) in chars.iter().zip(masks) {
            self.glyphs.retain(|g| !(g.char == char && g.mask.height == mask.height));
            self.glyphs.push(Glyph { char, mask: mask.clone() });
        }
        true
    }

    /// The character `mask` shows, if it matches a learned glyph closely enough.
    pub fn recognize(&self, mask: &Mask) -> Option<char> {
        self.glyphs
            .iter()
            .map(|g| (g.mask.similarity(mask), g.char))
            .filter(|&(score, _)| score >= MIN_SIMILARITY)
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, char)| char)
    }

    /// Reads every mask; `None` if any is unknown, since one wrong digit spoils a number.
    pub fn read(&self, masks: &[Mask]) -> Option<String> {
        masks.iter().map(|m| self.recognize(m)).collect()
    }

    /// Whether any glyph was learned at text height `height` (give or take a pixel).
    pub fn knows_size(&self, height: u32) -> bool {
        self.glyphs.iter().any(|g| g.mask.height.abs_diff(height) <= 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);
    const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);

    /// A frame drawn from rows of `#` (ink) and `.` (background).
    fn frame(rows: &[&str]) -> Frame {
        Frame::from_fn(rows[0].len() as u32, rows.len() as u32, |x, y| {
            if rows[y as usize].as_bytes()[x as usize] == b'#' { WHITE } else { BLACK }
        })
    }

    fn is_ink(px: &Rgba<u8>) -> bool {
        px.0[0] > 128
    }

    fn masks(frame: &Frame) -> Vec<Mask> {
        let all = Rect::new(0.0, 0.0, frame.width() as f32, frame.height() as f32);
        let rows = ink_rows(frame, all, is_ink).unwrap();
        ink_columns(frame, all, is_ink).into_iter().map(|cols| Mask::new(frame, cols, rows, is_ink)).collect()
    }

    // Two made-up glyphs: a bar and a box.
    const LEARN: [&str; 3] = ["#.###", "#.#.#", "#.###"];
    const READ: [&str; 3] = ["###.#.#", "#.#.#.#", "###.#.#"];

    #[test]
    fn learns_and_reads_glyphs() {
        let mut set = GlyphSet::default();
        assert!(set.learn(&masks(&frame(&LEARN)), "10"));
        assert_eq!(set.read(&masks(&frame(&READ))), Some("011".into()));
        assert!(set.knows_size(3));
        assert!(!set.knows_size(9));
    }

    #[test]
    fn unknown_glyph_spoils_the_reading() {
        let mut set = GlyphSet::default();
        set.learn(&masks(&frame(&LEARN)), "10");
        assert_eq!(set.read(&masks(&frame(&["#.##", "#.#.", "#.##"]))), None);
    }

    #[test]
    fn count_mismatch_learns_nothing() {
        let mut set = GlyphSet::default();
        assert!(!set.learn(&masks(&frame(&LEARN)), "100"));
        assert!(!set.knows_size(3));
    }

    #[test]
    fn masks_round_trip_through_json() {
        let mask = masks(&frame(&LEARN)).remove(1);
        let json = serde_json::to_string(&mask).unwrap();
        assert_eq!(json, r####"["###","#.#","###"]"####);
        assert_eq!(serde_json::from_str::<Mask>(&json).unwrap(), mask);
    }
}
