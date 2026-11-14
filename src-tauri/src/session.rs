//! The session tracker: EXP and meso rates over a run of timestamped readings, overall and per
//! map.
//!
//! Readings come from OCR and are sometimes wrong, so none is taken at face value:
//!
//! - **EXP points and percent must agree.** Together they imply how much EXP the level needs
//!   (`points / percent`), which cannot change within a level. A reading that implies a
//!   different amount is a misread and is dropped. The amount is trusted once two readings
//!   agree on it, and the tracker only starts then.
//! - **Levels only go up**, and a level-up counts once the EXP percent has wrapped round, or
//!   two readings in a row agree on the new level.
//! - **The level need not be readable.** Its digits are learned one by one from the player's
//!   screen, so a new character's are often unknown. EXP is followed anyway: two readings in a
//!   row that agree with each other but imply a different amount for the level are a level-up,
//!   and the first level actually read lines the count up.
//! - **Mesos count once several readings in a row agree**, and a big jump needs more of them.
//!   Changes while a shop, storage or trade window is open are kept apart, since they are not
//!   loot. The first amount such a window shows is from before anything was bought, so the
//!   change up to it is loot and one reading of it is enough. The mesos need not stay on
//!   screen: a gain counts over the active time between the two looks either side of it.
//! - **Quest counters count the same way**, and only going up: a counter that drops (a quest
//!   handed in and taken again) starts afresh. Their gains over active time give the player's
//!   own pace on each objective, per map.
//! - **The clock pauses** after [`TrackerConfig::idle_after`] seconds without EXP, and at once
//!   on a map change without EXP, so time in town or away does not dilute the rates.
//! - **EXP lost to a death is left out of the rates.** A drop that two readings agree on is
//!   counted as a death and tracking carries on from the new amount.
//! - **Past a certain level the HUD shows only a percentage.** Progress is then followed in
//!   percent of a level, which needs no EXP table: EXP per hour stays blank, while percent per
//!   hour and the time to the next level keep working.
//!
//! A missing reading is skipped, never guessed. No EXP table is built in: the EXP each level
//! needs is measured from the HUD itself, since Classic's table may differ from old GMS.

use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

use crate::pipeline::Analysis;
use crate::readers::hud::{self, Exp};
use crate::readers::map::MapReading;

/// What one frame says, at one moment.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sample {
    /// Seconds from any fixed point, such as the Unix epoch.
    pub at: f64,
    pub level: Option<u32>,
    pub exp: Option<Exp>,
    /// The EXP percentage, when the HUD shows no points beside it.
    pub percent: Option<f32>,
    pub mesos: Option<u64>,
    /// A shop, storage or trade window is open, so meso changes are not loot.
    pub exchange: bool,
    pub map: Option<String>,
    /// The map is a town, where nothing is worth logging.
    pub town: bool,
    pub name: Option<String>,
    pub job: Option<String>,
    /// How many are in the party, the player included.
    pub party: Option<u32>,
    /// The Quest Helper's counters.
    pub counters: Vec<Counter>,
    /// The base EXP of each monster on the map, when the map data knows it. Gains are matched
    /// against these to count kills and to tell quest EXP from hunting.
    pub kill_exp: Vec<u32>,
}

/// A quest counter as read: what it counts and how far along it is.
#[derive(Debug, Clone, PartialEq)]
pub struct Counter {
    pub target: String,
    pub current: u32,
    pub required: u32,
}

impl Sample {
    pub fn from_analysis(at: f64, analysis: &Analysis) -> Self {
        let hud = analysis.hud.as_ref();
        Self {
            at,
            level: hud.and_then(|h| h.level),
            exp: hud.and_then(|h| h.exp),
            percent: hud.and_then(|h| h.exp_percent),
            mesos: analysis.mesos.map(|m| m.mesos),
            exchange: analysis.meso_boxes >= 2 || analysis.shop_open,
            map: analysis.map.as_ref().map(MapReading::label),
            town: false,
            name: hud.and_then(|h| h.name.clone()),
            job: hud.and_then(|h| h.job.clone()).filter(|job| hud::is_known_job(job)),
            party: Some(analysis.party_size),
            counters: analysis
                .quest
                .iter()
                .flat_map(|q| &q.quests)
                .flat_map(|q| &q.objectives)
                // A line no reading has counted holds a 0 that isn't one: its first real count
                // would look like a burst of kills.
                .filter(|o| !o.unread)
                .map(|o| Counter { target: o.target.clone(), current: o.current, required: o.required })
                .collect(),
            kill_exp: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TrackerConfig {
    /// Seconds without EXP after which the clock pauses until the next gain.
    pub idle_after: f64,
    /// Readings in a row that must agree before a meso amount or quest counter counts. Live
    /// capture should use 2 or more; a folder of hand-taken screenshots, 1.
    pub confirmations: usize,
    /// Rates are shown only after this much active time, so one early kill does not read as a
    /// huge hourly rate.
    pub min_rate_seconds: f64,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self { idle_after: 15.0, confirmations: 2, min_rate_seconds: 30.0 }
    }
}

/// Active seconds behind the "last 5 minutes" rates.
const RECENT_SECONDS: f64 = 300.0;
/// Active seconds between points on the trend line, and how many it keeps.
const TREND_STEP: f64 = 15.0;
const TREND_POINTS: usize = 60;
/// Active seconds between points on the whole-session timeline, at first. When the timeline
/// fills, every other point goes and the step doubles, so a long session still fits.
const TIMELINE_STEP: f64 = 30.0;
const TIMELINE_POINTS: usize = 960;
/// Seconds without a look at the mesos after which a big change is believed at the usual count.
const MESO_GAP: f64 = 120.0;
/// Seconds after a shop, storage or trade window opens in which the first amount read in it is
/// still taken as the one from before the window. Buying takes a click and a confirmation, so a
/// first reading this soon almost always comes before it.
const FRESH_WINDOW: f64 = 10.0;
/// Seconds without the minimap's name before gains stop going to the last map read.
const MAP_BLIND_SECONDS: f64 = 45.0;
/// ...and before the player is told.
const MAP_HIDDEN_SECONDS: f64 = 6.0;
/// Readings in a row that must say the same before the party size or job changes.
const STEADY_READS: usize = 2;
/// Readings in a row of a lower level or another name that mean another character.
const OTHER_CHARACTER_READS: usize = 3;
/// A reading a second can catch a few kills at once; more than this in one gain is not taken as
/// kills.
const MAX_KILLS_PER_GAIN: u32 = 4;
/// A gain that is no sum of kills and over this many of the map's biggest kill is quest EXP (or
/// the like), not hunting.
const QUEST_GAIN_KILLS: f64 = 4.0;
/// Seconds of play an estimate counts for in the pace: the measured pace takes over as play
/// goes on, outweighing it four to one after a quarter of an hour.
const PRIOR_SECONDS: f64 = 180.0;
/// Active seconds on the current map before its own pace is used for the time to level.
const MAP_PACE_SECONDS: f64 = 120.0;
/// Gains behind a pace before a range is given for it, and how wide the range is: 1.28 standard
/// errors either side holds the true pace four times in five.
const RANGE_GAINS: u32 = 10;
const RANGE_Z: f64 = 1.28;
/// A wait between two frames longer than this is time away (the app closed or the game hidden),
/// not a break in play.
const REAL_GAP: f64 = 120.0;
/// Seconds of play, breaks included, before the time to level with breaks is given.
const BREAKS_AFTER: f64 = 1200.0;
/// The gaps between gains kept to tell how long a kill usually takes, and the guess before any.
const GAPS_KEPT: usize = 20;
const USUAL_GAP: f64 = 5.0;

/// A value that changes only once several readings in a row agree on the new one.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Steady<T> {
    value: Option<T>,
    candidate: Option<(T, usize)>,
}

impl<T> Default for Steady<T> {
    fn default() -> Self {
        Self { value: None, candidate: None }
    }
}

impl<T: Clone + PartialEq> Steady<T> {
    fn see(&mut self, reading: T, needed: usize) {
        if self.value.as_ref() == Some(&reading) {
            self.candidate = None;
            return;
        }
        let count = match &self.candidate {
            Some((value, count)) if *value == reading => count + 1,
            _ => 1,
        };
        if count >= needed {
            self.value = Some(reading);
            self.candidate = None;
        } else {
            self.candidate = Some((reading, count));
        }
    }
}

/// Everything the tracker has worked out so far.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub frames: usize,
    pub started_at: Option<f64>,
    pub updated_at: Option<f64>,
    /// From the first frame to the last.
    pub elapsed_seconds: f64,
    /// Time spent gaining EXP, with idle stretches and map changes paused out.
    pub active_seconds: f64,
    /// The clock is paused now.
    pub paused: bool,
    /// The player paused tracking by hand.
    pub held: bool,
    pub level: Option<u32>,
    pub exp: Option<Exp>,
    /// The latest EXP percentage, read with or without points.
    pub percent: Option<f32>,
    /// The HUD shows only a percentage, so EXP amounts are unknown.
    pub percent_only: bool,
    /// The EXP the current level needs, as measured from the HUD.
    pub exp_needed: Option<u64>,
    /// EXP gained; EXP lost to deaths is counted apart.
    pub exp_gained: i64,
    /// EXP that was no kill: quests handed in and the like. Left out of every rate.
    pub exp_other: i64,
    /// Kills counted from gains that match the map's monsters, solo only.
    pub kills: u64,
    /// Solo gains on maps the data knows that did and didn't match a sum of its monsters' EXP.
    /// Many misses would mean kills give other than the data's base EXP.
    pub gains_matched: u32,
    pub gains_unmatched: u32,
    pub exp_per_hour: Option<f64>,
    pub percent_per_hour: Option<f64>,
    /// Seconds to the next level at [`Self::pace_exp_per_hour`], in active time.
    pub seconds_to_level: Option<f64>,
    /// The same at the range's slow and fast ends, once enough gains are behind the pace.
    pub seconds_to_level_low: Option<f64>,
    pub seconds_to_level_high: Option<f64>,
    /// The same with breaks counted at this session's share of them.
    pub seconds_to_level_with_breaks: Option<f64>,
    /// The pace behind the time to level: this map's EXP an hour less deaths, eased in from an
    /// estimate when there is one, or the session's while this map has too little.
    pub pace_exp_per_hour: Option<f64>,
    /// How far the pace could be out, as a share of it, for a range of ±1.28 times this.
    pub pace_error: Option<f64>,
    /// An estimate still counts for more than a tenth of the pace.
    pub pace_from_estimate: bool,
    /// Kills an hour on this map, when kills were counted.
    pub kills_per_hour: Option<f64>,
    /// Seconds from the first reading to the latest, breaks in play included, time away not.
    pub real_seconds: f64,
    /// EXP an hour less deaths over that time.
    pub exp_per_hour_with_breaks: Option<f64>,
    /// Rates over the last five minutes of active time.
    pub recent_exp_per_hour: Option<f64>,
    pub recent_percent_per_hour: Option<f64>,
    pub recent_mesos_per_hour: Option<f64>,
    /// The recent EXP rate (or percent rate at high levels) over time, oldest first.
    pub trend: Vec<f64>,
    /// The recent rates over the whole session, oldest first, for graphs.
    pub timeline: Vec<TimelinePoint>,
    /// Level-ups and deaths, in order.
    pub events: Vec<SessionEvent>,
    pub level_ups: u32,
    pub deaths: u32,
    /// EXP lost to deaths, left out of the rates.
    pub exp_lost: u64,
    /// Percent of a level lost to deaths.
    pub percent_lost: f64,
    /// The latest confirmed amount.
    pub mesos: Option<u64>,
    /// Seconds since the mesos were last on screen.
    pub mesos_age: Option<f64>,
    /// Net mesos gained outside shop, storage and trade windows.
    pub mesos_gained: i64,
    pub mesos_per_hour: Option<f64>,
    /// The active time the meso rate is over: from one look at the mesos to a later one that
    /// shows a change. Less than `active_seconds` when the mesos were not looked at for a while.
    pub mesos_seconds: f64,
    /// Active seconds since the mesos were last on screen, or since the start when they never
    /// were. Mesos gained in that time are not counted yet.
    pub mesos_unseen_seconds: f64,
    /// Net change while one of those windows was open, left out of the rates.
    pub mesos_excluded: i64,
    pub map: Option<String>,
    /// The map is a town.
    pub town: bool,
    /// The minimap's name has not been seen for a while, so the map may be out of date.
    pub map_hidden: bool,
    /// The character's name, once read the same a few times in a row.
    pub name: Option<String>,
    pub job: Option<String>,
    pub party: Option<u32>,
    /// Per map, most active time first.
    pub maps: Vec<MapSummary>,
    /// Per map, level, party size and job, most active time first.
    pub lines: Vec<SessionLine>,
    /// Quest counter gains per objective and map, most active time first.
    pub counters: Vec<CounterRate>,
    /// Readings dropped as misreads.
    pub rejected: Rejected,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapSummary {
    /// `None` collects time and gains that cannot be placed: before any map was read, and
    /// spans that started on one map and ended on another.
    pub map: Option<String>,
    pub active_seconds: f64,
    pub exp_gained: i64,
    pub exp_per_hour: Option<f64>,
    pub mesos_gained: i64,
    pub mesos_per_hour: Option<f64>,
}

/// A stretch of play on one map at one level, party size and job.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SessionLine {
    pub map: Option<String>,
    pub town: bool,
    pub level: Option<u32>,
    pub party: Option<u32>,
    pub job: Option<String>,
    pub active_seconds: f64,
    pub exp_gained: i64,
    pub percent_gained: f64,
    pub mesos_gained: i64,
    pub mesos_seconds: f64,
    /// Kills counted from gains, `None` where none could be (a party, a map the data lacks).
    pub kills: Option<u32>,
    /// Quest EXP and the like, left out of `exp_gained`.
    pub other_exp: i64,
    pub deaths: u32,
    pub exp_lost: u64,
    /// Hunting gains on a map the data knows that matched no sum of its monsters' EXP, so the
    /// kills are short by about that many.
    pub gains_missed: u32,
    /// The character's damage and stats when the stretch began, as the estimates read them, so
    /// play can be compared with what the character could do then.
    pub fighter: Option<serde_json::Value>,
}

