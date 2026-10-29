//! The current map's name, from the minimap's header.
//!
//! The minimap is found by its orange WORLD button, which looks the same wherever the window
//! is dragged and however wide the map's name makes it. Everything else is placed relative to
//! the button, in units of its height. The minimap has two states:
//!
//! - **Expanded**: a light blue band under the `MINI MAP` title bar holds the map's emblem and
//!   two lines of white, outlined text, the street (`Victoria Road`) above the map (`Henesys`).
//! - **Collapsed**: a grey title bar reads `Victoria Road : Henesys` in black.

use image::{Rgba, imageops};
use serde::Serialize;

use crate::error::Result;
use crate::vision::{Frame, Line, Ocr, Rect, Word, color, group_lines, preprocess};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapReading {
    /// The street or area above the map's name, such as `Victoria Road`, when it was read.
    pub street: Option<String>,
    pub name: String,
    /// Read from the collapsed minimap's one-line title rather than the expanded header.
    pub collapsed: bool,
    /// The text area that was read.
    pub rect: Rect,
}

impl MapReading {
    /// `Victoria Road : Henesys`, as the collapsed minimap writes it.
    pub fn label(&self) -> String {
        match &self.street {
            Some(street) => format!("{street} : {}", self.name),
            None => self.name.clone(),
        }
    }
}

/// The WORLD button's size in its own heights, measured on Classic's minimap (34 x 10 canvas
/// pixels), with some slack for other UI sizes.
const BUTTON_ASPECT: std::ops::RangeInclusive<f32> = 2.9..=4.0;
/// Below this share of orange pixels a block is not the button (its white lettering takes up
/// about a third).
const BUTTON_FILL: f32 = 0.5;
/// How far left of the button a header can reach, in button heights. Wide enough for any
/// name that fits on screen at 2x UI.
const MAX_WIDTH: f32 = 60.0;

pub fn read(frame: &Frame, ocr: &Ocr) -> Result<Option<MapReading>> {
    for button in world_buttons(frame) {
        if let Some(band) = expanded_band(frame, button) {
            if let Some(reading) = read_expanded(frame, ocr, band)? {
                return Ok(Some(reading));
            }
        } else if let Some(bar) = collapsed_bar(frame, button)
            && let Some(reading) = read_collapsed(frame, ocr, bar)?
        {
            return Ok(Some(reading));
        }
    }
    Ok(None)
}

/// Orange blocks shaped like the WORLD button, largest first.
fn world_buttons(frame: &Frame) -> Vec<Rect> {
    let whole = Rect::new(0.0, 0.0, frame.width() as f32, frame.height() as f32);
    let min_height = frame.height() as f32 * 0.006;
    let mut buttons: Vec<Rect> = color::find_blocks(frame, whole, 2, is_button_orange)
        .into_iter()
        .filter(|b| b.h >= min_height && BUTTON_ASPECT.contains(&(b.w / b.h)))
        .filter(|b| share(frame, *b, is_button_orange) >= BUTTON_FILL)
        .collect();
    buttons.sort_by(|a, b| (b.w * b.h).total_cmp(&(a.w * a.h)));
    buttons
}

/// The expanded minimap's blue name band: its rows are found below the button, its left edge
/// where the blue gives way to the emblem (or the frame, for maps without one).
fn expanded_band(frame: &Frame, button: Rect) -> Option<Rect> {
    let h = button.h;
    let x = (button.x + 0.9 * button.w) as u32;
    let (top, bottom) = vertical_run(frame, x, button.bottom(), 1.0 * h, 8.0 * h, is_panel_blue)?;
    if bottom - top < 2.5 * h {
        return None;
    }
    let rows = Rect::from_edges(0.0, top, 0.0, bottom);
    let left = run_left(frame, rows, button.x, button.x - MAX_WIDTH * h, 0.0, 0.25, is_panel_blue)?;
    Some(Rect::from_edges(left, top, button.right(), bottom))
}

/// The collapsed minimap's grey title bar, up to the minimise and maximise buttons left of WORLD.
fn collapsed_bar(frame: &Frame, button: Rect) -> Option<Rect> {
    let h = button.h;
    let probe = Rect::from_edges(button.x - 6.0 * h, button.y, button.x - 3.0 * h, button.bottom());
    if share(frame, probe, is_bar_grey) < 0.35 {
        return None;
    }
    let (top, bottom) = (button.y - 0.4 * h, button.bottom() + 0.4 * h);
    let right = button.x - 2.9 * h;
    let rows = Rect::from_edges(0.0, top, 0.0, bottom);
    // Words leave short gaps of bare grey columns between them.
    let left = run_left(frame, rows, right, button.x - MAX_WIDTH * h, h, 0.2, is_bar_grey)?;
    Some(Rect::from_edges(left, top, right, bottom))
}

