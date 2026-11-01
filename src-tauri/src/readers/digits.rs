//! Reads the HUD's numbers by rules about their shape, with nothing to learn and no font data.
//!
//! The numbers use a tiny pixel font that general OCR misreads, but the digits are simple
//! shapes, seven pixels of the font tall, that a few questions tell apart: how many closed holes
//! a digit has, whether its top or bottom row is solid, and which sides have ink partway down.
//! The level's bolder digits are the same shapes drawn thicker, so the same questions hold.
//!
//! Anything that fits no rule is left unread rather than guessed: one wrong digit would spoil
//! a number, and a missing reading is simply skipped.

use crate::vision::glyphs::Mask;

/// How a font's digits are laid out, in pixels of the font. Masks arrive at twice this on the
/// design grid.
#[derive(Clone, Copy)]
struct Font {
    /// How tall a character is.
    rows: u32,
    /// Rows of the upper and lower part of a digit, where the sides tell digits apart.
    upper: u32,
    lower: u32,
}

/// The HUD's numbers, plain and bold.
const HUD: Font = Font { rows: 7, upper: 2, lower: 4 };
/// The taller, rounder numbers in windows such as Character Stat.
const WINDOW: Font = Font { rows: 9, upper: 2, lower: 6 };

/// The same round numbers a row shorter, in the Quest Helper.
const QUEST: Font = Font { rows: 8, upper: 2, lower: 5 };

/// A digit no wider than this is a 1, which is narrow in every font.
const ONE_WIDTH: u32 = 3;
/// The value font's digits are narrower than this, and its `%` isn't.
const PERCENT_WIDTH: u32 = 6;

/// A mask reduced to pixels of the font.
struct Shape<'a> {
    mask: &'a Mask,
    font: Font,
    cell: u32,
    width: u32,
}

impl<'a> Shape<'a> {
    fn new(mask: &'a Mask, font: Font) -> Option<Self> {
        let cell = mask.height() / font.rows;
        (cell > 0 && mask.height().is_multiple_of(font.rows) && mask.width().is_multiple_of(cell)).then(|| Self {
            mask,
            font,
            cell,
            width: mask.width() / cell,
        })
    }

    fn ink(&self, x: u32, y: u32) -> bool {
        self.mask.get(x * self.cell, y * self.cell)
    }

    fn left(&self, y: u32) -> bool {
        self.ink(0, y)
    }

    fn right(&self, y: u32) -> bool {
        self.ink(self.width - 1, y)
    }

    /// How much ink row `y` has.
    fn row(&self, y: u32) -> u32 {
        (0..self.width).filter(|&x| self.ink(x, y)).count() as u32
    }

    fn solid(&self, y: u32) -> bool {
        self.row(y) + 1 >= self.width
    }

    fn sparse(&self, y: u32) -> bool {
        self.row(y) * 2 <= self.width
    }

    /// The first row with ink.
    fn top(&self) -> Option<u32> {
        (0..self.font.rows).find(|&y| self.row(y) > 0)
    }

    /// How many blank areas the ink closes off from the edge.
    fn holes(&self) -> u32 {
        let (width, height) = (self.width as i32, self.font.rows as i32);
        let at = |x: i32, y: i32| (y * width + x) as usize;
        let mut seen = vec![false; (width * height) as usize];
        let mut holes = 0;
        for start in 0..width * height {
            let (x, y) = (start % width, start / width);
            if seen[at(x, y)] || self.ink(x as u32, y as u32) {
                continue;
            }
            let mut closed = true;
            let mut area = vec![(x, y)];
            seen[at(x, y)] = true;
            while let Some((x, y)) = area.pop() {
                closed &= x > 0 && y > 0 && x < width - 1 && y < height - 1;
                for (nx, ny) in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
                    let inside = nx >= 0 && ny >= 0 && nx < width && ny < height;
                    if inside && !seen[at(nx, ny)] && !self.ink(nx as u32, ny as u32) {
                        seen[at(nx, ny)] = true;
                        area.push((nx, ny));
                    }
                }
            }
            holes += u32::from(closed);
        }
        holes
    }

    fn digit(&self) -> Option<char> {
        let Font { rows, upper, lower } = self.font;
        if self.width <= ONE_WIDTH {
            return (0..rows).all(|y| self.row(y) > 0).then_some('1');
        }
        let upper = (self.left(upper), self.right(upper));
        let lower = (self.left(lower), self.right(lower));
        let left_near_top = self.left(1) || self.left(2);
        match self.holes() {
            2 => Some('8'),
            1 if self.solid(self.font.lower) && self.sparse(rows - 1) => Some('4'),
            1 => match (upper, lower) {
                ((true, true), (true, true)) => Some('0'),
                ((true, false), (true, true)) => Some('6'),
                ((true, true), (false, true)) => Some('9'),
                _ => None,
            },
            0 if self.solid(0) && left_near_top && !self.right(1) => Some('5'),
            0 if self.solid(0) && !left_near_top && self.sparse(rows - 1) => Some('7'),
            0 if self.solid(rows - 1) && self.right(1) && !self.right(rows - 2) => Some('2'),
            0 if upper == (false, true) && lower == (false, true) => Some('3'),
            _ => None,
        }
    }

    fn value(&self) -> Option<char> {
        match self.top()? {
            top if top >= self.font.rows - 2 => Some(if self.width == 1 { '.' } else { ',' }),
            _ if self.width >= PERCENT_WIDTH => Some('%'),
            1 => Some('/'),
            _ => self.digit(),
        }
    }
}

