//! How far the Pickup Log's stack moved between two captures, which says how many lines arrived.
//!
//! A new line appears at the bottom and pushes the others up one row each, and every line fades
//! on its own. The same words come again and again (`You have gained mesos (+27)` twice in a
//! row), so text can't tell an old line from a new one; where each line stands can. Each capture
//! is compared with the last clean one at every shift: the rows that moved up must look as they
//! did and be no brighter, and the rows the shift leaves at the bottom must be lines at full
//! brightness. The shift that explains most rows wins, the smallest on a tie, so a doubt costs a
//! line rather than inventing one.
//!
//! A capture no shift explains (a monster or a skill effect behind the text) is skipped, and
//! the next one is compared with the last clean one. If none fits for a while, the next capture
//! is taken as a fresh start and the lines in between are missed, never guessed.

use serde::Serialize;

/// A row whose ink agrees with another's this much or more is the same line, moved or not.
const SAME_SHAPE: f32 = 0.8;
/// A line that has just arrived is this bright at least, against a line at full brightness,
/// less [`FADE_PER_SECOND`] for each second since the last look. Lines fade by about 0.18 a
/// second (the recording of 2026-11-15); some fade faster.
const FRESH: f32 = 0.92;
const FADE_PER_SECOND: f32 = 0.25;
/// However long since the last look, a new line is this bright at least.
const FRESHEST_FLOOR: f32 = 0.6;
/// Below this brightness a line's ink is half the world behind it, and two looks at the same
/// line over a moving background may not agree on its shape.
const FAINT: f32 = 0.6;
/// A line only fades, but a capture's brightness wobbles a little.
const BRIGHTER_BY: f32 = 0.08;
/// How long captures may fit no shift before the next is a fresh start, in seconds.
const RESYNC_AFTER: f64 = 3.0;
/// A line at full brightness is still on screen this long after, in seconds: one gone sooner
/// was hidden, or the shift is wrong.
const STAYS_FOR: f64 = 1.0;

/// What a line looks like in one capture, as the reader sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    /// The ink of the line's right end, at the log's own size.
    pub shape: Shape,
    pub colour: Colour,
    /// How bright its ink is, against a line at full brightness: 1 for a new line, falling
    /// towards 0 as it fades.
    pub brightness: f32,
}

/// The colour a line is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Colour {
    White,
    /// A share of a partymate's EXP.
    Yellow,
    Other,
}

/// Ink as on or off, row by row, so a fading line keeps its shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shape {
    width: usize,
    height: usize,
    ink: Vec<bool>,
}

impl Shape {
    pub fn new(width: usize, height: usize, ink: Vec<bool>) -> Self {
        assert_eq!(ink.len(), width * height, "a shape's ink fills its size");
        Self { width, height, ink }
    }

    pub fn from_fn(width: usize, height: usize, on: impl Fn(usize, usize) -> bool) -> Self {
        Self::new(width, height, (0..height).flat_map(|y| (0..width).map(move |x| (x, y))).map(|(x, y)| on(x, y)).collect())
    }

    pub fn ink(&self) -> usize {
        self.ink.iter().filter(|&&on| on).count()
    }

    /// How much of the ink the two share, lined up by their right edges and allowed a pixel
    /// either way: 1 for the same line, near 0 for unrelated ones.
    pub fn similarity(&self, other: &Shape) -> f32 {
        if self.height != other.height {
            return 0.0;
        }
        let total = self.ink().max(other.ink());
        if total == 0 {
            return 1.0;
        }
        let at = |shape: &Shape, x: isize, y: isize| {
            // Columns counted from the right edge, so both line up there.
            let x = shape.width as isize - 1 - x;
            x >= 0 && y >= 0 && (x as usize) < shape.width && (y as usize) < shape.height && shape.ink[y as usize * shape.width + x as usize]
        };
        let width = self.width.min(other.width) as isize;
        let mut best = 0;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let shared = (0..self.height as isize)
                    .flat_map(|y| (0..width).map(move |x| (x, y)))
                    .filter(|&(x, y)| at(self, x, y) && at(other, x + dx, y + dy))
                    .count();
                best = best.max(shared);
            }
        }
        best as f32 / total as f32
    }
}

impl Row {
    /// Whether `self`, seen before, can be `later` now: the same shape and colour, no brighter.
    fn can_become(&self, later: &Row) -> bool {
        self.colour == later.colour
            && later.brightness <= self.brightness + BRIGHTER_BY
            && self.shape.similarity(&later.shape) >= SAME_SHAPE
    }
}