fn read_expanded(frame: &Frame, ocr: &Ocr, band: Rect) -> Result<Option<MapReading>> {
    // The letters are white blended into the blue at their edges; the band itself is about 180
    // and the dark outline about 110, so 200 keeps whole strokes and nothing else.
    let lines = read_text(frame, ocr, band, |p| preprocess::luminance(p) >= 200)?;
    let texts: Vec<String> = lines.iter().map(|l| clean(&l.text)).filter(|t| t.len() >= 2).collect();
    let reading = match texts.as_slice() {
        [] => return Ok(None),
        [name] => MapReading { street: None, name: name.clone(), collapsed: false, rect: band },
        [street, rest @ ..] => {
            MapReading { street: Some(street.clone()), name: rest.join(" "), collapsed: false, rect: band }
        }
    };
    Ok(Some(reading))
}

fn read_collapsed(frame: &Frame, ocr: &Ocr, bar: Rect) -> Result<Option<MapReading>> {
    let lines = read_text(frame, ocr, bar, |p| preprocess::luminance(p) <= 90)?;
    let text = lines.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join(" ");
    Ok(split_title(&text).map(|(street, name)| MapReading { street, name, collapsed: true, rect: bar }))
}

/// OCR of the text `is_text` keeps inside `rect`, read at several sizes and merged word by
/// word. Each size misreads or drops different words (`City` as `Citi/`, a trailing `1>`), so
/// every word takes the reading most passes agree on, and a word only one pass saw is kept.
fn read_text(frame: &Frame, ocr: &Ocr, rect: Rect, is_text: impl Fn(&Rgba<u8>) -> bool) -> Result<Vec<Line>> {
    use imageops::FilterType::{CatmullRom, Nearest, Triangle};
    let mut passes = Vec::new();
    for (scale, filter) in [(3, CatmullRom), (3, Triangle), (2, Triangle), (1, Nearest)] {
        passes.push(ocr.words_in(frame, rect, scale as f32, |crop| {
            let keyed = preprocess::keep(crop, &is_text);
            preprocess::pad(&imageops::resize(&keyed, keyed.width() * scale, keyed.height() * scale, filter), 24)
        })?);
    }
    Ok(group_lines(&merge_passes(&passes)))
}

/// Matches words across OCR passes by where they sit, and gives each place the text most
/// passes read there, the earliest pass breaking ties.
fn merge_passes(passes: &[Vec<Word>]) -> Vec<Word> {
    struct Slot {
        rect: Rect,
        readings: Vec<String>,
    }
    let same_place = |a: &Rect, b: &Rect| {
        let across = (a.right().min(b.right()) - a.x.max(b.x)).max(0.0);
        a.vertical_overlap(b) >= 0.5 * a.h.min(b.h) && across >= 0.5 * a.w.min(b.w)
    };
    let mut slots: Vec<Slot> = Vec::new();
    // Lone marks are noise, except the colon that splits a collapsed title.
    let meaningful = |w: &&Word| w.text.contains(':') || w.text.chars().any(char::is_alphanumeric);
    for word in passes.iter().flatten().filter(meaningful) {
        match slots.iter_mut().find(|s| same_place(&s.rect, &word.rect)) {
            Some(slot) => slot.readings.push(word.text.clone()),
            None => slots.push(Slot { rect: word.rect, readings: vec![word.text.clone()] }),
        }
    }
    slots
        .into_iter()
        .map(|slot| {
            let votes = |text: &String| slot.readings.iter().filter(|r| *r == text).count();
            let mut best = &slot.readings[0];
            for reading in &slot.readings {
                if votes(reading) > votes(best) {
                    best = reading;
                }
            }
            Word { text: best.clone(), rect: slot.rect }
        })
        .collect()
}

/// Splits a collapsed title such as `Victoria Road : Henesys` at its colon.
fn split_title(text: &str) -> Option<(Option<String>, String)> {
    let (street, name) = match text.split_once(':') {
        Some((street, name)) => (Some(clean(street)).filter(|s| s.len() >= 2), clean(name)),
        None => (None, clean(text)),
    };
    (name.len() >= 2).then_some((street, name))
}

/// Tidies OCR spacing and stray marks at either end, keeping the `<>` some names end with.
fn clean(text: &str) -> String {
    let trimmed = text.trim_matches(|c: char| !c.is_alphanumeric() && !"<>()'".contains(c));
    // OCR tends to pad brackets: `< Area 1 >`.
    trimmed.split_whitespace().collect::<Vec<_>>().join(" ").replace("< ", "<").replace(" >", ">")
}

/// The run of `matches` pixels down column `x`, starting within `search` of `from` and lasting
/// at most `max`. Returns its top and bottom.
fn vertical_run(
    frame: &Frame,
    x: u32,
    from: f32,
    search: f32,
    max: f32,
    matches: impl Fn(&Rgba<u8>) -> bool,
) -> Option<(f32, f32)> {
    if x >= frame.width() {
        return None;
    }
    let height = frame.height();
    let from = from.max(0.0) as u32;
    let top = (from..(from + search as u32).min(height)).find(|&y| matches(frame.get_pixel(x, y)))?;
    let limit = (top + max as u32).min(height);
    let bottom = (top..limit).find(|&y| !matches(frame.get_pixel(x, y))).unwrap_or(limit);
    Some((top as f32, bottom as f32))
}

