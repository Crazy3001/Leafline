//! The quick-slot bar: how many of each item sits on each of its eight slots, so the session
//! can count the potions drunk without the player typing them.
//!
//! The bar is found by its frame, a pure white line along its top, 141 font pixels long, drawn in
//! whole window pixels like the counts (two to a font pixel at 1440p). The slots stand in two
//! rows of four at fixed steps from that line. Only the counts are read; the pictures and the key
//! names are not looked at.
//!
//! Seen on one setup only: 2560 x 1440 with UI Size Default.

use serde::Serialize;

use super::counts;
use crate::vision::{Frame, Rect};

/// Slots on the bar: two rows of four, numbered left to right, top row first.
pub const SLOTS: usize = 8;
const COLUMNS: u32 = 4;

/// The white line along the top of the bar, in font pixels.
const TOP_LINE: u32 = 141;
/// The frame's left edge stands this far left of where the top line starts (its corner is cut).
const CORNER: u32 = 3;
/// The frame's left edge runs down at least this far below the top line.
const LEFT_EDGE: (u32, u32) = (6, 60);
/// Where the first slot's count starts, from the top line's start, and the step between slots.
const FIRST_COUNT: (u32, u32) = (3, 26);
const STEP: (u32, u32) = (35, 34);
/// The largest cell size looked for: 4 window pixels to a font pixel is past 4K.
const MAX_CELL: u32 = 4;

/// What the bar shows.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickSlots {
    /// The count on each slot, `None` where a slot shows none (a skill, or nothing set).
    pub counts: [Option<u32>; SLOTS],
    /// The bar in the window.
    pub panel: Rect,
}

/// Reads the bar, or `None` when it can't be found.
pub fn read(frame: &Frame) -> Option<QuickSlots> {
    let (start, top, cell) = find(frame)?;
    let mut counts = [None; SLOTS];
    for (slot, count) in counts.iter_mut().enumerate() {
        let (column, row) = (slot as u32 % COLUMNS, slot as u32 / COLUMNS);
        let x = start + (FIRST_COUNT.0 + column * STEP.0) * cell;
        let y = top + (FIRST_COUNT.1 + row * STEP.1) * cell;
        // From a little before where the count starts to the next slot, and its rows.
        let area = (x.saturating_sub(2 * cell), y.saturating_sub(2 * cell), x + (STEP.0 - 3) * cell, y + (counts::DIGIT_ROWS + 2) * cell);
        *count = counts::read(frame, area, Some(cell)).into_iter().min_by_key(|c| c.left).map(|c| c.value);
    }
    let left = start.saturating_sub(CORNER * cell) as f32;
    let panel = Rect::new(left, top as f32, ((TOP_LINE + 2 * CORNER) * cell) as f32, ((FIRST_COUNT.1 + 2 * STEP.1) * cell) as f32);
    Some(QuickSlots { counts, panel })
}

fn is_white(frame: &Frame, x: u32, y: u32) -> bool {
    frame.get_pixel(x, y).0[..3] == [255, 255, 255]
}

/// Where the bar's top line starts, its row, and the window pixels to a font pixel. Looks in the
/// lower right of the window, where the game keeps the bar.
fn find(frame: &Frame) -> Option<(u32, u32, u32)> {
    let (width, height) = (frame.width(), frame.height());
    for y in height * 11 / 20..height {
        let mut x = width * 2 / 5;
        while x < width {
            if !is_white(frame, x, y) {
                x += 1;
                continue;
            }
            let start = x;
            while x < width && is_white(frame, x, y) {
                x += 1;
            }
            let length = x - start;
            let Some(cell) = (1..=MAX_CELL).find(|c| length == TOP_LINE * c) else { continue };
            if left_edge(frame, start, y, cell) {
                return Some((start, y, cell));
            }
        }
    }
    None
}

/// Whether the frame's left edge runs down from the corner, white for nearly all of its length.
fn left_edge(frame: &Frame, start: u32, top: u32, cell: u32) -> bool {
    let Some(x) = start.checked_sub(CORNER * cell) else { return false };
    let (from, to) = (top + LEFT_EDGE.0 * cell, top + LEFT_EDGE.1 * cell);
    if to >= frame.height() {
        return false;
    }
    let white = (from..to).filter(|&y| is_white(frame, x, y)).count() as u32;
    white * 10 >= (to - from) * 9
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readers::counts::tests::draw;
    use image::Rgba;

    /// A made-up bar on a plain ground: the white top line and left edge, slots of grey, and the
    /// counts given, `cell` window pixels to a font pixel.
    fn bar(width: u32, height: u32, at: (u32, u32), cell: u32, counts: &[(usize, &str)]) -> Frame {
        let mut frame = Frame::from_pixel(width, height, Rgba([150, 120, 80, 255]));
        let (start, top) = at;
        let left = start - CORNER * cell;
        for y in top + cell..top + 78 * cell {
            for x in left + cell..left + 147 * cell {
                frame.put_pixel(x, y, Rgba([82, 88, 96, 255]));
            }
        }
        for x in start..start + TOP_LINE * cell {
            for y in top..top + cell {
                frame.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
        for y in top + 3 * cell..top + 78 * cell {
            for x in left..left + cell {
                frame.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
        for &(slot, text) in counts {
            let (column, row) = (slot as u32 % COLUMNS, slot as u32 / COLUMNS);
            let x = start + (FIRST_COUNT.0 + column * STEP.0) * cell;
            let y = top + (FIRST_COUNT.1 + row * STEP.1) * cell;
            draw(&mut frame, text, x, y, cell);
        }
        frame
    }

    #[test]
    fn reads_each_slots_count_and_leaves_the_rest_empty() {
        let frame = bar(1280, 720, (900, 560), 1, &[(1, "3"), (2, "32"), (5, "161"), (6, "156")]);
        let slots = read(&frame).expect("bar found");
        assert_eq!(slots.counts, [None, Some(3), Some(32), None, None, Some(161), Some(156), None]);
    }

    #[test]
    fn the_bar_is_found_at_twice_the_size_too() {
        let frame = bar(2560, 1440, (1790, 1152), 2, &[(5, "160"), (6, "155")]);
        let slots = read(&frame).expect("bar found");
        assert_eq!(slots.counts[5..7], [Some(160), Some(155)]);
        assert_eq!(slots.panel.x, 1784.0);
    }

    #[test]
    fn no_bar_no_reading() {
        assert_eq!(read(&Frame::from_pixel(800, 600, Rgba([255, 255, 255, 255]))), None);
    }
}