/// How fast one quest counter went up on one map.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterRate {
    pub target: String,
    pub required: u32,
    /// `None` for gains that spanned a map change.
    pub map: Option<String>,
    pub gained: u32,
    pub active_seconds: f64,
    /// Shown once the counter went up at least twice over enough active time.
    pub per_hour: Option<f64>,
}

/// The recent rates at one moment of a session.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelinePoint {
    /// Active seconds into the session.
    pub at: f64,
    pub exp_per_hour: Option<f64>,
    pub percent_per_hour: Option<f64>,
    pub mesos_per_hour: Option<f64>,
    pub level: Option<u32>,
    pub map: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EventKind {
    LevelUp,
    Death,
}

/// Something that happened at one moment of a session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionEvent {
    /// Active seconds into the session.
    pub at: f64,
    pub kind: EventKind,
    /// The level reached, for a level-up.
    pub level: Option<u32>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct Rejected {
    pub exp: usize,
    pub mesos: usize,
}

/// The EXP a level needs, as a range: the HUD's percent has two decimals, and whether it rounds
/// or truncates is not known, so the true percent is anywhere from 0.005 below to 0.01 above.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
struct Requirement {
    low: f64,
    high: f64,
}

impl Requirement {
    /// What a reading implies, or `None` at 0 points, which implies nothing.
    fn implied(exp: Exp) -> Option<Self> {
        if exp.points == 0 {
            return None;
        }
        let (points, percent) = (exp.points as f64, f64::from(exp.percent));
        let low = (points * 100.0 / (percent + 0.01)).max(points + 1.0);
        let high = if percent > 0.005 { points * 100.0 / (percent - 0.005) } else { f64::INFINITY };
        Some(Self { low, high })
    }

    fn intersect(self, other: Self) -> Option<Self> {
        let (low, high) = (self.low.max(other.low), self.high.min(other.high));
        (low <= high).then_some(Self { low, high })
    }

    fn estimate(self) -> Option<u64> {
        self.high.is_finite().then(|| ((self.low + self.high) / 2.0).round() as u64)
    }
}

/// Whether two readings could be on the same level; a reading that implies nothing fits any.
fn agree(a: Option<Requirement>, b: Option<Requirement>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a.intersect(b).is_some(),
        _ => true,
    }
}

fn narrow(a: Option<Requirement>, b: Option<Requirement>) -> Option<Requirement> {
    match (a, b) {
        (Some(a), Some(b)) => a.intersect(b).or(Some(a)),
        (a, b) => a.or(b),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExpPoint {
    at: f64,
    level: u32,
    exp: Exp,
    map: Option<String>,
}

/// Progress when the HUD shows only a percentage.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PercentPoint {
    at: f64,
    level: u32,
    percent: f32,
    map: Option<String>,
}

/// Running totals at one moment of active time, for the recent rates.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Mark {
    clock: f64,
    exp: i64,
    percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MesoPoint {
    mesos: u64,
    /// The active clock when this amount first appeared.
    clock: f64,
    map: Option<String>,
}

/// A shop, storage or trade window that is open now.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenWindow {
    opened_at: f64,
    /// An amount has been read in it.
    read: bool,
    /// The amount read in its latest frame, if one was.
    last: Option<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct MapTotals {
    map: Option<String>,
    active_seconds: f64,
    exp_gained: i64,
    mesos_gained: i64,
    meso_seconds: f64,
    kills: u64,
    exp_lost: u64,
    /// Hunting gains, and their sum and sum of squares, for how steady the pace is.
    gains: Gains,
}

/// How many gains there were and how much they varied: the spread of a pace comes from both.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
struct Gains {
    count: u32,
    sum: f64,
    squares: f64,
}

impl Gains {
    fn add(&mut self, gain: f64) {
        self.count += 1;
        self.sum += gain;
        self.squares += gain * gain;
    }

    /// The pace's likely error as a share of it: with n gains whose sizes vary by v (standard
    /// deviation over mean), about √((1 + v²) / n). `None` with too few gains to say.
    fn error(&self) -> Option<f64> {
        if self.count < RANGE_GAINS || self.sum <= 0.0 {
            return None;
        }
        let n = f64::from(self.count);
        let mean = self.sum / n;
        let variance = (self.squares / n - mean * mean).max(0.0);
        let v2 = variance / (mean * mean);
        Some(((1.0 + v2) / n).sqrt())
    }
}

/// The pace the time to level goes by, with how sure it is.
#[derive(Debug, Clone, Copy)]
struct Pace {
    exp_per_hour: Option<f64>,
    error: Option<f64>,
    from_estimate: bool,
    kills_per_hour: Option<f64>,
}

/// How a gain of EXP is counted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GainKind {
    /// This many kills of the map's monsters.
    Kills(u32),
    /// Hunting that matched no sum of kills, or couldn't be checked.
    Hunting { checked: bool },
    /// Too big to be hunting: a quest handed in, or the like.
    Other,
}

/// The fewest kills of monsters worth `values` EXP that add up to `gain`, up to
/// [`MAX_KILLS_PER_GAIN`]. Each kill may be one EXP out either way, for rounding.
fn kills_in(gain: u64, values: &[u32]) -> Option<u32> {
    /// Whether `kills` monsters from `values[from..]` can add up to between `low` and `high`.
    fn reach(low: i64, high: i64, values: &[u32], kills: u32, from: usize) -> bool {
        if kills == 0 {
            return low <= 0 && 0 <= high;
        }
        high >= 0
            && (from..values.len()).any(|i| {
                let v = i64::from(values[i]);
                reach(low - v, high - v, values, kills - 1, i)
            })
    }
    if gain == 0 {
        return None;
    }
    let mut values: Vec<u32> = values.iter().copied().filter(|&v| v > 0).collect();
    values.sort_unstable();
    values.dedup();
    let gain = gain as i64;
    (1..=MAX_KILLS_PER_GAIN).find(|&kills| {
        let slack = i64::from(kills);
        reach(gain - slack, gain + slack, &values, kills, 0)
    })
}

/// Sorts a gain made solo on a map whose monsters are worth `values`.
fn classify(gain: u64, values: &[u32]) -> GainKind {
    if values.is_empty() {
        return GainKind::Hunting { checked: false };
    }
    if let Some(kills) = kills_in(gain, values) {
        return GainKind::Kills(kills);
    }
    let biggest = values.iter().copied().max().unwrap_or(0);
    if gain as f64 > QUEST_GAIN_KILLS * f64::from(biggest) {
        GainKind::Other
    } else {
        GainKind::Hunting { checked: true }
    }
}

/// One quest counter as tracked: the value being read, and the last confirmed one.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CounterTrack {
    target: String,
    required: u32,
    /// The value being read and how many readings in a row agree on it.
    run: (u32, usize),
    /// The last confirmed value, the active clock then, and the map.
    confirmed: Option<(u32, f64, Option<String>)>,
    /// Seen in the latest frame that showed the Quest Helper.
    seen: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionTracker {
    #[serde(skip)]
    config: TrackerConfig,
    /// The base EXP of the current map's monsters, from the last reading that had them.
    kill_exp: Vec<u32>,
    kills: u64,
    exp_other: i64,
    gains_matched: u32,
    gains_unmatched: u32,
    /// Hunting gains over the whole session, for the pace's range when a map has too few.
    gains: Gains,
    /// Recent gaps between hunting gains, for how long a kill usually takes.
    gaps: VecDeque<f64>,
    /// The map changed since the last gain.
    new_map: bool,
    /// Seconds of play, breaks included, and when the last frame that read anything came.
    real_clock: f64,
    last_read_at: Option<f64>,
    /// An estimate of the pace on a map, from earlier play or the training estimates.
    prior: Option<(String, f64)>,
    /// The character's damage and stats as the estimates read them, copied into new lines.
    fighter: Option<serde_json::Value>,
    frames: usize,
    started_at: Option<f64>,
    updated_at: Option<f64>,
    /// The last map read, carried over frames where the minimap is hidden.
    map: Option<String>,

    /// A level has been read. Until then, levels are only counted from 0 and not reported.
    level_known: bool,
    /// The last accepted EXP reading.
    exp: Option<ExpPoint>,
    /// A reading that does not fit yet, kept in case the next one agrees with it.
    pending: Option<ExpPoint>,
    /// The EXP each level needs, narrowed with every accepted reading.
    requirements: Vec<(u32, Requirement)>,
    /// Active seconds so far.
    clock: f64,
    /// Seconds since EXP last went up.
    idle_for: f64,
    exp_gained: i64,
    level_ups: u32,
    deaths: u32,
    exp_lost: u64,
    percent_lost: f64,
    /// Percent of a level gained, across levels.
    percent_gained: f64,
    /// The last percentage-only reading. Set once the HUD stops showing points.
    percent_point: Option<PercentPoint>,
    /// A percentage that does not fit yet, kept in case the next one agrees with it.
    percent_pending: Option<PercentPoint>,
    /// Tracking was paused by hand: the time since the last reading doesn't count.
    held: bool,
    skip_gap: bool,

    /// When the map's name was last seen, and the same for the mesos.
    map_seen_at: Option<f64>,
    mesos_seen_at: Option<f64>,
    /// The active clock when the mesos were last seen.
    mesos_seen_clock: Option<f64>,
    town: bool,
    party: Steady<u32>,
    job: Steady<String>,
    name: Steady<String>,
    /// Readings in a row that looked like another character.
    other_character: usize,
    /// The name read last, if it wasn't this character's.
    stranger: Option<String>,

    /// Running totals, newest last, reaching a little past the recent window.
    marks: VecDeque<Mark>,
    trend: VecDeque<f64>,
    trend_clock: f64,
    timeline: Vec<TimelinePoint>,
    timeline_clock: f64,
    /// Active seconds between timeline points now; 0 until the first point.
    timeline_step: f64,
    events: Vec<SessionEvent>,

    /// The amount being read and how many readings in a row agree on it.
    meso_run: Option<(MesoPoint, usize)>,
    mesos: Option<MesoPoint>,
    /// A shop, storage or trade window was seen since the last confirmed amount.
    exchange_seen: bool,
    window: Option<OpenWindow>,
    mesos_gained: i64,
    meso_seconds: f64,
    mesos_excluded: i64,
    /// Each look at the mesos as (active clock, mesos gained by then), oldest first. Between two
    /// looks the gain is spread evenly, since the loot came in all along, not at the second look.
    meso_looks: VecDeque<(f64, i64)>,

    maps: Vec<MapTotals>,
    lines: Vec<SessionLine>,
    counters: Vec<CounterTrack>,
    counter_totals: Vec<CounterRate>,
    rejected: Rejected,
}

impl SessionTracker {
    pub fn new(config: TrackerConfig) -> Self {
        Self { config, ..Self::default() }
    }

    pub fn push(&mut self, sample: &Sample) {
        self.frames += 1;
        // A frame where nothing was read may not show the game at all; it does not stretch
        // the session.
        let read_any = sample.exp.is_some()
            || sample.percent.is_some()
            || sample.level.is_some()
            || sample.mesos.is_some()
            || sample.map.is_some();
        if read_any {
            self.started_at = Some(self.started_at.map_or(sample.at, |s| s.min(sample.at)));
            self.updated_at = Some(self.updated_at.map_or(sample.at, |u| u.max(sample.at)));
            // Breaks in play count; time away (the app closed, tracking held) doesn't.
            if let Some(last) = self.last_read_at {
                let gap = sample.at - last;
                if !self.skip_gap && (0.0..=REAL_GAP).contains(&gap) {
                    self.real_clock += gap;
                }
            }
            self.last_read_at = Some(sample.at);
        }
        if sample.map.is_some() {
            self.map.clone_from(&sample.map);
            self.map_seen_at = Some(sample.at);
            self.town = sample.town;
            self.kill_exp.clone_from(&sample.kill_exp);
        } else if self.map_seen_at.is_some_and(|seen| sample.at - seen >= MAP_BLIND_SECONDS) {
            // The player may have moved on: gains stop going to the old map.
            self.map = None;
            self.kill_exp.clear();
        }
        if let Some(party) = sample.party {
            self.party.see(party, STEADY_READS);
        }
        if let Some(job) = &sample.job {
            self.job.see(job.clone(), STEADY_READS);
        }
        if let Some(name) = &sample.name {
            self.name.see(name.clone(), OTHER_CHARACTER_READS);
        }
        self.push_exp(sample);
        if let Some(percent) = sample.percent {
            self.push_percent(sample, percent);
        }
        self.follow_window(sample);
        self.push_mesos(sample);
        if let Some(window) = &mut self.window {
            window.read |= sample.mesos.is_some();
            window.last = sample.mesos;
        }
        self.exchange_seen |= sample.exchange;
        self.push_counters(sample);
    }

