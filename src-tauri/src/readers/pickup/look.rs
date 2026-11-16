//! Finding the Pickup Log in a picture and seeing its rows.
//!
//! The log is drawn in a pixel font, every font pixel a square of window pixels (two at 1440p),
//! white with a black shadow one font pixel down and to the right, right-aligned near the play
//! area's right edge, a line every 14 font pixels. A line fades by turning see-through: its ink
//! and its shadow both let the world behind show. So a font pixel is ink where it is lighter
//! than the pixel its shadow falls on by the same amount in red, green and blue (white text) or
//! in red and green only (yellow text), whatever the world behind is, and that amount says how
//! far the line has faded. World textures rarely pass that test, and almost never in a row that
//! ends exactly where the log's lines end.

use image::Rgba;

use super::stack::{Colour, Row, Rows, Shape};
use crate::vision::Frame;

/// Rows of the log looked at, bottom first. The game shows six; a seventh costs nothing.
pub const SLOTS: usize = 7;
/// Font pixels from one line's top to the next one's.
const PITCH: u32 = 14;
/// Font pixels from a line's top to below its shadow under a descender.
const LINE_HEIGHT: u32 = 12;
/// How much lighter than its shadow a full-brightness ink pixel is (white `f6f6f5` over
/// black `020201`) where the font is drawn pixel for pixel.
pub const FULL: f32 = 244.0;
/// The same where the game smooths the font (UI Size Default Ratio) and the picture is redrawn
/// on the design grid: ink and shadow blur into each other (the recording of 2026-11-15 at
/// 1.875x: a new line's ink 150 to 155 lighter than its shadow).
pub const SMOOTHED_FULL: f32 = 155.0;
/// The least shadow difference that may be ink, before the line's own brightness is known.
const FAINTEST: i32 = 36;
/// A row fainter than this is too far gone to compare or read.
const MIN_BRIGHTNESS: f32 = 0.3;
/// Ink font pixels a row needs to be a line at all: `(+1)` alone has about this many.
const MIN_INK: usize = 24;
/// Font pixels of a line's right end kept as its shape: the figure or tab and the word before.
const SHAPE_WIDTH: usize = 48;
/// The widest line, in font pixels: a cut-short item name with its count and tab.
const WIDEST: u32 = 300;
/// Font pixels between words, at most; a longer gap ends the line.
const WORD_GAP: u32 = 8;

/// Where the log is in a picture of the game window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Layout {
    /// Window pixels to a font pixel.
    pub scale: u32,
    /// Just past the lines' right ends, where every line's closing bracket stops.
    pub right: u32,
    /// The top of the bottom row: the top of its capitals and brackets.
    pub bottom: u32,
    /// How much lighter than its shadow a line at full brightness is here.
    pub full: f32,
}

impl Layout {
    fn top(&self, slot: usize) -> Option<u32> {
        self.bottom.checked_sub(slot as u32 * PITCH * self.scale)
    }

    /// The same layout a row lower, for a log first seen with its bottom row empty.
    pub fn lower(&self) -> Self {
        Self { bottom: self.bottom + PITCH * self.scale, ..*self }
    }
}

/// One font pixel's ink, as told by its shadow.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Cell {
    /// How much lighter than its shadow, 0 for none.
    strength: f32,
    colour: Colour,
}

/// The font pixel whose top left is at (`x`, `y`), against the one its shadow falls on.
fn cell(frame: &Frame, x: u32, y: u32, scale: u32) -> Cell {
    let none = Cell { strength: 0.0, colour: Colour::Other };
    if x + scale >= frame.width() || y + scale >= frame.height() {
        return none;
    }
    let Rgba([pr, pg, pb, _]) = *frame.get_pixel(x, y);
    let Rgba([qr, qg, qb, _]) = *frame.get_pixel(x + scale, y + scale);
    let (r, g, b) = (i32::from(pr) - i32::from(qr), i32::from(pg) - i32::from(qg), i32::from(pb) - i32::from(qb));
    let low = r.min(g).min(b);
    let high = r.max(g).max(b);
    if low >= FAINTEST && high - low <= 24 + high / 5 {
        return Cell { strength: (r + g + b) as f32 / 3.0, colour: Colour::White };
    }
    // Yellow: red and green alike, blue well behind. Not yet seen in a recording.
    if r.min(g) >= FAINTEST && (r - g).abs() <= 24 + r.max(g) / 5 && b <= r.min(g) / 2 {
        return Cell { strength: (r + g) as f32 / 2.0, colour: Colour::Yellow };
    }
    none
}

