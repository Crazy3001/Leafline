//! Where each item tooltip's name is, found by the picture alone, for a reader that reads only
//! the name (the website's). Every tooltip, equipment or not, starts with a solid blue bullet and
//! its name in bold, white or (upgraded) amber, with the icon box below; see [`plain`]. A long
//! name wraps onto a second line in the same colour. The tooltip of a worn item (hovered in the
//! Equipment window) has no bullet, its name centred above the `REQ` block: those aren't found
//! here (the separator rule alone also finds the minimap and the Equip window's lines).

use image::Rgba;
use serde::Serialize;

use super::plain;
use crate::vision::{Frame, Rect, preprocess};

/// Game pixels, as in the rest of the tooltip.
/// How far right of the bullet the name's first letter may start.
const FIRST_LETTER: f32 = 8.0;
/// A gap this wide ends the name: the spaces between its words are narrower.
const END_GAP: f32 = 8.0;
/// How far the name's letters reach above and below the bullet's middle (descenders included).
const ABOVE: f32 = 7.0;
const BELOW: f32 = 8.0;
/// How wide the name can run.
const MAX_WIDTH: f32 = 300.0;
/// The distance between a wrapped name's lines, and how much it may be off.
const WRAP_PITCH: f32 = 14.0;
const WRAP_SLACK: f32 = 3.0;
/// The smallest UI's scale.
const MIN_UNIT: f32 = 0.95;
/// How tall a line of the name is, from the tallest capital to the lowest descender, in game pixels.
const NAME_HEIGHTS: std::ops::RangeInclusive<f32> = 6.0..=14.0;
/// A wrapped name's second line is at least a word.
const WRAP_WIDTH: f32 = 14.0;
/// The fewest ink pixels, in square game pixels, for a line to count.
const MIN_INK: f32 = 12.0;

/// One tooltip's name as drawn.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NameLines {
    /// The bullet before the name.
    pub bullet: Rect,
    /// Screen pixels per game pixel.
    pub unit: f32,
    /// The name's lines, tight around their ink: one, or two when it wraps.
    pub lines: Vec<Rect>,
    /// Drawn in amber: the item has passed at least one scroll.
    pub amber: bool,
}

/// The names of the tooltips in `frame`, top to bottom.
pub fn name_lines(frame: &Frame) -> Vec<NameLines> {
    let mut found: Vec<NameLines> = plain::bullets(frame)
        .into_iter()
        .filter_map(|(bullet, _)| {
            let unit = bullet.w.max(bullet.h) / 5.0;
            // Below the smallest UI: a speck of the same blue somewhere else.
            if unit < MIN_UNIT {
                return None;
            }
            let middle = bullet.center_y();
            let start = bullet.right();
            let (first, amber) = line_at(frame, start, start + FIRST_LETTER * unit, middle, unit, None)?;
            if !name_height(first.h, unit) {
                return None;
            }
            let mut lines = vec![first];
            // A wrapped name goes on in the same colour and size, starting where the first line did.
            let next = middle + WRAP_PITCH * unit;
            let slack = WRAP_SLACK * unit;
            if let Some((second, _)) = line_at(frame, first.x - slack, first.x + slack, next, unit, Some(amber))
                && (second.center_y() - first.center_y() - WRAP_PITCH * unit).abs() <= slack
                && name_height(second.h, unit)
                && second.w >= WRAP_WIDTH * unit
            {
                lines.push(second);
            }
            Some(NameLines { bullet, unit, lines, amber })
        })
        .collect();
    found.sort_by(|a, b| a.bullet.y.total_cmp(&b.bullet.y));
    found
}

/// Game pixels, for the lines under an equipment tooltip's separator (`Type:`, the stats,
/// `Remaining Enhancements:`): each starts with a small orange plus, three game pixels across, in
/// line with the name's bullet.
/// How far below the name's bullet the first line can be, and the last.
const STATS_FROM: f32 = 40.0;
const STATS_TO: f32 = 340.0;
/// How far the plus can sit left or right of the name's bullet.
const STAT_BULLET_LEFT: f32 = 5.0;
const STAT_BULLET_RIGHT: f32 = 8.0;
/// The distance between the lines: 16 at 2x, but 32 screen pixels at 1.6x too (20 game pixels
/// by the bullet's size).
const STAT_PITCH: std::ops::RangeInclusive<f32> = 12.0..=23.0;
/// How far a line's letters reach above and below the plus's middle.
const STAT_ABOVE: f32 = 7.0;
const STAT_BELOW: f32 = 7.0;
/// How far right of the name's bullet the panel's text can reach.
const PANEL_TEXT: f32 = 232.0;
/// A gap this wide ends a line (the spaces in `+56 (52 +4)` are narrower).
const STAT_END_GAP: f32 = 10.0;

