//! ocrs as an OCR engine for the program's readers that read a picture's words (the item
//! tooltip's), on the website. The program hands Windows OCR whole pictures; ocrs here gets only
//! the recognition model, so each picture is keyed to black text on white first and its lines
//! found from the ink. A picture already keyed (black on white) is read as it is; any other is
//! keyed by its light, grey text, the colour of a tooltip's.

use std::sync::Arc;

use image::Rgba;
use leafline_core::error::Result;
use leafline_core::vision::{Frame, TextEngine, Word, preprocess};

use crate::ocr::{self, Recogniser};

pub struct OcrsEngine {
    recogniser: Arc<Recogniser>,
}

impl OcrsEngine {
    pub fn new(recogniser: Arc<Recogniser>) -> Self {
        Self { recogniser }
    }
}

impl TextEngine for OcrsEngine {
    fn words(&self, image: &Frame) -> Result<Vec<Word>> {
        let keyed = keyed(image);
        let lines = ocr::ink_lines(&keyed, |p| p.0[0] == 0);
        Ok(self
            .recogniser
            .read_words(&keyed, &lines)
            .into_iter()
            .flatten()
            .filter(|(text, _)| !text.trim().is_empty())
            .map(|(text, rect)| Word { text, rect })
            .collect())
    }
}

/// `image` as black text on white.
fn keyed(image: &Frame) -> Frame {
    let two_tone = image.pixels().all(|p| p.0[..3].iter().all(|&c| c == 0 || c == 255));
    if two_tone {
        return image.clone();
    }
    let total: u64 = image.pixels().map(|p| u64::from(preprocess::luminance(p))).sum();
    let dark = total < 128 * u64::from(image.width()) * u64::from(image.height());
    Frame::from_fn(image.width(), image.height(), |x, y| {
        let p = image.get_pixel(x, y);
        let ink = if dark {
            preprocess::luminance(p) >= 150 && preprocess::saturation(p) <= 80
        } else {
            preprocess::luminance(p) <= 100
        };
        if ink { Rgba([0, 0, 0, 255]) } else { Rgba([255, 255, 255, 255]) }
    })
}