/// Walks left from column `start` to find where a run of columns, each with at least
/// `min_share` of `matches` pixels across `rows`, ends. Gaps up to `max_gap` wide are bridged.
fn run_left(
    frame: &Frame,
    rows: Rect,
    start: f32,
    limit: f32,
    max_gap: f32,
    min_share: f32,
    matches: impl Fn(&Rgba<u8>) -> bool,
) -> Option<f32> {
    let (top, bottom) = (rows.y.max(0.0) as u32, (rows.bottom() as u32).min(frame.height()));
    if bottom <= top {
        return None;
    }
    let column = |x: u32| {
        let hits = (top..bottom).filter(|&y| matches(frame.get_pixel(x, y))).count();
        hits as f32 >= min_share * (bottom - top) as f32
    };
    let start = (start.max(0.0) as u32).min(frame.width().checked_sub(1)?);
    let limit = limit.max(0.0) as u32;
    let mut left = None;
    let mut x = start;
    while x >= limit {
        if column(x) {
            left = Some(x);
        } else if left.is_none_or(|l| (l - x) as f32 > max_gap) {
            break;
        }
        if x == 0 {
            break;
        }
        x -= 1;
    }
    left.map(|l| l as f32)
}

/// Share of `rect`'s pixels that `matches` accepts.
fn share(frame: &Frame, rect: Rect, matches: impl Fn(&Rgba<u8>) -> bool) -> f32 {
    let (x, y, w, h) = rect.to_pixels(frame.width(), frame.height());
    if w == 0 || h == 0 {
        return 0.0;
    }
    let hits = (y..y + h)
        .flat_map(|py| (x..x + w).map(move |px| (px, py)))
        .filter(|&(px, py)| matches(frame.get_pixel(px, py)))
        .count();
    hits as f32 / (w * h) as f32
}

fn near(px: &Rgba<u8>, target: [u8; 3], tolerance: u8) -> bool {
    px.0.iter().zip(target).all(|(&c, t)| c.abs_diff(t) <= tolerance)
}

/// The WORLD button's orange, (221, 153, 17) on a clean capture.
fn is_button_orange(px: &Rgba<u8>) -> bool {
    near(px, [221, 153, 17], 16)
}

/// The expanded name band, (153, 187, 204).
fn is_panel_blue(px: &Rgba<u8>) -> bool {
    near(px, [153, 187, 204], 12)
}

/// The collapsed title bar, (221, 221, 221).
fn is_bar_grey(px: &Rgba<u8>) -> bool {
    near(px, [221, 221, 221], 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collapsed_titles() {
        assert_eq!(
            split_title("Victoria Road : Kerning City Self-Defence Item Store"),
            Some((Some("Victoria Road".into()), "Kerning City Self-Defence Item Store".into()))
        );
        assert_eq!(split_title("Henesys"), Some((None, "Henesys".into())));
        assert_eq!(split_title(" : "), None);
    }

    #[test]
    fn cleans_spacing_and_brackets() {
        assert_eq!(clean("Line 1  < Area 1 >"), "Line 1 <Area 1>");
        assert_eq!(clean("• Henesys ."), "Henesys");
    }

    #[test]
    fn passes_vote_per_word_and_keep_words_only_one_saw() {
        let word = |text: &str, x: f32| Word { text: text.into(), rect: Rect::new(x, 0.0, 40.0, 16.0) };
        let passes = vec![
            vec![word("Kerning", 0.0), word("City", 50.0)],
            vec![word("Kerning", 1.0), word("Citi/", 51.0)],
            vec![word("Kerninq", 0.0), word("City", 49.0), word("1>", 100.0)],
        ];
        let merged: Vec<String> = merge_passes(&passes).into_iter().map(|w| w.text).collect();
        assert_eq!(merged, ["Kerning", "City", "1>"]);
    }

    /// A drawn minimap: the WORLD button over a blue band, with no text to read.
    fn minimap(collapsed: bool) -> Frame {
        Frame::from_fn(400, 200, |x, y| {
            let button = (300..368).contains(&x) && (20..40).contains(&y);
            if button {
                Rgba([221, 153, 17, 255])
            } else if collapsed && (0..380).contains(&x) && (12..48).contains(&y) {
                Rgba([221, 221, 221, 255])
            } else if !collapsed && (6..376).contains(&x) && (46..130).contains(&y) {
                Rgba([153, 187, 204, 255])
            } else {
                Rgba([30, 40, 50, 255])
            }
        })
    }

    #[test]
    fn finds_the_expanded_band() {
        let frame = minimap(false);
        let buttons = world_buttons(&frame);
        assert_eq!(buttons, vec![Rect::new(300.0, 20.0, 68.0, 20.0)]);
        assert_eq!(expanded_band(&frame, buttons[0]), Some(Rect::from_edges(6.0, 46.0, 368.0, 130.0)));
        assert_eq!(collapsed_bar(&frame, buttons[0]), None);
    }

    #[test]
    fn finds_the_collapsed_bar() {
        let frame = minimap(true);
        let button = world_buttons(&frame)[0];
        assert_eq!(expanded_band(&frame, button), None);
        assert_eq!(collapsed_bar(&frame, button), Some(Rect::from_edges(0.0, 12.0, 242.0, 48.0)));
    }
}
