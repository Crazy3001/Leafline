//! The Pickup Log: the fading lines at the bottom right of the play area that say what the
//! player just gained. `You have gained mesos (+27)`, `Leaf x1 earned. (Etc)`, `You received
//! EXP (+21)`. Only the player's own gains show there; it is not the chat box.
//!
//! [`look`] finds the log and sees its rows, [`stack`] works out how many lines arrived from how
//! far the stack moved, [`follow`] reads each new line a few times and votes, and [`parse`] reads
//! one line's words. A line Leafline isn't sure of is left out: loot may be counted short, never
//! over.

pub mod follow;
pub mod look;
pub mod parse;
pub mod stack;

pub use follow::{Arrival, Follower, Line};
pub use look::Layout;
pub use parse::{Gain, Parsed};
pub use stack::{Colour, Row, Rows, Shape};

use crate::vision::{Frame, Ocr};

/// Font pixels to a square of the picture handed to OCR. Of 2 to 6, 3 read the most lines of
/// the recording of 2026-11-15 (about three in four at the first read).
const OCR_ZOOM: u32 = 3;
/// How long a log found but never read as one is kept, in seconds, before it is looked for
/// again: it may have been something else drawn the same way.
const UNCONFIRMED_FOR: f64 = 3.0;

/// Reads the Pickup Log from a stream of pictures of the game window, in order.
#[derive(Debug, Default)]
pub struct PickupReader {
    layout: Option<Layout>,
    /// When the layout was found, until a row of it has read as a log line.
    unconfirmed_since: Option<f64>,
    size: (u32, u32),
    follower: Follower,
}

/// What a picture showed of the log.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PickupRead {
    /// The lines settled since the last picture, oldest first.
    pub arrivals: Vec<Arrival>,
    /// Where the log is, once found and read.
    pub layout: Option<Layout>,
}

impl PickupReader {
    /// Reads `frame`, taken at `at` seconds.
    pub fn read(&mut self, at: f64, frame: &Frame, ocr: &Ocr) -> PickupRead {
        self.read_with(at, frame, |picture| read_text(ocr, picture))
    }

    /// [`Self::read`] with the OCR given, for tests.
    pub fn read_with(&mut self, at: f64, frame: &Frame, mut text: impl FnMut(&Frame) -> Option<String>) -> PickupRead {
        if frame.dimensions() != self.size {
            *self = Self { size: frame.dimensions(), ..Self::default() };
        }
        let mut read_slot = |layout: &Layout, slot: usize| {
            let picture = look::line_picture(frame, layout, slot, OCR_ZOOM)?;
            parse::parse(&text(&picture)?)
        };
        let Some(mut layout) = self.layout.or_else(|| look::locate(frame)) else {
            return PickupRead::default();
        };
        // Found while its bottom row was empty: the bottom is lower than it seemed.
        if look::line_below(frame, &layout) {
            layout = layout.lower();
            self.follower.reset();
            self.unconfirmed_since.get_or_insert(at);
        }
        if self.layout.is_none() {
            self.unconfirmed_since = Some(at);
        }
        self.layout = Some(layout);
        let rows = look::rows(frame, &layout);
        if let Some(since) = self.unconfirmed_since {
            // Nothing counts until one of its rows reads as a log line.
            let brightest = (0..rows.len())
                .filter_map(|slot| rows[slot].as_ref().map(|r| (r.brightness, slot)))
                .max_by(|a, b| a.0.total_cmp(&b.0));
            if brightest.is_some_and(|(_, slot)| read_slot(&layout, slot).is_some()) {
                self.unconfirmed_since = None;
                self.follower.reset();
                self.follower.push(at, rows, |_| None);
                return PickupRead { arrivals: Vec::new(), layout: Some(layout) };
            }
            if at - since >= UNCONFIRMED_FOR {
                self.layout = None;
                self.unconfirmed_since = None;
            }
            return PickupRead::default();
        }
        let arrivals = self.follower.push(at, rows, |slot| read_slot(&layout, slot));
        PickupRead { arrivals, layout: Some(layout) }
    }

    /// Settles every line followed, for the end of reading.
    pub fn finish(&mut self) -> Vec<Arrival> {
        self.follower.finish()
    }

    /// Starts afresh, for a new session or character: lines on screen came before it. The log's
    /// place is kept.
    pub fn reset(&mut self) {
        self.follower.reset();
    }
}

/// The text OCR reads in a line's picture, its words in order.
fn read_text(ocr: &Ocr, picture: &Frame) -> Option<String> {
    let mut words = ocr.words(picture).ok()?;
    words.sort_by(|a, b| a.rect.x.total_cmp(&b.rect.x));
    let text = words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ");
    (!text.is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::look::tests::{draw, glyphs, world};
    use super::*;

    const WHITE: [u8; 3] = [246, 246, 245];

    /// A picture of the log with `lines` (id, alpha) bottom first.
    fn picture(lines: &[(u32, f32)]) -> Frame {
        let mut frame = world(800, 450);
        for (slot, &(id, alpha)) in lines.iter().enumerate() {
            draw(&mut frame, &glyphs(id, 80 + id as usize * 7 % 40), 790, 380 - slot as u32 * 28, 2, alpha, WHITE);
        }
        frame
    }

    /// OCR that reads a made-up line's picture by its width, which differs for each id.
    fn text_by_width(picture: &Frame) -> Option<String> {
        Some(format!("You have gained mesos (+{})", picture.width()))
    }

    #[test]
    fn counts_each_line_once_after_the_log_is_confirmed() {
        let mut reader = PickupReader::default();
        let mut at = 0.0;
        let mut arrived = Vec::new();
        let mut step = |reader: &mut PickupReader, lines: &[(u32, f32)]| {
            at += 0.25;
            arrived.extend(reader.read_with(at, &picture(lines), text_by_width).arrivals);
        };
        step(&mut reader, &[(1, 1.0)]);
        step(&mut reader, &[(1, 0.95)]);
        step(&mut reader, &[(2, 1.0), (1, 0.9)]);
        for k in 0..6 {
            step(&mut reader, &[(2, 0.95 - k as f32 * 0.05), (1, 0.85 - k as f32 * 0.05)]);
        }
        step(&mut reader, &[(3, 1.0), (2, 0.6), (1, 0.5)]);
        arrived.extend(reader.finish());
        assert_eq!(arrived.len(), 2, "line 1 was there before; lines 2 and 3 arrived: {arrived:?}");
        assert!(arrived.iter().all(|a| a.line.is_some()));
    }

    #[test]
    fn a_log_that_never_reads_as_one_counts_nothing() {
        let mut reader = PickupReader::default();
        for k in 0..20 {
            let read = reader.read_with(k as f64 * 0.25, &picture(&[(k % 3 + 1, 1.0)]), |_| Some("Swiftleaf: hi".into()));
            assert!(read.arrivals.is_empty());
        }
    }
}