/// Finds the log in `area` of `frame` (left, top, right, bottom) from a line at full brightness
/// or near it: its font size, its right edge and its bottom row. `full` is [`FULL`] or
/// [`SMOOTHED_FULL`]. `None` when no such line shows.
pub fn locate(frame: &Frame, full: f32, area: (u32, u32, u32, u32)) -> Option<Layout> {
    let (width, height) = frame.dimensions();
    (1..=4)
        .filter_map(|scale| locate_at(frame, scale, full, area).map(|(layout, ink)| (ink, layout)))
        .max_by_key(|(ink, _)| *ink)
        .map(|(_, layout)| layout)
        .filter(|layout| layout.right + layout.scale * 40 >= width && layout.bottom < height)
}

/// The lowest bright line drawn at `scale`, with how much ink it has.
fn locate_at(frame: &Frame, scale: u32, full: f32, area: (u32, u32, u32, u32)) -> Option<(Layout, usize)> {
    let (width, height) = frame.dimensions();
    let (left, top, right_edge, bottom) = area;
    // Bright ink, row by row: (y, rightmost x, count).
    let mut rows: Vec<(u32, u32, usize)> = Vec::new();
    for y in top..bottom.min(height).saturating_sub(scale) {
        let mut right = None;
        let mut count = 0;
        for x in left..right_edge.min(width).saturating_sub(scale) {
            let c = cell(frame, x, y, scale);
            let [r, g, b, _] = frame.get_pixel(x, y).0;
            // White ink, or yellow (a partymate's share and the party bonus, `fefb25` or so).
            // Smoothed (Default Ratio), a bright line is light grey on dark grey.
            let lit = r >= 0xb0 && g >= 0xb0 && (b >= 0xb0 || c.colour == Colour::Yellow);
            if c.strength >= 0.75 * full && lit {
                count += 1;
                right = Some(x);
            }
        }
        if let Some(right) = right {
            rows.push((y, right, count));
        }
    }
    // Lines are runs of such rows; the lowest one tall enough to be a line wins.
    let mut lines: Vec<(u32, u32, u32, usize)> = Vec::new();
    for (y, right, count) in rows {
        match lines.last_mut() {
            Some((_, bottom, line_right, ink)) if y <= *bottom + 2 * scale => {
                *bottom = y;
                *line_right = (*line_right).max(right);
                *ink += count;
            }
            _ => lines.push((y, y, right, count)),
        }
    }
    lines
        .into_iter()
        .rfind(|(top, bottom, _, ink)| {
            let tall = bottom - top + 1;
            tall >= 8 * scale && tall <= 12 * scale && *ink >= MIN_INK * (scale * scale) as usize
        })
        .map(|(top, _, right, ink)| (Layout { scale, right: right + 1, bottom: top, full }, ink))
}

/// The cells of the row at `top`, `columns` font pixels leftwards from `right`, row by row:
/// `grid[row][column]`, column 0 at the right.
fn grid(frame: &Frame, layout: &Layout, top: u32, columns: u32) -> Vec<Vec<Cell>> {
    let s = layout.scale;
    (0..LINE_HEIGHT)
        .map(|j| {
            (0..columns)
                .map(|i| match layout.right.checked_sub((i + 1) * s) {
                    Some(x) => cell(frame, x, top + j * s, s),
                    None => Cell { strength: 0.0, colour: Colour::Other },
                })
                .collect()
        })
        .collect()
}

