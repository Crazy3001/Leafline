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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
    let (start, top, cell) = find(frame)?;
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
            let count = counts::read_walled(frame, area, Some(cell), &outside)
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
            slots.push(BagSlot { index: row * COLUMNS + column, count: count.map(|c| c.value), picture });
        }
    }
    let left = start.saturating_sub(CORNER * cell) as f32;
    let window = Rect::new(left, top as f32, ((TOP_LINE + 2 * CORNER) * cell) as f32, ((MESO_BOX.1 + 15) * cell) as f32);
    Some(Inventory { tab, slots, window })
}

/// A colour that isn't an empty slot's ground.
pub fn off_ground(px: [u8; 3]) -> bool {
    px.iter().zip(GROUND).map(|(&a, b)| u32::from(a.abs_diff(b))).sum::<u32>() > 12
}

fn is_white(frame: &Frame, x: u32, y: u32) -> bool {
    x < frame.width() && y < frame.height() && frame.get_pixel(x, y).0[..3] == [255, 255, 255]
}

/// Where the window's top line starts, its row, and the window pixels to a font pixel.
fn find(frame: &Frame) -> Option<(u32, u32, u32)> {
    let (width, height) = (frame.width(), frame.height());
    for y in 0..height {
        let mut x = 0;
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
            if left_edge(frame, start, y, cell) && meso_box(frame, start, y, cell) {
                return Some((start, y, cell));
            }
        }
    }
    None
}

fn left_edge(frame: &Frame, start: u32, top: u32, cell: u32) -> bool {
    let Some(x) = start.checked_sub(CORNER * cell) else { return false };
    let (from, to) = (top + LEFT_EDGE.0 * cell, top + LEFT_EDGE.1 * cell);
    let white = (from..to).filter(|&y| is_white(frame, x, y)).count() as u32;
    white * 10 >= (to - from) * 9
}

fn meso_box(frame: &Frame, start: u32, top: u32, cell: u32) -> bool {
    let (x, y) = (start + MESO_BOX.0 * cell, top + MESO_BOX.1 * cell);
    let length = MESO_BOX.2 * cell;
    is_white(frame, x, y) && !is_white(frame, x.wrapping_sub(1), y) && (x..x + length).all(|px| is_white(frame, px, y))
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

    #[test]
    fn no_window_no_reading() {
        assert_eq!(read(&Frame::from_pixel(800, 600, Rgba([255, 255, 255, 255]))), None);
    }
}