    /// Notes a shop, storage or trade window opening or closing.
    fn follow_window(&mut self, sample: &Sample) {
        if sample.exchange {
            if self.window.is_none() {
                self.window = Some(OpenWindow { opened_at: sample.at, read: false, last: None });
            }
            return;
        }
        // A window that closed showing the amount counted last changed nothing after it, so the
        // next change is loot again, even if the inventory was not opened in between.
        if let Some(window) = self.window.take()
            && window.last.is_some()
            && window.last == self.mesos.as_ref().map(|m| m.mesos)
        {
            self.exchange_seen = false;
        }
    }

    /// The first amount read in a window that has just opened: it is from before anything in
    /// the window was bought, sold or moved.
    fn first_in_window(&self, sample: &Sample) -> bool {
        sample.exchange
            && self.window.as_ref().is_some_and(|w| !w.read && sample.at - w.opened_at <= FRESH_WINDOW)
    }

    /// Takes a new "seconds without EXP" setting.
    pub fn set_idle_after(&mut self, seconds: f64) {
        self.config.idle_after = seconds;
    }

    /// Gives an estimate of the EXP an hour on `map` (a minimap label), which the pace behind the
    /// time to level starts from until enough play there is measured. `None` forgets it.
    pub fn set_prior(&mut self, map: &str, exp_per_hour: Option<f64>) {
        self.prior = exp_per_hour.filter(|r| r.is_finite() && *r > 0.0).map(|r| (map.to_string(), r));
    }

    /// The map the tracker counts gains to now, if any.
    pub fn current_map(&self) -> Option<&str> {
        self.map.as_deref()
    }

    /// Takes what the estimates know of the character's damage and stats, to copy into the
    /// stretches of play from now on.
    pub fn set_fighter(&mut self, fighter: Option<serde_json::Value>) {
        self.fighter = fighter;
    }

    /// Stops or restarts tracking by hand. Readings are not pushed while held, and the time
    /// held doesn't count once they come back.
    /// Takes a saved tracker back (it serialises as a whole, see `serde_json::to_value`). The
    /// time the app was away is not counted: the next reading starts a new stretch, and the
    /// rates carry on from where they were.
    pub fn restore(config: TrackerConfig, saved: serde_json::Value) -> Option<Self> {
        let mut tracker: Self = serde_json::from_value(saved).ok()?;
        tracker.config = config;
        tracker.skip_gap = true;
        Some(tracker)
    }

    pub fn set_held(&mut self, held: bool) {
        self.held = held;
        self.skip_gap = true;
    }

    /// Whether this reading, with the ones before it, shows another character: a lower level or
    /// a different name, several readings in a row. Levels never go down and names never
    /// change, so the session belongs to someone else from here on.
    pub fn is_other_character(&mut self, sample: &Sample) -> bool {
        let tracked = self.level().filter(|_| self.level_known);
        let lower = sample.level.zip(tracked).is_some_and(|(level, tracked)| level < tracked);
        let renamed = sample.name.as_deref().zip(self.name.value.as_deref()).is_some_and(|(read, known)| {
            strsim::normalized_levenshtein(&read.to_lowercase(), &known.to_lowercase()) < 0.5
        });
        // OCR can garble a name once, but not the same way each time.
        let same_stranger = renamed && self.stranger == sample.name;
        self.stranger = sample.name.clone().filter(|_| renamed);
        if lower || same_stranger {
            self.other_character += 1;
        } else if sample.level.is_some() || sample.name.is_some() {
            self.other_character = 0;
        }
        self.other_character >= OTHER_CHARACTER_READS
    }

    fn level(&self) -> Option<u32> {
        self.percent_point.as_ref().map(|p| p.level).or_else(|| self.exp.as_ref().map(|e| e.level))
    }

    fn push_exp(&mut self, sample: &Sample) {
        let Some(exp) = sample.exp else { return };
        if !(0.0..100.0).contains(&exp.percent) {
            self.rejected.exp += 1;
            return;
        }
        let level = match sample.level {
            Some(level) => {
                self.line_up_levels(level, exp);
                level
            }
            None => self.expected_level(exp),
        };
        let point = ExpPoint { at: sample.at, level, exp, map: self.map.clone() };
        let Some(last) = self.exp.clone() else {
            // Nothing to check the first reading against: wait for a second that agrees.
            match self.pending.take() {
                Some(first)
                    if first.level == point.level
                        && agree(Requirement::implied(first.exp), Requirement::implied(point.exp)) =>
                {
                    self.set_requirement(first.level, Requirement::implied(first.exp));
                    self.exp = Some(first.clone());
                    self.accept(&first, point);
                }
                dropped => {
                    self.rejected.exp += usize::from(dropped.is_some());
                    self.pending = Some(point);
                }
            }
            return;
        };

        let implied = Requirement::implied(point.exp);
        if point.level == last.level {
            if !agree(self.requirement(level), implied) {
                self.rejected.exp += 1;
            } else if point.exp.points < last.exp.points {
                // Less EXP than before: a death, once a second reading agrees.
                match self.pending.take() {
                    Some(first)
                        if first.level == point.level && (first.exp.percent - point.exp.percent).abs() <= 1.0 =>
                    {
                        self.lose(&last, first.clone());
                        self.accept(&first, point);
                    }
                    dropped => {
                        self.rejected.exp += usize::from(dropped.is_some());
                        self.pending = Some(point);
                    }
                }
            } else {
                self.rejected.exp += usize::from(self.pending.take().is_some());
                self.accept(&last, point);
            }
        } else if point.level > last.level {
            let wrapped = point.level == last.level + 1 && point.exp.percent < last.exp.percent;
            let fits = agree(self.requirement(level), implied);
            match self.pending.take() {
                // A level that was only inferred needs a second reading, like any other surprise.
                _ if wrapped && fits && sample.level.is_some() => self.accept(&last, point),
                Some(first) if first.level == point.level && agree(Requirement::implied(first.exp), implied) => {
                    self.accept(&last, first.clone());
                    self.accept(&first, point);
                }
                dropped => {
                    self.rejected.exp += usize::from(dropped.is_some());
                    self.pending = Some(point);
                }
            }
        } else {
            // Levels never go down.
            self.rejected.exp += 1;
        }
    }

    /// The level `exp` belongs to when none could be read.
    fn expected_level(&self, exp: Exp) -> u32 {
        match (&self.exp, &self.pending) {
            (Some(last), _) => {
                // 0 points implies no amount for the level, but only a new level starts there.
                let restarted = exp.points == 0 && last.exp.points > 0;
                let fits = agree(self.requirement(last.level), Requirement::implied(exp));
                if fits && !restarted { last.level } else { last.level + 1 }
            }
            (None, Some(first)) => first.level,
            (None, None) => 0,
        }
    }

    /// The first level read gives the levels counted so far their real numbers.
    fn line_up_levels(&mut self, level: u32, exp: Exp) {
        if std::mem::replace(&mut self.level_known, true) {
            return;
        }
        let Some(base) = self.exp.as_ref().or(self.pending.as_ref()).map(|p| p.level) else { return };
        let fits = self.exp.is_none() || agree(self.requirement(base), Requirement::implied(exp));
        let anchor = if fits { level } else { level.saturating_sub(1) };
        let shift = |counted: u32| (i64::from(counted) + i64::from(anchor) - i64::from(base)).max(0) as u32;
        if let Some(point) = &mut self.exp {
            point.level = shift(point.level);
        }
        if let Some(point) = &mut self.pending {
            point.level = shift(point.level);
        }
        for (counted, _) in &mut self.requirements {
            *counted = shift(*counted);
        }
    }

    /// Moves the EXP state from `last` to `next`, counting the gain and the time between them.
    fn accept(&mut self, last: &ExpPoint, next: ExpPoint) {
        let gain: Option<i64> = if next.level == last.level {
            Some(next.exp.points as i64 - last.exp.points as i64)
        } else if next.level == last.level + 1 {
            // The rest of the old level, then the new level's progress.
            self.requirement(last.level)
                .and_then(Requirement::estimate)
                .map(|needed| needed as i64 - last.exp.points as i64 + next.exp.points as i64)
        } else {
            // Several levels at once: the levels in between were never measured.
            None
        };

        let percent = if next.level == last.level {
            f64::from(next.exp.percent - last.exp.percent).max(0.0)
        } else if next.level == last.level + 1 {
            100.0 - f64::from(last.exp.percent) + f64::from(next.exp.percent)
        } else {
            0.0
        };

        let dt = (next.at - last.at).max(0.0);
        let moved = matches!((&last.map, &next.map), (Some(a), Some(b)) if a != b);
        let solo = self.party.value.is_none_or(|p| p <= 1);
        // Only a gain within one level is exact enough to match against kills.
        let kind = match gain {
            Some(g) if g > 0 && next.level == last.level && solo && !moved => {
                let kind = classify(g as u64, &self.kill_exp);
                match kind {
                    GainKind::Kills(_) => self.gains_matched += 1,
                    GainKind::Hunting { checked: true } => self.gains_unmatched += 1,
                    _ => {}
                }
                kind
            }
            _ => GainKind::Hunting { checked: false },
        };
        let other = kind == GainKind::Other;
        // Quest EXP is no sign of hunting: the clock runs as if nothing came in.
        let counted = self.run_clock(gain.map(|g| g > 0 && !other), dt, moved);
        let (hunted, other_exp) = if other { (0, gain.unwrap_or(0)) } else { (gain.unwrap_or(0), 0) };
        let percent = if other { 0.0 } else { percent };
        let kills = match kind {
            GainKind::Kills(k) => Some(k),
            _ => None,
        };
        self.exp_gained += hunted;
        self.exp_other += other_exp;
        self.kills += u64::from(kills.unwrap_or(0));
        if hunted > 0 {
            self.gains.add(hunted as f64);
        }
        self.percent_gained += percent;
        self.level_ups += next.level - last.level;
        if next.level > last.level && self.level_known {
            self.event(EventKind::LevelUp, Some(next.level));
        }

        if counted > 0.0 || gain.is_some_and(|g| g != 0) {
            let map = if moved { None } else { last.map.clone().or_else(|| next.map.clone()) };
            let totals = self.totals(map.clone());
            totals.active_seconds += counted;
            totals.exp_gained += hunted;
            totals.kills += u64::from(kills.unwrap_or(0));
            if hunted > 0 {
                totals.gains.add(hunted as f64);
            }
            let level = self.level_known.then_some(next.level);
            let line = self.line(map, level);
            line.active_seconds += counted;
            line.exp_gained += hunted;
            line.other_exp += other_exp;
            line.percent_gained += percent;
            match kind {
                GainKind::Kills(k) => *line.kills.get_or_insert(0) += k,
                GainKind::Hunting { checked: true } => line.gains_missed += 1,
                _ => {}
            }
        }

        let implied = Requirement::implied(next.exp);
        self.set_requirement(next.level, implied);
        self.exp = Some(next);
        self.mark();
    }

    /// EXP went down: the loss is noted and left out of the rates.
    fn lose(&mut self, last: &ExpPoint, next: ExpPoint) {
        let dt = (next.at - last.at).max(0.0);
        self.run_clock(Some(false), dt, false);
        self.deaths += 1;
        self.event(EventKind::Death, None);
        let lost = last.exp.points.saturating_sub(next.exp.points);
        self.exp_lost += lost;
        self.percent_lost += f64::from(last.exp.percent - next.exp.percent).max(0.0);
        let map = last.map.clone();
        self.totals(map.clone()).exp_lost += lost;
        let level = self.level_known.then_some(last.level);
        let line = self.line(map, level);
        line.deaths += 1;
        line.exp_lost += lost;
        self.exp = Some(next);
    }

    /// How long a kill usually takes here: the middle of the recent gaps between gains.
    fn usual_gap(&self) -> f64 {
        if self.gaps.is_empty() {
            return USUAL_GAP.min(self.config.idle_after);
        }
        let mut gaps: Vec<f64> = self.gaps.iter().copied().collect();
        gaps.sort_by(f64::total_cmp);
        gaps[gaps.len() / 2].clamp(1.0, self.config.idle_after.max(1.0))
    }