/// The rows of one capture, bottom first: `None` where no line shows at that row.
pub type Rows = Vec<Option<Row>>;

/// What a capture says about the stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// The first capture, or a fresh start: what is on screen came before it and counts for
    /// nothing.
    Start,
    /// The stack moved up `shift` rows: rows `0..shift` are new lines. 0 when nothing arrived.
    Moved { shift: usize },
    /// No shift explains this capture, so it was left out.
    Skipped,
}

#[derive(Debug, Default)]
pub struct Stack {
    /// The last clean capture's rows, and when it was taken.
    last: Option<(f64, Rows)>,
    /// When captures stopped fitting.
    lost_since: Option<f64>,
}

impl Stack {
    /// Takes in the capture taken at `at` (seconds), the next after the last one taken.
    pub fn push(&mut self, at: f64, rows: Rows) -> Step {
        let Some((last_at, last)) = &self.last else {
            self.last = Some((at, rows));
            return Step::Start;
        };
        let fresh = (FRESH - FADE_PER_SECOND * (at - last_at) as f32).max(FRESHEST_FLOOR);
        let look = Look { fresh, soon: at - last_at < STAYS_FOR };
        match best_shift(last, &rows, look) {
            Some(shift) => {
                let rows = cleaned(last, rows, shift, fresh);
                self.last = Some((at, rows));
                self.lost_since = None;
                Step::Moved { shift }
            }
            None => {
                let since = *self.lost_since.get_or_insert(at);
                if at - since >= RESYNC_AFTER {
                    self.lost_since = None;
                    self.last = Some((at, rows));
                    Step::Start
                } else {
                    Step::Skipped
                }
            }
        }
    }

