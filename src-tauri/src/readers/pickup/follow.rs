//! Each line that arrived, read again while it stays in view until its reads agree.
//!
//! The stack's shift says a line arrived, and it is counted once for that. What it says is read
//! at that capture and the next few: the first read is often spoiled by the world behind the
//! text, and a digit can slip (`+26` read `+28`). The reads vote on the figures (the kind,
//! amount and quantity), and the most common item name among the winners names it. A line whose
//! reads don't settle is unreadable: it arrived, but adds nothing.

use std::collections::VecDeque;

use super::parse::{Gain, Parsed};
use super::stack::{Colour, Rows, Stack, Step};

/// The most reads of one line.
const MOST_READS: usize = 5;
/// Reads that agree and settle a line before [`MOST_READS`].
const READS_AGREE: usize = 3;
/// How long a line is followed, in seconds.
const FOLLOW_FOR: f64 = 1.5;

/// A line that arrived, as its reads settled it.
#[derive(Debug, Clone, PartialEq)]
pub struct Arrival {
    /// When the stack's shift brought it in, in the captures' seconds.
    pub at: f64,
    /// What it says, or `None` when its reads didn't settle.
    pub line: Option<Line>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub gain: Gain,
    /// Drawn in yellow: EXP shared from a partymate's kill.
    pub party: bool,
}

/// Reads a stream of captures into arrivals, each line once.
#[derive(Debug, Default)]
pub struct Follower {
    stack: Stack,
    following: VecDeque<Following>,
}

#[derive(Debug)]
struct Following {
    /// Its row in the last clean capture, bottom first.
    row: usize,
    at: f64,
    colour: Colour,
    reads: Vec<Parsed>,
    tries: usize,
    done: bool,
}

impl Follower {
    /// Takes in the capture taken at `at`, reading the rows asked for with `read` (by their
    /// place in `rows`, bottom first). Returns the lines settled, oldest first.
    pub fn push(&mut self, at: f64, rows: Rows, mut read: impl FnMut(usize) -> Option<Parsed>) -> Vec<Arrival> {
        match self.stack.push(at, rows.clone()) {
            Step::Start => {
                // Where the lines being followed went is lost: they settle on what they have.
                self.following.iter_mut().for_each(|f| f.done = true);
            }
            Step::Skipped => {}
            Step::Moved { shift } => {
                for line in &mut self.following {
                    line.row += shift;
                    if rows.get(line.row).is_none_or(Option::is_none) {
                        line.done = true;
                    }
                }
                for row in (0..shift).rev() {
                    let colour = rows[row].as_ref().map_or(Colour::Other, |r| r.colour);
                    self.following.push_back(Following { row, at, colour, reads: Vec::new(), tries: 0, done: false });
                }
                for line in self.following.iter_mut().filter(|f| !f.done) {
                    line.tries += 1;
                    if let Some(parsed) = read(line.row) {
                        line.reads.push(parsed);
                    }
                    let agreeing = line.reads.last().map_or(0, |last| line.reads.iter().filter(|r| r.figures() == last.figures()).count());
                    line.done = line.tries >= MOST_READS || agreeing >= READS_AGREE || at - line.at >= FOLLOW_FOR;
                }
            }
        }
        self.settled()
    }

    /// The lines that are done, oldest first, stopping at the first that isn't so arrivals stay
    /// in order.
    fn settled(&mut self) -> Vec<Arrival> {
        let mut out = Vec::new();
        while self.following.front().is_some_and(|f| f.done) {
            let line = self.following.pop_front().expect("front checked");
            out.push(Arrival { at: line.at, line: vote(&line.reads).map(|gain| Line { gain, party: line.colour == Colour::Yellow }) });
        }
        out
    }

    /// Settles every line followed so far, for the end of a stream.
    pub fn finish(&mut self) -> Vec<Arrival> {
        self.following.iter_mut().for_each(|f| f.done = true);
        let out = self.settled();
        self.stack.reset();
        out
    }

