//! The numbers the game draws on items: how many sit on a quick slot or in an inventory slot.
//!
//! They are drawn in a pixel font, white shading to light blue towards the bottom, with a pure
//! black outline all round, in whole pixels of the window (each font pixel a square of `cell`
//! window pixels, 2 at 1440p). The outline is what tells them from the item pictures under them,
//! which are rarely outlined in pure black on every side. Digits are read by the shape rules in
//! `digits`, so no font is kept.

use super::digits;
use crate::vision::glyphs::Mask;
use crate::vision::{Frame, Rect};

/// A count's digits are this many font pixels tall.
pub const DIGIT_ROWS: u32 = 9;
/// Digits of one number stand at most this many font pixels apart.
const DIGIT_GAP: u32 = 3;
/// Channels at or below this are the outline's black.
const OUTLINE: u8 = 12;

/// A number read from the game's item counts, and where it stands in the window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Count {
    pub value: u32,
    /// Window pixels: left, top, right and bottom edges, the last two exclusive.
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

impl Count {
    pub fn rect(&self) -> Rect {
        Rect::new(self.left as f32, self.top as f32, (self.right - self.left) as f32, (self.bottom - self.top) as f32)
    }
}

/// The digits' fill: white at the top, shading to a light blue.
fn is_fill(frame: &Frame, x: u32, y: u32) -> bool {
    let [r, g, b, _] = frame.get_pixel(x, y).0;
    (r, g, b) == (255, 255, 255) || (b >= 250 && g >= 150 && r >= 140 && b - r >= 8)
}

fn is_outline(frame: &Frame, x: u32, y: u32) -> bool {
    frame.get_pixel(x, y).0[..3].iter().all(|&c| c <= OUTLINE)
}

/// One outlined shape: its bounding box and its fill.
struct Glyph {
    left: u32,
    top: u32,
    width: u32,
    height: u32,
    mask: Mask,
}

/// Outlined shapes in `left..right`, `top..bottom` of the window: runs of fill joined at edges or
/// corners, with nothing but fill or black beside them on any side.
fn glyphs(frame: &Frame, (left, top, right, bottom): (u32, u32, u32, u32)) -> Vec<Glyph> {
    let (right, bottom) = (right.min(frame.width()), bottom.min(frame.height()));
    if left >= right || top >= bottom {
        return Vec::new();
    }
    let (w, h) = (right - left, bottom - top);
    let at = |x: u32, y: u32| (y * w + x) as usize;
    let fill: Vec<bool> = (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| is_fill(frame, left + x, top + y)).collect();
    let mut seen = vec![false; (w * h) as usize];
    let mut found = Vec::new();
    for start in 0..w * h {
        let (sx, sy) = (start % w, start / w);
        if seen[at(sx, sy)] || !fill[at(sx, sy)] {
            continue;
        }
        seen[at(sx, sy)] = true;
        let mut stack = vec![(sx, sy)];
        let mut points = Vec::new();
        let mut closed = true;
        while let Some((x, y)) = stack.pop() {
            points.push((x, y));
            for (dx, dy) in [(-1i32, -1i32), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                let straight = dx == 0 || dy == 0;
                if nx < 0 || ny < 0 || nx >= w as i32 || ny >= h as i32 {
                    closed &= !straight;
                    continue;
                }
                let (nx, ny) = (nx as u32, ny as u32);
                if fill[at(nx, ny)] {
                    if !seen[at(nx, ny)] {
                        seen[at(nx, ny)] = true;
                        stack.push((nx, ny));
                    }
                } else if straight && !is_outline(frame, left + nx, top + ny) {
                    closed = false;
                }
            }
        }
        if !closed {
            continue;
        }
        let (min_x, max_x) = points.iter().fold((u32::MAX, 0), |(lo, hi), &(x, _)| (lo.min(x), hi.max(x)));
        let (min_y, max_y) = points.iter().fold((u32::MAX, 0), |(lo, hi), &(_, y)| (lo.min(y), hi.max(y)));
        let (gw, gh) = (max_x - min_x + 1, max_y - min_y + 1);
        let mut ink = vec![false; (gw * gh) as usize];
        for &(x, y) in &points {
            ink[((y - min_y) * gw + (x - min_x)) as usize] = true;
        }
        let mask = Mask::from_fn(gw, gh, |x, y| ink[(y * gw + x) as usize]);
        found.push(Glyph { left: left + min_x, top: top + min_y, width: gw, height: gh, mask });
    }
    found
}

