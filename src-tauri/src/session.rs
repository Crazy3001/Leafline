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
//! - **Mesos count once several readings in a row agree.** Changes while a storage or trade
//!   window is open (a second meso box on screen) are kept apart, since they are not loot.
//! - **Quest counters count the same way**, and only going up: a counter that drops (a quest
//!   handed in and taken again) starts afresh. Their gains over active time give the player's
//!   own pace on each objective, per map.
//! - **The clock pauses** after [`TrackerConfig::idle_after`] seconds without EXP, and at once
//!   on a map change without EXP, so time in town or away does not dilute the rates.
//!
//! A missing reading is skipped, never guessed. No EXP table is built in: the EXP each level
//! needs is measured from the HUD itself, since Classic's table may differ from old GMS.

use serde::Serialize;

use crate::pipeline::Analysis;
use crate::readers::hud::Exp;
use crate::readers::map::MapReading;

/// What one frame says, at one moment.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sample {
    /// Seconds from any fixed point, such as the Unix epoch.
    pub at: f64,
    pub level: Option<u32>,
    pub exp: Option<Exp>,
    pub mesos: Option<u64>,
    /// A storage or trade window is open, so meso changes are not loot.
    pub exchange: bool,
    pub map: Option<String>,
    /// The Quest Helper's counters.
    pub counters: Vec<Counter>,
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
            mesos: analysis.mesos.map(|m| m.mesos),
            exchange: analysis.meso_boxes >= 2,
            map: analysis.map.as_ref().map(MapReading::label),
            counters: analysis
                .quest
                .iter()
                .flat_map(|q| &q.quests)
                .flat_map(|q| &q.objectives)
                .map(|o| Counter { target: o.target.clone(), current: o.current, required: o.required })
                .collect(),
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
    pub level: Option<u32>,
    pub exp: Option<Exp>,
    /// The EXP the current level needs, as measured from the HUD.
    pub exp_needed: Option<u64>,
    /// Net EXP gained; EXP lost (on death, for example) is subtracted.
    pub exp_gained: i64,
    pub exp_per_hour: Option<f64>,
    pub percent_per_hour: Option<f64>,
    pub seconds_to_level: Option<f64>,
    pub level_ups: u32,
    /// The latest confirmed amount.
    pub mesos: Option<u64>,
    /// Net mesos gained outside storage and trade windows.
    pub mesos_gained: i64,
    pub mesos_per_hour: Option<f64>,
    /// Net change while a storage or trade window was open, left out of the rates.
    pub mesos_excluded: i64,
    pub map: Option<String>,
    /// Per map, most active time first.
    pub maps: Vec<MapSummary>,
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

/// How fast one quest counter went up on one map.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Rejected {
    pub exp: usize,
    pub mesos: usize,
}

/// The EXP a level needs, as a range: the HUD's percent has two decimals, and whether it rounds
/// or truncates is not known, so the true percent is anywhere from 0.005 below to 0.01 above.
#[derive(Debug, Clone, Copy, PartialEq)]
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

#[derive(Debug, Clone)]
struct ExpPoint {
    at: f64,
    level: u32,
    exp: Exp,
    map: Option<String>,
}

#[derive(Debug, Clone)]
struct MesoPoint {
    mesos: u64,
    /// The active clock when this amount first appeared.
    clock: f64,
    map: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct MapTotals {
    map: Option<String>,
    active_seconds: f64,
    exp_gained: i64,
    mesos_gained: i64,
    meso_seconds: f64,
}

/// One quest counter as tracked: the value being read, and the last confirmed one.
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Default)]
pub struct SessionTracker {
    config: TrackerConfig,
    frames: usize,
    started_at: Option<f64>,
    updated_at: Option<f64>,
    /// The last map read, carried over frames where the minimap is hidden.
    map: Option<String>,

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

    /// The amount being read and how many readings in a row agree on it.
    meso_run: Option<(MesoPoint, usize)>,
    mesos: Option<MesoPoint>,
    /// A storage or trade window was seen since the last confirmed amount.
    exchange_seen: bool,
    mesos_gained: i64,
    meso_seconds: f64,
    mesos_excluded: i64,

