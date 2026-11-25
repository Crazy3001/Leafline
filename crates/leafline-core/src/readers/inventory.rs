//! The Item Inventory window, read once when the player asks: which tab is open, and for each of
//! its slots the picture shown and the count on it.
//!
//! The window is drawn in whole window pixels (two to a font pixel at 1440p), framed by a pure
//! white line with cut corners like the quick-slot bar's, and holds a white meso box at a fixed
//! step below the line. Tabs and slots stand at fixed steps from the line too. The pictures are
//! kept as the game drew them, on its own pixel grid, for [`crate::mapdata::bag`] to name.
//!
//! Seen on one setup only: 2560 x 1440 with UI Size Default.

use serde::Serialize;

use super::counts;
use crate::vision::{Frame, Rect};

/// The tabs, left to right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Tab {
    Equip,
    Use,
    Setup,
    Etc,
    Cash,
    Deco,
}

const TABS: [Tab; 6] = [Tab::Equip, Tab::Use, Tab::Setup, Tab::Etc, Tab::Cash, Tab::Deco];

/// All in font pixels, from where the window's white top line starts.
const TOP_LINE: u32 = 201;
const CORNER: u32 = 3;
/// The left edge runs down at least this far below the top line.
const LEFT_EDGE: (u32, u32) = (6, 250);
/// The meso box's white top: where it starts and how long it is.
const MESO_BOX: (u32, u32, u32) = (21, 267, 143);
/// The tabs: the row looked at, where the first starts, the step between them and how wide each is.
const TAB_ROW: u32 = 27;
const TAB_LEFT: u32 = 1;
const TAB_STEP: u32 = 33;
const TAB_WIDTH: u32 = 33;
/// The slots: the first one's inside, the step between them, and the inside's size.
const FIRST_SLOT: (u32, u32) = (5, 50);
const SLOT_STEP: (u32, u32) = (36, 34);
pub const SLOT_SIZE: u32 = 31;
pub const COLUMNS: u32 = 5;
pub const ROWS: u32 = 6;
const MAX_CELL: u32 = 4;
/// An empty slot's ground.
pub const GROUND: [u8; 3] = [225, 223, 213];
/// Fewer pixels than this off the ground is an empty slot.
const EMPTY_BELOW: usize = 5;

/// The tab in a picked slot's top left: columns and rows of the slot it covers.
const PICKED_TAB: (u32, u32) = (10, 9);

/// Whether the slot whose inside starts at `(x0, y0)` is the picked one: the game frames it in
/// blue, along the rows just above its inside.
fn picked(frame: &Frame, x0: u32, y0: u32, cell: u32) -> bool {
    let blue_row = |above: u32| {
        let Some(y) = y0.checked_sub(above * cell) else { return false };
        let blue = (2..SLOT_SIZE - 2)
            .filter(|&x| {
                let [r, g, b, _] = frame.get_pixel(x0 + x * cell, y).0;
                b >= 150 && u32::from(b) >= u32::from(r) + 60 && g >= 100
            })
            .count() as u32;
        blue * 3 >= (SLOT_SIZE - 4) * 2
    };
    blue_row(1) || blue_row(2)
}

/// What the open tab shows.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inventory {
    /// `None` when no tab looks open.
    pub tab: Option<Tab>,
    /// The slots in view that hold something, top row first.
    pub slots: Vec<BagSlot>,
    pub window: Rect,
}

/// One slot that holds something.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BagSlot {
    /// Row times five plus column, counting from the top left of the slots in view.
    pub index: u32,
    pub count: Option<u32>,
    /// Read from a video, the count didn't read the same twice: how many there are isn't known.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub count_unsure: bool,
    #[serde(skip)]
    pub picture: SlotPicture,
}

/// A slot's inside as the game drew it, one colour to each of its pixels, with the pixels the
/// count covers (and those next to it) marked to be left out.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SlotPicture {
    pub pixels: Vec<[u8; 3]>,
    pub hidden: Vec<bool>,
}

impl SlotPicture {
    pub fn get(&self, x: u32, y: u32) -> [u8; 3] {
        self.pixels[(y * SLOT_SIZE + x) as usize]
    }