/// Every count in `area` of the window whose digits are `cell` window pixels to a font pixel, or
/// at any size when `cell` is `None`. A number with a digit no rule fits is left out.
pub fn read(frame: &Frame, area: (u32, u32, u32, u32), cell: Option<u32>) -> Vec<Count> {
    let mut digits: Vec<Glyph> = glyphs(frame, area)
        .into_iter()
        .filter(|g| g.height.is_multiple_of(DIGIT_ROWS))
        .filter(|g| cell.is_none_or(|c| g.height == DIGIT_ROWS * c))
        .collect();
    digits.sort_by_key(|g| (g.top, g.left));
    let mut counts = Vec::new();
    let mut used = vec![false; digits.len()];
    for first in 0..digits.len() {
        if used[first] {
            continue;
        }
        let cell = digits[first].height / DIGIT_ROWS;
        let mut number = vec![first];
        used[first] = true;
        loop {
            let last = &digits[*number.last().unwrap()];
            let next = (0..digits.len()).find(|&i| {
                let g = &digits[i];
                !used[i]
                    && g.top == last.top
                    && g.height == last.height
                    && g.left > last.left
                    && g.left <= last.left + last.width + DIGIT_GAP * cell
            });
            match next {
                Some(i) => {
                    used[i] = true;
                    number.push(i);
                }
                None => break,
            }
        }
        let masks: Vec<Mask> = number.iter().map(|&i| digits[i].mask.clone()).collect();
        let Some(value) = digits::count_number(&masks).and_then(|text| text.parse().ok()) else { continue };
        let (head, tail) = (&digits[number[0]], &digits[*number.last().unwrap()]);
        counts.push(Count { value, left: head.left, top: head.top, right: tail.left + tail.width, bottom: head.top + head.height });
    }
    counts
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use image::Rgba;

    /// Made-up count digits, square-cornered (see `digits`), drawn in the game's way: fill
    /// shading from white to light blue, a black outline round every side.
    pub fn shape(c: char) -> [&'static str; 9] {
        match c {
            '0' => ["#####", "#...#", "#...#", "#...#", "#...#", "#...#", "#...#", "#...#", "#####"],
            '1' => ["..#", ".##", "..#", "..#", "..#", "..#", "..#", "..#", "..#"],
            '2' => ["#####", "#...#", "....#", "....#", "...#.", "..#..", ".#...", "#....", "#####"],
            '3' => ["####.", "....#", "....#", "....#", ".###.", "....#", "....#", "....#", "####."],
            '4' => ["...#.", "..##.", ".#.#.", "#..#.", "#..#.", "#####", "...#.", "...#.", "...#."],
            '5' => ["#####", "#....", "#....", "####.", "....#", "....#", "....#", "#...#", "####."],
            '6' => ["..##.", ".#...", "#....", "####.", "#...#", "#...#", "#...#", "#...#", "####."],
            '7' => ["#####", "....#", "...#.", "...#.", "..#..", "..#..", ".#...", ".#...", ".#..."],
            '8' => ["#####", "#...#", "#...#", "#...#", "#####", "#...#", "#...#", "#...#", "#####"],
            _ => ["#####", "#...#", "#...#", "#...#", "#...#", ".####", "....#", "....#", "#####"],
        }
    }

    /// Draws `text` at `(x, y)` in made-up count digits, `cell` window pixels to a font pixel.
    pub fn draw(frame: &mut Frame, text: &str, x: u32, y: u32, cell: u32) {
        let mut left = x;
        for c in text.chars() {
            let rows = shape(c);
            let width = rows[0].len() as u32;
            let ink = |fx: i32, fy: i32| {
                fx >= 0
                    && fy >= 0
                    && (fx as u32) < width
                    && fy < 9
                    && rows[fy as usize].as_bytes()[fx as usize] == b'#'
            };
            for fy in -1..10i32 {
                for fx in -1..width as i32 + 1 {
                    let near = [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|&(dx, dy)| ink(fx + dx, fy + dy));
                    let colour = if ink(fx, fy) {
                        let shade = fy as u8;
                        Rgba([255 - shade * 12, 255 - shade * 6, 255, 255])
                    } else if near {
                        Rgba([0, 0, 0, 255])
                    } else {
                        continue;
                    };
                    for py in 0..cell {
                        for px in 0..cell {
                            let (wx, wy) = (left as i32 + fx * cell as i32 + px as i32, y as i32 + fy * cell as i32 + py as i32);
                            if wx >= 0 && wy >= 0 && (wx as u32) < frame.width() && (wy as u32) < frame.height() {
                                frame.put_pixel(wx as u32, wy as u32, colour);
                            }
                        }
                    }
                }
            }
            left += (width + 2) * cell;
        }
    }

    #[test]
    fn outlined_counts_are_read_and_white_picture_bits_are_not() {
        let mut frame = Frame::from_pixel(200, 80, Rgba([248, 249, 250, 255]));
        draw(&mut frame, "161", 10, 10, 2);
        draw(&mut frame, "45", 120, 10, 1);
        // A white patch with no outline, as an item picture might have.
        for y in 50..70 {
            for x in 20..40 {
                frame.put_pixel(x, y, Rgba([255, 255, 255, 255]));
            }
        }
        let counts = read(&frame, (0, 0, 200, 80), None);
        let values: Vec<u32> = counts.iter().map(|c| c.value).collect();
        assert_eq!(values, [161, 45]);
        assert_eq!(read(&frame, (0, 0, 200, 80), Some(2)).len(), 1, "a cell size keeps to that size");
    }
}
