//! NPC name tags: the yellow names under NPCs. A pass over the whole frame misses them, being
//! small and over scenery, so the yellow is keyed out and read alone. They only matter on maps
//! that share a header with another, so this runs for those and not on every picture.

use image::{Rgba, imageops};

use crate::error::Result;
use crate::vision::{Frame, Ocr, Rect, color, preprocess};

/// How far down the frame the game world goes, above the chat and the HUD.
const WORLD_BOTTOM: f32 = 0.84;
/// The tags are small, and their font reads best enlarged without smoothing. OCR also does far
/// better on one tag than on a tag in a big empty picture, so each is read alone.
const SCALE: u32 = 4;
/// A tag's size as parts of the frame's height, and the most space between its letters.
const MIN_HEIGHT: f32 = 0.008;
const MAX_HEIGHT: f32 = 0.03;
const LETTER_GAP: f32 = 0.012;

/// Tag text is pure yellow, about (251, 252, 0), with almost no blue even at its soft edges.
/// Yellow flowers and mushrooms lean orange, and leaves have some blue in them.
fn is_tag(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0.map(i16::from);
    r >= 150 && g >= 150 && b <= 40 && (r - g).abs() <= 30
}

/// Runs of yellow letters on one line, joined into tags. Scattered yellow in the scenery is too
/// small or too tall to pass.
fn tags(frame: &Frame) -> Vec<Rect> {
    let (width, height) = (frame.width() as f32, frame.height() as f32);
    let world = Rect::new(0.0, 0.0, width, height * WORLD_BOTTOM);
    let mut letters: Vec<Rect> =
        color::find_blocks(frame, world, 2, is_tag).into_iter().filter(|r| r.h <= MAX_HEIGHT * height).collect();
    letters.sort_by(|a, b| a.x.total_cmp(&b.x));
    let mut tags: Vec<Rect> = Vec::new();
    for letter in letters {
        match tags.iter_mut().find(|t| t.vertical_overlap(&letter) > 0.0 && letter.x - t.right() <= LETTER_GAP * height)
        {
            Some(tag) => *tag = tag.union(&letter),
            None => tags.push(letter),
        }
    }
    tags.retain(|t| (MIN_HEIGHT * height..=MAX_HEIGHT * height).contains(&t.h) && t.w >= t.h);
    tags
}

/// The text of every name tag on screen.
pub fn names(frame: &Frame, ocr: &Ocr) -> Result<Vec<String>> {
    let mut names = Vec::new();
    for tag in tags(frame) {
        let around = Rect::new(tag.x - 4.0, tag.y - 4.0, tag.w + 8.0, tag.h + 8.0);
        let words = ocr.words_in(frame, around, SCALE as f32, |crop| {
            let keyed = preprocess::keep(crop, is_tag);
            let (w, h) = (keyed.width() * SCALE, keyed.height() * SCALE);
            preprocess::pad(&imageops::resize(&keyed, w, h, imageops::FilterType::Nearest), 24)
        })?;
        let text: Vec<&str> = words.iter().map(|word| word.text.as_str()).collect();
        if !text.is_empty() {
            names.push(text.join(" "));
        }
    }
    Ok(names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_pure_yellow_is_a_tag() {
        for tag in [[251, 252, 0], [223, 229, 4], [182, 194, 6]] {
            assert!(is_tag(&Rgba([tag[0], tag[1], tag[2], 255])));
        }
        // A leaf, an orange flower, a white cloud.
        for other in [[154, 204, 85], [255, 204, 0], [250, 250, 250]] {
            assert!(!is_tag(&Rgba([other[0], other[1], other[2], 255])));
        }
    }
}
