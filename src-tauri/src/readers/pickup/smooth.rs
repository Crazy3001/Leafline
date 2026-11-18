//! The log read where the game smooths its font (UI Size Default Ratio).
//!
//! There the picture is redrawn on the design grid, a cell per font pixel, but each cell is part
//! of its neighbours: ink is a matter of how much, not yes or no. So a glyph is compared by how
//! well the line's ink strength follows the glyph softened the same way (a correlation, 1 for a
//! perfect match), allowed a pixel of slack across and down, and it counts only when it follows
//! one glyph clearly better than any other. The glyphs are the same game glyphs the sharp font is
//! read by.

use super::figure::Figure;
use super::font;

/// A line's ink strength, `[row][column]` in screen order (column 0 at the left), rows from a
/// font pixel above the line's top: 0 for none, about 1 for full ink.
pub type Strength = Vec<Vec<f32>>;

/// Strength that counts as ink when finding where glyphs are.
const INKED: f32 = 0.3;
/// A column is clear of ink below this everywhere: the blur between letters is stronger.
const CLEAR: f32 = 0.15;
/// How well a figure's glyph must follow its softened form, and by how much better than any
/// other glyph (on the recording of 2026-11-15 at 1.875x, right reads scored 0.76 to 0.85 and
/// the next best 0.54 to 0.59).
const FIGURE_FITS: f32 = 0.65;
const FIGURE_LEAD: f32 = 0.1;
/// The same for letters, which have more lookalikes.
const LETTER_FITS: f32 = 0.6;
const LETTER_LEAD: f32 = 0.03;
/// Blank columns from which a gap is a space: smoothing narrows the five of the sharp font.
const SPACE: usize = 3;
/// The most blank columns between glyphs of one figure.
const GLYPH_GAP: usize = 5;

/// A glyph softened as the game's smoothing and the redraw soften it: each font pixel shared
/// with its neighbours, padded by one all round. `rows[y][x]`.
fn softened(rows: &[&str; 11]) -> Vec<Vec<f32>> {
    let (h, w) = (rows.len() + 2, rows[0].len() + 2);
    let sharp: Vec<Vec<f32>> = (0..h)
        .map(|y| {
            (0..w)
                .map(|x| {
                    let inside = y >= 1 && x >= 1 && y <= rows.len() && x <= rows[0].len();
                    if inside && rows[y - 1].as_bytes()[x - 1] == b'#' { 1.0 } else { 0.0 }
                })
                .collect()
        })
        .collect();
    let k = [0.2, 0.6, 0.2];
    let across: Vec<Vec<f32>> = (0..h)
        .map(|y| (0..w).map(|x| (0..3).map(|i| k[i] * at(&sharp, x as isize + i as isize - 1, y as isize)).sum()).collect())
        .collect();
    (0..h).map(|y| (0..w).map(|x| (0..3).map(|i| k[i] * at(&across, x as isize, y as isize + i as isize - 1)).sum()).collect()).collect()
}

fn at(grid: &[Vec<f32>], x: isize, y: isize) -> f32 {
    if x < 0 || y < 0 {
        return 0.0;
    }
    grid.get(y as usize).and_then(|row| row.get(x as usize)).copied().unwrap_or(0.0)
}

/// How well the strength around a glyph whose sharp left edge is column `left` and top is row
/// `top` follows `glyph` softened: a correlation, 1 for a perfect match.
fn follows(strength: &Strength, glyph: &[Vec<f32>], left: isize, top: isize) -> f32 {
    let (mut sa, mut sb, mut saa, mut sbb, mut sab, mut n) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for (y, row) in glyph.iter().enumerate() {
        for (x, &b) in row.iter().enumerate() {
            let a = at(strength, left + x as isize - 1, top + y as isize - 1);
            sa += a;
            sb += b;
            saa += a * a;
            sbb += b * b;
            sab += a * b;
            n += 1.0;
        }
    }
    let cov = sab - sa * sb / n;
    let spread = ((saa - sa * sa / n) * (sbb - sb * sb / n)).sqrt();
    if spread > 0.0 { cov / spread } else { 0.0 }
}

