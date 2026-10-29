//! The meso count: the white box with a gold coin at its left, at the bottom of the inventory
//! (and in shop windows).
//!
//! It is found by appearance rather than by the window's title, which is in a small pixel font
//! OCR rarely reads, and which other windows often cover.

use std::sync::LazyLock;

use image::{Rgba, imageops};
use regex::Regex;
use serde::Serialize;

use super::text::parse_grouped;
use crate::error::Result;
use crate::vision::{Frame, Ocr, Rect, color, preprocess};

#[derive(Debug, Clone, Copy, Serialize)]
pub struct MesoReading {
    pub mesos: u64,
    pub rect: Rect,
}

/// A grouped amount such as `36,644`, or a small one such as `500`.
static AMOUNT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:\d{1,3}(?:[,.]\d{3})+|\d{1,3})$").unwrap());

/// Reads every meso box on screen. Storage and shop windows show a second box (the storage's
/// own mesos, for one), so the amount most boxes agree on is taken as the player's.
pub fn read(frame: &Frame, ocr: &Ocr) -> Result<Option<MesoReading>> {
    let mut readings = Vec::new();
    for rect in locate(frame) {
        // The amount is right-aligned; the empty left of the box only confuses OCR.
        let amount = Rect::from_edges(
            rect.x + 0.4 * rect.w,
            rect.y - 0.15 * rect.h,
            rect.right(),
            rect.bottom() + 0.15 * rect.h,
        );
        if let Some(mesos) = read_amount(frame, amount, ocr)? {
            readings.push(MesoReading { mesos, rect });
        }
    }
    let agreeing = |m: &MesoReading| readings.iter().filter(|r| r.mesos == m.mesos).count();
    let mut best: Option<(MesoReading, usize)> = None;
    for reading in &readings {
        let n = agreeing(reading);
        if best.is_none_or(|(_, most)| n > most) {
            best = Some((*reading, n));
        }
    }
    Ok(best.map(|(reading, _)| reading))
}

/// OCR of the box's dark digits at several sizes. The pixel font's 6 and 8 are easily
/// confused, so an amount counts only when at least two readings agree on it and no other
/// amount is read as often.
fn read_amount(frame: &Frame, rect: Rect, ocr: &Ocr) -> Result<Option<u64>> {
    use imageops::FilterType::{CatmullRom, Triangle};
    let mut readings = Vec::new();
    for (scale, filter) in [(1, Triangle), (2, Triangle), (2, CatmullRom), (3, Triangle), (3, CatmullRom)] {
        let words = ocr.words_in(frame, rect, scale as f32, |crop| {
            let keyed = preprocess::keep(crop, |p| preprocess::luminance(p) <= 80);
            let (w, h) = (keyed.width() * scale, keyed.height() * scale);
            preprocess::pad(&imageops::resize(&keyed, w, h, filter), 40)
        })?;
        let text: String = words.iter().map(|w| w.text.as_str()).collect();
        if AMOUNT.is_match(&text)
            && let Some(amount) = parse_grouped(&text)
        {
            readings.push(amount);
        }
    }
    let votes = |amount: u64| readings.iter().filter(|&&r| r == amount).count();
    let most = readings.iter().map(|&r| votes(r)).max().unwrap_or(0);
    let winners: Vec<u64> = readings.iter().copied().filter(|&r| votes(r) == most).collect();
    Ok((most >= 2 && winners.iter().all(|&w| w == winners[0])).then(|| winners[0]))
}

/// White boxes about one text line tall with a gold coin just to their left.
pub fn locate(frame: &Frame) -> Vec<Rect> {
    let (width, height) = (frame.width() as f32, frame.height() as f32);
    let whole = Rect::new(0.0, 0.0, width, height);
    color::find_blocks(frame, whole, (width * 0.04) as u32, is_box_white)
        .into_iter()
        .filter(|b| (0.01 * height..=0.03 * height).contains(&b.h) && b.w >= 5.0 * b.h)
        .filter(|b| {
            let coin = Rect::from_edges(b.x - 1.5 * b.h, b.y, b.x, b.bottom());
            let gold = color::find_blocks(frame, coin, 2, is_coin_gold);
            gold.iter().map(|g| g.w * g.h).sum::<f32>() >= 0.15 * b.h * b.h
        })
        .collect()
}

fn is_box_white(px: &Rgba<u8>) -> bool {
    preprocess::luminance(px) >= 235 && preprocess::saturation(px) <= 20
}

fn is_coin_gold(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0;
    r >= 200 && g >= 140 && b <= 90
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts() {
        for good in ["36,644", "1,234,567", "500", "42.944"] {
            assert!(AMOUNT.is_match(good), "{good}");
        }
        for bad in ["36,64", "1234", "", "12,3456"] {
            assert!(!AMOUNT.is_match(bad), "{bad}");
        }
    }

    #[test]
    fn finds_the_box_beside_a_coin() {
        let frame = Frame::from_fn(400, 600, |x, y| {
            let in_box = (60..300).contains(&x) && (100..112).contains(&y);
            let in_coin = (44..56).contains(&x) && (100..112).contains(&y);
            if in_box {
                Rgba([255, 255, 255, 255])
            } else if in_coin {
                Rgba([255, 200, 0, 255])
            } else {
                Rgba([40, 40, 60, 255])
            }
        });
        assert_eq!(locate(&frame), vec![Rect::new(60.0, 100.0, 240.0, 12.0)]);
    }
}
