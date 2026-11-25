//! What Leafline sees: when each reader last gave a reading Leafline kept, what the last picture of
//! the game was like (status bar found, HDR, UI size, black or frozen), and a short list of the
//! readings left out as doubtful, with why. The program's live loop and the website's capture
//! worker keep one each as they read; the windows show it as "What Leafline sees" and the setup's
//! reading check ticks off what it reads. It only reports: nothing here changes what is read, and
//! nothing leaves the PC.

use serde::{Deserialize, Serialize};

use crate::pipeline::Analysis;
use crate::session::{Rejected, SessionSummary};
use crate::vision::Frame;

/// Emitted to every window with the latest [`Sight`].
pub const SIGHT_EVENT: &str = "sight";

/// Readings kept in the list of those left out.
const REJECTED_KEPT: usize = 40;

/// One part of the game Leafline reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Reader {
    /// The status bar along the bottom, found at all.
    StatusBar,
    Level,
    Exp,
    Name,
    Mesos,
    /// The minimap's title.
    Map,
    QuestHelper,
    PickupLog,
    ItemTooltip,
    /// The Character Stat and Skill windows.
    CharacterWindows,
    Inventory,
    QuestWindow,
}

/// When a reader last gave a reading that was kept, Unix seconds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Seen {
    pub reader: Reader,
    pub at: f64,
}

/// A reading left out as doubtful.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rejection {
    pub reader: Reader,
    pub at: f64,
    /// Why, in a few plain words, with what was read where it is known.
    pub reason: String,
}

/// What the last picture of the game was like.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Picture {
    pub at: f64,
    pub width: u32,
    pub height: u32,
    /// The status bar was found in it.
    pub status_bar: bool,
    /// Windows HDR is on for the game's display, which washes the picture out.
    pub hdr: bool,
    /// The game's UI Size draws its text smoothed (Default Ratio).
    pub smoothed_ui: bool,
    /// Nearly all black: the capture sees the window but not the game drawn in it.
    pub dark: bool,
    /// The same picture over and over: the capture isn't following the game.
    pub still: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sight {
    /// Each reader that has read anything since reading started, with when it last did.
    pub seen: Vec<Seen>,
    /// The readings left out as doubtful, the newest first.
    pub rejected: Vec<Rejection>,
    pub picture: Option<Picture>,
    /// When reading started, Unix seconds.
    pub since: f64,
    /// The tracker's counts of readings left out, at the last look.
    #[serde(skip)]
    counted: Rejected,
}

impl Sight {
    pub fn new(at: f64) -> Self {
        Self { since: at, ..Self::default() }
    }

    /// When `reader` last read, if it has.
    pub fn last(&self, reader: Reader) -> Option<f64> {
        self.seen.iter().find(|s| s.reader == reader).map(|s| s.at)
    }

    /// Notes that `reader` gave a reading at `at`.
    pub fn saw(&mut self, reader: Reader, at: f64) {
        match self.seen.iter_mut().find(|s| s.reader == reader) {
            Some(seen) => seen.at = seen.at.max(at),
            None => self.seen.push(Seen { reader, at }),
        }
    }

    /// Notes a reading left out, and why.
    pub fn reject(&mut self, reader: Reader, at: f64, reason: impl Into<String>) {
        self.rejected.insert(0, Rejection { reader, at, reason: reason.into() });
        self.rejected.truncate(REJECTED_KEPT);
    }

    /// Notes what one picture was like, and which of the HUD's readers read in it.
    pub fn picture(&mut self, at: f64, frame: &Frame, analysis: &Analysis, hdr: bool, smoothed_ui: bool) {
        let status_bar = analysis.hud_layout.is_some();
        self.picture = Some(Picture {
            at,
            width: frame.width(),
            height: frame.height(),
            status_bar,
            hdr,
            smoothed_ui,
            dark: !status_bar && mostly_dark(frame),
            still: false,
        });
        if status_bar {
            self.saw(Reader::StatusBar, at);
        }
        if let Some(hud) = &analysis.hud {
            if hud.level.is_some() {
                self.saw(Reader::Level, at);
            }
            if hud.exp.is_some() || hud.exp_percent.is_some() {
                self.saw(Reader::Exp, at);
            }
            if hud.name.is_some() {
                self.saw(Reader::Name, at);
            }
        }
        if analysis.mesos.is_some() {
            self.saw(Reader::Mesos, at);
        }
        if analysis.map.is_some() {
            self.saw(Reader::Map, at);
        }
        if analysis.quest.as_ref().is_some_and(|q| !q.quests.is_empty()) {
            self.saw(Reader::QuestHelper, at);
        }
        if !analysis.tooltips.is_empty() {
            self.saw(Reader::ItemTooltip, at);
        }
    }

