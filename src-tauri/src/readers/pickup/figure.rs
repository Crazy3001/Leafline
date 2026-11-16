//! A line's figure read by matching its glyphs against the game's own, not by OCR.
//!
//! Every figure the log shows ends a line as `(+27)`: a closing bracket at the log's right edge,
//! the digits, a plus and an opening bracket. They are drawn in a pixel font, nine font pixels
//! tall, the same every time. OCR reads that font poorly (`(+10)` as `(+113)`, a 9 as a `g`), so
//! the glyphs are matched against the font's own, below: from the bracket leftwards, each place
//! takes the glyph that fits best, and only when it fits well and clearly better than any other.
//! A glyph that fits none, or two alike, leaves the figure to OCR rather than guessing.
//!
//! The glyphs below are the game's, taken from recordings of the log. The game's number glyphs
//! were allowed in Leafline on 2026-11-15; no other game art is kept.

use super::parse::{Gain, Parsed};

/// The figure's glyphs as the game draws them, a row per font pixel, `#` for ink. Rows below
/// the ninth are where only the brackets reach.
pub(super) const GLYPHS: [(char, [&str; 11]); 13] = [
    ('0', [".###.", "#...#", "#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###.", ".....", "....."]),
    ('1', ["..#", ".##", "#.#", "..#", "..#", "..#", "..#", "..#", "..#", "...", "..."]),
    ('2', [".###.", "#...#", "....#", "....#", "...#.", "...#.", "..#..", ".#...", "#####", ".....", "....."]),
    ('3', [".###.", "#...#", "....#", "....#", "..##.", "....#", "....#", "#...#", ".###.", ".....", "....."]),
    ('4', ["...#.", "..##.", "..##.", ".#.#.", ".#.#.", "#..#.", "#####", "...#.", "...#.", ".....", "....."]),
    ('5', [".####", ".#...", "#....", "####.", "#...#", "....#", "....#", "#...#", ".###.", ".....", "....."]),
    ('6', [".###.", "#...#", "#....", "#.##.", "##..#", "#...#", "#...#", "#...#", ".###.", ".....", "....."]),
    ('7', ["#####", "...#.", "...#.", "..#..", "..#..", "..#..", ".#...", ".#...", ".#...", ".....", "....."]),
    ('8', [".###.", "#...#", "#...#", "#...#", ".###.", "#...#", "#...#", "#...#", ".###.", ".....", "....."]),
    ('9', [".###.", "#...#", "#...#", "#...#", "#..##", ".##.#", "....#", "#...#", ".###.", ".....", "....."]),
    ('+', [".....", ".....", "..#..", "..#..", "#####", "..#..", "..#..", ".....", ".....", ".....", "....."]),
    ('(', ["..#", ".#.", ".#.", "#..", "#..", "#..", "#..", "#..", ".#.", ".#.", "..#"]),
    (')', ["#..", ".#.", ".#.", "..#", "..#", "..#", "..#", "..#", ".#.", ".#.", "#.."]),
];

/// How much of a glyph's place must agree with it, and by how much more than any other glyph.
const FITS: f32 = 0.88;
const LEAD: f32 = 0.05;
/// The most digits a figure has.
const MOST_DIGITS: usize = 9;
/// How wide each fixed line's words are before its figure, in font pixels. The font draws the
/// same words the same width every time, so the width alone tells the lines apart (measured on
/// the recordings of 2026-11-15: 117 of 121 EXP lines at 99, 67 of 76 meso lines at 136; wider
/// readings had the world behind the line's start taken for ink).
const WORD_WIDTHS: [(usize, Fixed); 3] = [(99, Fixed::Exp), (123, Fixed::PartyBonus), (136, Fixed::Mesos)];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fixed {
    Exp,
    PartyBonus,
    Mesos,
}

/// A figure read by its glyphs, and how wide the words before it are.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Figure {
    pub amount: u64,
    /// Font pixels from the line's start to the figure's opening bracket.
    pub words: usize,
}

impl Figure {
    /// The line this figure ends, told by the width of its words alone: no OCR needed.
    pub fn line(&self) -> Option<Parsed> {
        let (_, kind) = WORD_WIDTHS.iter().find(|(width, _)| self.words.abs_diff(*width) <= 1)?;
        let amount = self.amount;
        let gain = match kind {
            Fixed::Exp => Gain::Exp { amount },
            Fixed::PartyBonus => Gain::PartyBonus { amount },
            Fixed::Mesos => Gain::Mesos { amount },
        };
        Some(Parsed { gain, sure: true })
    }
}

/// The amount in the figure that ends a line whose ink is `ink` (`ink[row][column]`, column 0 at
/// the right), when every glyph matches.
pub fn read(ink: &[Vec<bool>]) -> Option<u64> {
    figure(ink).map(|f| f.amount)
}

/// [`read`], with the width of the words before the figure.
pub fn figure(ink: &[Vec<bool>]) -> Option<Figure> {
    let columns = ink.first()?.len();
    let inked = |i: usize| ink.iter().any(|row| row.get(i).copied().unwrap_or(false));
    // A narrow 1 is drawn in a wider space: up to four blank columns lie between glyphs.
    let next_ink = |from: usize| (from..(from + 5).min(columns)).find(|&i| inked(i));
    // The closing bracket, at the right edge.
    let mut at = next_ink(0)?;
    if best(ink, at, &[')'])? != ')' {
        return None;
    }
    at = next_ink(at + width(')'))?;
    let mut digits = Vec::new();
    loop {
        let glyph = best(ink, at, &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '+'])?;
        at += width(glyph);
        if glyph == '+' {
            break;
        }
        digits.push(glyph);
        if digits.len() > MOST_DIGITS {
            return None;
        }
        at = next_ink(at)?;
    }
    if digits.is_empty() {
        return None;
    }
    // Read right to left.
    let amount = digits.iter().rev().collect::<String>().parse().ok()?;
    let open = next_ink(at)?;
    if best(ink, open, &['('])? != '(' {
        return None;
    }
    // The words end a space before the opening bracket's left edge.
    let words = (columns - 1).checked_sub(open + width('(') - 1)?;
    Some(Figure { amount, words })
}