/// Every glyph's softened form, with its character and sharp width.
fn glyph_forms(among: impl Fn(char) -> bool) -> Vec<(char, usize, Vec<Vec<f32>>)> {
    font::all_glyphs().filter(|(c, _)| among(*c)).map(|(c, rows)| (c, rows[0].len(), softened(rows))).collect()
}

/// The glyph that the strength follows best with its left edge at `left`, or (`from_right`) its
/// right edge at `left`, to a pixel either way, and its top at the line's, to a pixel either way:
/// (character, its left edge, its width), when it fits and leads every other character by `lead`.
fn best_placed(
    strength: &Strength,
    forms: &[(char, usize, Vec<Vec<f32>>)],
    edge: isize,
    from_right: bool,
    fits: f32,
    lead: f32,
) -> Option<(char, isize, usize)> {
    let (sure, guess) = placed(strength, forms, edge, from_right, fits, lead)?;
    sure.then_some(guess)
}

/// [`best_placed`]'s best guess, sure or not: whether it fits and leads, and (character, left
/// edge, width).
fn placed(
    strength: &Strength,
    forms: &[(char, usize, Vec<Vec<f32>>)],
    edge: isize,
    from_right: bool,
    fits: f32,
    lead: f32,
) -> Option<(bool, (char, isize, usize))> {
    let mut by_char: Vec<(f32, char, isize, usize)> = Vec::new();
    for (c, width, form) in forms {
        let left = if from_right { edge - *width as isize + 1 } else { edge };
        let mut best = (f32::MIN, 0);
        for dx in -1..=1 {
            for top in 0..=2 {
                let score = follows(strength, form, left + dx, top);
                if score > best.0 {
                    best = (score, left + dx);
                }
            }
        }
        by_char.push((best.0, *c, best.1, *width));
    }
    by_char.sort_by(|a, b| b.0.total_cmp(&a.0));
    let (top, c, at, width) = *by_char.first()?;
    let next = by_char.get(1).map_or(f32::MIN, |n| n.0);
    Some((top >= fits && top - next >= lead, (c, at, width)))
}

/// A column with no ink at all, as between words, rather than the blur between two letters.
fn clear(strength: &Strength, x: isize) -> bool {
    x < 0 || strength.iter().all(|row| row.get(x as usize).is_none_or(|&v| v < CLEAR))
}

fn inked(strength: &Strength, x: isize) -> bool {
    x >= 0 && strength.iter().any(|row| row.get(x as usize).is_some_and(|&v| v >= INKED))
}

/// The line's first and last inked columns, the line ending at the first gap wider than words
/// have, counted from the right.
fn extent(strength: &Strength) -> Option<(isize, isize)> {
    let columns = strength.first()?.len() as isize;
    let last = (0..columns).rev().find(|&x| inked(strength, x))?;
    let mut first = last;
    let mut gap = 0;
    for x in (0..last).rev() {
        if inked(strength, x) {
            first = x;
            gap = 0;
        } else {
            gap += 1;
            if gap > 8 {
                break;
            }
        }
    }
    Some((first, last))
}

/// The figure that ends a line, read from its smoothed glyphs, with the width of the words
/// before it.
pub fn figure(strength: &Strength) -> Option<Figure> {
    let (first, last) = extent(strength)?;
    let close = glyph_forms(|c| c == ')');
    let digits = glyph_forms(|c| c.is_ascii_digit() || c == '+');
    let open = glyph_forms(|c| c == '(');
    // The closing bracket: its right edge is the line's last inked column.
    let (_, at, _) = best_placed(strength, &close, last, true, 0.55, 0.0)?;
    let mut right = at - 1;
    let mut found = Vec::new();
    loop {
        // The next glyph's right edge, past a gap.
        let edge = (0..GLYPH_GAP as isize).map(|g| right - g).find(|&x| inked(strength, x))?;
        let (c, at, _) = best_placed(strength, &digits, edge, true, FIGURE_FITS, FIGURE_LEAD)?;
        right = at - 1;
        if c == '+' {
            break;
        }
        found.push(c);
        if found.len() > 9 {
            return None;
        }
    }
    if found.is_empty() {
        return None;
    }
    let amount: u64 = found.iter().rev().collect::<String>().parse().ok()?;
    let edge = (0..GLYPH_GAP as isize).map(|g| right - g).find(|&x| inked(strength, x))?;
    let (_, open_at, _) = best_placed(strength, &open, edge, true, 0.55, 0.0)?;
    let words = usize::try_from(open_at - first).ok()?;
    Some(Figure { amount, words })
}