    pub fn is_hidden(&self, x: u32, y: u32) -> bool {
        self.hidden[(y * SLOT_SIZE + x) as usize]
    }
}

/// Reads the window, or `None` when no Item Inventory is open.
pub fn read(frame: &Frame) -> Option<Inventory> {
    read_found(frame, frame, find(frame, is_white, 0)?)
}

/// [`read`] for a picture that went through a video encoding (a browser's shared window), whose
/// white comes out a shade off, whose edges may bleed a pixel, and whose colour is kept only for
/// each square of four pixels. The window is found with that give, then read on its own grid of
/// font pixels: each the average of the window pixels it covers, the grid lined up where the
/// window's white frame comes out whitest. The counts are read from a copy of that grid with
/// their white fill and black outline made pure again.
pub fn read_video(frame: &Frame) -> Option<Inventory> {
    if let Some(found) = find(frame, is_white, 0) {
        return read_found(frame, frame, found);
    }
    // With the give, other white lines can pass for the window's: the one with a tab open is it.
    found_all(frame, is_near_white, VIDEO_BLEED)
        .into_iter()
        .filter_map(|found| read_video_at(frame, found))
        .find(|inventory| inventory.tab.is_some())
}

fn read_video_at(frame: &Frame, (start, top, cell): (u32, u32, u32)) -> Option<Inventory> {
    let (width, height) = (TOP_LINE + 2 * CORNER, MESO_BOX.1 + 15);
    let left = |dx: u32| (start + dx).checked_sub(VIDEO_BLEED + CORNER * cell);
    let grids = (0..=2 * VIDEO_BLEED).flat_map(|dx| (0..=cell).map(move |dy| (dx, dy)));
    let whitest = grids
        .filter_map(|(dx, dy)| {
            let (x, y) = (left(dx)?, (top + dy).checked_sub(1)?);
            let grid = on_grid(frame, x, y, cell, width, height)?;
            Some((frame_whiteness(&grid), x, y, grid))
        })
        .max_by_key(|(white, ..)| *white)?;
    let (_, x, y, grid) = whitest;
    let keyed = |loose: bool| Frame::from_fn(grid.width(), grid.height(), |gx, gy| image::Rgba(keyed_count(grid.get_pixel(gx, gy).0, loose)));
    let mut inventory = read_found(&grid, &keyed(false), (CORNER, 0, 1))?;
    // The counts read again with the fill and outline taken more loosely: a count that reads
    // otherwise (a 1 of 11 lost where colour blurred) isn't known.
    if let Some(again) = read_found(&grid, &keyed(true), (CORNER, 0, 1)) {
        for slot in &mut inventory.slots {
            let other = again.slots.iter().find(|s| s.index == slot.index).and_then(|s| s.count);
            if other != slot.count {
                slot.count_unsure = true;
            }
        }
    }
    inventory.window = Rect::new(x as f32, y as f32, (width * cell) as f32, (height * cell) as f32);
    Some(inventory)
}

/// The `width` x `height` font pixels from `(x, y)` of the window, each the average of its
/// `cell` x `cell` window pixels.
fn on_grid(frame: &Frame, x: u32, y: u32, cell: u32, width: u32, height: u32) -> Option<Frame> {
    if x + width * cell > frame.width() || y + height * cell > frame.height() {
        return None;
    }
    Some(Frame::from_fn(width, height, |gx, gy| {
        let mut sum = [0u32; 3];
        for py in 0..cell {
            for px in 0..cell {
                let p = frame.get_pixel(x + gx * cell + px, y + gy * cell + py).0;
                for c in 0..3 {
                    sum[c] += u32::from(p[c]);
                }
            }
        }
        let n = cell * cell;
        image::Rgba([(sum[0] / n) as u8, (sum[1] / n) as u8, (sum[2] / n) as u8, 255])
    }))
}

/// How white the window's frame comes out on a grid: its top line and left edge, the font pixels
/// that are white when the grid lines up.
fn frame_whiteness(grid: &Frame) -> u32 {
    let sum = |x: u32, y: u32| grid.get_pixel(x, y).0[..3].iter().map(|&c| u32::from(c)).sum::<u32>();
    let line: u32 = (CORNER..CORNER + TOP_LINE).map(|x| sum(x, 0)).sum();
    let edge: u32 = (LEFT_EDGE.0..LEFT_EDGE.1).map(|y| sum(0, y)).sum();
    line + edge
}