/// Reads gauge values and EXP: digits, `/`, separators and `%`. `None` if any can't be read.
pub fn values(masks: &[Mask]) -> Option<String> {
    masks.iter().map(|mask| Shape::new(mask, HUD)?.value()).collect()
}

/// Reads digits alone, such as the bold level digits. `None` if any can't be read.
pub fn number(masks: &[Mask]) -> Option<String> {
    masks.iter().map(|mask| Shape::new(mask, HUD)?.digit()).collect()
}

/// Reads a number from a window. A slash or bracket is narrow like a 1, but a 1's right side has
/// ink all the way down. `None` if any can't be read.
pub fn window_number(masks: &[Mask]) -> Option<String> {
    round_number(masks, WINDOW)
}

/// Reads a number from the Quest Helper. `None` if any can't be read.
pub fn quest_number(masks: &[Mask]) -> Option<String> {
    round_number(masks, QUEST)
}

fn round_number(masks: &[Mask], font: Font) -> Option<String> {
    let digit = |mask: &Mask| {
        let shape = Shape::new(mask, font)?;
        let straight = shape.width > ONE_WIDTH || (0..font.rows).all(|y| shape.right(y));
        shape.digit().filter(|_| straight)
    };
    masks.iter().map(digit).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A mask from rows of `#` and `.`, each drawn at twice its size as on the design grid.
    fn mask(rows: [&str; 7]) -> Mask {
        let doubled: Vec<String> = rows
            .iter()
            .flat_map(|row| {
                let wide: String = row.chars().flat_map(|c| [c, c]).collect();
                [wide.clone(), wide]
            })
            .collect();
        Mask::try_from(doubled).unwrap()
    }

    // Blocky made-up digits, square rather than rounded, so that the rules are tested on shapes
    // of their own and none of the game's font is kept here.
    fn digit(c: char) -> [&'static str; 7] {
        match c {
            '0' => ["####", "#..#", "#..#", "#..#", "#..#", "#..#", "####"],
            '2' => ["####", "...#", "...#", "####", "#...", "#...", "####"],
            '3' => ["###.", "...#", "...#", ".##.", "...#", "...#", "###."],
            '4' => ["..#.", ".##.", "#.#.", "#.#.", "####", "..#.", "..#."],
            '5' => ["####", "#...", "#...", "####", "...#", "...#", "####"],
            '6' => ["####", "#...", "#...", "####", "#..#", "#..#", "####"],
            '7' => ["####", "...#", "...#", "..#.", "..#.", ".#..", ".#.."],
            '8' => ["####", "#..#", "#..#", "####", "#..#", "#..#", "####"],
            '9' => ["####", "#..#", "#..#", "####", "...#", "...#", "####"],
            _ => [".#", ".#", ".#", ".#", ".#", ".#", ".#"],
        }
    }

    /// A mask of the taller window font, from rows of `#` and `.`.
    fn tall(rows: [&str; 9]) -> Mask {
        let doubled: Vec<String> = rows
            .iter()
            .flat_map(|row| {
                let wide: String = row.chars().flat_map(|c| [c, c]).collect();
                [wide.clone(), wide]
            })
            .collect();
        Mask::try_from(doubled).unwrap()
    }

    // Made up, like the digits above, but with the corners cut and the 4 open at the bottom.
    fn tall_digit(c: char) -> [&'static str; 9] {
        match c {
            '0' => [".###.", "#...#", "#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."],
            '1' => ["..#", ".##", "#.#", "..#", "..#", "..#", "..#", "..#", "..#"],
            '2' => [".###.", "#...#", "....#", "....#", "...#.", "..#..", "..#..", ".#...", "#####"],
            '3' => [".###.", "#...#", "....#", "....#", ".###.", "....#", "....#", "#...#", ".###."],
            '4' => ["...#.", "..##.", "..##.", ".#.#.", ".#.#.", "#..#.", "#####", "...#.", "...#."],
            '5' => [".####", ".#...", "#....", "####.", "#...#", "....#", "....#", "#...#", ".###."],
            '6' => [".###.", "#...#", "#....", "#.##.", "##..#", "#...#", "#...#", "#...#", ".###."],
            '7' => ["#####", "...#.", "...#.", "..#..", "..#..", "..#..", ".#...", ".#...", ".#..."],
            '8' => [".###.", "#...#", "#...#", ".###.", "#...#", "#...#", "#...#", "#...#", ".###."],
            _ => [".###.", "#...#", "#...#", "#...#", ".####", "....#", "....#", "#...#", ".###."],
        }
    }

    #[test]
    fn every_window_digit_has_its_own_answer() {
        for c in "0123456789".chars() {
            assert_eq!(window_number(&[tall(tall_digit(c))]), Some(c.to_string()));
        }
    }

    #[test]
    fn the_quest_helpers_shorter_digits_read_too() {
        let shorter = |rows: [&str; 8]| {
            let doubled: Vec<String> = rows
                .iter()
                .flat_map(|row| {
                    let wide: String = row.chars().flat_map(|c| [c, c]).collect();
                    [wide.clone(), wide]
                })
                .collect();
            Mask::try_from(doubled).unwrap()
        };
        let zero = shorter([".###.", "#...#", "#...#", "#...#", "#...#", "#...#", "#...#", ".###."]);
        let one = shorter(["..#", ".##", "#.#", "..#", "..#", "..#", "..#", "..#"]);
        assert_eq!(quest_number(&[one, zero]), Some("10".to_string()));
    }

    #[test]
    fn a_window_number_stays_unread_with_anything_but_digits() {
        let slash = tall(["..#", "..#", ".#.", ".#.", ".#.", ".#.", ".#.", "#..", "#.."]);
        assert_eq!(window_number(&[tall(tall_digit('1')), slash]), None);
        assert_eq!(window_number(&[]), Some(String::new()), "the caller's parse refuses this");
    }

    #[test]
    fn every_digit_has_its_own_answer() {
        for c in "0123456789".chars() {
            let masks = [mask(digit(c))];
            assert_eq!(values(&masks), Some(c.to_string()), "as a value");
            assert_eq!(number(&masks), Some(c.to_string()), "as a level");
        }
    }

    #[test]
    fn symbols_are_told_by_where_they_sit() {
        let dot = mask([".", ".", ".", ".", ".", ".", "#"]);
        let comma = mask(["..", "..", "..", "..", "..", ".#", "#."]);
        let slash = mask(["...", "..#", "..#", ".#.", ".#.", "#..", "#.."]);
        let percent = mask(["##...#", "##..#.", "...#..", "..#...", ".#....", "#...##", "....##"]);
        let one = mask(digit('1'));
        assert_eq!(values(&[one.clone(), comma, one.clone(), dot, slash, one, percent]), Some("1,1./1%".to_string()));
    }

    #[test]
    fn a_shape_no_rule_fits_stays_unread() {
        let blot = mask(["##.##", "#####", "..#..", "#.#.#", "..#..", "#####", ".#.#."]);
        assert_eq!(values(std::slice::from_ref(&blot)), None);
        assert_eq!(number(&[blot]), None);
        // One unreadable digit leaves the whole number unread.
        assert_eq!(number(&[mask(digit('3')), Mask::try_from(vec!["#".to_string(); 5]).unwrap()]), None);
    }
}
