//! The meso count: the white box with a gold coin at its left, at the bottom of the inventory
//! (and in shop windows).
//!
//! It is found by appearance rather than by the window's title, which is in a small pixel font
//! OCR rarely reads, and which other windows often cover.

use std::sync::LazyLock;

use image::{Rgba, imageops};
use regex::Regex;
use serde::Serialize;

use super::text::{digitize, parse_grouped};
use crate::error::Result;
use crate::vision::glyphs::{MIN_SIMILARITY, Mask, ink_columns, ink_rows};
use crate::vision::{Frame, Ocr, Rect, color, preprocess};

#[derive(Debug, Clone, Copy, Serialize)]
pub struct MesoReading {
    pub mesos: u64,
    pub rect: Rect,
}

/// A grouped amount such as `36,644`, or a small one such as `500`.
static AMOUNT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:\d{1,3}(?:[,.]\d{3})+|\d{1,3})$").unwrap());

/// Amounts of up to this many characters get a second try when OCR finds nothing.
const SHORT_AMOUNT: usize = 3;
/// How many times a short amount is drawn, and the pixels between copies. Tried on the game's
/// own `0`: OCR read fewer copies only some of the time, and these every time.
const COPIES: usize = 5;
const COPY_GAP: u32 = 2;
/// Left off the box's right end, in box heights: past the border, short of the last digit,
/// which ends about 0.35 box heights from the edge.
const BORDER: f32 = 0.15;

/// Reads every meso box on screen. Storage and trade windows show more boxes (the storage's
/// own mesos, for one), so the amount most boxes agree on is taken as the player's. Also
/// returns how many boxes there were, read or not.
/// How dark a pixel must be to count as the amount's ink.
#[derive(Debug, Clone, Copy)]
pub struct Ink {
    /// Kept for OCR, at or below this luminance.
    pub ocr: u8,
    /// Used to cut the digits apart and compare their shapes. Lower than `ocr` where the strokes
    /// are smoothed, so that touching edges don't join two digits.
    pub shape: u8,
}

impl Ink {
    /// The game's crisp digits as drawn at Ideal Ratio.
    pub const CRISP: Ink = Ink { ocr: 80, shape: 80 };
    /// Thin smoothed strokes, as in a frame redrawn from UI Size Default Ratio. Tried on one
    /// frame with Filter Applied: OCR read the amount at 180 but not at 150, and the digits
    /// came apart at 150 but touched at 180.
    pub const SMOOTHED: Ink = Ink { ocr: 180, shape: 150 };
}

pub fn read(frame: &Frame, ocr: &Ocr) -> Result<(Option<MesoReading>, usize)> {
    read_with(frame, ocr, Ink::CRISP)
}

