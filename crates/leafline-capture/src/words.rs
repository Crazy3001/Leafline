//! Single lines of dark text on a light panel read by ocrs, for the readers that hand the rest to
//! the game's own fonts (the Quest Helper's titles, header and some targets). Each picture's
//! reading is kept: the same few titles stay on screen for minutes.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use image::Rgba;
use leafline_core::vision::{Frame, Rect};

use crate::ocr::{self, InkLine, Recogniser};

/// Brightness (of a pixel's strongest channel) taken to white.
const STRETCH: u32 = 130;
/// Readings kept before the oldest are forgotten.
const KEEP: usize = 256;

#[derive(Default)]
pub struct DarkText {
    read: HashMap<u64, Option<String>>,
}

impl DarkText {
    /// The text in `area` of `frame`, read as one line; `None` with no ink there.
    pub fn read(&mut self, recogniser: &Recogniser, frame: &Frame, area: Rect) -> Option<String> {
        let crop = ocr::crop(frame, area)?;
        let keyed = Frame::from_fn(crop.width(), crop.height(), |x, y| {
            if is_dark(crop.get_pixel(x, y)) { Rgba([0, 0, 0, 255]) } else { Rgba([255, 255, 255, 255]) }
        });
        let key = {
            let mut hasher = DefaultHasher::new();
            keyed.dimensions().hash(&mut hasher);
            keyed.as_raw().hash(&mut hasher);
            hasher.finish()
        };
        if let Some(text) = self.read.get(&key) {
            return text.clone();
        }
        // The words where the keyed text is, read from the picture with its contrast stretched
        // as the program's own passes stretch it: scenery seen through the panel fades to white,
        // while the letters keep their soft edges, which ocrs reads better than a hard key.
        let stretched = Frame::from_fn(crop.width(), crop.height(), |x, y| {
            let [r, g, b, _] = crop.get_pixel(x, y).0;
            let v = (u32::from(r.max(g).max(b)) * 255 / STRETCH).min(255) as u8;
            Rgba([v, v, v, 255])
        });
        let text = bounds(&keyed)
            .and_then(|line| recogniser.read_lines(&stretched, &[line]).pop())
            .filter(|t| !t.is_empty());
        if self.read.len() >= KEEP {
            self.read.clear();
        }
        self.read.insert(key, text.clone());
        text
    }
}

/// The ink's bounds in a keyed picture, as one line.
fn bounds(keyed: &Frame) -> Option<InkLine> {
    let black = |x: u32, y: u32| keyed.get_pixel(x, y).0[0] == 0;
    let (w, h) = keyed.dimensions();
    let top = (0..h).find(|&y| (0..w).any(|x| black(x, y)))?;
    let bottom = (0..h).rev().find(|&y| (0..w).any(|x| black(x, y)))? + 1;
    let left = (0..w).find(|&x| (0..h).any(|y| black(x, y)))?;
    let right = (0..w).rev().find(|&x| (0..h).any(|y| black(x, y)))? + 1;
    Some(InkLine { top, bottom, left, right })
}

/// The panel's black text and the soft edges scaling gives it, as the program's keyed pass takes
/// it (all channels under 64).
fn is_dark(px: &Rgba<u8>) -> bool {
    px.0[..3].iter().all(|&c| c < 64)
}