/// The strength most of a row's ink reaches, as one cell's own world can make it a little more
/// or less.
fn strength_of(cells: &[Vec<Cell>]) -> Option<f32> {
    let mut strengths: Vec<f32> = cells.iter().flatten().map(|c| c.strength).filter(|&s| s > 0.0).collect();
    if strengths.len() < MIN_INK {
        return None;
    }
    strengths.sort_by(f32::total_cmp);
    Some(strengths[strengths.len() * 4 / 5])
}

/// How bright a row's ink is against a line at full brightness in this layout.
fn brightness(cells: &[Vec<Cell>], layout: &Layout) -> Option<f32> {
    Some(strength_of(cells)? / layout.full)
}

/// The log's rows in `frame`, bottom first: `None` where no line shows, or one too faint to
/// tell.
pub fn rows(frame: &Frame, layout: &Layout) -> Rows {
    (0..SLOTS).map(|slot| row(frame, layout, slot)).collect()
}

fn row(frame: &Frame, layout: &Layout, slot: usize) -> Option<Row> {
    let top = layout.top(slot)?;
    let cells = grid(frame, layout, top, SHAPE_WIDTH as u32);
    let brightness = brightness(&cells, layout)?;
    if brightness < MIN_BRIGHTNESS {
        return None;
    }
    let ink = |c: &Cell| c.strength >= 0.5 * brightness * layout.full;
    // Every line ends with its closing bracket at the log's right edge.
    let rightmost = (0..SHAPE_WIDTH).find(|&i| cells.iter().any(|row| ink(&row[i])))?;
    if rightmost > 1 {
        return None;
    }
    let (mut white, mut yellow) = (0, 0);
    for c in cells.iter().flatten().filter(|c| ink(c)) {
        match c.colour {
            Colour::White => white += 1,
            Colour::Yellow => yellow += 1,
            Colour::Other => {}
        }
    }
    if white + yellow < MIN_INK {
        return None;
    }
    let colour = if yellow > white * 2 { Colour::Yellow } else { Colour::White };
    let shape = Shape::from_fn(SHAPE_WIDTH, LINE_HEIGHT as usize, |x, y| ink(&cells[y][SHAPE_WIDTH - 1 - x]));
    Some(Row { shape, colour, brightness: brightness.min(1.0) })
}

/// Whether a line shows a row below the layout's bottom row, the log having been found while
/// its bottom row was empty.
pub fn line_below(frame: &Frame, layout: &Layout) -> bool {
    let lower = layout.lower();
    lower.bottom + LINE_HEIGHT * lower.scale < frame.height()
        && row(frame, &lower, 0).is_some_and(|r| r.brightness >= 0.85 && r.colour == Colour::White)
}

/// The ink of the line in `slot`, font pixel by font pixel: `ink[row][column]` with column 0 at
/// the right, out to the line's start. Font pixels the shadow test misses (a stroke running down
/// to the right covers its own shadow) are taken back where they look like the line's ink.
pub fn line_ink(frame: &Frame, layout: &Layout, slot: usize) -> Option<Vec<Vec<bool>>> {
    line_ink_at(frame, layout, slot, INK_LEVEL)
}

/// How much of a line's own brightness a font pixel's ink must reach to count, as a share.
pub const INK_LEVEL: f32 = 0.5;