/// A whole line read from its smoothed glyphs, left to right; a glyph no character follows well
/// is [`font::UNKNOWN`]. `None` when more than a quarter are unknown.
pub fn read_line(strength: &Strength) -> Option<String> {
    let (first, last) = extent(strength)?;
    let forms = glyph_forms(|_| true);
    let mut text = String::new();
    let mut x = first;
    let mut blank = 0;
    let (mut glyphs, mut unknown) = (0, 0);
    while x <= last {
        if clear(strength, x) {
            blank += 1;
            x += 1;
            continue;
        }
        if !text.is_empty() && blank >= SPACE {
            text.push(' ');
        }
        blank = 0;
        glyphs += 1;
        // The next letter starts a blank column after this one; smoothing can fill that column,
        // so it is stepped over rather than looked for. A letter in doubt is a gap, stepped over
        // by the width of its likeliest reading so the rest of the word stays in step.
        let (sure, (c, at, width)) = placed(strength, &forms, x, false, LETTER_FITS, LETTER_LEAD)?;
        if sure {
            text.push(c);
        } else {
            unknown += 1;
            text.push(font::UNKNOWN);
        }
        x = at + width as isize + 1;
    }
    (glyphs > 0 && unknown * 4 <= glyphs).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line drawn from the glyphs and softened as the game smooths it.
    fn smoothed(text: &str) -> Strength {
        let glyph = |c: char| font::all_glyphs().find(|(g, _)| *g == c).expect("a known glyph").1;
        let mut sharp: Vec<String> = vec![String::new(); 11];
        for (i, word) in text.split(' ').enumerate() {
            for (j, c) in word.chars().enumerate() {
                for (y, row) in sharp.iter_mut().enumerate() {
                    if i > 0 && j == 0 {
                        row.push_str(".....");
                    } else if j > 0 {
                        row.push('.');
                    }
                    row.push_str(glyph(c)[y]);
                }
            }
        }
        // A row and a column of margin, as the reader's grid has.
        let rows: Vec<&str> = sharp.iter().map(String::as_str).collect();
        let padded: [&str; 11] = rows.try_into().expect("eleven rows");
        let soft = softened(&padded);
        let mut out: Strength = vec![vec![0.0; soft[0].len() + 4]; soft.len() + 1];
        for (y, row) in soft.iter().enumerate() {
            for (x, v) in row.iter().enumerate() {
                out[y + 1][x + 2] = *v;
            }
        }
        out
    }

    #[test]
    fn reads_a_smoothed_figure() {
        let line = smoothed("Slate (+47)");
        assert_eq!(figure(&line).map(|f| f.amount), Some(47));
    }

    #[test]
    fn reads_a_smoothed_line() {
        let line = smoothed("Red Snail Shell x1 earned. (Etc)");
        let text = read_line(&line).expect("read");
        // Smoothing blurs `rn` together and the full stop is tiny: those may stay gaps, but the
        // line still reads as what it is.
        let parsed = super::super::parse::parse(&text).expect("parses");
        assert_eq!(
            parsed.gain,
            super::super::parse::Gain::Item {
                name: "Red Snail Shell".into(),
                quantity: 1,
                tab: Some(crate::readers::inventory::Tab::Etc),
                cut_short: false
            },
            "{text}"
        );
    }
}