fn glyph(c: char) -> &'static [&'static str; 11] {
    &GLYPHS.iter().find(|(g, _)| *g == c).expect("a glyph of the figure").1
}

fn width(c: char) -> usize {
    glyph(c)[0].len()
}

/// How much of the place whose right edge is column `at` agrees with glyph `c`, 0 to 1.
fn fit(ink: &[Vec<bool>], at: usize, c: char) -> f32 {
    let rows = glyph(c);
    let w = rows[0].len();
    let mut agree = 0;
    let mut total = 0;
    for (y, row) in rows.iter().enumerate().take(ink.len()) {
        for (x, pixel) in row.chars().enumerate() {
            // Column `x` from the glyph's left is `w - 1 - x` from its right edge.
            let on = ink[y].get(at + w - 1 - x).copied().unwrap_or(false);
            agree += usize::from(on == (pixel == '#'));
            total += 1;
        }
    }
    agree as f32 / total.max(1) as f32
}

/// The glyph of `among` that fits the place at `at`, when it fits well and clearly better than
/// the others; with one candidate, when it fits well.
fn best(ink: &[Vec<bool>], at: usize, among: &[char]) -> Option<char> {
    let mut scored: Vec<(f32, char)> = among.iter().map(|&c| (fit(ink, at, c), c)).collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0));
    let (top, c) = scored[0];
    let lead = scored.get(1).map_or(1.0, |(next, _)| top - next);
    (top >= FITS && lead >= LEAD).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line's right end drawn from the glyphs, left to right as on screen, with a column
    /// between glyphs, after `words` columns of made-up words.
    fn drawn(text: &str, words: usize) -> Vec<Vec<bool>> {
        let mut rows: Vec<String> = vec![String::new(); 11];
        for (y, row) in rows.iter_mut().enumerate() {
            row.push_str(&(0..words).map(|x| if (x + y) % 3 == 0 { '#' } else { '.' }).collect::<String>());
            row.push('.');
        }
        for (i, c) in text.chars().enumerate() {
            for (y, row) in rows.iter_mut().enumerate() {
                if i > 0 {
                    row.push('.');
                }
                row.push_str(glyph(c)[y]);
            }
        }
        rows.iter().map(|row| row.chars().rev().map(|c| c == '#').collect()).collect()
    }

    #[test]
    fn reads_every_digit() {
        for n in [0u64, 7, 10, 23, 45, 68, 91, 1234, 56789] {
            let ink = drawn(&format!("(+{n})"), 40);
            assert_eq!(read(&ink), Some(n), "{n}");
        }
    }

    #[test]
    fn measures_the_words_before_the_figure() {
        let ink = drawn("(+27)", 99);
        let figure = figure(&ink).unwrap();
        assert_eq!(figure.words, 100, "the words and the space after them");
    }

    #[test]
    fn a_pixel_or_two_lost_still_reads() {
        // From the right: `)` in columns 0-2, the 8 in 4-8, the 3 in 10-14.
        let mut ink = drawn("(+38)", 20);
        ink[0][6] = false;
        ink[4][11] = false;
        assert_eq!(read(&ink), Some(38));
    }

    #[test]
    fn a_glyph_that_fits_none_leaves_the_figure_unread() {
        let mut ink = drawn("(+38)", 20);
        // Blot out the 8.
        for row in ink.iter_mut() {
            row[4..9].iter_mut().for_each(|c| *c = true);
        }
        assert_eq!(read(&ink), None);
    }

    #[test]
    fn a_narrow_one_in_a_wide_space_still_reads() {
        // `(+11)` as the game spaces it: blank columns around each 1 (from a recording).
        let rows = [
            ".....#..........#......#...#..",
            "....#..........##.....##....#.",
            "....#....#....#.#....#.#....#.",
            "...#.....#......#......#.....#",
            "...#...#####....#......#.....#",
            "...#.....#......#......#.....#",
            "...#.....#......#......#.....#",
            "...#............#......#.....#",
            "....#...........#......#....#.",
            "....#.......................#.",
            ".....#.....................#..",
        ];
        let ink: Vec<Vec<bool>> = rows.iter().map(|r| r.chars().rev().map(|c| c == '#').collect()).collect();
        assert_eq!(read(&ink), Some(11));
    }

    #[test]
    fn brackets_without_a_figure_are_no_figure() {
        assert_eq!(read(&drawn("()", 20)), None);
    }

    #[test]
    fn the_width_of_the_words_tells_the_fixed_lines_apart() {
        let line = |words| Figure { amount: 8, words }.line().map(|p| p.gain);
        assert_eq!(line(99), Some(Gain::Exp { amount: 8 }));
        assert_eq!(line(124), Some(Gain::PartyBonus { amount: 8 }));
        assert_eq!(line(136), Some(Gain::Mesos { amount: 8 }));
        assert_eq!(line(110), None, "an item's name, or the world behind");
    }
}