/// A count's fill and outline as drawn, from their averages through a video: white or pale blue
/// to pure white, near black to pure black.
/// `loose` takes paler fill and greyer outline too.
fn keyed_count([r, g, b, a]: [u8; 4], loose: bool) -> [u8; 4] {
    // The loose fill stays clear of an empty slot's ground, (225, 223, 213).
    let (fill, dark) = if loose { ((110, 120, 219), 72) } else { ((130, 140, 228), 48) };
    if r >= fill.0 && g >= fill.1 && b >= fill.2 {
        [255, 255, 255, a]
    } else if r.max(g).max(b) <= dark {
        [0, 0, 0, a]
    } else {
        [r, g, b, a]
    }
}

/// How far a video's edges may run past or short of where they're drawn, in pixels.
const VIDEO_BLEED: u32 = 2;

/// The window whose top line starts at `(start, top)`, `cell` window pixels to a font pixel:
/// pictures from `frame`, counts from `counted` (the same, but for a video's).
fn read_found(frame: &Frame, counted: &Frame, (start, top, cell): (u32, u32, u32)) -> Option<Inventory> {
    let at = |x: u32, y: u32| (start + x * cell, top + y * cell);
    let tab = open_tab(frame, start, top, cell);
    let mut slots = Vec::new();
    for row in 0..ROWS {
        for column in 0..COLUMNS {
            let (x0, y0) = at(FIRST_SLOT.0 + column * SLOT_STEP.0, FIRST_SLOT.1 + row * SLOT_STEP.1);
            if x0 + SLOT_SIZE * cell > frame.width() || y0 + SLOT_SIZE * cell > frame.height() {
                continue;
            }
            let side = SLOT_SIZE * cell;
            // The count's outline can lie on the slot's frame, so look a little past it, and keep
            // to counts that start in this slot.
            let margin = 2 * cell;
            // A picked slot's frame is drawn over the count's outline where they meet.
            let outside = |x: u32, y: u32| x < x0 || y < y0 || x >= x0 + side || y >= y0 + side;
            let area = (x0 - margin, y0 - margin, x0 + side + margin, y0 + side + margin);
            let count = counts::read_walled(counted, area, Some(cell), &outside)
                .into_iter()
                .filter(|c| c.left + margin >= x0 && c.left < x0 + side)
                .max_by_key(|c| c.bottom);
            let mut picture = SlotPicture { pixels: Vec::new(), hidden: Vec::new() };
            for y in 0..SLOT_SIZE {
                for x in 0..SLOT_SIZE {
                    let px = frame.get_pixel(x0 + x * cell, y0 + y * cell).0;
                    picture.pixels.push([px[0], px[1], px[2]]);
                    // The count's digits and their outline, a font pixel round them.
                    let (wx, wy) = (x0 + x * cell, y0 + y * cell);
                    let near_count = count.is_some_and(|c| {
                        wx + 2 * cell > c.left && wx < c.right + 2 * cell && wy + 2 * cell > c.top && wy < c.bottom + 2 * cell
                    });
                    picture.hidden.push(near_count);
                }
            }
            if picked(frame, x0, y0, cell) {
                // The picked slot's thick frame and the tab in its top left cover the picture's edges.
                for y in 0..SLOT_SIZE {
                    for x in 0..SLOT_SIZE {
                        let frame_part = x < 2 || y < 2 || x + 2 >= SLOT_SIZE || y + 2 >= SLOT_SIZE;
                        let tab_part = x < PICKED_TAB.0 && y < PICKED_TAB.1;
                        if frame_part || tab_part {
                            picture.hidden[(y * SLOT_SIZE + x) as usize] = true;
                        }
                    }
                }
            }
            let used = picture
                .pixels
                .iter()
                .zip(&picture.hidden)
                .filter(|(px, hidden)| !**hidden && off_ground(**px))
                .count();
            if used < EMPTY_BELOW && count.is_none() {
                continue;
            }
            // Through a video a digit can lose its outline and drop out of the count unseen: fill
            // left over beside the count, or in the corner where a count stands, makes it unsure.
            let video = !std::ptr::eq(frame, counted);
            let count_unsure = video && leftover_fill(counted, count.as_ref(), x0, y0, side);
            slots.push(BagSlot { index: row * COLUMNS + column, count: count.map(|c| c.value), count_unsure, picture });
        }
    }
    let left = start.saturating_sub(CORNER * cell) as f32;
    let window = Rect::new(left, top as f32, ((TOP_LINE + 2 * CORNER) * cell) as f32, ((MESO_BOX.1 + 15) * cell) as f32);
    Some(Inventory { tab, slots, window })
}

