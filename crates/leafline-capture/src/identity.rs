//! The character's job and name from the HUD on the website, which has no Windows OCR. The HUD
//! draws both in the Pickup Log's pixel font, so they are read by its glyphs
//! (`hud::identity_lines`); ocrs only fills in glyphs the font hasn't shown yet. The job is matched
//! against the game's jobs as the program matches its own reading.
//!
//! Measured on 1,043 saved frames against the program's (Windows OCR) readings: the name read
//! right on 1,025 of the 1,027 frames showing one, never wrong (the program misread 20, such as
//! a `zX` read as `nD<`); the job on all 1,025.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use leafline_core::readers::pickup::font::UNKNOWN;
use leafline_core::readers::hud::{self, HudLayout};
use leafline_core::vision::Frame;

use crate::ocr::{self, Recogniser};

/// The job and name as read: `None` for a part not read.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Identity {
    pub job: Option<String>,
    pub name: Option<String>,
}

#[derive(Default)]
pub struct IdentityReader {
    /// The last picture of the two lines (as ink) and what it read as; they rarely change.
    last: Option<(u64, Identity)>,
}

impl IdentityReader {
    pub fn read(&mut self, recogniser: &Recogniser, frame: &Frame, layout: &HudLayout) -> Identity {
        let drawn = hud::identity_lines(frame, layout);
        let key = {
            let mut hasher = DefaultHasher::new();
            for (_, rows) in &drawn {
                rows.hash(&mut hasher);
            }
            hasher.finish()
        };
        if let Some((last, identity)) = &self.last
            && *last == key
        {
            return identity.clone();
        }
        let identity = read(recogniser, frame, layout, &drawn);
        self.last = Some((key, identity.clone()));
        identity
    }
}

fn read(recogniser: &Recogniser, frame: &Frame, layout: &HudLayout, drawn: &[(String, Vec<Vec<bool>>)]) -> Identity {
    let known = |text: &str| !text.contains(UNKNOWN);
    let mut job = drawn.first().map(|(text, _)| text.clone()).filter(|t| known(t));
    let mut name = drawn.get(1).map(|(text, _)| text.clone()).filter(|t| known(t));
    // ocrs for a line with a glyph the font doesn't know yet, read as dark text on light, which it
    // reads best. Its reading fills in only the unknown glyphs of a name: on its own it confuses
    // letters a name can't be checked for (`y` as `v`, `f` as `t`). A job it reads is
    // matched against the game's jobs.
    if job.is_none() || name.is_none() {
        let read = ocr::crop(frame, layout.identity)
            .map(|crop| {
                let lines = ocr::ink_lines(&crop, hud::is_light_text);
                let inverted = Frame::from_fn(crop.width(), crop.height(), |x, y| {
                    let [r, g, b, a] = crop.get_pixel(x, y).0;
                    image::Rgba([255 - r, 255 - g, 255 - b, a])
                });
                recogniser.read_lines(&inverted, &lines)
            })
            .unwrap_or_default();
        if read.len() == 2 && drawn.len() == 2 {
            job = job.or_else(|| Some(read[0].clone()));
            name = name.or_else(|| fill(&drawn[1].0, &read[1]));
        }
    }
    Identity {
        job: job.and_then(|t| hud::job_named(&t)).map(String::from),
        name: name.and_then(|t| valid_name(&t)),
    }
}

/// A line read by the game's font with its unknown glyphs taken from ocrs's reading of it, where
/// the two line up glyph for glyph; a glyph the font knows outranks ocrs. `None` when they don't
/// line up.
fn fill(drawn: &str, read: &str) -> Option<String> {
    let (drawn, read): (Vec<char>, Vec<char>) = (drawn.chars().collect(), read.chars().collect());
    if drawn.len() != read.len() {
        return None;
    }
    drawn
        .iter()
        .zip(&read)
        .map(|(&d, &r)| match d {
            UNKNOWN if r != ' ' => Some(r),
            UNKNOWN => None,
            d if (d == ' ') != (r == ' ') => None,
            d => Some(d),
        })
        .collect()
}

/// A character name: letters and digits only, no spaces.
fn valid_name(text: &str) -> Option<String> {
    (text.len() >= 2 && text.len() <= 13 && text.chars().all(|c| c.is_ascii_alphanumeric())).then(|| text.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ocrs_fills_only_the_unknown_glyphs() {
        assert_eq!(fill("P?bbleThief", "pebbIeThiet").as_deref(), Some("PebbleThief"));
        assert_eq!(fill("P?bbleThief", "PebbleThie"), None, "they don't line up");
    }

    #[test]
    fn a_name_is_letters_and_digits() {
        assert_eq!(valid_name("Mossbank7").as_deref(), Some("Mossbank7"));
        assert_eq!(valid_name("tab +1"), None);
    }
}