    maps: Vec<MapTotals>,
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
        let read_any = sample.exp.is_some() || sample.level.is_some() || sample.mesos.is_some() || sample.map.is_some();
        if read_any {
            self.started_at = Some(self.started_at.map_or(sample.at, |s| s.min(sample.at)));
            self.updated_at = Some(self.updated_at.map_or(sample.at, |u| u.max(sample.at)));
        }
        if sample.map.is_some() {
            self.map.clone_from(&sample.map);
        }
        self.exchange_seen |= sample.exchange;
        self.push_exp(sample);
        self.push_mesos(sample);
        self.push_counters(sample);
    }

    fn push_exp(&mut self, sample: &Sample) {
        let (Some(level), Some(exp)) = (sample.level, sample.exp) else { return };
        if !(0.0..100.0).contains(&exp.percent) {
            self.rejected.exp += 1;
            return;
        }
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
            if agree(self.requirement(level), implied) {
                self.rejected.exp += usize::from(self.pending.take().is_some());
                self.accept(&last, point);
            } else {
                self.rejected.exp += 1;
            }
        } else if point.level > last.level {
            let wrapped = point.level == last.level + 1 && point.exp.percent < last.exp.percent;
            let fits = agree(self.requirement(level), implied);
            match self.pending.take() {
                _ if wrapped && fits => self.accept(&last, point),
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

        let dt = (next.at - last.at).max(0.0);
        let moved = matches!((&last.map, &next.map), (Some(a), Some(b)) if a != b);
        let counted = match gain {
            None => 0.0,
            Some(g) if g > 0 => {
                self.idle_for = 0.0;
                dt
            }
            Some(_) if moved => {
                self.idle_for = self.config.idle_after;
                0.0
            }
            Some(_) => {
                let counted = (self.config.idle_after - self.idle_for).clamp(0.0, dt);
                self.idle_for += dt;
                counted
            }
        };
        self.clock += counted;
        self.exp_gained += gain.unwrap_or(0);
        self.level_ups += next.level - last.level;

        if counted > 0.0 || gain.is_some_and(|g| g != 0) {
            let map = if moved { None } else { last.map.clone().or_else(|| next.map.clone()) };
            let totals = self.totals(map);
            totals.active_seconds += counted;
            totals.exp_gained += gain.unwrap_or(0);
        }

        let implied = Requirement::implied(next.exp);
        self.set_requirement(next.level, implied);
        self.exp = Some(next);
    }

    fn push_mesos(&mut self, sample: &Sample) {
        let Some(mesos) = sample.mesos else { return };
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
        if count != self.config.confirmations.max(1) {
            // The confirmed amount seen again: any window open before has closed without
            // changing it, unless it is still open.
            if self.mesos.as_ref().is_some_and(|m| m.mesos == mesos) {
                self.exchange_seen = sample.exchange;
            }
            return;
        }
        let run = self.meso_run.as_ref().map(|(run, _)| run.clone()).expect("a run was just updated");
        if let Some(previous) = self.mesos.replace(run.clone()) {
            let delta = run.mesos as i64 - previous.mesos as i64;
            if self.exchange_seen {
                self.mesos_excluded += delta;
            } else if delta != 0 {
                let seconds = run.clock - previous.clock;
                self.mesos_gained += delta;
                self.meso_seconds += seconds;
                let map = if previous.map == run.map { previous.map } else { None };
                let totals = self.totals(map);
                totals.mesos_gained += delta;
                totals.meso_seconds += seconds;
            }
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

    pub fn summary(&self) -> SessionSummary {
        let rate = |amount: i64, seconds: f64| {
            (seconds >= self.config.min_rate_seconds).then(|| amount as f64 * 3600.0 / seconds)
        };
        let exp_per_hour = rate(self.exp_gained, self.clock);
        let exp_needed = self.exp.as_ref().and_then(|e| self.requirement(e.level)).and_then(Requirement::estimate);
        let seconds_to_level = match (exp_per_hour, exp_needed, &self.exp) {
            (Some(per_hour), Some(needed), Some(last)) if per_hour > 0.0 => {
                Some(needed.saturating_sub(last.exp.points) as f64 / per_hour * 3600.0)
            }
            _ => None,
        };
        let mut maps: Vec<MapSummary> = self
            .maps
            .iter()
            .map(|t| MapSummary {
                map: t.map.clone(),
                active_seconds: t.active_seconds,
                exp_gained: t.exp_gained,
                exp_per_hour: rate(t.exp_gained, t.active_seconds),
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

        SessionSummary {
            frames: self.frames,
            started_at: self.started_at,
            updated_at: self.updated_at,
            elapsed_seconds: self.started_at.zip(self.updated_at).map_or(0.0, |(s, u)| u - s),
            active_seconds: self.clock,
            paused: self.exp.is_some() && self.idle_for >= self.config.idle_after,
            level: self.exp.as_ref().map(|e| e.level),
            exp: self.exp.as_ref().map(|e| e.exp),
            exp_needed,
            exp_gained: self.exp_gained,
            exp_per_hour,
            percent_per_hour: exp_per_hour.zip(exp_needed).map(|(per_hour, needed)| per_hour * 100.0 / needed as f64),
            seconds_to_level,
            level_ups: self.level_ups,
            mesos: self.mesos.as_ref().map(|m| m.mesos),
            mesos_gained: self.mesos_gained,
            mesos_per_hour: rate(self.mesos_gained, self.meso_seconds),
            mesos_excluded: self.mesos_excluded,
            map: self.map.clone(),
            maps,
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

    fn track(samples: &[Sample]) -> SessionSummary {
        let mut tracker = SessionTracker::new(TrackerConfig::default());
        samples.iter().for_each(|s| tracker.push(s));
        tracker.summary()
    }

    fn close(actual: Option<f64>, expected: f64, tolerance: f64) -> bool {
        actual.is_some_and(|a| (a - expected).abs() <= tolerance)
    }

    /// Real numbers: 24,331 EXP (25.42%) to 45,930 (47.99%) and 28,640 to 42,944 mesos,
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
        // 1 s to the first gain, 15 s of the idle minute, 1 s to the last gain.
        assert_eq!(summary.active_seconds, 17.0);
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
        // 4 to 5 over 31 s, and 0 to 1 over 30 s.
        let rate = &summary.counters[0];
        assert_eq!((rate.gained, rate.active_seconds), (2, 61.0));
        assert!(close(rate.per_hour, 118.0, 0.1), "{:?}", rate.per_hour);
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
}