/// [`line_ink`] with the ink cut at `level` of the line's brightness instead of [`INK_LEVEL`].
pub fn line_ink_at(frame: &Frame, layout: &Layout, slot: usize, level: f32) -> Option<Vec<Vec<bool>>> {
    let top = layout.top(slot)?;
    let cells = grid(frame, layout, top, WIDEST);
    let brightness = brightness(&cells, layout)?;
    let threshold = level * brightness * layout.full;
    let mut ink: Vec<Vec<bool>> = cells.iter().map(|row| row.iter().map(|c| c.strength >= threshold).collect()).collect();
    // Where the line starts: the last ink before a gap wider than words have.
    let columns = ink[0].len();
    let filled: Vec<bool> = (0..columns).map(|i| ink.iter().any(|row| row[i])).collect();
    let mut start = 0;
    let mut gap = 0;
    for (i, &on) in filled.iter().enumerate() {
        if on {
            start = i;
            gap = 0;
        } else {
            gap += 1;
            if gap > WORD_GAP as usize && i > SHAPE_WIDTH {
                break;
            }
        }
    }
    let s = layout.scale;
    let pixel = |i: usize, j: usize| {
        let x = layout.right.checked_sub((i as u32 + 1) * s)?;
        let y = top + j as u32 * s;
        (x < frame.width() && y < frame.height()).then(|| frame.get_pixel(x, y).0)
    };
    // The line's ink colour: every ink pixel of a line drawn at full brightness is close to the
    // same colour, where a light world behind it varies.
    let ink_colour: Option<[u8; 3]> = {
        let mut channels: [Vec<u8>; 3] = Default::default();
        for (j, row) in ink.iter().enumerate() {
            for (i, &on) in row.iter().enumerate().take(start + 1) {
                if let (true, Some(px)) = (on, pixel(i, j)) {
                    (0..3).for_each(|c| channels[c].push(px[c]));
                }
            }
        }
        channels.iter_mut().for_each(|c| c.sort_unstable());
        (channels[0].len() >= MIN_INK && brightness >= 0.9).then(|| [0, 1, 2].map(|c| channels[c][channels[c].len() / 2]))
    };
    // Exactly the ink's colour: tells ink from a light world at full brightness.
    let exact = |i: usize, j: usize| match (ink_colour, pixel(i, j)) {
        // Yellow ink varies by ten levels or so from pixel to pixel; white by one or two.
        (Some(colour), Some(px)) => (0..3).all(|c| px[c].abs_diff(colour[c]) <= 12),
        _ => false,
    };
    // As light as the ink and about as grey: the only test left once a line has faded a little.
    let light = brightness * 246.0 * 0.8;
    let yellow = cells.iter().flatten().filter(|c| c.colour == Colour::Yellow).count()
        > 2 * cells.iter().flatten().filter(|c| c.colour == Colour::White).count();
    let pale = |i: usize, j: usize| {
        pixel(i, j).is_some_and(|px| {
            px[..2].iter().all(|&v| f32::from(v) >= light) && (yellow || px.iter().max().unwrap() - px.iter().min().unwrap() <= 24)
        })
    };
    // A font pixel fails the shadow test only where the one its shadow falls on, down and to the
    // right, is ink too. Going up from the bottom row, such a pixel that looks like the ink is
    // ink, so a stroke running down to the right comes back whole. Over a world as light as the
    // ink (a pale stone wall, clouds) that would fill in the letters, so a test counts only
    // where few other pixels that aren't ink pass it nearby.
    let passing = |test: &dyn Fn(usize, usize) -> bool| -> Vec<Vec<bool>> {
        (0..LINE_HEIGHT as usize).map(|j| (0..=start).map(|i| !ink[j][i] && test(i, j)).collect()).collect()
    };
    let (exact_cells, pale_cells) = (passing(&exact), passing(&pale));
    let alone = |cells: &[Vec<bool>], i: usize, j: usize| {
        let near = (j.saturating_sub(2)..(j + 3).min(LINE_HEIGHT as usize))
            .flat_map(|y| (i.saturating_sub(2)..(i + 3).min(start + 1)).map(move |x| (x, y)))
            .filter(|&(x, y)| (x, y) != (i, j) && cells[y][x])
            .count();
        cells[j][i] && near <= 2
    };
    for j in (0..LINE_HEIGHT as usize - 1).rev() {
        for i in 1..=start {
            if !ink[j][i] && ink[j + 1][i - 1] && (alone(&exact_cells, i, j) || alone(&pale_cells, i, j)) {
                ink[j][i] = true;
            }
        }
    }
    ink.iter_mut().for_each(|row| row.truncate(start + 1));
    Some(ink)
}

/// The line in `slot` as black ink on white, one font pixel to `zoom` square pixels, for OCR.
pub fn line_picture(frame: &Frame, layout: &Layout, slot: usize, zoom: u32) -> Option<Frame> {
    line_picture_at(frame, layout, slot, zoom, INK_LEVEL)
}