    /// Drops what is being followed and starts afresh, for a new session or character: lines
    /// still in view came before it.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// What a line's reads agree it says. Reads with their `+` outvote those without; of those, the
/// figures read most often win, and a tie between two is no answer. A single read stands.
fn vote(reads: &[Parsed]) -> Option<Gain> {
    let sure: Vec<&Parsed> = reads.iter().filter(|r| r.sure).collect();
    let reads: Vec<&Parsed> = if sure.is_empty() { reads.iter().collect() } else { sure };
    let mut groups: Vec<(_, Vec<&Parsed>)> = Vec::new();
    for read in reads {
        match groups.iter_mut().find(|(figures, _)| *figures == read.figures()) {
            Some((_, group)) => group.push(read),
            None => groups.push((read.figures(), vec![read])),
        }
    }
    groups.sort_by_key(|(_, group)| std::cmp::Reverse(group.len()));
    let winner = &groups.first()?.1;
    if groups.get(1).is_some_and(|(_, next)| next.len() == winner.len()) {
        return None;
    }
    let first = winner[0].gain.clone();
    let Gain::Item { quantity, tab, .. } = first else { return Some(first) };
    // The name read most often, the earliest of equals.
    let names: Vec<(&String, bool)> = winner
        .iter()
        .filter_map(|r| match &r.gain {
            Gain::Item { name, cut_short, .. } => Some((name, *cut_short)),
            _ => None,
        })
        .collect();
    let (name, cut_short) = names
        .iter()
        .max_by_key(|(name, _)| (names.iter().filter(|(n, _)| n == name).count(), std::cmp::Reverse(names.iter().position(|(n, _)| n == name))))
        .copied()?;
    Some(Gain::Item { name: name.clone(), quantity, tab, cut_short })
}

#[cfg(test)]
mod tests {
    use super::super::parse::parse;
    use super::super::stack::tests::{line, line_in};
    use super::*;

    /// A follower fed captures of made-up lines, each of which reads as `texts[id]`.
    struct Log {
        follower: Follower,
        at: f64,
        arrived: Vec<Arrival>,
    }

    impl Log {
        fn new() -> Self {
            Self { follower: Follower::default(), at: 0.0, arrived: Vec::new() }
        }

        /// One capture a quarter second after the last, its rows bottom first as (id, brightness),
        /// each read as `text(id, read number)`.
        fn capture(&mut self, rows: &[Option<(u32, f32)>], text: impl Fn(u32) -> &'static str) {
            self.at += 0.25;
            let ids: Vec<Option<u32>> = rows.iter().map(|r| r.map(|(id, _)| id)).collect();
            let rows: Rows = rows.iter().map(|r| r.and_then(|(id, b)| line(id, b))).collect();
            let got = self.follower.push(self.at, rows, |row| ids[row].and_then(|id| parse(text(id))));
            self.arrived.extend(got);
        }

        fn gains(&mut self) -> Vec<Option<Gain>> {
            self.arrived.extend(self.follower.finish());
            self.arrived.iter().map(|a| a.line.as_ref().map(|l| l.gain.clone())).collect()
        }
    }

    fn mesos(amount: u64) -> Option<Gain> {
        Some(Gain::Mesos { amount })
    }

    #[test]
    fn each_line_counts_once_however_long_it_stays() {
        let mut log = Log::new();
        let text = |_| "You have gained mesos (+27)";
        log.capture(&[None, None, None], text);
        log.capture(&[Some((1, 1.0)), None, None], text);
        for _ in 0..10 {
            log.capture(&[Some((1, 1.0)), None, None], text);
        }
        assert_eq!(log.gains(), vec![mesos(27)]);
    }

    #[test]
    fn lines_already_on_screen_at_the_start_count_for_nothing() {
        let mut log = Log::new();
        let text = |_| "You have gained mesos (+27)";
        log.capture(&[Some((1, 1.0)), None], text);
        log.capture(&[Some((1, 1.0)), None], text);
        assert_eq!(log.gains(), vec![]);
    }