    /// Moves the active clock on. `dt` is the time since the last reading; `gained` says whether
    /// EXP went up, or is `None` when that can't be told. Returns the seconds counted.
    ///
    /// Time seen to pass without EXP is held back until the next gain, since the wait before a
    /// kill is part of it. A wait up to `idle_after` counts in full. A longer one was a break: only
    /// a usual kill's time of it counts, as the hunt that ended it. Counting the first
    /// `idle_after` of every break, as before, made a session with many breaks look slower than
    /// its play was. The time since the reading before a gain always counts: with readings far
    /// apart (a folder of screenshots) nothing says it was idle.
    fn run_clock(&mut self, gained: Option<bool>, dt: f64, moved: bool) -> f64 {
        let dt = if std::mem::take(&mut self.skip_gap) { 0.0 } else { dt };
        let counted = match gained {
            None => 0.0,
            Some(true) => {
                let waited = std::mem::take(&mut self.idle_for);
                let gap = waited + dt;
                if std::mem::take(&mut self.new_map) {
                    dt
                } else if gap <= self.config.idle_after {
                    self.gaps.push_back(gap);
                    if self.gaps.len() > GAPS_KEPT {
                        self.gaps.pop_front();
                    }
                    gap
                } else if waited > 0.0 {
                    dt + self.usual_gap().min(waited)
                } else {
                    dt
                }
            }
            Some(false) if moved => {
                // A new map with no EXP yet: whatever led up to it was no hunting here, and the
                // first gain there counts only the time since the reading before it.
                self.idle_for = self.config.idle_after;
                self.new_map = true;
                0.0
            }
            Some(false) => {
                self.idle_for += dt;
                0.0
            }
        };
        self.clock += counted;
        counted
    }

    /// Active seconds to show: the clock, plus the wait since the last gain as far as a usual
    /// kill's time, which the next gain will count.
    fn shown_clock(&self) -> f64 {
        if self.held || self.clock == 0.0 {
            return self.clock;
        }
        self.clock + self.idle_for.min(self.usual_gap())
    }

    /// Follows a HUD that shows only a percentage. Each level is 100, so nothing else is needed.
    fn push_percent(&mut self, sample: &Sample, percent: f32) {
        if !(0.0..100.0).contains(&percent) {
            self.rejected.exp += 1;
            return;
        }
        // Coming from a level that showed points, they are where this picks up from.
        let seeded = self.exp.as_ref().map(|e| PercentPoint {
            at: e.at,
            level: e.level,
            percent: e.exp.percent,
            map: e.map.clone(),
        });
        let last = self.percent_point.clone().or(seeded);
        let level = sample.level.or(last.as_ref().map(|l| l.level)).unwrap_or(0);
        self.level_known |= sample.level.is_some();
        let point = PercentPoint { at: sample.at, level, percent, map: self.map.clone() };
        let Some(last) = last else {
            match self.percent_pending.take() {
                Some(first) if first.level == point.level && (first.percent - percent).abs() <= 5.0 => {
                    self.percent_point = Some(first.clone());
                    self.advance_percent(&first, point);
                }
                dropped => {
                    self.rejected.exp += usize::from(dropped.is_some());
                    self.percent_pending = Some(point);
                }
            }
            return;
        };

        let confirmed = self
            .percent_pending
            .as_ref()
            .is_some_and(|first| first.level == point.level && (first.percent - percent).abs() <= 1.0);
        let wrapped = point.level == last.level + 1 && percent < last.percent;
        let steady = point.level == last.level && (0.0..=20.0).contains(&(percent - last.percent));
        if wrapped || steady {
            self.percent_pending = None;
            self.advance_percent(&last, point);
        } else if point.level < last.level {
            self.rejected.exp += 1;
        } else if confirmed {
            self.percent_pending = None;
            if point.level == last.level && percent < last.percent {
                self.deaths += 1;
                self.percent_lost += f64::from(last.percent - percent);
                self.run_clock(Some(false), (point.at - last.at).max(0.0), false);
                self.event(EventKind::Death, None);
                self.percent_point = Some(point);
            } else {
                self.advance_percent(&last, point);
            }
        } else {
            self.percent_pending = Some(point);
        }
    }

    fn advance_percent(&mut self, last: &PercentPoint, next: PercentPoint) {
        let gain = if next.level == last.level {
            f64::from(next.percent - last.percent).max(0.0)
        } else if next.level == last.level + 1 {
            100.0 - f64::from(last.percent) + f64::from(next.percent)
        } else {
            0.0
        };
        let dt = (next.at - last.at).max(0.0);
        let moved = matches!((&last.map, &next.map), (Some(a), Some(b)) if a != b);
        let counted = self.run_clock(Some(gain > 0.0), dt, moved);
        self.percent_gained += gain;
        self.level_ups += next.level.saturating_sub(last.level);
        if next.level > last.level && self.level_known {
            self.event(EventKind::LevelUp, Some(next.level));
        }
        if counted > 0.0 || gain > 0.0 {
            let map = if moved { None } else { last.map.clone().or_else(|| next.map.clone()) };
            self.totals(map.clone()).active_seconds += counted;
            let level = self.level_known.then_some(next.level);
            let line = self.line(map, level);
            line.active_seconds += counted;
            line.percent_gained += gain;
        }
        self.percent_point = Some(next);
        self.mark();
    }

    /// Notes the running totals, for the recent rates and the trend line.
    fn mark(&mut self) {
        let mark =
            Mark { clock: self.clock, exp: self.exp_gained, percent: self.percent_gained };
        self.marks.push_back(mark);
        // One mark at or before the window's start is kept: the totals then are the window's base.
        while self.marks.get(1).is_some_and(|m| m.clock <= self.clock - RECENT_SECONDS) {
            self.marks.pop_front();
        }
        if self.clock - self.trend_clock >= TREND_STEP {
            let rate = if self.percent_point.is_some() {
                self.recent_rate(|m| m.percent)
            } else {
                self.recent_rate(|m| m.exp as f64)
            };
            if let Some(rate) = rate {
                self.trend_clock = self.clock;
                self.trend.push_back(rate);
                if self.trend.len() > TREND_POINTS {
                    self.trend.pop_front();
                }
            }
        }
        self.timeline_point();
    }

    /// Adds a point to the whole-session timeline once enough active time has passed.
    fn timeline_point(&mut self) {
        let step = if self.timeline_step > 0.0 { self.timeline_step } else { TIMELINE_STEP };
        if self.clock - self.timeline_clock < step {
            return;
        }
        let percent_only = self.percent_point.is_some();
        let exp = self.recent_rate(|m| m.exp as f64).filter(|_| !percent_only);
        let percent = self.recent_rate(|m| m.percent);
        if exp.is_none() && percent.is_none() {
            return;
        }
        self.timeline_clock = self.clock;
        self.timeline_step = step;
        let level = self.level().filter(|_| self.level_known);
        self.timeline.push(TimelinePoint {
            at: self.clock,
            exp_per_hour: exp,
            percent_per_hour: percent,
            mesos_per_hour: self.meso_rate_at(self.clock),
            level,
            map: self.map.clone().filter(|_| !self.town),
        });
        if self.timeline.len() > TIMELINE_POINTS {
            let mut i = 0;
            self.timeline.retain(|_| {
                i += 1;
                i % 2 == 1
            });
            self.timeline_step *= 2.0;
        }
    }

    fn event(&mut self, kind: EventKind, level: Option<u32>) {
        self.events.push(SessionEvent { at: self.clock, kind, level });
    }

    /// The rate of a running total over the last five minutes of active time, exactly: the totals
    /// only change at a mark, so at the window's start they are those of the last mark before it.
    /// Measuring from the first mark inside the window instead dropped the gain there but kept
    /// its time, which read low. Before five minutes, the whole session from its start at 0.
    fn recent_rate(&self, total: impl Fn(&Mark) -> f64) -> Option<f64> {
        let last = self.marks.back()?;
        let from = last.clock - RECENT_SECONDS;
        let seconds = last.clock.min(RECENT_SECONDS);
        if seconds < self.config.min_rate_seconds {
            return None;
        }
        let base = if from <= 0.0 {
            0.0
        } else {
            // Marks are dropped only once a later one is at or before the start, so one is here.
            self.marks.iter().rev().find(|m| m.clock <= from).map_or(0.0, &total)
        };
        Some((total(last) - base) * 3600.0 / seconds)
    }

    /// Notes a look at the mesos, then redraws the graph's points since the look before, which
    /// could not know what was looted until now.
    fn meso_look(&mut self, clock: f64) {
        let total = self.mesos_gained;
        let previous = self.meso_looks.back().copied();
        let clock = previous.map_or(clock, |(last, _)| clock.max(last));
        // A flat stretch needs only its two ends.
        let len = self.meso_looks.len();
        if len >= 2 && self.meso_looks[len - 2].1 == total && self.meso_looks[len - 1].1 == total {
            self.meso_looks[len - 1].0 = clock;
        } else if previous == Some((clock, total)) {
            return;
        } else if previous.is_some_and(|(last, _)| last == clock) {
            self.meso_looks[len - 1].1 = total;
        } else {
            self.meso_looks.push_back((clock, total));
        }
        // Points after the look before this one may still need it, and their windows reach back
        // one recent window further.
        let len = self.meso_looks.len();
        if len >= 2 {
            let keep_from = self.meso_looks[len - 2].0 - RECENT_SECONDS;
            while self.meso_looks.len() > 2 && self.meso_looks[1].0 <= keep_from {
                self.meso_looks.pop_front();
            }
        }
        let since = previous.map_or(f64::NEG_INFINITY, |(last, _)| last);
        for i in (0..self.timeline.len()).rev() {
            let at = self.timeline[i].at;
            if at <= since {
                break;
            }
            self.timeline[i].mesos_per_hour = self.meso_rate_at(at);
        }
    }

    /// Mesos gained by this moment, spread evenly between looks. `None` outside the looks.
    fn mesos_at(&self, clock: f64) -> Option<f64> {
        let first = *self.meso_looks.front()?;
        let last = *self.meso_looks.back()?;
        if clock < first.0 || clock > last.0 {
            return None;
        }
        let after = self.meso_looks.iter().position(|&(c, _)| c >= clock)?;
        let (c1, t1) = self.meso_looks[after];
        if after == 0 || c1 == clock {
            return Some(t1 as f64);
        }
        let (c0, t0) = self.meso_looks[after - 1];
        Some(t0 as f64 + (t1 - t0) as f64 * (clock - c0) / (c1 - c0))
    }

    /// Mesos per hour over the recent window ending at this moment. Unknown before the first
    /// look and after the latest one, since the loot since then hasn't been seen.
    fn meso_rate_at(&self, clock: f64) -> Option<f64> {
        let first = self.meso_looks.front()?.0;
        let from = (clock - RECENT_SECONDS).max(first);
        let seconds = clock - from;
        if seconds < self.config.min_rate_seconds {
            return None;
        }
        Some((self.mesos_at(clock)? - self.mesos_at(from)?) * 3600.0 / seconds)
    }

    /// How many readings in a row a new amount needs. A big change is usually a digit misread,
    /// and a tenfold one nearly always is, unless the mesos have been out of sight for a while.
    /// The first amount in a window that has just opened needs only one, since the player may
    /// buy something before a second.
    fn meso_confirmations(&self, mesos: u64, at: f64, first_in_window: bool) -> usize {
        let base = self.config.confirmations.max(1);
        let usual = if first_in_window { 1 } else { base };
        let Some(last) = self.mesos.as_ref().filter(|_| base >= 2) else { return usual };
        if self.mesos_seen_at.is_none_or(|seen| at - seen > MESO_GAP) {
            return usual;
        }
        let (old, new) = (last.mesos as f64, mesos as f64);
        let ratio = if old > 0.0 { new / old } else { 1.0 };
        if (9.0..=11.0).contains(&ratio) || (0.09..=0.11).contains(&ratio) {
            8
        } else if (new - old).abs() > 10_000f64.max(0.05 * old.max(new)) {
            base.max(3)
        } else {
            usual
        }
    }

    fn push_mesos(&mut self, sample: &Sample) {
        let Some(mesos) = sample.mesos else { return };
        let first_in_window = self.first_in_window(sample);
        let needed = self.meso_confirmations(mesos, sample.at, first_in_window);
        let count = match &mut self.meso_run {
            Some((run, count)) if run.mesos == mesos => {
                *count += 1;
                *count
            }
            run => {
                // An amount that never got enough agreeing readings was a misread.
                if run.as_ref().is_some_and(|(_, count)| *count < self.config.confirmations) {
                    self.rejected.mesos += 1;
                }
                *run = Some((MesoPoint { mesos, clock: self.clock, map: self.map.clone() }, 1));
                1
            }
        };
        // The confirmed amount seen again: any window open before has closed without changing
        // it, unless it is still open.
        if self.mesos.as_ref().is_some_and(|m| m.mesos == mesos) {
            self.mesos_seen_at = Some(sample.at);
            self.mesos_seen_clock = Some(self.clock);
            self.exchange_seen = sample.exchange;
            self.meso_look(self.clock);
            return;
        }
        if count < needed {
            return;
        }
        let run = self.meso_run.as_ref().map(|(run, _)| run.clone()).expect("a run was just updated");
        self.mesos_seen_at = Some(sample.at);
        self.mesos_seen_clock = Some(self.clock);
        if let Some(previous) = self.mesos.replace(run.clone()) {
            let delta = run.mesos as i64 - previous.mesos as i64;
            // Read in a window: not loot, unless it is the window's first amount, from before
            // anything in it changed hands.
            if self.exchange_seen || (sample.exchange && !first_in_window) {
                self.mesos_excluded += delta;
            } else if delta != 0 {
                let seconds = run.clock - previous.clock;
                self.mesos_gained += delta;
                self.meso_seconds += seconds;
                let map = if previous.map == run.map { previous.map } else { None };
                let totals = self.totals(map.clone());
                totals.mesos_gained += delta;
                totals.meso_seconds += seconds;
                let level = self.level().filter(|_| self.level_known);
                let line = self.line(map, level);
                line.mesos_gained += delta;
                line.mesos_seconds += seconds;
                self.meso_look(run.clock);
                self.mark();
            }
        }
        // The first amount, a change left out, or no change: still a look that ends a stretch.
        if self.meso_looks.back().is_none_or(|&(clock, _)| clock < run.clock) {
            self.meso_look(run.clock);
        }
        // A window still open now also covers the next change.
        self.exchange_seen = sample.exchange;
    }