/// [`line_picture`] with the ink cut at `level` of the line's brightness.
pub fn line_picture_at(frame: &Frame, layout: &Layout, slot: usize, zoom: u32, level: f32) -> Option<Frame> {
    let ink = line_ink_at(frame, layout, slot, level)?;
    let start = ink[0].len().checked_sub(1)?;
    let margin = 4 * zoom;
    let (w, h) = ((start as u32 + 1) * zoom + 2 * margin, LINE_HEIGHT * zoom + 2 * margin);
    Some(Frame::from_fn(w, h, |x, y| {
        let on = x >= margin && y >= margin && x < w - margin && y < h - margin && {
            let column = start - ((x - margin) / zoom) as usize;
            ink[((y - margin) / zoom) as usize][column]
        };
        if on { Rgba([0, 0, 0, 255]) } else { Rgba([255, 255, 255, 255]) }
    }))
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// A made-up log line drawn the game's way: `glyphs` font pixels of ink (`true`), right-aligned
    /// at `right`, top at `top`, `scale` pixels to a font pixel, faded to `alpha` over `world`.
    pub fn draw(frame: &mut Frame, glyphs: &[Vec<bool>], right: u32, top: u32, scale: u32, alpha: f32, ink: [u8; 3]) {
        let width = glyphs[0].len() as u32;
        let blend = |under: Rgba<u8>, over: [u8; 3]| {
            let mix = |a: u8, b: u8| (f32::from(b) * alpha + f32::from(a) * (1.0 - alpha)).round() as u8;
            Rgba([mix(under[0], over[0]), mix(under[1], over[1]), mix(under[2], over[2]), 255])
        };
        // Shadow first, then ink over it.
        for (pass, colour) in [(1, [2, 2, 1]), (0, ink)] {
            for (j, row) in glyphs.iter().enumerate() {
                for (i, &on) in row.iter().enumerate() {
                    if !on {
                        continue;
                    }
                    let x0 = right - (width - i as u32) * scale + pass * scale;
                    let y0 = top + j as u32 * scale + pass * scale;
                    for y in y0..y0 + scale {
                        for x in x0..x0 + scale {
                            let under = *frame.get_pixel(x, y);
                            frame.put_pixel(x, y, blend(under, colour));
                        }
                    }
                }
            }
        }
    }

    /// Made-up glyph rows for line `id`, 9 font pixels tall and ending in a bracket-like stroke,
    /// so lines are right-aligned as the game's are.
    pub fn glyphs(id: u32, width: usize) -> Vec<Vec<bool>> {
        (0..9)
            .map(|y| {
                (0..width)
                    .map(|x| {
                        if x == width - 1 {
                            return (1..8).contains(&y);
                        }
                        if x % 6 == 5 || x == width - 2 {
                            return false;
                        }
                        let seed = id.wrapping_mul(2_654_435_761) ^ (x as u32 / 6).wrapping_mul(40_503);
                        (seed.rotate_left(((x % 6) * 9 + y) as u32 % 31) & 3) == 0 || (y == 0 || y == 8) && (seed >> (x % 6)) & 1 == 1
                    })
                    .collect()
            })
            .collect()
    }

    /// A busy world: blotches of brown and green.
    pub fn world(width: u32, height: u32) -> Frame {
        Frame::from_fn(width, height, |x, y| {
            let n = (x / 7 * 31 + y / 5 * 17 + (x * y) % 13) % 64;
            Rgba([60 + n as u8, 50 + (n / 2) as u8, 20 + (n / 3) as u8, 255])
        })
    }

    #[test]
    fn finds_the_log_from_a_bright_line() {
        let mut frame = world(800, 450);
        draw(&mut frame, &glyphs(1, 120), 790, 380, 2, 1.0, [246, 246, 245]);
        draw(&mut frame, &glyphs(2, 90), 790, 352, 2, 0.6, [246, 246, 245]);
        let layout = locate(&frame, FULL, (0, 0, 800, 450)).expect("found");
        assert_eq!((layout.scale, layout.right, layout.bottom), (2, 790, 380));
    }

    #[test]
    fn finds_nothing_in_a_world_without_a_log() {
        assert_eq!(locate(&world(800, 450), FULL, (0, 0, 800, 450)), None);
    }

    #[test]
    fn sees_each_row_and_how_far_it_has_faded() {
        let mut frame = world(800, 450);
        draw(&mut frame, &glyphs(1, 120), 790, 380, 2, 1.0, [246, 246, 245]);
        draw(&mut frame, &glyphs(2, 90), 790, 352, 2, 0.6, [246, 246, 245]);
        draw(&mut frame, &glyphs(3, 100), 790, 324, 2, 0.15, [246, 246, 245]);
        let rows = rows(&frame, &Layout { scale: 2, right: 790, bottom: 380, full: FULL });
        assert_eq!(rows.len(), SLOTS);
        let seen: Vec<Option<f32>> = rows.iter().map(|r| r.as_ref().map(|r| (r.brightness * 10.0).round() / 10.0)).collect();
        assert_eq!(seen, vec![Some(1.0), Some(0.6), None, None, None, None, None], "too faint, then empty");
    }

    #[test]
    fn a_row_keeps_its_shape_as_it_fades() {
        let layout = Layout { scale: 2, right: 790, bottom: 380, full: FULL };
        let mut bright = world(800, 450);
        draw(&mut bright, &glyphs(4, 100), 790, 380, 2, 1.0, [246, 246, 245]);
        let mut faded = world(800, 450);
        draw(&mut faded, &glyphs(4, 100), 790, 380, 2, 0.45, [246, 246, 245]);
        let (a, b) = (row(&bright, &layout, 0).unwrap(), row(&faded, &layout, 0).unwrap());
        assert!(a.shape.similarity(&b.shape) >= 0.9, "{}", a.shape.similarity(&b.shape));
        let mut other = world(800, 450);
        draw(&mut other, &glyphs(5, 100), 790, 380, 2, 1.0, [246, 246, 245]);
        assert!(a.shape.similarity(&row(&other, &layout, 0).unwrap().shape) < 0.8);
    }

    #[test]
    fn yellow_lines_are_told_from_white() {
        let layout = Layout { scale: 2, right: 790, bottom: 380, full: FULL };
        let mut frame = world(800, 450);
        draw(&mut frame, &glyphs(6, 100), 790, 380, 2, 1.0, [255, 255, 0]);
        draw(&mut frame, &glyphs(7, 100), 790, 352, 2, 1.0, [246, 246, 245]);
        let rows = rows(&frame, &layout);
        assert_eq!(rows[0].as_ref().map(|r| r.colour), Some(Colour::Yellow));
        assert_eq!(rows[1].as_ref().map(|r| r.colour), Some(Colour::White));
    }

    #[test]
    fn text_not_ending_at_the_logs_edge_is_no_row() {
        let layout = Layout { scale: 2, right: 790, bottom: 380, full: FULL };
        let mut frame = world(800, 450);
        draw(&mut frame, &glyphs(8, 100), 760, 380, 2, 1.0, [246, 246, 245]);
        assert_eq!(rows(&frame, &layout)[0], None);
    }

    #[test]
    fn the_picture_for_ocr_is_the_whole_line_in_black_on_white() {
        let layout = Layout { scale: 2, right: 790, bottom: 380, full: FULL };
        let mut frame = world(800, 450);
        let glyphs = glyphs(9, 150);
        draw(&mut frame, &glyphs, 790, 380, 2, 1.0, [246, 246, 245]);
        let picture = line_picture(&frame, &layout, 0, 3).unwrap();
        let (margin, zoom) = (12, 3);
        // The line's first column of ink is the picture's first after the margin.
        let first = (0..150).find(|&i| glyphs.iter().any(|r| r[i])).unwrap() as u32;
        assert_eq!(picture.width(), (150 - first) * zoom + 2 * margin);
        let black = picture.pixels().filter(|p| p.0[0] == 0).count() as u32;
        let ink = glyphs.iter().flatten().filter(|&&on| on).count() as u32;
        assert!(black >= ink * zoom * zoom * 9 / 10, "{black} of {}", ink * zoom * zoom);
    }
}