    /// Forgets the stack: the next capture is a fresh start.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// What one look can tell, by how long it came after the last.
#[derive(Debug, Clone, Copy)]
struct Look {
    /// How bright a line that arrived since must be.
    fresh: f32,
    /// Too soon for a line at full brightness to have faded out.
    soon: bool,
}

/// `now` with every row that can't be a line taken out: one that showed up faded where no line
/// was. Lines arrive bright, so it is something in the world behind the log that looks like one,
/// and kept it would hold the stack still when a line arrives below it.
fn cleaned(before: &[Option<Row>], mut now: Rows, shift: usize, fresh: f32) -> Rows {
    for (at, row) in now.iter_mut().enumerate().skip(shift) {
        let was = before.get(at - shift).is_some_and(Option::is_some);
        if !was && row.as_ref().is_some_and(|r| r.brightness < fresh) {
            *row = None;
        }
    }
    now
}

/// The shift that explains `now` from `before` best, or `None` when none does.
fn best_shift(before: &[Option<Row>], now: &[Option<Row>], look: Look) -> Option<usize> {
    (0..=now.len())
        .filter_map(|shift| score(before, now, shift, look).map(|score| (score, shift)))
        // The highest score; of equal ones the smallest shift, which counts fewest lines.
        .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
        .map(|(_, shift)| shift)
}

/// How well moving `before` up `shift` rows explains `now`: one for each line seen where it
/// should be, less half for each bright line where none was. A faded row where none was is no
/// line and counts for nothing. `None` when the shift can't be: a line moved onto a different
/// one, a new row that isn't a line at full brightness, or (`soon`) a line at full brightness
/// gone from a row still in view.
fn score(before: &[Option<Row>], now: &[Option<Row>], shift: usize, look: Look) -> Option<f32> {
    if now[..shift].iter().any(|row| row.as_ref().is_none_or(|r| r.brightness < look.fresh)) {
        return None;
    }
    let mut score = 0.0;
    for (at, row) in now.iter().enumerate().skip(shift) {
        let was = before.get(at - shift).and_then(Option::as_ref);
        match (was, row) {
            (Some(was), Some(row)) if was.can_become(row) => score += 1.0,
            // Two faint rows that disagree: the same line over a world that moved, or a wrong
            // shift. It counts against the shift rather than ruling it out.
            (Some(was), Some(row)) if was.brightness < FAINT && row.brightness < FAINT && row.colour == was.colour => score -= 1.0,
            (Some(_), Some(_)) => return None,
            // A line where none was: one the last capture missed, or the wrong shift.
            (None, Some(row)) if row.brightness >= look.fresh => score -= 0.5,
            (None, Some(_)) => {}
            (Some(was), None) if look.soon && was.brightness >= FRESH => return None,
            // Faded, or hidden for now.
            (_, None) => {}
        }
    }
    Some(score)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// A made-up line: a pattern of ink that differs for each `id`, drawn at `brightness`.
    pub fn line(id: u32, brightness: f32) -> Option<Row> {
        line_in(id, brightness, Colour::White)
    }

    pub fn line_in(id: u32, brightness: f32, colour: Colour) -> Option<Row> {
        let shape = Shape::from_fn(24, 6, |x, y| {
            let seed = id.wrapping_mul(2_654_435_761).rotate_left((x * 5 + y * 3) as u32 % 31);
            (seed ^ (x as u32 * 7 + y as u32 * 13)).count_ones().is_multiple_of(3)
        });
        Some(Row { shape, colour, brightness })
    }

    #[test]
    fn made_up_lines_differ_from_each_other() {
        let a = line(1, 1.0).unwrap();
        for id in 2..8 {
            let other = line(id, 1.0).unwrap();
            assert!(a.shape.similarity(&other.shape) < SAME_SHAPE, "line {id} looks like line 1");
        }
        assert_eq!(a.shape.similarity(&a.shape), 1.0);
    }

    #[test]
    fn a_pixel_out_is_still_the_same_shape() {
        let a = Shape::from_fn(20, 6, |x, y| (x + y) % 4 == 0 || x == 3);
        let moved = Shape::from_fn(21, 6, |x, y| x >= 1 && ((x - 1 + y) % 4 == 0 || x - 1 == 3));
        // Lined up by the right edge, the extra column on the left changes nothing.
        assert!(a.similarity(&moved) >= SAME_SHAPE);
    }

    #[test]
    fn the_first_capture_counts_nothing() {
        let mut stack = Stack::default();
        assert_eq!(stack.push(0.0, vec![line(1, 1.0), line(2, 0.9)]), Step::Start);
    }

    #[test]
    fn a_new_line_pushes_the_rest_up() {
        let mut stack = Stack::default();
        stack.push(0.0, vec![line(1, 1.0)]);
        assert_eq!(stack.push(0.25, vec![line(1, 1.0)]), Step::Moved { shift: 0 });
        assert_eq!(stack.push(0.5, vec![line(2, 1.0), line(1, 0.95)]), Step::Moved { shift: 1 });
        assert_eq!(stack.push(0.75, vec![line(3, 1.0), line(4, 1.0), line(2, 1.0), line(1, 0.9)]), Step::Moved { shift: 2 });
    }

    #[test]
    fn the_same_words_twice_are_two_lines() {
        let mut stack = Stack::default();
        stack.push(0.0, vec![line(7, 1.0)]);
        assert_eq!(stack.push(0.25, vec![line(7, 1.0), line(7, 1.0)]), Step::Moved { shift: 1 });
        assert_eq!(stack.push(0.5, vec![line(7, 1.0), line(7, 1.0)]), Step::Moved { shift: 0 });
    }

    #[test]
    fn a_line_brighter_than_before_is_a_new_one_in_its_place() {
        let mut stack = Stack::default();
        stack.push(0.0, vec![line(7, 0.4)]);
        // The old one has moved up and faded out; the same words came again at full brightness.
        assert_eq!(stack.push(0.25, vec![line(7, 1.0)]), Step::Moved { shift: 1 });
    }

    #[test]
    fn lines_fading_from_the_top_are_no_arrivals() {
        let mut stack = Stack::default();
        stack.push(0.0, vec![line(3, 0.9), line(2, 0.5), line(1, 0.2)]);
        assert_eq!(stack.push(0.25, vec![line(3, 0.8), line(2, 0.4), None]), Step::Moved { shift: 0 });
        assert_eq!(stack.push(0.5, vec![None, None, None]), Step::Moved { shift: 0 });
        assert_eq!(stack.push(0.75, vec![line(4, 1.0)]), Step::Moved { shift: 1 }, "a line on an empty log is new");
    }

    #[test]
    fn a_faded_row_is_never_counted_as_new() {
        let mut stack = Stack::default();
        stack.push(0.0, vec![line(1, 1.0)]);
        // A different line, but faded: it can't have just arrived, and it can't be line 1.
        assert_eq!(stack.push(0.25, vec![line(2, 0.5)]), Step::Skipped);
    }

    #[test]
    fn a_covered_capture_is_skipped_and_the_next_compared_with_the_last_clean_one() {
        let mut stack = Stack::default();
        stack.push(0.0, vec![line(1, 1.0), line(2, 0.9)]);
        // Something behind the text changed both rows' ink.
        assert_eq!(stack.push(0.25, vec![line(8, 0.5), line(9, 0.5)]), Step::Skipped);
        assert_eq!(stack.push(0.5, vec![line(3, 1.0), line(1, 1.0), line(2, 0.85)]), Step::Moved { shift: 1 });
    }

    #[test]
    fn captures_that_never_fit_end_in_a_fresh_start() {
        let mut stack = Stack::default();
        stack.push(0.0, vec![line(1, 1.0)]);
        assert_eq!(stack.push(1.0, vec![line(8, 0.5)]), Step::Skipped);
        assert_eq!(stack.push(2.0, vec![line(8, 0.5)]), Step::Skipped);
        assert_eq!(stack.push(4.0, vec![line(8, 0.5)]), Step::Start);
        assert_eq!(stack.push(4.25, vec![line(8, 0.5)]), Step::Moved { shift: 0 });
    }

    #[test]
    fn a_faded_row_from_nowhere_is_the_world_and_doesnt_hold_the_stack_still() {
        let mut stack = Stack::default();
        stack.push(0.0, vec![None, None, None]);
        // Something behind the log looks like a faded line, and stays.
        assert_eq!(stack.push(0.25, vec![None, None, line(9, 0.3)]), Step::Moved { shift: 0 });
        assert_eq!(stack.push(0.5, vec![None, None, line(9, 0.3)]), Step::Moved { shift: 0 });
        assert_eq!(stack.push(0.75, vec![line(1, 1.0), None, line(9, 0.3)]), Step::Moved { shift: 1 });
    }

    #[test]
    fn a_new_line_seen_late_is_still_new() {
        let mut stack = Stack::default();
        stack.push(0.0, vec![line(1, 0.9), None]);
        // A second after: the new line has faded a little, the old one more.
        assert_eq!(stack.push(1.0, vec![line(2, 0.78), line(1, 0.7)]), Step::Moved { shift: 1 });
    }

    #[test]
    fn faint_lines_that_no_longer_look_alike_dont_stop_a_burst_being_counted() {
        let mut stack = Stack::default();
        stack.push(0.0, vec![line(1, 0.48), line(2, 0.35), None, None]);
        // Two new lines; the faded ones moved up over a world that changed under them.
        assert_eq!(stack.push(0.25, vec![line(3, 1.0), line(4, 1.0), line(8, 0.46), line(9, 0.32)]), Step::Moved { shift: 2 });
    }

    #[test]
    fn colour_tells_two_lines_apart() {
        let mut stack = Stack::default();
        stack.push(0.0, vec![line_in(5, 1.0, Colour::White)]);
        assert_eq!(stack.push(0.25, vec![line_in(5, 1.0, Colour::Yellow), line_in(5, 1.0, Colour::White)]), Step::Moved { shift: 1 });
    }

    #[test]
    fn a_line_that_faded_as_another_came_leaves_one_arrival() {
        let mut stack = Stack::default();
        stack.push(0.0, vec![line(1, 0.5), None]);
        assert_eq!(stack.push(0.25, vec![line(2, 1.0), None]), Step::Moved { shift: 1 });
    }

    #[test]
    fn a_bright_line_cant_vanish_so_a_garbled_capture_is_not_all_new() {
        let mut stack = Stack::default();
        // The reader always gives the log's every row, so lines pushed up stay in view.
        stack.push(0.0, vec![line(1, 1.0), line(2, 1.0), None, None, None]);
        // Three bright rows unlike before: something behind the text, not three arrivals.
        assert_eq!(stack.push(0.25, vec![line(8, 1.0), line(9, 1.0), line(10, 1.0), None, None]), Step::Skipped);
        // Long enough after, the old lines may have gone.
        stack.reset();
        stack.push(0.0, vec![line(1, 1.0), None]);
        assert_eq!(stack.push(2.0, vec![line(8, 1.0), None]), Step::Moved { shift: 1 });
    }

    #[test]
    fn an_unchanged_line_is_no_arrival() {
        let mut stack = Stack::default();
        // The line came back unchanged, or its twin arrived as it faded out: both explain
        // nothing more, and the one that counts nothing is taken.
        stack.push(0.0, vec![None, None]);
        stack.push(0.25, vec![line(7, 1.0), None]);
        assert_eq!(stack.push(2.0, vec![line(7, 1.0), None]), Step::Moved { shift: 0 });
    }
}