    /// Notes the readings the session tracker left out since the last look, as doubtful: an EXP
    /// reading that didn't fit the ones before it, or a meso amount never read the same twice.
    /// `analysis` is the picture just pushed, whose readings were most likely the ones dropped.
    pub fn session(&mut self, at: f64, session: &SessionSummary, analysis: &Analysis) {
        let now = session.rejected;
        if now.exp > self.counted.exp {
            let read = analysis.hud.as_ref().and_then(|h| match (h.level, h.exp, h.exp_percent) {
                (level, Some(exp), _) => Some(format!("{}{} EXP ({:.2}%)", lv(level), exp.points, exp.percent)),
                (level, None, Some(percent)) => Some(format!("{}{percent:.2}%", lv(level))),
                _ => None,
            });
            let reason = match read {
                Some(read) => format!("An EXP reading didn't fit the ones around it and was left out (read as {read})."),
                None => "An EXP reading didn't fit the ones around it and was left out.".into(),
            };
            for _ in 0..(now.exp - self.counted.exp).min(3) {
                self.reject(Reader::Exp, at, reason.clone());
            }
        }
        if now.mesos > self.counted.mesos {
            self.reject(Reader::Mesos, at, "A meso amount that wasn't read the same twice was left out.");
        }
        // A new session starts its counts again.
        self.counted = now;
    }
}

fn lv(level: Option<u32>) -> String {
    level.map(|l| format!("Lv {l}, ")).unwrap_or_default()
}

/// Nearly all black, from a sample of its pixels: what a capture gives when it sees the window but
/// not what the game draws in it.
pub fn mostly_dark(frame: &Frame) -> bool {
    let (w, h) = frame.dimensions();
    if w == 0 || h == 0 {
        return true;
    }
    let (mut sum, mut n) = (0u64, 0u64);
    let step_x = (w / 64).max(1);
    let step_y = (h / 48).max(1);
    for y in (0..h).step_by(step_y as usize) {
        for x in (0..w).step_by(step_x as usize) {
            let p = frame.get_pixel(x, y).0;
            sum += u64::from(p[0]) + u64::from(p[1]) + u64::from(p[2]);
            n += 3;
        }
    }
    n > 0 && sum / n < 8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::readers::hud::{Exp, HudReading};

    #[test]
    fn keeps_the_last_read_of_each_reader_and_the_newest_rejections_first() {
        let mut sight = Sight::new(100.0);
        sight.saw(Reader::Level, 101.0);
        sight.saw(Reader::Level, 105.0);
        sight.saw(Reader::Map, 103.0);
        assert_eq!(sight.last(Reader::Level), Some(105.0));
        assert_eq!(sight.last(Reader::Mesos), None);

        for i in 0..50 {
            sight.reject(Reader::Exp, f64::from(i), "doubtful");
        }
        assert_eq!(sight.rejected.len(), REJECTED_KEPT);
        assert_eq!(sight.rejected[0].at, 49.0);
    }

    #[test]
    fn the_trackers_left_out_readings_become_rejections_with_what_was_read() {
        let mut sight = Sight::new(0.0);
        let analysis = Analysis {
            hud: Some(HudReading { level: Some(12), exp: Some(Exp { points: 830, percent: 41.5 }), ..HudReading::default() }),
            ..Analysis::default()
        };
        let mut session = SessionSummary::default();
        sight.session(1.0, &session, &analysis);
        assert!(sight.rejected.is_empty());
        session.rejected = Rejected { exp: 1, mesos: 1 };
        sight.session(2.0, &session, &analysis);
        assert_eq!(sight.rejected.len(), 2);
        assert!(sight.rejected.iter().any(|r| r.reader == Reader::Exp && r.reason.contains("Lv 12, 830 EXP (41.50%)")));
        // Counted once.
        sight.session(3.0, &session, &analysis);
        assert_eq!(sight.rejected.len(), 2);
    }

    #[test]
    fn a_black_picture_is_dark_and_a_game_picture_is_not() {
        let black = Frame::new(320, 240);
        assert!(mostly_dark(&black));
        let lit = Frame::from_pixel(320, 240, image::Rgba([90, 120, 160, 255]));
        assert!(!mostly_dark(&lit));
    }
}