    /// Counts each quest counter's rises, once enough readings in a row agree on the new value.
    /// A counter is known by its target and the amount it needs.
    fn push_counters(&mut self, sample: &Sample) {
        if sample.counters.is_empty() {
            return;
        }
        for track in &mut self.counters {
            track.seen = false;
        }
        let needed = self.config.confirmations.max(1);
        for counter in &sample.counters {
            let known = self.counters.iter().position(|t| t.target == counter.target && t.required == counter.required);
            let index = known.unwrap_or_else(|| {
                self.counters.push(CounterTrack {
                    target: counter.target.clone(),
                    required: counter.required,
                    run: (counter.current, 0),
                    confirmed: None,
                    seen: false,
                });
                self.counters.len() - 1
            });
            let track = &mut self.counters[index];
            track.seen = true;
            let agreeing = if track.run.0 == counter.current { track.run.1 + 1 } else { 1 };
            track.run = (counter.current, agreeing);
            if agreeing != needed {
                continue;
            }
            let next = (counter.current, self.clock, self.map.clone());
            let Some((value, clock, map)) = track.confirmed.replace(next) else { continue };
            if counter.current <= value {
                continue;
            }
            let (target, required) = (track.target.clone(), track.required);
            let map = if map == self.map { map } else { None };
            let (gained, seconds) = (counter.current - value, self.clock - clock);
            let index =
                self.counter_totals.iter().position(|t| t.target == target && t.required == required && t.map == map);
            let totals = match index {
                Some(index) => &mut self.counter_totals[index],
                None => {
                    self.counter_totals.push(CounterRate { target, required, map, ..CounterRate::default() });
                    self.counter_totals.last_mut().expect("just pushed")
                }
            };
            totals.gained += gained;
            totals.active_seconds += seconds;
        }
        // Quests no longer shown were handed in or dropped; taking one again starts afresh.
        self.counters.retain(|t| t.seen);
    }

    fn requirement(&self, level: u32) -> Option<Requirement> {
        self.requirements.iter().find(|(l, _)| *l == level).map(|(_, r)| *r)
    }

    fn set_requirement(&mut self, level: u32, implied: Option<Requirement>) {
        let Some(narrowed) = narrow(self.requirement(level), implied) else { return };
        match self.requirements.iter_mut().find(|(l, _)| *l == level) {
            Some((_, requirement)) => *requirement = narrowed,
            None => self.requirements.push((level, narrowed)),
        }
    }

    fn totals(&mut self, map: Option<String>) -> &mut MapTotals {
        let index = match self.maps.iter().position(|t| t.map == map) {
            Some(index) => index,
            None => {
                self.maps.push(MapTotals { map, ..MapTotals::default() });
                self.maps.len() - 1
            }
        };
        &mut self.maps[index]
    }

    fn line(&mut self, map: Option<String>, level: Option<u32>) -> &mut SessionLine {
        let (party, job, town) = (self.party.value, self.job.value.clone(), self.town);
        let index =
            self.lines.iter().position(|l| l.map == map && l.level == level && l.party == party && l.job == job);
        let index = index.unwrap_or_else(|| {
            self.lines.push(SessionLine { map, town, level, party, job, ..SessionLine::default() });
            self.lines.len() - 1
        });
        let line = &mut self.lines[index];
        // The character as the stretch began, or as soon as it was known.
        if line.fighter.is_none() {
            line.fighter.clone_from(&self.fighter);
        }
        line
    }

    /// The pace behind the time to level. It is this map's, since a session that moved maps
    /// says little about the one the player is on now, less EXP lost to deaths here, which set
    /// the level back for real. An estimate for the map, when given, counts for
    /// [`PRIOR_SECONDS`] of play, so the pace is steady from the first kill and the player's own
    /// play takes over as it goes on. Without one, the session's pace stands in until the map has
    /// [`MAP_PACE_SECONDS`] of play.
    fn pace(&self) -> Pace {
        let here = self.map.as_ref().and_then(|map| self.maps.iter().find(|t| t.map.as_ref() == Some(map)));
        let prior = self.prior.as_ref().filter(|(map, _)| self.map.as_ref() == Some(map)).map(|(_, rate)| *rate);
        let map_ready = here.is_some_and(|t| t.active_seconds >= MAP_PACE_SECONDS);
        let (exp, seconds, gains, kills) = match here {
            Some(t) if map_ready || prior.is_some() => {
                (t.exp_gained - t.exp_lost as i64, t.active_seconds, t.gains, Some(t.kills))
            }
            _ => (self.exp_gained - self.exp_lost as i64, self.clock, self.gains, None),
        };
        let exp_per_hour = match prior {
            Some(rate) => Some((exp as f64 + rate * PRIOR_SECONDS / 3600.0) * 3600.0 / (seconds + PRIOR_SECONDS)),
            None => (seconds >= self.config.min_rate_seconds).then(|| exp as f64 * 3600.0 / seconds),
        };
        let kills_per_hour = kills
            .filter(|&k| k > 0 && seconds >= self.config.min_rate_seconds)
            .map(|k| k as f64 * 3600.0 / seconds);
        Pace {
            exp_per_hour,
            // The estimate's own error isn't known; its weight only shrinks the measured one.
            error: gains.error().map(|e| prior.map_or(e, |_| e * seconds / (seconds + PRIOR_SECONDS))),
            from_estimate: prior.is_some() && PRIOR_SECONDS > 0.1 * (seconds + PRIOR_SECONDS),
            kills_per_hour,
        }
    }