/// The lines under the separator of the equipment tooltip whose name is `name`, top to bottom,
/// each tight around its text (the plus left out). Empty when it shows none (a plain item).
pub fn stat_lines(frame: &Frame, name: &NameLines) -> Vec<Rect> {
    let unit = name.unit;
    let b = name.bullet;
    let (width, height) = (frame.width(), frame.height());
    let left = (b.x - STAT_BULLET_LEFT * unit).max(0.0) as u32;
    let right = ((b.x + STAT_BULLET_RIGHT * unit) as u32).min(width);
    let top = ((b.y + STATS_FROM * unit) as u32).min(height);
    let bottom = ((b.y + STATS_TO * unit) as u32).min(height);
    if left >= right || top >= bottom {
        return Vec::new();
    }
    // Runs of rows with the plus's orange, each one plus if it is the size of one.
    let count = |y: u32| (left..right).filter(|&x| is_stat_bullet(frame.get_pixel(x, y))).count();
    let mut runs: Vec<(u32, u32, usize)> = Vec::new();
    for y in top..bottom {
        let n = count(y);
        if n == 0 {
            continue;
        }
        match runs.last_mut() {
            Some(run) if run.1 == y => {
                run.1 = y + 1;
                run.2 += n;
            }
            _ => runs.push((y, y + 1, n)),
        }
    }
    let side = |run: &(u32, u32, usize)| (run.1 - run.0) as f32 / unit;
    let area = |run: &(u32, u32, usize)| run.2 as f32 / (unit * unit);
    let pluses: Vec<f32> = runs
        .iter()
        .filter(|run| (2.0..=4.0).contains(&side(run)) && (3.0..=7.5).contains(&area(run)))
        .map(|run| (run.0 + run.1) as f32 / 2.0)
        .collect();
    // The lines follow each other evenly from the first; anything after a gap is something else.
    let mut middles: Vec<f32> = Vec::new();
    for middle in pluses {
        match middles.last() {
            None => middles.push(middle),
            Some(&last) if STAT_PITCH.contains(&((middle - last) / unit)) => middles.push(middle),
            Some(_) => break,
        }
    }
    let text_right = ((b.x + PANEL_TEXT * unit) as u32).min(width);
    middles.into_iter().filter_map(|middle| stat_line(frame, (left, right), text_right, middle, unit)).collect()
}

/// The text of the line whose plus is at `middle` (somewhere in the columns `zone`), from just
/// right of the plus to the first wide gap, no further right than `right`.
fn stat_line(frame: &Frame, zone: (u32, u32), right: u32, middle: f32, unit: f32) -> Option<Rect> {
    let top = (middle - STAT_ABOVE * unit).max(0.0) as u32;
    let bottom = ((middle + STAT_BELOW * unit) as u32).min(frame.height());
    let plus_row = (middle as u32).saturating_sub(unit as u32)..(middle + unit + 1.0) as u32;
    let plus_end = (zone.0..zone.1).filter(|&x| plus_row.clone().any(|y| is_stat_bullet(frame.get_pixel(x, y)))).max()? + 1;
    let column = |x: u32| (top..bottom).any(|y| is_stat_text(frame.get_pixel(x, y)));
    // The text keeps its own scale at 1.6x (as the pitch does), so this is generous.
    let first = (plus_end..right.min(plus_end + (10.0 * unit) as u32)).find(|&x| column(x))?;
    let gap = (STAT_END_GAP * unit).ceil() as u32;
    let (mut last, mut x) = (first, first);
    while x < right && x - last <= gap {
        if column(x) {
            last = x;
        }
        x += 1;
    }
    let inked_row = |y: u32| (first..=last).any(|x| is_stat_text(frame.get_pixel(x, y)));
    let row_top = (top..bottom).find(|&y| inked_row(y))?;
    let row_bottom = (top..bottom).rev().find(|&y| inked_row(y))? + 1;
    Some(Rect::from_edges(first as f32, row_top as f32, (last + 1) as f32, row_bottom as f32))
}

/// The orange plus before each line, solid (255, 102, 0).
fn is_stat_bullet(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0.map(i16::from);
    r >= 215 && (60..=140).contains(&g) && b <= 60
}

/// The lines' text: a cream label, a white value, a bonus in lavender, and a loss (`-1`) in
/// magenta (255, 0, 102).
pub fn is_stat_text(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0.map(i16::from);
    let light = preprocess::luminance(px) >= 150 && preprocess::saturation(px) <= 70;
    let lavender = b >= 215 && b >= r + 40 && r >= 130 && g >= 130;
    let magenta = r >= 200 && g <= 70 && (60..=170).contains(&b);
    light || lavender || magenta
}

