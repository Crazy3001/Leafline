//! Whole Pickup Log lines read by matching their glyphs against the game's own, not by OCR.
//!
//! The log's pixel font draws each letter the same every time, a blank column or two apart and
//! about five apart between words. A line is cut into glyphs at its blank columns, and each takes
//! the letter it fits best, only when it fits well and clearly better than any other of its width.
//! A glyph that fits none (a letter not seen yet, two letters touching, the world behind it) leaves
//! the whole line to OCR rather than guessing.
//!
//! The letters below are the game's, taken from recordings of the log; the game's font glyphs
//! were allowed in Leafline on 2026-11-15. Letters not seen yet are left to OCR.

use super::figure;

/// The log font's letters and marks, a row per font pixel, `#` for ink. The digits, the plus and
/// the brackets are in [`figure`].
const LETTERS: [(char, [&str; 11]); N] = [
    ('.', [".", ".", ".", ".", ".", ".", ".", ".", "#", ".", "."]),
    ('B', ["#####.", "#....#", "#....#", "#....#", "######", "#....#", "#....#", "#....#", "#####.", "......", "......"]),
    ('C', ["..###..", ".#...#.", "#.....#", "#......", "#......", "#......", "#.....#", ".#...#.", "..###..", ".......", "......."]),
    ('E', ["######", "#.....", "#.....", "#.....", "######", "#.....", "#.....", "#.....", "######", "......", "......"]),
    ('L', ["#.....", "#.....", "#.....", "#.....", "#.....", "#.....", "#.....", "#.....", "######", "......", "......"]),
    ('M', ["#.....#", "##...##", "##...##", "#.#.#.#", "#.#.#.#", "#.#.#.#", "#.#.#.#", "#..#..#", "#..#..#", ".......", "......."]),
    ('P', ["#####.", "#....#", "#....#", "#....#", "#####.", "#.....", "#.....", "#.....", "#.....", "......", "......"]),
    ('R', ["######.", "#.....#", "#.....#", "#.....#", "######.", "#...#..", "#....#.", "#....#.", "#.....#", ".......", "......."]),
    ('S', [".####.", "#....#", "#....#", "#.....", ".####.", ".....#", "#....#", "#....#", ".####.", "......", "......"]),
    ('W', ["#....#....#", "#...#.#...#", "#...#.#..#.", ".#..#.#..#.", ".#.#...#.#.", ".#.#...#.#.", ".#.#...#.#.", "..#.....#..", "..#.....#..", "...........", "..........."]),
    ('X', ["#.....#", ".#...#.", ".#...#.", "..#.#..", "...#...", "..#.#..", ".#...#.", ".#...#.", "#.....#", ".......", "......."]),
    ('Y', ["#.....#", ".#...#.", ".#...#.", "..#.#..", "...#...", "...#...", "...#...", "...#...", "...#...", ".......", "......."]),
    ('a', [".....", ".....", ".###.", "#...#", "....#", ".####", "#...#", "#..##", ".##.#", ".....", "....."]),
    ('c', ["....", "....", ".##.", "#..#", "#...", "#...", "#...", "#..#", ".##.", "....", "...."]),
    ('d', ["....#", "....#", ".##.#", "#..##", "#...#", "#...#", "#...#", "#...#", ".####", ".....", "....."]),
    ('e', [".....", ".....", ".###.", "#...#", "#...#", "#####", "#....", "#...#", ".###.", ".....", "....."]),
    ('g', [".....", ".....", ".##.#", "#..##", "#...#", "#...#", "#...#", "#..##", ".##.#", "....#", "#...."]),
    ('h', ["#....", "#....", "#.##.", "##..#", "#...#", "#...#", "#...#", "#...#", "#...#", ".....", "....."]),
    ('i', ["#", ".", "#", "#", "#", "#", "#", "#", "#", ".", "."]),
    ('k', ["#....", "#....", "#...#", "#..#.", "#.#..", "###..", "#..#.", "#..#.", "#...#", ".....", "....."]),
    ('l', ["#", "#", "#", "#", "#", "#", "#", "#", "#", ".", "."]),
    ('m', [".........", ".........", "#.##..##.", "##..##..#", "#...#...#", "#...#...#", "#...#...#", "#...#...#", "#...#...#", ".........", "........."]),
    ('n', [".....", ".....", "#.##.", "##..#", "#...#", "#...#", "#...#", "#...#", "#...#", ".....", "....."]),
    ('o', [".....", ".....", ".###.", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.", ".....", "....."]),
    ('p', [".....", ".....", "#.##.", "##..#", "#...#", "#...#", "#...#", "##..#", "#.##.", "#....", "#...."]),
    ('q', [".....", ".....", ".##.#", "#..##", "#...#", "#...#", "#...#", "#..##", ".##.#", "....#", "....#"]),
    ('r', ["...", "...", "#.#", "##.", "#..", "#..", "#..", "#..", "#..", "...", "..."]),
    ('s', [".....", ".....", ".###.", "#...#", "#....", ".###.", "....#", "#...#", ".###.", ".....", "....."]),
    ('t', [".#.", ".#.", "###", ".#.", ".#.", ".#.", ".#.", ".#.", ".##", "...", "..."]),
    ('u', [".....", ".....", "#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".####", ".....", "....."]),
    ('v', [".....", ".....", "#...#", "#...#", ".#.#.", ".#.#.", ".#.#.", "..#..", "..#..", ".....", "....."]),
    ('w', [".........", ".........", "#...#...#", "#...#...#", ".#.#.#.#.", ".#.#.#.#.", ".#.#.#.#.", "..#...#..", "..#...#..", ".........", "........."]),
    ('x', [".....", ".....", "#...#", ".#.#.", ".#.#.", "..#..", ".#.#.", ".#.#.", "#...#", ".....", "....."]),
];
const N: usize = 33;

/// How much of a glyph must agree with a letter.
const FITS: f32 = 0.88;
/// How many more pixels it must agree on than with any other letter of its width: i and l differ
/// by one, g and q or h and n by two.
const LEAD_PIXELS: f32 = 0.5;
/// Blank columns from which a gap is a space between words.
const SPACE: usize = 4;
/// A glyph no letter fits is written as this, for name matching to take as any one letter.
pub const UNKNOWN: char = '?';
/// The most of a line's glyphs that may be unknown before the line is left to OCR.
const MOST_UNKNOWN: f32 = 0.25;

/// Every glyph the reader knows: the letters and the figure's glyphs.
fn glyphs() -> impl Iterator<Item = (char, &'static [&'static str; 11])> {
    LETTERS.iter().map(|(c, rows)| (*c, rows)).chain(figure::GLYPHS.iter().map(|(c, rows)| (*c, rows)))
}

/// The text of a line whose ink is `ink` (`ink[row][column]`, column 0 at the right). A glyph no
/// known letter fits (one not seen yet, or spoiled by the world behind it) is [`UNKNOWN`]; with
/// more than a quarter of them unknown, the line is left to OCR.
pub fn read_line(ink: &[Vec<bool>]) -> Option<String> {
    let columns = ink.first()?.len();
    // Screen order, left to right.
    let at = |x: usize, y: usize| ink.get(y).and_then(|row| row.get(columns - 1 - x)).copied().unwrap_or(false);
    let inked = |x: usize| (0..ink.len()).any(|y| at(x, y));
    let mut text = String::new();
    let mut x = 0;
    let mut last_end: Option<usize> = None;
    while x < columns {
        if !inked(x) {
            x += 1;
            continue;
        }
        let first = x;
        while x < columns && inked(x) {
            x += 1;
        }
        let width = x - first;
        if let Some(end) = last_end
            && first - end > SPACE
        {
            text.push(' ');
        }
        last_end = Some(x - 1);
        text.push(best(&at, first, width).unwrap_or(UNKNOWN));
    }
    let glyphs = text.chars().filter(|c| *c != ' ').count();
    let unknown = text.chars().filter(|c| *c == UNKNOWN).count();
    (glyphs > 0 && unknown as f32 <= MOST_UNKNOWN * glyphs as f32).then_some(text)
}

/// The glyph of `width` that fits columns `first..` best, when it fits well and clearly better
/// than any other of that width.
fn best(at: &impl Fn(usize, usize) -> bool, first: usize, width: usize) -> Option<char> {
    let mut scored: Vec<(f32, char)> = glyphs()
        .filter(|(_, rows)| rows[0].len() == width)
        .map(|(c, rows)| {
            let mut agree = 0;
            for (y, row) in rows.iter().enumerate() {
                for (dx, pixel) in row.chars().enumerate() {
                    agree += usize::from(at(first + dx, y) == (pixel == '#'));
                }
            }
            (agree as f32, c)
        })
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    let (top, c) = *scored.first()?;
    let lead = scored.get(1).map_or(f32::MAX, |(next, _)| top - next);
    (top / (width * 11) as f32 >= FITS && lead >= LEAD_PIXELS).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line drawn from the glyphs, left to right, a blank column between letters and five
    /// between words.
    fn drawn(text: &str) -> Vec<Vec<bool>> {
        let glyph = |c: char| glyphs().find(|(g, _)| *g == c).map(|(_, rows)| rows).expect("a known glyph");
        let mut rows = vec![String::new(); 11];
        for (i, word) in text.split(' ').enumerate() {
            for (j, c) in word.chars().enumerate() {
                for (y, row) in rows.iter_mut().enumerate() {
                    if i > 0 && j == 0 {
                        row.push_str(".....");
                    } else if j > 0 {
                        row.push('.');
                    }
                    row.push_str(glyph(c)[y]);
                }
            }
        }
        rows.iter().map(|row| row.chars().rev().map(|c| c == '#').collect()).collect()
    }

    #[test]
    fn reads_a_whole_line() {
        let line = "You have gained mesos (+27)";
        assert_eq!(read_line(&drawn(line)).as_deref(), Some(line));
        let item = "Red Snail Shell x1 earned. (Etc)";
        assert_eq!(read_line(&drawn(item)).as_deref(), Some(item));
    }

    #[test]
    fn a_letter_not_known_leaves_the_line_to_ocr() {
        let mut ink = drawn("Slate x1");
        // A blot no letter looks like, in place of the S.
        let columns = ink[0].len();
        for row in ink.iter_mut() {
            for x in 0..6 {
                row[columns - 1 - x] = true;
            }
        }
        assert_eq!(read_line(&ink).as_deref(), Some("?late x1"), "one unknown glyph stands as a gap");
        for row in ink.iter_mut() {
            row.iter_mut().for_each(|c| *c = !*c);
        }
        assert_eq!(read_line(&ink), None, "a line of nothing but unknowns is left to OCR");
    }
}