    pub fn summary(&self) -> SessionSummary {
        let rate = |amount: i64, seconds: f64| {
            (seconds >= self.config.min_rate_seconds).then(|| amount as f64 * 3600.0 / seconds)
        };
        let percent_only = self.percent_point.is_some();
        let exp_per_hour = rate(self.exp_gained, self.clock).filter(|_| !percent_only);
        let exp_needed = self.exp.as_ref().and_then(|e| self.requirement(e.level)).and_then(Requirement::estimate);
        let percent = self.percent_point.as_ref().map(|p| p.percent).or(self.exp.as_ref().map(|e| e.exp.percent));
        let percent_per_hour =
            exp_per_hour.zip(exp_needed).map(|(per_hour, needed)| per_hour * 100.0 / needed as f64).or_else(|| {
                (self.clock >= self.config.min_rate_seconds).then(|| self.percent_gained * 3600.0 / self.clock)
            });
        let pace = self.pace();
        let remaining = match (&self.exp, exp_needed) {
            (Some(last), Some(needed)) if !percent_only => Some(needed.saturating_sub(last.exp.points) as f64),
            _ => None,
        };
        let eta = |per_hour: f64| remaining.filter(|_| per_hour > 0.0).map(|left| left / per_hour * 3600.0);
        let seconds_to_level = match pace.exp_per_hour.and_then(eta) {
            Some(seconds) => Some(seconds),
            None => percent
                .zip(percent_per_hour.filter(|&p| p > 0.0 && percent_only))
                .map(|(percent, per_hour)| (100.0 - f64::from(percent)) / per_hour * 3600.0),
        };
        let spread = |sign: f64| {
            let (per_hour, error) = (pace.exp_per_hour?, pace.error?);
            eta(per_hour * (1.0 + sign * RANGE_Z * error).max(0.2))
        };
        // Breaks as this session has had them, once it has run long enough to say.
        let breaks = (self.real_clock >= BREAKS_AFTER && self.clock > 0.0)
            .then(|| (self.real_clock / self.clock).clamp(1.0, 5.0));
        let exp_per_hour_with_breaks = (self.real_clock >= BREAKS_AFTER && !percent_only)
            .then(|| (self.exp_gained - self.exp_lost as i64) as f64 * 3600.0 / self.real_clock);
        let mut maps: Vec<MapSummary> = self
            .maps
            .iter()
            .map(|t| MapSummary {
                map: t.map.clone(),
                active_seconds: t.active_seconds,
                exp_gained: t.exp_gained,
                exp_per_hour: rate(t.exp_gained, t.active_seconds).filter(|_| t.exp_gained != 0 || !percent_only),
                mesos_gained: t.mesos_gained,
                mesos_per_hour: rate(t.mesos_gained, t.meso_seconds),
            })
            .collect();
        maps.sort_by(|a, b| b.active_seconds.total_cmp(&a.active_seconds));
        let mut counters: Vec<CounterRate> = self
            .counter_totals
            .iter()
            .map(|t| CounterRate {
                per_hour: if t.gained >= 2 { rate(i64::from(t.gained), t.active_seconds) } else { None },
                ..t.clone()
            })
            .collect();
        counters.sort_by(|a, b| b.active_seconds.total_cmp(&a.active_seconds));
        let mut lines = self.lines.clone();
        lines.sort_by(|a, b| b.active_seconds.total_cmp(&a.active_seconds));

        SessionSummary {
            frames: self.frames,
            started_at: self.started_at,
            updated_at: self.updated_at,
            elapsed_seconds: self.started_at.zip(self.updated_at).map_or(0.0, |(s, u)| u - s),
            active_seconds: self.shown_clock(),
            paused: self.held || (self.level().is_some() && self.idle_for >= self.config.idle_after),
            held: self.held,
            level: self.level().filter(|_| self.level_known),
            exp: self.exp.as_ref().filter(|_| !percent_only).map(|e| e.exp),
            percent,
            percent_only,
            exp_needed: exp_needed.filter(|_| !percent_only),
            exp_gained: self.exp_gained,
            exp_other: self.exp_other,
            kills: self.kills,
            gains_matched: self.gains_matched,
            gains_unmatched: self.gains_unmatched,
            exp_per_hour,
            percent_per_hour,
            seconds_to_level,
            seconds_to_level_low: spread(1.0),
            seconds_to_level_high: spread(-1.0),
            seconds_to_level_with_breaks: seconds_to_level.zip(breaks).map(|(s, b)| s * b),
            pace_exp_per_hour: pace.exp_per_hour.filter(|_| !percent_only),
            pace_error: pace.error,
            pace_from_estimate: pace.from_estimate,
            kills_per_hour: pace.kills_per_hour,
            real_seconds: self.real_clock,
            exp_per_hour_with_breaks,
            recent_exp_per_hour: self.recent_rate(|m| m.exp as f64).filter(|_| !percent_only),
            recent_percent_per_hour: self.recent_rate(|m| m.percent),
            recent_mesos_per_hour: self.meso_looks.back().and_then(|&(clock, _)| self.meso_rate_at(clock)),
            trend: self.trend.iter().copied().collect(),
            timeline: self.timeline.clone(),
            events: self.events.clone(),
            level_ups: self.level_ups,
            deaths: self.deaths,
            exp_lost: self.exp_lost,
            percent_lost: self.percent_lost,
            mesos: self.mesos.as_ref().map(|m| m.mesos),
            mesos_age: self.mesos_seen_at.zip(self.updated_at).map(|(seen, now)| (now - seen).max(0.0)),
            mesos_gained: self.mesos_gained,
            mesos_per_hour: rate(self.mesos_gained, self.meso_seconds),
            mesos_seconds: self.meso_seconds,
            mesos_unseen_seconds: (self.clock - self.mesos_seen_clock.unwrap_or(0.0)).max(0.0),
            mesos_excluded: self.mesos_excluded,
            map: self.map.clone(),
            town: self.town,
            map_hidden: self
                .map_seen_at
                .zip(self.updated_at)
                .is_some_and(|(seen, now)| now - seen >= MAP_HIDDEN_SECONDS),
            name: self.name.value.clone(),
            job: self.job.value.clone(),
            party: self.party.value,
            maps,
            lines,
            counters,
            rejected: self.rejected,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const KERNING: &str = "Victoria Road : Kerning City";
    const KPQ: &str = "Kerning City : First Accompaniment";

    fn sample(at: f64, level: u32, points: u64, percent: f32) -> Sample {
        Sample {
            at,
            level: Some(level),
            exp: Some(Exp { points, percent }),
            map: Some(KERNING.into()),
            ..Sample::default()
        }
    }

    fn with_mesos(mut sample: Sample, mesos: u64) -> Sample {
        sample.mesos = Some(mesos);
        sample
    }

    fn on(mut sample: Sample, map: &str) -> Sample {
        sample.map = Some(map.into());
        sample
    }

    #[test]
    fn a_restored_tracker_carries_on_without_counting_the_time_away() {
        let config = TrackerConfig { confirmations: 1, ..TrackerConfig::default() };
        let mut tracker = SessionTracker::new(config);
        let mut points = 100;
        for second in 0..=120 {
            points += 20;
            tracker.push(&with_mesos(sample(f64::from(second), 30, points, points as f32 / 10_000.0), 1_000 + 50 * points));
        }
        let before = tracker.summary();
        assert!(before.active_seconds > 100.0 && before.exp_gained > 0);

        let saved = serde_json::to_value(&tracker).unwrap();
        let mut back = SessionTracker::restore(config, saved).expect("restores");
        assert_eq!(back.summary().exp_gained, before.exp_gained);
        assert_eq!(back.summary().maps, before.maps);

        // Back an hour later: the hour is not play time, and the next gain counts in full.
        points += 20;
        back.push(&sample(4_000.0, 30, points, points as f32 / 10_000.0));
        points += 20;
        back.push(&sample(4_001.0, 30, points, points as f32 / 10_000.0));
        let after = back.summary();
        assert!(after.active_seconds - before.active_seconds < 5.0, "{}", after.active_seconds);
        assert_eq!(after.exp_gained, before.exp_gained + 40);
        assert_eq!(after.started_at, before.started_at);
    }

    #[test]
    fn a_snapshot_that_cannot_be_read_is_refused() {
        let config = TrackerConfig::default();
        assert!(SessionTracker::restore(config, serde_json::json!({ "frames": "many" })).is_none());
    }

    #[test]
    fn keeps_a_timeline_with_level_ups_and_deaths() {
        let mut samples = vec![sample(0.0, 30, 100, 0.01)];
        let mut points = 100;
        for second in 1..=600 {
            points += 20;
            samples.push(sample(f64::from(second), 30, points, points as f32 / 10_000.0));
        }
        // A death, then a level-up.
        samples.push(sample(601.0, 30, points - 500, (points - 500) as f32 / 10_000.0));
        samples.push(sample(602.0, 30, points - 500, (points - 500) as f32 / 10_000.0));
        samples.push(sample(603.0, 31, 50, 0.01));
        let summary = track(&samples);
        assert!(summary.timeline.len() >= 15, "{}", summary.timeline.len());
        assert!(summary.timeline.windows(2).all(|w| w[1].at - w[0].at >= TIMELINE_STEP - 1e-9));
        assert!(summary.timeline.last().unwrap().exp_per_hour.unwrap() > 50_000.0);
        let kinds: Vec<EventKind> = summary.events.iter().map(|e| e.kind).collect();
        assert_eq!(kinds, [EventKind::Death, EventKind::LevelUp]);
        assert_eq!(summary.events[1].level, Some(31));
    }

    #[test]
    fn a_long_timeline_thins_out() {
        let mut tracker = SessionTracker::new(TrackerConfig::default());
        let mut points = 100;
        tracker.push(&sample(0.0, 30, points, 0.01));
        let seconds = (TIMELINE_STEP as usize) * (TIMELINE_POINTS + 10);
        for second in 1..=seconds {
            points += 20;
            tracker.push(&sample(second as f64, 30, points, points as f32 / 10_000.0));
        }
        let summary = tracker.summary();
        assert!(summary.timeline.len() <= TIMELINE_POINTS);
        assert!(summary.timeline.len() > TIMELINE_POINTS / 2 - 10);
    }

    fn track(samples: &[Sample]) -> SessionSummary {
        let mut tracker = SessionTracker::new(TrackerConfig::default());
        samples.iter().for_each(|s| tracker.push(s));
        tracker.summary()
    }

    fn close(actual: Option<f64>, expected: f64, tolerance: f64) -> bool {
        actual.is_some_and(|a| (a - expected).abs() <= tolerance)
    }

    #[test]
    fn a_death_is_noted_and_left_out_of_the_rates() {
        let summary = track(&[
            sample(0.0, 30, 10_000, 10.0),
            sample(60.0, 30, 20_000, 20.0),
            // Dying costs 5%, seen on two readings.
            sample(70.0, 30, 15_000, 15.0),
            sample(75.0, 30, 15_000, 15.0),
            sample(135.0, 30, 25_000, 25.0),
        ]);
        assert_eq!(summary.deaths, 1);
        assert_eq!(summary.exp_lost, 5_000);
        assert_eq!(summary.exp_gained, 20_000);
    }

    #[test]
    fn a_single_low_reading_is_no_death() {
        let summary = track(&[
            sample(0.0, 30, 10_000, 10.0),
            sample(60.0, 30, 20_000, 20.0),
            sample(65.0, 30, 15_000, 15.0),
            sample(70.0, 30, 21_000, 21.0),
        ]);
        assert_eq!(summary.deaths, 0);
        assert_eq!(summary.exp_gained, 11_000);
        assert_eq!(summary.rejected.exp, 1);
    }

    #[test]
    fn recent_rates_follow_the_last_five_minutes() {
        let mut samples = vec![sample(0.0, 30, 100, 0.01)];
        // 10 minutes slow, then 10 minutes twice as fast.
        let mut points = 100;
        for minute in 1..=20u32 {
            points += if minute <= 10 { 1_000 } else { 2_000 };
            samples.push(sample(f64::from(minute) * 60.0, 30, points, points as f32 / 10_000.0));
        }
        let summary = track(&samples);
        assert!(close(summary.exp_per_hour, 90_000.0, 1.0), "{:?}", summary.exp_per_hour);
        assert!(close(summary.recent_exp_per_hour, 120_000.0, 1.0), "{:?}", summary.recent_exp_per_hour);
        assert!(summary.trend.len() >= 2 && summary.trend.last() > summary.trend.first(), "{:?}", summary.trend);
    }

    fn percent_sample(at: f64, level: u32, percent: f32) -> Sample {
        Sample { at, level: Some(level), percent: Some(percent), map: Some(KERNING.into()), ..Sample::default() }
    }

    #[test]
    fn a_percent_only_hud_gives_percent_rates_and_time_to_level() {
        let summary = track(&[
            percent_sample(0.0, 60, 40.0),
            percent_sample(5.0, 60, 40.0),
            percent_sample(1805.0, 60, 50.0),
            percent_sample(3605.0, 60, 60.0),
        ]);
        assert!(summary.percent_only);
        assert_eq!(summary.exp_per_hour, None);
        assert!(close(summary.percent_per_hour, 20.0, 0.1), "{:?}", summary.percent_per_hour);
        assert!(close(summary.seconds_to_level, 7200.0, 40.0), "{:?}", summary.seconds_to_level);
        assert_eq!(summary.level, Some(60));
    }

    #[test]
    fn a_percent_only_hud_counts_level_ups_and_deaths() {
        let summary = track(&[
            percent_sample(0.0, 60, 95.0),
            percent_sample(5.0, 60, 95.0),
            percent_sample(60.0, 61, 1.0),
            percent_sample(120.0, 61, 3.0),
            percent_sample(130.0, 61, 1.0),
            percent_sample(135.0, 61, 1.0),
        ]);
        assert_eq!(summary.level_ups, 1);
        assert_eq!(summary.deaths, 1);
        assert!((summary.percent_lost - 2.0).abs() < 0.01, "{}", summary.percent_lost);
    }

    #[test]
    fn the_hud_can_stop_showing_points_at_a_level_up() {
        let summary = track(&[
            sample(0.0, 56, 900_000, 90.0),
            sample(60.0, 56, 950_000, 95.0),
            percent_sample(120.0, 57, 1.0),
            percent_sample(180.0, 57, 3.0),
        ]);
        assert_eq!(summary.level_ups, 1);
        assert_eq!(summary.level, Some(57));
        assert!(summary.percent_only);
    }

    #[test]
    fn lines_split_by_party_and_level() {
        let party = |mut s: Sample, n| {
            s.party = Some(n);
            s
        };
        let summary = track(&[
            party(sample(0.0, 30, 1_000, 1.0), 1),
            party(sample(60.0, 30, 2_000, 2.0), 1),
            party(sample(120.0, 30, 3_000, 3.0), 1),
            party(sample(180.0, 30, 4_000, 4.0), 3),
            party(sample(240.0, 30, 5_000, 5.0), 3),
            party(sample(300.0, 30, 6_000, 6.0), 3),
        ]);
        assert_eq!(summary.party, Some(3));
        assert_eq!(summary.lines.len(), 2);
        assert!(summary.lines.iter().any(|l| l.party == Some(1)));
        assert!(summary.lines.iter().any(|l| l.party == Some(3)));
    }

    #[test]
    fn a_hidden_minimap_stops_gains_going_to_the_old_map() {
        let hidden = |mut s: Sample| {
            s.map = None;
            s
        };
        let summary = track(&[
            sample(0.0, 30, 1_000, 1.0),
            sample(30.0, 30, 2_000, 2.0),
            hidden(sample(40.0, 30, 3_000, 3.0)),
            hidden(sample(90.0, 30, 4_000, 4.0)),
            hidden(sample(120.0, 30, 5_000, 5.0)),
        ]);
        assert!(summary.map_hidden);
        assert_eq!(summary.map, None);
        assert!(summary.maps.iter().any(|m| m.map.is_none() && m.exp_gained > 0), "{:?}", summary.maps);
    }

    #[test]
    fn big_meso_changes_need_more_readings_while_in_sight() {
        let m = |at, mesos| with_mesos(sample(at, 30, 1_000, 1.0), mesos);
        let summary = track(&[m(0.0, 12_345), m(2.0, 12_345), m(4.0, 123_450), m(6.0, 123_450), m(8.0, 12_345)]);
        assert_eq!(summary.mesos, Some(12_345), "a tenfold misread is not believed");
        assert_eq!(summary.mesos_gained, 0);
    }

    #[test]
    fn pausing_by_hand_leaves_out_the_time_paused() {
        let mut tracker = SessionTracker::new(TrackerConfig::default());
        tracker.push(&sample(0.0, 30, 1_000, 1.0));
        tracker.push(&sample(60.0, 30, 2_000, 2.0));
        tracker.set_held(true);
        assert!(tracker.summary().paused);
        tracker.set_held(false);
        tracker.push(&sample(3600.0, 30, 3_000, 3.0));
        assert!(close(Some(tracker.summary().active_seconds), 60.0, 0.1));
    }

    #[test]
    fn another_character_is_noticed_after_a_few_readings() {
        let mut tracker = SessionTracker::new(TrackerConfig::default());
        tracker.push(&sample(0.0, 30, 1_000, 1.0));
        tracker.push(&sample(60.0, 30, 2_000, 2.0));
        let low = sample(120.0, 12, 100, 1.0);
        assert!(!tracker.is_other_character(&low));
        assert!(!tracker.is_other_character(&low));
        assert!(tracker.is_other_character(&low));
        let mut same = SessionTracker::new(TrackerConfig::default());
        same.push(&sample(0.0, 30, 1_000, 1.0));
        same.push(&sample(60.0, 30, 2_000, 2.0));
        assert!(!same.is_other_character(&sample(120.0, 30, 3_000, 3.0)));
    }

    #[test]
    fn a_garbled_name_is_not_another_character() {
        let named = |at, name: &str| Sample { name: Some(name.into()), ..sample(at, 30, 1_000, 1.0) };
        let mut tracker = SessionTracker::new(TrackerConfig::default());
        for i in 0..3 {
            tracker.push(&named(f64::from(i), "PebbleThief"));
        }
        for garbled in ["xq3", "7y#", "w0!", "pz%", "m2~"] {
            assert!(!tracker.is_other_character(&named(9.0, garbled)));
        }
        let mut stable = 0;
        for _ in 0..5 {
            stable += usize::from(tracker.is_other_character(&named(9.0, "Mossbank")));
        }
        assert!(stable > 0, "the same other name again and again is another character");
    }

    /// Numbers from real play: 24,331 EXP (25.42%) to 45,930 (47.99%) and 28,640 to 42,944 mesos,
    /// taken here as 30 minutes of steady grinding.
    #[test]
    fn half_an_hour_of_grinding() {
        let summary = track(&[
            // Two frames of each screen, as live capture would see it.
            with_mesos(sample(0.0, 30, 24_331, 25.42), 28_640),
            with_mesos(sample(0.0, 30, 24_331, 25.42), 28_640),
            sample(900.0, 30, 35_000, 36.57),
            with_mesos(sample(1800.0, 30, 45_930, 47.99), 42_944),
            with_mesos(sample(1800.0, 30, 45_930, 47.99), 42_944),
        ]);
        assert_eq!(summary.exp_gained, 21_599);
        assert_eq!(summary.active_seconds, 1800.0);
        assert!(close(summary.exp_per_hour, 43_198.0, 0.5), "{:?}", summary.exp_per_hour);
        // 45,930 / 47.99% and 24,331 / 25.42% both put level 30 at about 95,700 EXP.
        assert!(summary.exp_needed.is_some_and(|n| (95_690..=95_730).contains(&n)), "{:?}", summary.exp_needed);
        assert!(close(summary.percent_per_hour, 45.1, 0.1), "{:?}", summary.percent_per_hour);
        assert!(close(summary.seconds_to_level, 4148.0, 5.0), "{:?}", summary.seconds_to_level);
        assert_eq!(summary.mesos_gained, 14_304);
        assert!(close(summary.mesos_per_hour, 28_608.0, 0.5), "{:?}", summary.mesos_per_hour);
        assert_eq!(summary.rejected, Rejected::default());
        assert_eq!(summary.maps.len(), 1);
        assert_eq!(summary.maps[0].map.as_deref(), Some(KERNING));
        assert_eq!(summary.maps[0].exp_gained, 21_599);
    }

    /// A level digit the HUD font has not learned yet.
    fn unread_level(sample: Sample) -> Sample {
        Sample { level: None, ..sample }
    }

    #[test]
    fn follows_exp_when_the_level_cannot_be_read() {
        let summary = track(&[
            unread_level(sample(0.0, 0, 10, 10.0)),
            unread_level(sample(1.0, 0, 20, 20.0)),
            unread_level(sample(2.0, 0, 50, 50.0)),
            unread_level(sample(3.0, 0, 90, 90.0)),
            // A new level, needing 200: one reading could be a misread, two agree.
            unread_level(sample(4.0, 0, 10, 5.0)),
            unread_level(sample(5.0, 0, 20, 10.0)),
            unread_level(sample(6.0, 0, 30, 15.0)),
        ]);
        assert_eq!(summary.exp_gained, 120);
        assert_eq!(summary.level_ups, 1);
        assert_eq!(summary.level, None);
        assert_eq!(summary.exp_needed, Some(200));
        assert_eq!(summary.rejected, Rejected::default());
    }

    #[test]
    fn a_misread_is_not_a_level_up_when_the_level_cannot_be_read() {
        let summary = track(&[
            unread_level(sample(0.0, 0, 10, 10.0)),
            unread_level(sample(1.0, 0, 20, 20.0)),
            unread_level(sample(2.0, 0, 70, 5.0)),
            unread_level(sample(3.0, 0, 30, 30.0)),
        ]);
        assert_eq!(summary.level_ups, 0);
        assert_eq!(summary.exp_gained, 20);
        assert_eq!(summary.rejected.exp, 1);
    }

    #[test]
    fn a_new_level_can_start_at_zero_exp() {
        let summary = track(&[
            sample(0.0, 2, 10, 10.0),
            sample(1.0, 2, 90, 90.0),
            unread_level(sample(2.0, 0, 0, 0.0)),
            unread_level(sample(3.0, 0, 20, 10.0)),
            unread_level(sample(4.0, 0, 40, 20.0)),
        ]);
        assert_eq!(summary.level_ups, 1);
        assert_eq!(summary.level, Some(3));
        assert_eq!(summary.exp_gained, 130);
    }

    #[test]
    fn levels_keep_counting_past_digits_the_font_lacks() {
        let summary = track(&[
            sample(0.0, 1, 3, 20.0),
            sample(1.0, 1, 6, 40.0),
            unread_level(sample(2.0, 0, 2, 5.0)),
            unread_level(sample(3.0, 0, 4, 10.0)),
        ]);
        assert_eq!(summary.level, Some(2));
        assert_eq!(summary.level_ups, 1);
    }

    #[test]
    fn the_first_level_read_lines_the_count_up() {
        let summary = track(&[
            unread_level(sample(0.0, 0, 10, 10.0)),
            unread_level(sample(1.0, 0, 20, 20.0)),
            unread_level(sample(2.0, 0, 90, 90.0)),
            unread_level(sample(3.0, 0, 10, 5.0)),
            unread_level(sample(4.0, 0, 20, 10.0)),
            sample(5.0, 3, 30, 15.0),
            sample(6.0, 3, 40, 20.0),
        ]);
        assert_eq!(summary.level, Some(3));
        assert_eq!(summary.level_ups, 1);
        assert_eq!(summary.exp_needed, Some(200));
        assert_eq!(summary.rejected, Rejected::default());
    }

    #[test]
    fn waits_for_two_agreeing_readings_before_starting() {
        let summary = track(&[sample(0.0, 30, 24_331, 25.42)]);
        assert_eq!(summary.level, None);
        let summary =
            track(&[sample(0.0, 30, 24_331, 52.42), sample(1.0, 30, 24_331, 25.42), sample(2.0, 30, 24_331, 25.42)]);
        assert_eq!(summary.exp.map(|e| e.percent), Some(25.42));
        assert_eq!(summary.rejected.exp, 1);
    }

    #[test]
    fn points_that_disagree_with_the_percent_are_dropped() {
        let summary = track(&[
            sample(0.0, 30, 24_331, 25.42),
            sample(1.0, 30, 24_331, 25.42),
            // A 3 read as an 8: the points no longer fit 25.44%.
            sample(2.0, 30, 24_881, 25.44),
            sample(3.0, 30, 24_351, 25.44),
        ]);
        assert_eq!(summary.exp_gained, 20);
        assert_eq!(summary.rejected.exp, 1);
    }

    #[test]
    fn missing_readings_are_skipped() {
        let blank = Sample { at: 2.0, ..Sample::default() };
        let summary =
            track(&[sample(0.0, 30, 100, 0.10), sample(1.0, 30, 100, 0.10), blank, sample(3.0, 30, 200, 0.21)]);
        assert_eq!(summary.exp_gained, 100);
        assert_eq!(summary.frames, 4);
        assert_eq!((summary.started_at, summary.updated_at), (Some(0.0), Some(3.0)));
        // Nor does an empty frame after the last reading stretch the session.
        let late = Sample { at: 99.0, ..Sample::default() };
        assert_eq!(track(&[sample(0.0, 30, 100, 0.10), late]).elapsed_seconds, 0.0);
        assert_eq!(summary.rejected, Rejected::default());
    }

    #[test]
    fn clock_pauses_when_no_exp_comes_in() {
        let mut samples = vec![sample(0.0, 30, 24_331, 25.42), sample(1.0, 30, 24_400, 25.49)];
        // A minute standing still, one frame a second, then one more kill.
        samples.extend((2..=61).map(|t| sample(f64::from(t), 30, 24_400, 25.49)));
        samples.push(sample(62.0, 30, 24_500, 25.60));
        let summary = track(&samples);
        // 1 s to the first gain; of the idle minute only a usual kill's time (1 s, from the first
        // gain); 1 s from the reading before the last gain.
        assert_eq!(summary.active_seconds, 3.0);
        assert!(!summary.paused);
        assert_eq!(summary.exp_gained, 169);
    }

    #[test]
    fn a_map_change_pauses_at_once_and_splits_the_totals() {
        let summary = track(&[
            sample(0.0, 30, 24_331, 25.42),
            sample(60.0, 30, 30_000, 31.34),
            on(sample(120.0, 30, 30_000, 31.34), KPQ),
            on(sample(180.0, 30, 31_000, 32.39), KPQ),
            on(sample(240.0, 30, 32_000, 33.43), KPQ),
        ]);
        assert_eq!(summary.active_seconds, 180.0);
        assert_eq!(summary.map.as_deref(), Some(KPQ));
        let kpq = summary.maps.iter().find(|m| m.map.as_deref() == Some(KPQ)).unwrap();
        let kerning = summary.maps.iter().find(|m| m.map.as_deref() == Some(KERNING)).unwrap();
        assert_eq!((kpq.exp_gained, kpq.active_seconds), (2000, 120.0));
        assert_eq!((kerning.exp_gained, kerning.active_seconds), (5669, 60.0));
    }

    #[test]
    fn level_ups_carry_the_rest_of_the_old_level() {
        let summary = track(&[
            sample(0.0, 30, 95_000, 99.26),
            sample(1.0, 30, 95_000, 99.26),
            sample(10.0, 31, 100, 0.08),
            sample(20.0, 31, 600, 0.48),
        ]);
        assert_eq!(summary.level, Some(31));
        assert_eq!(summary.level_ups, 1);
        // 95,000 / 99.26% puts level 30 at about 95,708, so about 708 + 600.
        assert!((1290..=1330).contains(&summary.exp_gained), "{}", summary.exp_gained);
    }

    #[test]
    fn misread_levels_are_ignored() {
        let summary = track(&[
            sample(0.0, 30, 24_331, 25.42),
            sample(1.0, 30, 24_331, 25.42),
            // 30 read as 38 once, and as 20 once.
            sample(2.0, 38, 24_400, 25.49),
            sample(3.0, 20, 24_400, 25.49),
            sample(4.0, 30, 24_400, 25.49),
        ]);
        assert_eq!(summary.level, Some(30));
        assert_eq!(summary.level_ups, 0);
        assert_eq!(summary.rejected.exp, 2);
    }

    #[test]
    fn a_level_up_without_a_wrap_needs_a_second_reading() {
        let summary =
            track(&[sample(0.0, 30, 95_000, 99.26), sample(1.0, 30, 95_000, 99.26), sample(2.0, 31, 96_000, 99.50)]);
        assert_eq!(summary.level, Some(30));
        let summary = track(&[
            sample(0.0, 30, 95_000, 99.26),
            sample(1.0, 30, 95_000, 99.26),
            sample(10.0, 32, 100, 0.08),
            sample(11.0, 32, 100, 0.08),
        ]);
        // Two levels at once: the level in between was never measured, so no gain is guessed.
        assert_eq!((summary.level, summary.level_ups, summary.exp_gained), (Some(32), 2, 0));
    }

    #[test]
    fn mesos_need_agreeing_readings_and_skip_trades() {
        let mut storage = with_mesos(sample(40.0, 30, 24_335, 25.42), 50_000);
        storage.exchange = true;
        let summary = track(&[
            with_mesos(sample(0.0, 30, 24_331, 25.42), 28_640),
            with_mesos(sample(1.0, 30, 24_331, 25.42), 28_640),
            // A misread 6 for an 8, once.
            with_mesos(sample(20.0, 30, 24_332, 25.42), 28_840),
            with_mesos(sample(30.0, 30, 24_333, 25.42), 30_000),
            with_mesos(sample(31.0, 30, 24_334, 25.42), 30_000),
            storage.clone(),
            Sample { at: 41.0, ..storage },
            with_mesos(sample(50.0, 30, 24_336, 25.42), 50_000),
            with_mesos(sample(60.0, 30, 24_337, 25.42), 51_000),
            with_mesos(sample(61.0, 30, 24_337, 25.42), 51_000),
        ]);
        assert_eq!(summary.mesos, Some(51_000));
        assert_eq!(summary.mesos_gained, 1_360 + 1_000);
        assert_eq!(summary.mesos_excluded, 20_000);
        assert_eq!(summary.rejected.mesos, 1);
    }

    /// Thirty minutes of hunting from `from`, a kill every 30 s, the mesos out of sight.
    fn hunt(from: f64, exp: &mut u64, samples: &mut Vec<Sample>) {
        for i in 1..=60 {
            *exp += 100;
            samples.push(sample(from + f64::from(i) * 30.0, 30, *exp, (*exp as f64 * 100.0 / 95_710.0) as f32));
        }
    }

    /// A frame with a shop open, at the EXP last reached.
    fn in_shop(at: f64, exp: u64, mesos: Option<u64>) -> Sample {
        let mut s = sample(at, 30, exp, (exp as f64 * 100.0 / 95_710.0) as f32);
        s.mesos = mesos;
        s.exchange = true;
        s
    }

    #[test]
    fn a_town_trip_counts_the_loot_before_the_shop_and_not_the_shopping() {
        let mut exp = 24_331;
        let first = sample(0.0, 30, exp, (exp as f64 * 100.0 / 95_710.0) as f32);
        let mut samples = vec![with_mesos(first.clone(), 100_000), with_mesos(Sample { at: 1.0, ..first }, 100_000)];
        hunt(0.0, &mut exp, &mut samples);
        // The shop's first frame shows the mesos from before buying; two potion buys follow.
        for (at, mesos) in [(1801.0, 130_000), (1803.0, 120_000), (1805.0, 120_000), (1807.0, 110_000), (1809.0, 110_000)] {
            samples.push(in_shop(at, exp, Some(mesos)));
        }
        // Closed, and back to hunting without opening the inventory.
        hunt(1811.0, &mut exp, &mut samples);
        for (at, mesos) in [(3612.0, 135_000), (3614.0, 135_000), (3616.0, 125_000), (3618.0, 125_000)] {
            samples.push(in_shop(at, exp, Some(mesos)));
        }
        let summary = track(&samples);
        assert_eq!(summary.mesos_gained, 30_000 + 25_000);
        assert_eq!(summary.mesos_excluded, -20_000 - 10_000);
        assert!(summary.mesos_seconds > summary.active_seconds - 60.0, "{} of {}", summary.mesos_seconds, summary.active_seconds);
        assert!(close(summary.mesos_per_hour, 55_000.0 * 3600.0 / summary.mesos_seconds, 1.0));
    }

    #[test]
    fn a_shop_read_late_is_not_trusted_as_loot() {
        let mut exp = 24_331;
        let first = sample(0.0, 30, exp, (exp as f64 * 100.0 / 95_710.0) as f32);
        let mut samples = vec![with_mesos(first.clone(), 100_000), with_mesos(Sample { at: 1.0, ..first }, 100_000)];
        hunt(0.0, &mut exp, &mut samples);
        // The amount is first read 12 s into the window, time enough to have bought something.
        samples.push(in_shop(1801.0, exp, None));
        samples.push(in_shop(1813.0, exp, Some(130_000)));
        samples.push(in_shop(1815.0, exp, Some(130_000)));
        let summary = track(&samples);
        assert_eq!((summary.mesos_gained, summary.mesos_excluded), (0, 30_000));
    }

    #[test]
    fn mesos_out_of_sight_are_not_counted_yet() {
        let mut exp = 24_331;
        let first = sample(0.0, 30, exp, (exp as f64 * 100.0 / 95_710.0) as f32);
        let mut samples = vec![with_mesos(first.clone(), 100_000), with_mesos(Sample { at: 1.0, ..first }, 100_000)];
        hunt(0.0, &mut exp, &mut samples);
        let summary = track(&samples);
        assert!(close(Some(summary.mesos_unseen_seconds), summary.active_seconds, 1.0), "{summary:?}");
        assert!(summary.active_seconds > 1700.0);
        assert_eq!((summary.mesos_seconds, summary.mesos_per_hour), (0.0, None));
    }

    fn with_counter(mut sample: Sample, current: u32) -> Sample {
        sample.counters = vec![Counter { target: "Evil Eye Tail".into(), current, required: 30 }];
        sample
    }

    #[test]
    fn quest_counters_give_a_pace_per_map() {
        let mut samples = Vec::new();
        // Ten minutes, a kill every 30 s and a counter tick every minute, two frames each.
        for i in 0..=20u32 {
            let at = f64::from(i) * 30.0;
            let points = 24_331 + u64::from(i) * 100;
            let s = with_counter(sample(at, 30, points, (points as f64 * 100.0 / 95_710.0) as f32), i / 2);
            samples.push(s.clone());
            samples.push(Sample { at: at + 0.5, ..s });
        }
        let summary = track(&samples);
        assert_eq!(summary.counters.len(), 1);
        let rate = &summary.counters[0];
        assert_eq!((rate.target.as_str(), rate.required, rate.map.as_deref()), ("Evil Eye Tail", 30, Some(KERNING)));
        assert_eq!(rate.gained, 10);
        assert!(close(rate.per_hour, 60.0, 4.0), "{:?} over {}", rate.per_hour, rate.active_seconds);
    }

    #[test]
    fn quest_counters_ignore_misreads_and_hand_ins() {
        let summary = track(&[
            with_counter(sample(0.0, 30, 24_331, 25.42), 4),
            with_counter(sample(1.0, 30, 24_331, 25.42), 4),
            // A 5 misread as 9 once.
            with_counter(sample(30.0, 30, 24_431, 25.53), 9),
            with_counter(sample(31.0, 30, 24_431, 25.53), 5),
            with_counter(sample(32.0, 30, 24_431, 25.53), 5),
            // Handed in and taken again: it starts from 0, which is no gain.
            with_counter(sample(60.0, 30, 24_531, 25.63), 0),
            with_counter(sample(61.0, 30, 24_531, 25.63), 0),
            with_counter(sample(90.0, 30, 24_631, 25.74), 1),
            with_counter(sample(91.0, 30, 24_631, 25.74), 1),
        ]);
        // 4 to 5 over 30 s, and 0 to 1 over 30 s: the idle second after the last gain waits for
        // the next one.
        let rate = &summary.counters[0];
        assert_eq!((rate.gained, rate.active_seconds), (2, 60.0));
        assert!(close(rate.per_hour, 120.0, 0.1), "{:?}", rate.per_hour);
    }

    /// A look at the mesos at the start, six minutes with the inventory shut, then another look.
    fn two_looks(until: u32) -> SessionSummary {
        let mut samples = vec![with_mesos(sample(0.0, 30, 24_000, 25.0), 1_000)];
        samples.push(with_mesos(sample(1.0, 30, 24_000, 25.0), 1_000));
        for step in 1..=until / 10 {
            let at = f64::from(step * 10);
            let s = sample(at, 30, 24_000 + u64::from(step) * 10, 25.0 + step as f32 * 10.0 / 960.0);
            if step * 10 == 360 {
                // A new amount needs two agreeing readings.
                samples.push(with_mesos(s.clone(), 1_420));
                samples.push(with_mesos(Sample { at: at + 1.0, ..s }, 1_420));
            } else {
                samples.push(s);
            }
        }
        track(&samples)
    }

    #[test]
    fn the_meso_graph_spreads_a_gain_over_the_time_between_looks() {
        // Before the second look the loot is unseen, so the graph has no meso rate yet rather
        // than a falling one.
        let before = two_looks(350);
        assert!(before.timeline.len() >= 5);
        assert!(before.timeline.iter().all(|p| p.mesos_per_hour.is_none()), "{:?}", before.timeline);

        // 420 mesos over 6 minutes is 4,200 an hour, all along, not a jump at the end.
        let after = two_looks(360);
        assert_eq!(after.mesos_gained, 420);
        let rates: Vec<f64> = after.timeline.iter().filter_map(|p| p.mesos_per_hour).collect();
        assert!(rates.len() >= 10, "{:?}", after.timeline);
        assert!(rates.iter().all(|&r| (r - 4_200.0).abs() < 50.0), "{rates:?}");
        assert!(close(after.recent_mesos_per_hour, 4_200.0, 50.0), "{:?}", after.recent_mesos_per_hour);

        // Play on with the inventory shut: the new stretch waits for the next look.
        let later = two_looks(420);
        assert!(later.timeline.last().is_some_and(|p| p.at > 370.0 && p.mesos_per_hour.is_none()));
    }

    #[test]
    fn a_single_screenshot_per_amount_can_be_trusted_when_asked() {
        let config = TrackerConfig { confirmations: 1, ..TrackerConfig::default() };
        let mut tracker = SessionTracker::new(config);
        for s in [
            with_mesos(sample(0.0, 30, 24_331, 25.42), 33_040),
            sample(60.0, 30, 30_000, 31.34),
            with_mesos(sample(120.0, 30, 45_930, 47.99), 42_944),
        ] {
            tracker.push(&s);
        }
        let summary = tracker.summary();
        assert_eq!(summary.mesos_gained, 9_904);
        assert!(close(summary.mesos_per_hour, 9_904.0 * 30.0, 0.5), "{:?}", summary.mesos_per_hour);
    }

    /// The EXP level 30 needs in these tests, which `sample`'s numbers agree with.
    const NEED: f64 = 95_717.0;

    /// A reading at `points` of level 30 on Kerning, whose monsters give `kill_exp`.
    fn hunting(at: f64, points: u64, kill_exp: &[u32]) -> Sample {
        let percent = ((points as f64 * 100.0 / NEED) * 100.0).floor() as f32 / 100.0;
        Sample { kill_exp: kill_exp.to_vec(), ..sample(at, 30, points, percent) }
    }

    /// A reading a second for `seconds`, with a kill worth `exp` every `every` seconds.
    fn grind(from: f64, seconds: u32, every: u32, exp: u32, points: &mut u64, kill_exp: &[u32]) -> Vec<Sample> {
        (1..=seconds)
            .map(|s| {
                if s % every == 0 {
                    *points += u64::from(exp);
                }
                hunting(from + f64::from(s), *points, kill_exp)
            })
            .collect()
    }

    #[test]
    fn gains_are_matched_to_kills() {
        assert_eq!(kills_in(43, &[43, 52]), Some(1));
        assert_eq!(kills_in(95, &[52, 43]), Some(2));
        assert_eq!(kills_in(44, &[43, 52]), Some(1), "a kill may be one EXP out");
        assert_eq!(kills_in(150, &[43, 52]), Some(3));
        assert_eq!(kills_in(120, &[100]), None);
        assert_eq!(kills_in(300, &[43, 52]), None, "more than four kills in one reading");
        assert_eq!(classify(120, &[100]), GainKind::Hunting { checked: true });
        assert_eq!(classify(5_000, &[43, 52]), GainKind::Other);
        assert_eq!(classify(5_000, &[]), GainKind::Hunting { checked: false });
    }

    #[test]
    fn quest_exp_is_left_out_of_the_rates() {
        let mut points = 24_000;
        let mut samples = vec![hunting(0.0, points, &[50]), hunting(1.0, points, &[50])];
        samples.extend(grind(1.0, 300, 5, 50, &mut points, &[50]));
        // A quest handed in on the map.
        points += 5_000;
        samples.push(hunting(302.0, points, &[50]));
        samples.push(hunting(303.0, points, &[50]));
        let summary = track(&samples);
        assert_eq!(summary.exp_other, 5_000);
        assert_eq!(summary.exp_gained, 60 * 50);
        assert_eq!(summary.kills, 60);
        assert_eq!((summary.gains_matched, summary.gains_unmatched), (60, 0));
        let line = &summary.lines[0];
        assert_eq!((line.kills, line.other_exp, line.exp_gained), (Some(60), 5_000, 3_000));
        assert!(close(summary.kills_per_hour, 720.0, 15.0), "{:?}", summary.kills_per_hour);
    }

    #[test]
    fn the_last_five_minutes_are_measured_exactly() {
        let mut points = 24_000;
        let mut samples = vec![hunting(0.0, points, &[]), hunting(1.0, points, &[])];
        samples.extend(grind(1.0, 600, 10, 100, &mut points, &[]));
        let summary = track(&samples);
        // 100 EXP every 10 s is 36,000 an hour, however the window falls between gains.
        assert!(close(summary.recent_exp_per_hour, 36_000.0, 1.0), "{:?}", summary.recent_exp_per_hour);
        assert!(close(summary.exp_per_hour, 36_000.0, 100.0), "{:?}", summary.exp_per_hour);
    }

    #[test]
    fn an_estimate_steadies_the_pace_until_play_takes_over() {
        let mut tracker = SessionTracker::new(TrackerConfig::default());
        tracker.set_prior(KERNING, Some(30_000.0));
        let mut points = 24_000;
        let mut samples = vec![hunting(0.0, points, &[]), hunting(1.0, points, &[])];
        samples.extend(grind(1.0, 20, 10, 100, &mut points, &[]));
        samples.iter().for_each(|s| tracker.push(s));
        let early = tracker.summary();
        // 200 EXP in about 20 s, plus 180 s of the estimate's 30,000 an hour.
        let here = &early.maps[0];
        assert_eq!(here.exp_gained, 200);
        let expected = (200.0 + 30_000.0 * 0.05) * 3600.0 / (here.active_seconds + 180.0);
        assert!(close(early.pace_exp_per_hour, expected, 1.0), "{:?}", early.pace_exp_per_hour);
        assert!(early.pace_from_estimate);
        assert!(early.seconds_to_level.is_some());

        // Half an hour later the measured 36,000 an hour has all but taken over.
        let mut later = Vec::new();
        later.extend(grind(21.0, 1800, 10, 100, &mut points, &[]));
        later.iter().for_each(|s| tracker.push(s));
        let late = tracker.summary();
        assert!(!late.pace_from_estimate);
        assert!(close(late.pace_exp_per_hour, 35_450.0, 200.0), "{:?}", late.pace_exp_per_hour);
        // Steady kills of one size: the range is tight.
        assert!(late.pace_error.is_some_and(|e| e < 0.1), "{:?}", late.pace_error);
        let (low, high) = (late.seconds_to_level_low.unwrap(), late.seconds_to_level_high.unwrap());
        let eta = late.seconds_to_level.unwrap();
        assert!(low < eta && eta < high, "{low} {eta} {high}");
    }

    #[test]
    fn breaks_count_once_the_session_has_run_a_while() {
        let mut points = 24_000;
        let mut samples = vec![hunting(0.0, points, &[]), hunting(1.0, points, &[])];
        samples.extend(grind(1.0, 900, 10, 100, &mut points, &[]));
        // Ten minutes standing in town, a frame a second.
        samples.extend((902..1502).map(|t| hunting(f64::from(t), points, &[])));
        samples.extend(grind(1502.0, 300, 10, 100, &mut points, &[]));
        let summary = track(&samples);
        // The ten minutes count only as a usual kill's 10 s.
        let active = summary.active_seconds;
        assert!((1195.0..1215.0).contains(&active), "{active}");
        assert!((summary.real_seconds - 1802.0).abs() < 1.0, "{}", summary.real_seconds);
        let (eta, with_breaks) = (summary.seconds_to_level.unwrap(), summary.seconds_to_level_with_breaks.unwrap());
        assert!((with_breaks / eta - summary.real_seconds / summary.active_seconds).abs() < 0.02);
        assert!(close(summary.exp_per_hour_with_breaks, 120.0 * 100.0 * 3600.0 / 1802.0, 50.0));
    }

    #[test]
    fn deaths_set_the_pace_back_on_their_map() {
        let mut points = 30_000;
        let mut samples = vec![hunting(0.0, points, &[]), hunting(1.0, points, &[])];
        samples.extend(grind(1.0, 300, 10, 300, &mut points, &[]));
        // Died: 10% of the level gone, read twice.
        points -= 9_571;
        samples.push(hunting(302.0, points, &[]));
        samples.push(hunting(303.0, points, &[]));
        samples.extend(grind(303.0, 300, 10, 300, &mut points, &[]));
        let summary = track(&samples);
        assert_eq!(summary.deaths, 1);
        assert!(close(summary.exp_per_hour, 108_000.0, 2_000.0), "deaths stay out of the pace shown");
        // The time to level goes by what is left after the loss.
        let net = (18_000.0 - 9_571.0) * 3600.0 / summary.active_seconds;
        assert!(close(summary.pace_exp_per_hour, net, 600.0), "{:?} {net}", summary.pace_exp_per_hour);
        assert_eq!(summary.lines[0].deaths, 1);
        assert_eq!(summary.lines[0].exp_lost, 9_571);
    }
}