/// Font pixels of a count's fill, keyed pure white, standing against its keyed black outline and
/// not part of the count read: to the right of it along its rows, or with no count read, in the
/// slot's bottom-left corner where counts stand. Read on a video's grid, one window pixel to a
/// font pixel.
fn leftover_fill(keyed: &Frame, count: Option<&counts::Count>, x0: u32, y0: u32, side: u32) -> bool {
    const BESIDE: u32 = 7;
    const CORNER_WIDTH: u32 = 20;
    let (left, top, right, bottom) = match count {
        Some(c) => (c.right, c.top, (c.right + BESIDE).min(x0 + side), c.bottom),
        None => (x0, (y0 + side).saturating_sub(counts::DIGIT_ROWS + 2), x0 + CORNER_WIDTH, y0 + side),
    };
    let at = |x: u32, y: u32| (x < keyed.width() && y < keyed.height()).then(|| keyed.get_pixel(x, y).0);
    let fill = |x: u32, y: u32| at(x, y).is_some_and(|p| p[..3] == [255, 255, 255]);
    let outline = |x: u32, y: u32| at(x, y).is_some_and(|p| p[..3] == [0, 0, 0]);
    let edged = (top..bottom)
        .flat_map(|y| (left..right).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            fill(x, y) && (outline(x + 1, y) || outline(x, y + 1) || x.checked_sub(1).is_some_and(|x| outline(x, y)) || y.checked_sub(1).is_some_and(|y| outline(x, y)))
        })
        .count();
    edged >= 3
}

/// A colour that isn't an empty slot's ground.
pub fn off_ground(px: [u8; 3]) -> bool {
    px.iter().zip(GROUND).map(|(&a, b)| u32::from(a.abs_diff(b))).sum::<u32>() > 12
}

fn is_white(frame: &Frame, x: u32, y: u32) -> bool {
    x < frame.width() && y < frame.height() && frame.get_pixel(x, y).0[..3] == [255, 255, 255]
}

/// White as a video encoding gives it back.
fn is_near_white(frame: &Frame, x: u32, y: u32) -> bool {
    x < frame.width() && y < frame.height() && frame.get_pixel(x, y).0[..3].iter().all(|&c| c >= 236)
}

type White = fn(&Frame, u32, u32) -> bool;

/// Where the window's top line starts, its row, and the window pixels to a font pixel.
/// `bleed`: how far the top line may run past or short of its length at either end.
fn find(frame: &Frame, white: White, bleed: u32) -> Option<(u32, u32, u32)> {
    first_found(frame, white, bleed, &mut |found| Some(found))
}

/// Every place [`find`] would take for the window, top first.
fn found_all(frame: &Frame, white: White, bleed: u32) -> Vec<(u32, u32, u32)> {
    let mut all = Vec::new();
    first_found(frame, white, bleed, &mut |found| {
        all.push(found);
        None::<()>
    });
    all
}

/// Hands each place the window's top line, left edge and meso box are found at to `take`, top
/// first, until it takes one.
fn first_found<T>(frame: &Frame, white: White, bleed: u32, take: &mut dyn FnMut((u32, u32, u32)) -> Option<T>) -> Option<T> {
    let (width, height) = (frame.width(), frame.height());
    for y in 0..height {
        let mut x = 0;
        while x < width {
            if !white(frame, x, y) {
                x += 1;
                continue;
            }
            let start = x;
            while x < width && white(frame, x, y) {
                x += 1;
            }
            let length = x - start;
            let Some(cell) = (1..=MAX_CELL).find(|c| length.abs_diff(TOP_LINE * c) <= 2 * bleed) else { continue };
            // A bled line starts early by as much as it runs long, give or take.
            let starts = (0..=bleed).flat_map(|d| [start + d, start.saturating_sub(d)]);
            for start in starts {
                if left_edge(frame, start, y, cell, white, bleed)
                    && meso_box(frame, start, y, cell, white, bleed)
                    && let Some(taken) = take((start, y, cell))
                {
                    return Some(taken);
                }
            }
        }
    }
    None
}