pub fn read_with(frame: &Frame, ocr: &Ocr, ink: Ink) -> Result<(Option<MesoReading>, usize)> {
    let boxes = locate(frame);
    let mut readings = Vec::new();
    for &rect in &boxes {
        if let Some(mesos) = read_amount(frame, amount_area(rect), ocr, ink, whole_box(frame, rect))? {
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
    Ok((best.map(|(reading, _)| reading), boxes.len()))
}

/// Where the amount is in a meso box. It is right-aligned; the empty left of the box and its
/// border only confuse OCR. Redrawn from UI Size Default Ratio, the box can take in its dark
/// right border too.
fn amount_area(rect: Rect) -> Rect {
    Rect::from_edges(rect.x + 0.4 * rect.w, rect.y, rect.right() - BORDER * rect.h, rect.bottom())
}

/// The amount's glyphs in each meso box, for building the digit table from frames.
pub fn box_glyphs(frame: &Frame, ink: Ink) -> Vec<Vec<Mask>> {
    locate(frame).into_iter().map(|rect| glyphs(frame, amount_area(rect), ink.shape)).collect()
}

/// Whether each box ends at its own border rather than under an item's tooltip: just past a
/// box's thin border the window's frame shows no tooltip blue, while just past a box a tooltip
/// cuts short most of it is (measured on 2026-11-20 frames: none against 60-100% from the fifth
/// column on). A tooltip further right, over the MESOS label, leaves the amount whole.
pub fn whole_boxes(frame: &Frame) -> Vec<bool> {
    locate(frame).into_iter().map(|rect| whole_box(frame, rect)).collect()
}

/// Above this share of tooltip blue just right of a box, the box is taken as cut short.
const COVERED: f32 = 0.3;

fn whole_box(frame: &Frame, rect: Rect) -> bool {
    let (x0, y0, w, h) = rect.to_pixels(frame.width(), frame.height());
    // Past the border (two pixels at Ideal Ratio, less than 0.15 box heights), short of the label.
    let from = x0 + w + (0.15 * h as f32).ceil() as u32;
    let to = (x0 + w + (0.4 * h as f32).ceil() as u32).min(frame.width());
    if from >= to {
        return true;
    }
    let mut blue = 0;
    for x in from..to {
        for y in y0..y0 + h {
            blue += usize::from(is_tooltip_blue(frame.get_pixel(x, y)));
        }
    }
    (blue as f32) < COVERED * ((to - from) * h) as f32
}

/// An item tooltip's dark, see-through blue.
fn is_tooltip_blue(px: &Rgba<u8>) -> bool {
    let luma = preprocess::luminance(px);
    (40..=130).contains(&luma) && i32::from(px.0[2]) - i32::from(px.0[0]) >= 30
}

/// How far each box's amount stops short of its right end, in box heights.
pub fn right_gaps(frame: &Frame, ink: Ink) -> Vec<Option<f32>> {
    locate(frame).into_iter().map(|rect| right_gap(frame, amount_area(rect), ink)).collect()
}

/// The most room, in box heights, between an amount's last digit and the right end of
/// [`amount_area`]: the game leaves about 0.2.
const RIGHT_GAP: f32 = 0.45;

/// The blank between the amount's last ink and `rect`'s right end, in box heights.
fn right_gap(frame: &Frame, rect: Rect, ink: Ink) -> Option<f32> {
    let (_, end) = *ink_columns(frame, rect, |px| is_digit(px, ink.shape)).last()?;
    Some((rect.right() - end as f32) / rect.h)
}

/// OCR of the box's dark digits at several sizes. The pixel font's 6 and 8 are easily
/// confused, so an amount counts only when at least two readings agree on it, no other
/// amount is read as often, and it fits the digits' shapes.
///
/// The game's own digit shapes come first: when every glyph is one of them, that is the amount,
/// and OCR (which the website doesn't have) is only asked when one isn't known.
fn read_amount(frame: &Frame, rect: Rect, ocr: &Ocr, ink: Ink, whole: bool) -> Result<Option<u64>> {
    use imageops::FilterType::{CatmullRom, Triangle};
    let glyphs = glyphs(frame, rect, ink.shape);
    // The amount is right-aligned, so one that stops short of the right end is partly covered
    // (an item's tooltip over the inventory) and its digits so far aren't the amount.
    if whole
        && right_gap(frame, rect, ink).is_some_and(|gap| gap <= RIGHT_GAP)
        && let Some(amount) = read_shapes(&glyphs)
    {
        return Ok(Some(amount));
    }
    let mut readings = Vec::new();
    for (scale, filter) in [(1, Triangle), (2, Triangle), (2, CatmullRom), (3, Triangle), (3, CatmullRom)] {
        let words = ocr.words_in(frame, rect, scale as f32, |crop| {
            let keyed = preprocess::keep(crop, |p| preprocess::luminance(p) <= ink.ocr);
            let (w, h) = (keyed.width() * scale, keyed.height() * scale);
            preprocess::pad(&imageops::resize(&keyed, w, h, filter), 40)
        })?;
        let text: String = words.iter().map(|w| w.text.as_str()).collect();
        let text = settle_sixes(&text, &glyphs);
        if AMOUNT.is_match(&text)
            && fits_shapes(&text, &glyphs)
            && let Some(amount) = parse_grouped(&text)
        {
            readings.push(amount);
        }
    }
    if readings.is_empty() && (1..=SHORT_AMOUNT).contains(&glyphs.len()) {
        readings = read_repeated(frame, rect, &glyphs, ocr, ink.shape)?;
    }
    let votes = |amount: u64| readings.iter().filter(|&&r| r == amount).count();
    let most = readings.iter().map(|&r| votes(r)).max().unwrap_or(0);
    let winners: Vec<u64> = readings.iter().copied().filter(|&r| votes(r) == most).collect();
    Ok((most >= 2 && winners.iter().all(|&w| w == winners[0])).then(|| winners[0]))
}

/// The amount's characters in the game's pixel fonts, a row of font pixels per string and `#`
/// for ink: the inventory's digits are 8 font pixels tall, the shop window's 9. Taken on
/// 2026-11-20 from about 190 amounts on frames at Ideal Ratio (1x and 2x, the program's and a
/// browser's window share alike), labelled by the program's OCR; every copy of a character was
/// drawn the same. Characters not seen yet are left out (most of the shop's digits), so an amount
/// holding one goes to OCR. The comma keeps the blank rows above it, down from the digits' top.
const SHAPES: &[(char, &[&str])] = &[
    ('0', &[".###.", "#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."]),
    ('1', &[".#.", "##.", ".#.", ".#.", ".#.", ".#.", ".#.", "###"]),
    ('2', &[".###.", "#...#", "....#", "...#.", "..#..", ".#...", "#....", "#####"]),
    ('3', &[".###.", "#...#", "....#", "..##.", "....#", "....#", "#...#", ".###."]),
    ('4', &["...#.", "..##.", ".#.#.", "#..#.", "#####", "...#.", "...#.", "...#."]),
    ('5', &["#####", "#....", "#....", "####.", "....#", "....#", "#...#", ".###."]),
    ('6', &["..##.", ".#...", "#....", "####.", "#...#", "#...#", "#...#", ".###."]),
    ('7', &["#####", "....#", "...#.", "...#.", "..#..", "..#..", ".#...", ".#..."]),
    ('8', &[".###.", "#...#", "#...#", ".###.", "#...#", "#...#", "#...#", ".###."]),
    ('9', &[".###.", "#...#", "#...#", "#...#", ".####", "....#", "...#.", ".##.."]),
    (',', &["..", "..", "..", "..", "..", "..", ".#", ".#", ".#", "#."]),
    ('4', &["...#.", "..##.", "..##.", ".#.#.", ".#.#.", "#..#.", "#####", "...#.", "...#."]),
    ('5', &[".####", ".#...", "#....", "####.", "#...#", "....#", "....#", "#...#", ".###."]),
    ('8', &[".###.", "#...#", "#...#", "#...#", ".###.", "#...#", "#...#", "#...#", ".###."]),
];

/// How much of a glyph must agree with a known shape for it to count as that character.
const SHAPE_FITS: f32 = 0.9;
/// Screen pixels per font pixel tried: UI sizes that keep the fonts crisp draw them whole.
const FONT_SCALES: [u32; 3] = [1, 2, 3];

/// The amount when every glyph is a known shape and together they make one.
fn read_shapes(glyphs: &[Mask]) -> Option<u64> {
    if glyphs.is_empty() {
        return None;
    }
    let text: String = glyphs.iter().map(known_shape).collect::<Option<_>>()?;
    (AMOUNT.is_match(&text) && fits_shapes(&text, glyphs)).then(|| parse_grouped(&text)).flatten()
}

/// The character a glyph is drawn as, when it fits one known shape well and no other character
/// nearly as well.
fn known_shape(glyph: &Mask) -> Option<char> {
    let glyph = without_blank_bottom(glyph);
    let mut best: Option<(f32, char)> = None;
    let mut runner_up = 0.0f32;
    for (c, rows) in SHAPES {
        for scale in FONT_SCALES {
            let (w, h) = (rows[0].len() as u32 * scale, rows.len() as u32 * scale);
            if w != glyph.width() || h.abs_diff(glyph.height()) > scale {
                continue;
            }
            let shape = Mask::from_fn(w, h, |x, y| rows[(y / scale) as usize].as_bytes()[(x / scale) as usize] == b'#');
            let score = shape.similarity(&glyph);
            match best {
                Some((top, top_char)) if score > top => {
                    if top_char != *c {
                        runner_up = runner_up.max(top);
                    }
                    best = Some((score, *c));
                }
                Some((_, top_char)) if top_char != *c => runner_up = runner_up.max(score),
                None => best = Some((score, *c)),
                _ => {}
            }
        }
    }
    let (score, c) = best?;
    (score >= SHAPE_FITS && score - runner_up >= 0.1).then_some(c)
}

/// The glyph without the blank rows under it: the amount's rows reach down to the comma's tail.
fn without_blank_bottom(glyph: &Mask) -> Mask {
    let height = (0..glyph.height()).rev().find(|&y| (0..glyph.width()).any(|x| glyph.get(x, y))).map_or(0, |y| y + 1);
    Mask::from_fn(glyph.width(), height, |x, y| glyph.get(x, y))
}

/// OCR of a short amount drawn several times over, since OCR skips a lone digit as not being
/// text. Every copy must read the same.
fn read_repeated(frame: &Frame, rect: Rect, glyphs: &[Mask], ocr: &Ocr, shape: u8) -> Result<Vec<u64>> {
    let is_digit = |px: &Rgba<u8>| is_digit(px, shape);
    use imageops::FilterType::Triangle;
    let columns = ink_columns(frame, rect, is_digit);
    let (Some(rows), Some(first), Some(last)) = (ink_rows(frame, rect, is_digit), columns.first(), columns.last())
    else {
        return Ok(Vec::new());
    };
    let (left, width) = (first.0, last.1 - first.0);
    let pitch = width + COPY_GAP;
    let copies = Frame::from_fn(COPIES as u32 * pitch, rows.1 - rows.0, |x, y| {
        let column = x % pitch;
        let ink = column < width && is_digit(frame.get_pixel(left + column, rows.0 + y));
        if ink { Rgba([0, 0, 0, 255]) } else { Rgba([255, 255, 255, 255]) }
    });
    let whole = Rect::new(0.0, 0.0, copies.width() as f32, copies.height() as f32);
    let mut readings = Vec::new();
    for scale in [1, 2, 3] {
        let words = ocr.words_in(&copies, whole, scale as f32, |crop| {
            let (w, h) = (crop.width() * scale, crop.height() * scale);
            preprocess::pad(&imageops::resize(crop, w, h, Triangle), 40)
        })?;
        let text: String = words.iter().map(|w| w.text.as_str()).collect();
        if let Some(amount) = one_copy(&text).map(|t| settle_sixes(&t, glyphs))
            && AMOUNT.is_match(&amount)
            && fits_shapes(&amount, glyphs)
            && let Some(amount) = parse_grouped(&amount)
        {
            readings.push(amount);
        }
    }
    Ok(readings)
}

/// Whether a digit has ink at its upper right. An 8 does and a 6 doesn't, which is what OCR
/// misses when it confuses them.
fn inked_upper_right(glyph: &Mask) -> bool {
    let height = (0..glyph.height()).rev().find(|&y| (0..glyph.width()).any(|x| glyph.get(x, y))).map_or(0, |y| y + 1);
    let (from, to) = (height * 2 / 16, height * 5 / 16);
    let right = glyph.width().saturating_sub(glyph.width() / 5).min(glyph.width().saturating_sub(1));
    (from..to).any(|y| (right..glyph.width()).any(|x| glyph.get(x, y)))
}

/// Corrects every 6 and 8 in `text` to the one its glyph's shape shows. `text` must have one
/// character per glyph, or it is returned as it is.
fn settle_sixes(text: &str, glyphs: &[Mask]) -> String {
    let chars: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    if chars.len() != glyphs.len() {
        return text.to_string();
    }
    chars
        .into_iter()
        .zip(glyphs)
        .map(|(c, glyph)| match c {
            '6' | '8' if inked_upper_right(glyph) => '8',
            '6' | '8' => '6',
            other => other,
        })
        .collect()
}

/// The text of one copy, if `text` is [`COPIES`] copies of the same thing. OCR takes a row of
/// digits for letters as readily as digits: `ggggg` for 9s, `aaaaa` for 8s.
fn one_copy(text: &str) -> Option<String> {
    let digits = digitize(text);
    let as_digit = |c| match c {
        'g' | 'q' => '9',
        'a' => '8',
        other => other,
    };
    let chars: Vec<char> = digits.chars().filter(|c| !c.is_whitespace()).map(as_digit).collect();
    let size = chars.len() / COPIES;
    let whole = size > 0 && chars.len().is_multiple_of(COPIES);
    (whole && chars.chunks(size).all(|copy| copy == &chars[..size])).then(|| chars[..size].iter().collect())
}

/// The amount's characters (digits and separators), cut out by shape.
fn glyphs(frame: &Frame, rect: Rect, shape: u8) -> Vec<Mask> {
    let is_digit = |px: &Rgba<u8>| is_digit(px, shape);
    let Some(rows) = ink_rows(frame, rect, is_digit) else { return Vec::new() };
    ink_columns(frame, rect, is_digit).into_iter().map(|cols| Mask::new(frame, cols, rows, is_digit)).collect()
}

fn is_digit(px: &Rgba<u8>, shape: u8) -> bool {
    preprocess::luminance(px) <= shape
}

/// Whether a reading agrees with the drawn characters: one per glyph, and since the pixel
/// font draws each character the same way every time, the same character wherever two
/// glyphs match and different ones wherever they do not. This catches a 6 read as 8 when the
/// amount shows both.
fn fits_shapes(text: &str, glyphs: &[Mask]) -> bool {
    let chars: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    if chars.len() != glyphs.len() {
        return false;
    }
    let digits: Vec<(char, &Mask)> = chars.into_iter().zip(glyphs).filter(|(c, _)| c.is_ascii_digit()).collect();
    digits.iter().enumerate().all(|(i, (a, first))| {
        digits[i + 1..].iter().all(|(b, second)| (first.similarity(second) >= MIN_SIMILARITY) == (a == b))
    })
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

    /// A strip with made-up 3×5 glyphs drawn dark on white, one blank column apart.
    fn amount(text: &str) -> (Frame, Rect) {
        let glyph = |c: char| match c {
            '6' => ["#..", "#..", "###", "#.#", "###"],
            '8' => ["###", "#.#", "###", "#.#", "###"],
            '4' => ["#.#", "#.#", "###", "..#", "..#"],
            _ => [".#.", ".#.", ".#.", ".#.", "##."],
        };
        let columns: Vec<[bool; 5]> = text
            .chars()
            .flat_map(|c| {
                let rows = glyph(c);
                (0..4).map(move |x| std::array::from_fn(|y| x < 3 && rows[y].as_bytes()[x] == b'#'))
            })
            .collect();
        let frame = Frame::from_fn(columns.len() as u32 + 4, 9, |x, y| {
            let (col, row) = ((x as usize).wrapping_sub(2), (y as usize).wrapping_sub(2));
            if col < columns.len() && row < 5 && columns[col][row] { Rgba([20, 20, 20, 255]) } else { Rgba([255; 4]) }
        });
        (frame, Rect::new(0.0, 0.0, columns.len() as f32 + 4.0, 9.0))
    }

    #[test]
    fn a_digits_shape_settles_six_or_eight() {
        // Made-up glyphs shaped like the real thing: a 6 is open at the upper right.
        let six = ["....####..", "..##......", "##........", "########..", "##......##", "##......##", "..######.."];
        let eight = ["..######..", "##......##", "##......##", "..######..", "##......##", "##......##", "..######.."];
        let mask = |rows: &[&str]| {
            Mask::try_from(rows.iter().flat_map(|r| [r.to_string(), r.to_string()]).collect::<Vec<_>>()).unwrap()
        };
        let (six, eight) = (mask(&six), mask(&eight));
        assert!(!inked_upper_right(&six));
        assert!(inked_upper_right(&eight));
        assert_eq!(settle_sixes("6", std::slice::from_ref(&eight)), "8");
        assert_eq!(settle_sixes("8", std::slice::from_ref(&six)), "6");
        assert_eq!(settle_sixes("68", &[eight.clone(), six.clone()]), "86");
        // Anything else, or a reading that doesn't line up with the glyphs, is left alone.
        assert_eq!(settle_sixes("3", std::slice::from_ref(&six)), "3");
        assert_eq!(settle_sixes("66", &[six]), "66");
    }

    /// `text` drawn in [`SHAPES`]' first (inventory) font at `scale`, dark on white, a font
    /// pixel apart, and the glyphs cut from it.
    fn drawn(text: &str, scale: u32) -> Vec<Mask> {
        let shape = |c: char| SHAPES.iter().find(|(s, _)| *s == c).map(|(_, rows)| *rows);
        let mut columns: Vec<Vec<bool>> = Vec::new();
        for c in text.chars() {
            // Anything else is a made-up glyph no known shape fits.
            let rows = shape(c).unwrap_or(&["#...#", ".#.#.", "..#..", ".#.#.", "#...#", "#...#", ".#.#.", "#...#"]);
            for x in 0..rows[0].len() {
                let column: Vec<bool> = (0..10).map(|y| rows.get(y).is_some_and(|r| r.as_bytes()[x] == b'#')).collect();
                columns.extend(std::iter::repeat_n(column, scale as usize));
            }
            columns.extend(std::iter::repeat_n(vec![false; 10], scale as usize));
        }
        let (w, h) = (columns.len() as u32 + 4, 10 * scale + 4);
        let frame = Frame::from_fn(w, h, |x, y| {
            let (col, row) = ((x as usize).wrapping_sub(2), (y.wrapping_sub(2) / scale) as usize);
            let ink = col < columns.len() && y >= 2 && row < 10 && columns[col][row];
            if ink { Rgba([0, 0, 0, 255]) } else { Rgba([255; 4]) }
        });
        glyphs(&frame, Rect::new(0.0, 0.0, w as f32, h as f32), Ink::CRISP.shape)
    }

    #[test]
    fn reads_amounts_by_the_games_digit_shapes() {
        assert_eq!(read_shapes(&drawn("28,640", 2)), Some(28_640));
        assert_eq!(read_shapes(&drawn("153,814", 2)), Some(153_814));
        assert_eq!(read_shapes(&drawn("458", 2)), Some(458));
        assert_eq!(read_shapes(&drawn("1,234,569", 1)), Some(1_234_569));
        assert_eq!(read_shapes(&drawn("157,262", 2)), Some(157_262));
        assert_eq!(read_shapes(&drawn("90", 3)), Some(90));
        // A character not in the table leaves the amount to OCR, as does one that isn't an amount.
        assert_eq!(read_shapes(&drawn("1,2x4", 2)), None);
        assert_eq!(read_shapes(&drawn("1234", 2)), None);
        assert_eq!(read_shapes(&[]), None);
    }

    #[test]
    fn repeated_amounts_must_agree_in_every_copy() {
        assert_eq!(one_copy("00000"), Some("0".into()));
        assert_eq!(one_copy("OOOOO"), Some("0".into()));
        assert_eq!(one_copy("O0o0O"), Some("0".into()));
        assert_eq!(one_copy("77777"), Some("7".into()));
        assert_eq!(one_copy("ggggg"), Some("9".into()));
        assert_eq!(one_copy("aaaaaaaaaa"), Some("88".into()));
        assert_eq!(one_copy("25 25 25 25 25"), Some("25".into()));
        assert_eq!(one_copy("0000"), None);
        assert_eq!(one_copy("00800"), None);
        assert_eq!(one_copy("oooor-l"), None);
        assert_eq!(one_copy(""), None);
    }

    #[test]
    fn readings_must_fit_the_digit_shapes() {
        let (frame, rect) = amount("8,646");
        let shapes = glyphs(&frame, rect, Ink::CRISP.shape);
        assert_eq!(shapes.len(), 5);
        assert!(fits_shapes("8,646", &shapes));
        // A 6 read as 8: the two 6s no longer agree with each other, or the 8 with the 6.
        assert!(!fits_shapes("8,846", &shapes));
        assert!(!fits_shapes("6,646", &shapes));
        assert!(!fits_shapes("8,64", &shapes));
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
        assert_eq!(whole_boxes(&frame), vec![true]);
    }

    #[test]
    fn a_box_a_tooltip_cuts_short_is_not_whole() {
        // The box's white ends at x 200, where a see-through blue tooltip begins.
        let frame = Frame::from_fn(400, 600, |x, y| {
            if (44..56).contains(&x) && (100..112).contains(&y) {
                Rgba([255, 200, 0, 255])
            } else if (60..200).contains(&x) && (100..112).contains(&y) {
                Rgba([255, 255, 255, 255])
            } else if (200..380).contains(&x) && (60..300).contains(&y) {
                Rgba([83, 85, 132, 255])
            } else {
                Rgba([200, 205, 215, 255])
            }
        });
        assert_eq!(whole_boxes(&frame), vec![false]);
    }
}
