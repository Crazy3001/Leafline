//! Lines of text read by matching their glyphs against the game's own fonts, not by OCR.
//!
//! The game draws much of its UI text in pixel fonts that look the same every time: at Ideal
//! Ratio each font pixel is a crisp 2 x 2 block, and a glyph never changes. So a line is cut into
//! glyphs at its blank columns and each takes the glyph of its width that it fits best, only when
//! it fits well and clearly better than any other. A glyph that fits none (one not seen yet, two
//! touching, scenery behind) is [`UNKNOWN`] rather than a guess; callers decide what a line with
//! unknowns is still good for, and OCR stays their fallback.
//!
//! The fonts' glyphs are the game's own, taken from frames of the game. The game's font glyphs were
//! allowed in Leafline on 2026-11-15; no other game art is kept. The Pickup Log's font has its
//! own reader in `pickup::font`, which this follows.

use image::Rgba;

use crate::vision::{Frame, Rect};

/// A pixel font: its glyphs, a row of font pixels per string and `#` for ink, all as tall as
/// [`Font::rows`].
pub struct Font {
    pub glyphs: &'static [(char, &'static [&'static str])],
    pub rows: usize,
    /// The row a capital's bottom sits on; rows below it hold descenders.
    pub baseline: usize,
    /// Blank columns from which a gap is a space between words.
    pub space: usize,
    /// Glyphs drawn narrower than the room the font gives them, and how many more blank columns
    /// follow them than follow other glyphs: a 1 in a digit's width.
    pub narrow: &'static [(char, usize)],
}

/// A glyph no known one fits.
pub const UNKNOWN: char = '?';
/// How much of a glyph's place must agree with it.
const FITS: f32 = 0.9;
/// How many more pixels it must agree on than with any other glyph of its width: i and l differ
/// by a few, so one pixel is enough to tell them apart, but not half of one.
const LEAD_PIXELS: f32 = 0.5;

/// The text of a line whose ink is `ink` (`ink[row][column]`, left to right, rows lined up with
/// the font's). Glyphs no known one fits are [`UNKNOWN`].
pub fn read_line(font: &Font, ink: &[Vec<bool>]) -> String {
    let columns = ink.first().map_or(0, Vec::len);
    let at = |x: usize, y: usize| ink.get(y).and_then(|row| row.get(x)).copied().unwrap_or(false);
    let inked = |x: usize| (0..font.rows).any(|y| at(x, y));
    let mut text = String::new();
    let mut x = 0;
    let mut last: Option<(usize, char)> = None;
    while x < columns {
        if !inked(x) {
            x += 1;
            continue;
        }
        let first = x;
        while x < columns && inked(x) {
            x += 1;
        }
        let glyph = best(font, &at, first, x - first).unwrap_or(UNKNOWN);
        if let Some((end, previous)) = last {
            let extra = font.narrow.iter().find(|(c, _)| *c == previous).map_or(0, |(_, extra)| *extra);
            let blank = first - end - 1;
            if blank >= font.space + extra {
                text.push(' ');
            }
        }
        last = Some((x - 1, glyph));
        text.push(glyph);
    }
    text
}

/// The glyph of `width` that fits columns `first..` best, when it fits well and clearly better
/// than any other of that width.
fn best(font: &Font, at: &impl Fn(usize, usize) -> bool, first: usize, width: usize) -> Option<char> {
    let mut scored: Vec<(f32, char)> = font
        .glyphs
        .iter()
        .filter(|(_, rows)| rows[0].len() == width)
        .map(|(c, rows)| {
            let mut agree = 0;
            for (y, row) in rows.iter().enumerate() {
                for (dx, pixel) in row.bytes().enumerate() {
                    agree += usize::from(at(first + dx, y) == (pixel == b'#'));
                }
            }
            (agree as f32, *c)
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    let (top, c) = *scored.first()?;
    let lead = scored.get(1).map_or(f32::MAX, |(next, _)| top - next);
    (top / (width * font.rows) as f32 >= FITS && lead >= LEAD_PIXELS).then_some(c)
}

/// `area` of `frame` in font pixels `scale` screen pixels wide, ink where `is_ink`: the grid's
/// phase is the one whose blocks are most nearly all ink or all not. `None` when the area is
/// empty or no phase gives whole blocks, as when the picture was resized.
pub fn ink_grid(frame: &Frame, area: Rect, scale: u32, is_ink: impl Fn(&Rgba<u8>) -> bool) -> Option<Vec<Vec<bool>>> {
    let (x0, y0, w, h) = area.to_pixels(frame.width(), frame.height());
    if w < 2 * scale || h < 2 * scale {
        return None;
    }
    let ink: Vec<Vec<bool>> =
        (y0..y0 + h).map(|y| (x0..x0 + w).map(|x| is_ink(frame.get_pixel(x, y))).collect()).collect();
    let mut best: Option<(usize, usize, Vec<Vec<bool>>)> = None;
    for oy in 0..scale as usize {
        for ox in 0..scale as usize {
            let s = scale as usize;
            let (rows, columns) = ((h as usize - oy) / s, (w as usize - ox) / s);
            let mut broken = 0;
            let grid: Vec<Vec<bool>> = (0..rows)
                .map(|r| {
                    (0..columns)
                        .map(|c| {
                            let block = (0..s).flat_map(|dy| (0..s).map(move |dx| (dy, dx)));
                            let on = block.clone().filter(|&(dy, dx)| ink[oy + r * s + dy][ox + c * s + dx]).count();
                            broken += usize::from(on != 0 && on != s * s);
                            on * 2 >= s * s
                        })
                        .collect()
                })
                .collect();
            if best.as_ref().is_none_or(|(least, ..)| broken < *least) {
                best = Some((broken, oy, grid));
            }
        }
    }
    let (broken, _, grid) = best?;
    let inked = grid.iter().flatten().filter(|&&on| on).count();
    // A few broken blocks are scenery's specks; many mean the picture isn't on the font's grid.
    (broken * 10 <= inked.max(1)).then_some(grid)
}

/// The rows of `grid` that hold the line of text nearest row `middle`, lined up with `font`'s
/// rows: the line's baseline is the bottom most of its glyphs share, since only descenders reach
/// below it. `None` when no ink is near.
pub fn line_rows(font: &Font, grid: &[Vec<bool>], middle: usize) -> Option<Vec<Vec<bool>>> {
    let inked_row = |y: usize| grid.get(y).is_some_and(|row| row.iter().any(|&on| on));
    // The run of inked rows at or nearest the middle.
    let start = (0..grid.len()).filter(|&y| inked_row(y)).min_by_key(|&y| y.abs_diff(middle))?;
    let (mut top, mut bottom) = (start, start);
    while top > 0 && inked_row(top - 1) {
        top -= 1;
    }
    while inked_row(bottom + 1) {
        bottom += 1;
    }
    // Each glyph's lowest row, by runs of inked columns within the line.
    let columns = grid[0].len();
    let inked_column = |x: usize| (top..=bottom).any(|y| grid[y][x]);
    let mut bottoms: Vec<usize> = Vec::new();
    let mut x = 0;
    while x < columns {
        if !inked_column(x) {
            x += 1;
            continue;
        }
        let first = x;
        while x < columns && inked_column(x) {
            x += 1;
        }
        bottoms.push((top..=bottom).rev().find(|&y| (first..x).any(|c| grid[y][c]))?);
    }
    let baseline = (top..=bottom).max_by_key(|&y| (bottoms.iter().filter(|&&b| b == y).count(), std::cmp::Reverse(y)))?;
    let first_row = baseline.checked_sub(font.baseline)?;
    Some(
        (first_row..first_row + font.rows)
            .map(|y| grid.get(y).cloned().unwrap_or_else(|| vec![false; columns]))
            .collect(),
    )
}

/// The columns of `rows` from its first ink up to a gap of `gap` blank columns or more: one
/// stretch of text, without what stands well off to its right.
pub fn first_stretch(rows: &[Vec<bool>], gap: usize) -> Vec<Vec<bool>> {
    let columns = rows.first().map_or(0, Vec::len);
    let inked = |x: usize| rows.iter().any(|row| row[x]);
    let Some(start) = (0..columns).find(|&x| inked(x)) else { return Vec::new() };
    let mut end = start;
    let mut blank = 0;
    for x in start..columns {
        if inked(x) {
            end = x;
            blank = 0;
        } else {
            blank += 1;
            if blank >= gap {
                break;
            }
        }
    }
    rows.iter().map(|row| row[start..=end].to_vec()).collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A made-up font: blocky 3 x 4 letters on a 5-row grid, the last row a descender.
    pub(crate) const MADE_UP: Font = Font {
        glyphs: &[
            ('1', &[".#", "##", ".#", ".#", ".."]),
            ('0', &["###", "#.#", "#.#", "###", "..."]),
            ('H', &["#.#", "###", "#.#", "#.#", "..."]),
            ('I', &["###", ".#.", ".#.", "###", "..."]),
            ('p', &["...", "###", "#.#", "###", "#.."]),
        ],
        rows: 5,
        baseline: 3,
        space: 3,
        narrow: &[('1', 1)],
    };

    /// `text` drawn in the made-up font: a blank column between glyphs, two more after a 1, and
    /// three for a space.
    pub(crate) fn drawn(text: &str) -> Vec<Vec<bool>> {
        let mut rows = vec![String::new(); MADE_UP.rows];
        let mut previous: Option<char> = None;
        for c in text.chars() {
            if c == ' ' {
                rows.iter_mut().for_each(|r| r.push_str("..."));
                continue;
            }
            let glyph = MADE_UP.glyphs.iter().find(|(g, _)| *g == c).expect("a made-up glyph").1;
            for (y, row) in rows.iter_mut().enumerate() {
                if previous.is_some() {
                    row.push('.');
                }
                if previous == Some('1') {
                    row.push('.');
                }
                row.push_str(glyph[y]);
            }
            previous = Some(c);
        }
        rows.iter().map(|r| r.chars().map(|c| c == '#').collect()).collect()
    }

    #[test]
    fn reads_glyphs_and_spaces() {
        assert_eq!(read_line(&MADE_UP, &drawn("10 HIp")), "10 HIp");
        assert_eq!(read_line(&MADE_UP, &drawn("101")), "101", "the wider room after a 1 is no space");
    }

    #[test]
    fn a_glyph_no_one_fits_is_unknown() {
        let mut ink = drawn("H0");
        for row in ink.iter_mut() {
            row[0] = true;
            row[1] = true;
            row[2] = true;
        }
        assert_eq!(read_line(&MADE_UP, &ink), "?0");
    }

    #[test]
    fn a_line_is_found_by_its_baseline() {
        // Two blank rows above, the line, a blank row, and part of another line below.
        let line = drawn("Hp");
        let width = line[0].len();
        let mut grid = vec![vec![false; width]; 2];
        grid.extend(line.iter().cloned());
        grid.push(vec![false; width]);
        grid.push(vec![true; width]);
        let rows = line_rows(&MADE_UP, &grid, 4).unwrap();
        assert_eq!(rows, line);
    }

    #[test]
    fn whole_blocks_make_the_grid() {
        // `H` at 2 x 2, one pixel off the frame's even grid.
        let glyph = ["#.#", "###", "#.#", "#.#"];
        let frame = Frame::from_fn(10, 10, |x, y| {
            let (c, r) = ((x as i32 - 1).div_euclid(2), (y as i32 - 1).div_euclid(2));
            let on = (0..3).contains(&c) && (0..4).contains(&r) && glyph[r as usize].as_bytes()[c as usize] == b'#';
            if on { Rgba([0, 0, 0, 255]) } else { Rgba([255, 255, 255, 255]) }
        });
        let grid = ink_grid(&frame, Rect::new(0.0, 0.0, 10.0, 10.0), 2, |px| px.0[0] == 0).unwrap();
        assert_eq!(&grid[0][..3], &[true, false, true]);
        assert_eq!(&grid[1][..3], &[true, true, true]);
    }
}