fn name_height(height: f32, unit: f32) -> bool {
    NAME_HEIGHTS.contains(&(height / unit))
}

/// The line of name text around `middle`, its first ink between `from` and `first_by`, and
/// whether it is amber. With `colour`, only ink of that colour (amber or not) counts.
fn line_at(frame: &Frame, from: f32, first_by: f32, middle: f32, unit: f32, colour: Option<bool>) -> Option<(Rect, bool)> {
    let (width, height) = (frame.width() as f32, frame.height() as f32);
    let top = (middle - ABOVE * unit).max(0.0) as u32;
    let bottom = ((middle + BELOW * unit).min(height)) as u32;
    let left = from.max(0.0) as u32;
    let right = ((from + MAX_WIDTH * unit).min(width)) as u32;
    if top >= bottom || left >= right {
        return None;
    }
    let ink = |px: &Rgba<u8>| match colour {
        Some(true) => is_amber(px),
        Some(false) => is_white(px),
        None => is_white(px) || is_amber(px),
    };
    let column = |x: u32| (top..bottom).filter(|&y| ink(frame.get_pixel(x, y))).count();
    let first = (left..right.min(first_by.max(0.0) as u32 + 1)).find(|&x| column(x) > 0)?;
    let gap = (END_GAP * unit).ceil() as u32;
    let mut last = first;
    let mut x = first;
    while x < right && x - last <= gap {
        if column(x) > 0 {
            last = x;
        }
        x += 1;
    }
    let inked_row = |y: u32| (first..=last).any(|x| ink(frame.get_pixel(x, y)));
    let row_top = (top..bottom).find(|&y| inked_row(y))?;
    let row_bottom = (top..bottom).rev().find(|&y| inked_row(y))? + 1;
    let (mut white, mut amber) = (0usize, 0usize);
    for y in row_top..row_bottom {
        for x in first..=last {
            let px = frame.get_pixel(x, y);
            white += usize::from(is_white(px));
            amber += usize::from(is_amber(px));
        }
    }
    if ((white + amber) as f32) < MIN_INK * unit * unit {
        return None;
    }
    let rect = Rect::from_edges(first as f32, row_top as f32, (last + 1) as f32, row_bottom as f32);
    Some((rect, amber > white))
}

/// A name's white, a little soft where video blurs it.
pub fn is_white(px: &Rgba<u8>) -> bool {
    preprocess::luminance(px) >= 190 && preprocess::saturation(px) <= 60
}

/// An upgraded name's amber, solid (255, 170, 0), as loose as video needs.
pub fn is_amber(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0.map(i16::from);
    r >= 200 && (120..=210).contains(&g) && b <= 90 && r >= g + 40
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tooltip at 2x: the bullet, a bold white name of `words` blocks after it, and the icon box.
    fn tooltip(words: &[u32], amber: bool) -> Frame {
        const ROWS: [(u32, u32); 10] = [(4, 6), (4, 6), (2, 8), (2, 8), (0, 10), (0, 10), (2, 8), (2, 8), (4, 6), (4, 6)];
        let text = if amber { Rgba([255, 170, 0, 255]) } else { Rgba([255, 255, 255, 255]) };
        // Words start 14 pixels right of the bullet, 8 pixels apart, letters 16 tall.
        let mut spans = Vec::new();
        let mut x = 124;
        for &w in words {
            spans.push((x, x + w));
            x += w + 8;
        }
        Frame::from_fn(600, 400, |x, y| {
            let bullet = (100..110).contains(&y) && {
                let (from, to) = ROWS[(y - 100) as usize];
                (100 + from..100 + to).contains(&x)
            };
            if bullet {
                Rgba([85, 204, 255, 255])
            } else if (97..113).contains(&y) && spans.iter().any(|&(a, b)| (a..b).contains(&x)) && x % 6 != 0 {
                text
            } else if (100..236).contains(&x) && (135..271).contains(&y) {
                Rgba([204, 204, 219, 255])
            } else {
                Rgba([74, 76, 120, 255])
            }
        })
    }

    #[test]
    fn the_name_runs_from_the_bullet_to_its_last_word() {
        let found = name_lines(&tooltip(&[60, 80], false));
        assert_eq!(found.len(), 1);
        let line = found[0].lines[0];
        assert_eq!((line.x, line.right()), (124.0, 272.0));
        assert_eq!((line.y, line.bottom()), (97.0, 113.0));
        assert!(!found[0].amber);
        assert_eq!(found[0].lines.len(), 1);
    }

    #[test]
    fn an_upgraded_name_is_amber() {
        assert!(name_lines(&tooltip(&[60], true))[0].amber);
    }
}