    #[test]
    fn a_digit_slipped_once_is_outvoted() {
        let mut log = Log::new();
        let reads = std::cell::Cell::new(0);
        let text = |_| {
            reads.set(reads.get() + 1);
            if reads.get() == 2 { "You have gained mesos (+28)" } else { "You have gained mesos (+26)" }
        };
        log.capture(&[None, None], text);
        for _ in 0..6 {
            log.capture(&[Some((1, 1.0)), None], text);
        }
        assert_eq!(log.gains(), vec![mesos(26)]);
    }

    #[test]
    fn a_line_whose_reads_never_agree_adds_nothing() {
        let mut log = Log::new();
        let reads = std::cell::Cell::new(0);
        let text = |_| {
            reads.set(reads.get() + 1);
            if reads.get() % 2 == 0 { "You have gained mesos (+28)" } else { "You have gained mesos (+26)" }
        };
        log.capture(&[None, None], text);
        for _ in 0..4 {
            log.capture(&[Some((1, 1.0)), None], text);
        }
        assert_eq!(log.gains(), vec![None], "it arrived, but its amount is unknown");
    }

    #[test]
    fn a_read_with_its_plus_outvotes_those_without() {
        let mut log = Log::new();
        let reads = std::cell::Cell::new(0);
        let text = |_| {
            reads.set(reads.get() + 1);
            if reads.get() == 3 { "You have gained mesos (+43)" } else { "You have gained mesos (243)" }
        };
        log.capture(&[None], text);
        for _ in 0..5 {
            log.capture(&[Some((1, 1.0))], text);
        }
        assert_eq!(log.gains(), vec![mesos(43)]);
    }

    #[test]
    fn a_burst_comes_out_in_order() {
        let mut log = Log::new();
        let text = |id| match id {
            1 => "You have gained mesos (+10)",
            2 => "Leaf x1 earned. (Etc)",
            _ => "You have gained mesos (+30)",
        };
        log.capture(&[None, None, None, None], text);
        log.capture(&[Some((1, 1.0)), None, None, None], text);
        log.capture(&[Some((3, 1.0)), Some((2, 1.0)), Some((1, 1.0)), None], text);
        for _ in 0..8 {
            log.capture(&[Some((3, 1.0)), Some((2, 1.0)), Some((1, 1.0)), None], text);
        }
        let leaf = Some(Gain::Item { name: "Leaf".into(), quantity: 1, tab: Some(crate::readers::inventory::Tab::Etc), cut_short: false });
        assert_eq!(log.gains(), vec![mesos(10), leaf, mesos(30)]);
    }

    #[test]
    fn a_line_that_leaves_before_its_reads_agree_settles_on_what_it_had() {
        let mut log = Log::new();
        let text = |_| "You have gained mesos (+5)";
        log.capture(&[None, None], text);
        log.capture(&[Some((1, 1.0)), None], text);
        log.capture(&[None, None], text);
        assert_eq!(log.gains(), vec![mesos(5)]);
    }

    #[test]
    fn yellow_exp_is_a_partymates_share() {
        let mut follower = Follower::default();
        follower.push(0.0, vec![None, None], |_| None);
        let rows = vec![line_in(1, 1.0, Colour::Yellow), line_in(2, 1.0, Colour::White)];
        let text = |row: usize| parse(if row == 0 { "You received EXP (+4)" } else { "You received EXP (+20)" });
        follower.push(0.25, rows, text);
        let lines: Vec<Line> = follower.finish().into_iter().filter_map(|a| a.line).collect();
        assert_eq!(
            lines,
            vec![Line { gain: Gain::Exp { amount: 20 }, party: false }, Line { gain: Gain::Exp { amount: 4 }, party: true }]
        );
    }

    #[test]
    fn the_name_read_most_often_names_the_item() {
        let reads: Vec<Parsed> =
            ["Snail Shel x1 earned. (Etc)", "Snail Shell x1 earned. (Etc)", "Snail Shell x1 earned. (Etc)"].iter().filter_map(|t| parse(t)).collect();
        assert!(matches!(vote(&reads), Some(Gain::Item { name, .. }) if name == "Snail Shell"));
    }
}