fn left_edge(frame: &Frame, start: u32, top: u32, cell: u32, white: White, bleed: u32) -> bool {
    let Some(x) = start.checked_sub(CORNER * cell) else { return false };
    let (from, to) = (top + LEFT_EDGE.0 * cell, top + LEFT_EDGE.1 * cell);
    let column = |x: u32| (from..to).filter(|&y| white(frame, x, y)).count() as u32;
    let most = (x.saturating_sub(bleed)..=x + bleed).map(column).max().unwrap_or(0);
    most * 10 >= (to - from) * 9
}

fn meso_box(frame: &Frame, start: u32, top: u32, cell: u32, white: White, bleed: u32) -> bool {
    let (x, y) = (start + MESO_BOX.0 * cell, top + MESO_BOX.1 * cell);
    let length = MESO_BOX.2 * cell;
    if bleed == 0 {
        return white(frame, x, y) && !white(frame, x.wrapping_sub(1), y) && (x..x + length).all(|px| white(frame, px, y));
    }
    // The box's top, give or take the bleed at its ends and a row either way.
    (y.saturating_sub(1)..=y + 1).any(|y| (x + bleed..x + length - bleed).all(|px| white(frame, px, y)))
}

/// The tab drawn pink, the open one.
fn open_tab(frame: &Frame, start: u32, top: u32, cell: u32) -> Option<Tab> {
    let y = top + TAB_ROW * cell;
    let pink = |i: u32| {
        let left = start + (TAB_LEFT + i * TAB_STEP) * cell;
        (left..left + TAB_WIDTH * cell)
            .filter(|&x| x < frame.width())
            .filter(|&x| {
                let [r, g, _, _] = frame.get_pixel(x, y).0;
                r >= 150 && u32::from(r) >= u32::from(g) + 60
            })
            .count() as u32
    };
    let scores: Vec<u32> = (0..TABS.len() as u32).map(pink).collect();
    let (best, &most) = scores.iter().enumerate().max_by_key(|(_, s)| **s)?;
    (most * 3 >= TAB_WIDTH * cell).then_some(TABS[best])
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::readers::counts::tests::draw;
    use image::Rgba;

    /// A made-up inventory window, `cell` window pixels to a font pixel, with `open` as the pink
    /// tab and a made-up picture (a square of one colour) in each slot given, with its count.
    pub fn window(cell: u32, open: usize, items: &[(u32, [u8; 3], Option<&str>)]) -> (Frame, (u32, u32)) {
        let (start, top) = (100 * cell, 40 * cell);
        let mut frame = Frame::from_pixel(600 * cell, 360 * cell, Rgba([150, 120, 80, 255]));
        let fill = |frame: &mut Frame, x: u32, y: u32, w: u32, h: u32, c: [u8; 3]| {
            for py in y..y + h {
                for px in x..x + w {
                    frame.put_pixel(px, py, Rgba([c[0], c[1], c[2], 255]));
                }
            }
        };
        let left = start - CORNER * cell;
        fill(&mut frame, left + cell, top + cell, 205 * cell, 285 * cell, [245, 246, 247]);
        fill(&mut frame, start, top, TOP_LINE * cell, cell, [255, 255, 255]);
        fill(&mut frame, left, top + 3 * cell, cell, 280 * cell, [255, 255, 255]);
        fill(&mut frame, start + MESO_BOX.0 * cell, top + MESO_BOX.1 * cell, MESO_BOX.2 * cell, 14 * cell, [255, 255, 255]);
        for i in 0..6u32 {
            let colour = if i as usize == open { [255, 138, 168] } else { [189, 189, 189] };
            fill(&mut frame, start + (TAB_LEFT + i * TAB_STEP) * cell, top + (TAB_ROW - 4) * cell, (TAB_WIDTH - 1) * cell, 10 * cell, colour);
        }
        for row in 0..ROWS {
            for column in 0..COLUMNS {
                let (x, y) = (start + (FIRST_SLOT.0 + column * SLOT_STEP.0) * cell, top + (FIRST_SLOT.1 + row * SLOT_STEP.1) * cell);
                fill(&mut frame, x, y, SLOT_SIZE * cell, SLOT_SIZE * cell, GROUND);
            }
        }
        for &(index, colour, count) in items {
            let (row, column) = (index / COLUMNS, index % COLUMNS);
            let (x, y) = (start + (FIRST_SLOT.0 + column * SLOT_STEP.0) * cell, top + (FIRST_SLOT.1 + row * SLOT_STEP.1) * cell);
            fill(&mut frame, x + 4 * cell, y + 3 * cell, 22 * cell, 22 * cell, colour);
            if let Some(text) = count {
                draw(&mut frame, text, x + cell, y + 20 * cell, cell);
            }
        }
        (frame, (start, top))
    }

    #[test]
    fn reads_the_open_tab_and_each_filled_slot() {
        for cell in [1, 2] {
            let (frame, _) = window(cell, 3, &[(0, [200, 30, 30], Some("27")), (6, [30, 160, 30], Some("109")), (29, [40, 40, 200], None)]);
            let bag = read(&frame).expect("window found");
            assert_eq!(bag.tab, Some(Tab::Etc), "cell {cell}");
            let slots: Vec<(u32, Option<u32>)> = bag.slots.iter().map(|s| (s.index, s.count)).collect();
            assert_eq!(slots, [(0, Some(27)), (6, Some(109)), (29, None)], "cell {cell}");
            let picture = &bag.slots[0].picture;
            assert_eq!(picture.get(10, 5), [200, 30, 30]);
            assert!(picture.is_hidden(3, 24), "the count's corner is left out");
            assert!(!picture.is_hidden(28, 3));
        }
    }

    /// A window as a browser's video gives it back: white a shade off, every pixel's colour
    /// nudged, and the top line a pixel long at its start.
    fn through_a_video(frame: &Frame, (start, top): (u32, u32)) -> Frame {
        let mut out = Frame::from_fn(frame.width(), frame.height(), |x, y| {
            let [r, g, b, a] = frame.get_pixel(x, y).0;
            let nudge = |c: u8| if c == 255 { 250 } else { c.saturating_add(((x + y) % 3) as u8) };
            Rgba([nudge(r), nudge(g), nudge(b), a])
        });
        out.put_pixel(start - 1, top, Rgba([248, 249, 250, 255]));
        out
    }

    #[test]
    fn a_window_through_a_video_reads_on_its_own_grid() {
        let items = [(0, [200, 30, 30], Some("27")), (6, [30, 160, 30], Some("109")), (29, [40, 40, 200], None)];
        let (frame, at) = window(2, 1, &items);
        let video = through_a_video(&frame, at);
        assert!(read(&video).is_none(), "the exact reading wants pure white");
        let bag = read_video(&video).expect("window found");
        assert_eq!(bag.tab, Some(Tab::Use));
        let slots: Vec<(u32, Option<u32>, bool)> = bag.slots.iter().map(|s| (s.index, s.count, s.count_unsure)).collect();
        assert_eq!(slots, [(0, Some(27), false), (6, Some(109), false), (29, None, false)]);
        // A slot's picture is its font pixels' colours, near what was drawn.
        let px = bag.slots[0].picture.get(10, 10);
        assert!(px.iter().zip([200u8, 30, 30]).all(|(a, b)| a.abs_diff(b) <= 3), "{px:?}");
    }

    #[test]
    fn a_window_seen_exactly_reads_the_same_by_either_way() {
        let (frame, _) = window(2, 3, &[(4, [90, 90, 200], Some("5"))]);
        assert_eq!(read_video(&frame), read(&frame));
    }

    #[test]
    fn no_window_no_reading() {
        assert_eq!(read(&Frame::from_pixel(800, 600, Rgba([255, 255, 255, 255]))), None);
    }
}
