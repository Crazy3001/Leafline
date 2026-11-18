//! A session the player types in: the level and EXP the HUD showed at the start and at the end,
//! and how long they played. The website has no other way to know, and the program uses it
//! while live reading is off. It is saved like any session, so the training log, the training
//! spot finder and the share card use it the same way.
//!
//! EXP percent is always enough. EXP points, when typed too, give EXP an hour: within one level
//! directly, across levels from what each level takes (worked out from the points and percent
//! typed, or from what the HUD showed before).

use serde::{Deserialize, Serialize};

use super::host::{Host, emit};
use super::records::HISTORY_EVENT;
use crate::clock::now_f64;
use crate::error::{Error, Result};
use crate::session::{MapSummary, SessionLine, SessionSummary};

/// The longest session that can be typed in, in minutes.
const MOST_MINUTES: f64 = 24.0 * 60.0;
/// The highest level.
const MAX_LEVEL: u32 = 200;

/// What the HUD showed at one moment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HudAt {
    pub level: u32,
    /// The EXP bar's percent, 0 to 100.
    pub percent: f64,
    /// The EXP points beside it, if typed.
    pub points: Option<u64>,
    /// The mesos in the inventory, if typed.
    pub mesos: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypedSession {
    pub character: Option<String>,
    /// The map, as `Street : Name` when the street is known, as the program saves it.
    pub map: Option<String>,
    pub start: HudAt,
    pub end: HudAt,
    /// Minutes of play.
    pub minutes: f64,
    /// When it started, in Unix seconds; it ended `minutes` later. Without it, it ended now.
    pub started_at: Option<f64>,
    pub note: Option<String>,
}

/// What a typed-in session works out to.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypedOutcome {
    pub percent_gained: f64,
    pub percent_per_hour: f64,
    /// `None` when the EXP points weren't typed, or a level in between isn't known.
    pub exp_gained: Option<i64>,
    pub exp_per_hour: Option<f64>,
    pub mesos_gained: Option<i64>,
    pub mesos_per_hour: Option<f64>,
    /// Minutes to the next level at this pace.
    pub minutes_to_level: Option<f64>,
    pub level_ups: u32,
}

/// What a level takes, from points and percent read at the same moment.
fn needed_from(hud: &HudAt) -> Option<u64> {
    let points = hud.points? as f64;
    (hud.percent >= 1.0 && points > 0.0).then(|| (points * 100.0 / hud.percent).round() as u64)
}

fn check(typed: &TypedSession) -> Result<()> {
    let (start, end) = (&typed.start, &typed.end);
    for hud in [start, end] {
        if hud.level == 0 || hud.level > MAX_LEVEL {
            return Err(Error::Invalid(format!("A level is from 1 to {MAX_LEVEL}.")));
        }
        if !(0.0..100.0).contains(&hud.percent) || !hud.percent.is_finite() {
            return Err(Error::Invalid("EXP percent is from 0 to just under 100.".into()));
        }
    }
    if !(typed.minutes.is_finite() && typed.minutes >= 1.0 && typed.minutes <= MOST_MINUTES) {
        return Err(Error::Invalid("Type how many minutes you played, from 1 to a day.".into()));
    }
    let gained = (f64::from(end.level) - f64::from(start.level)) * 100.0 + end.percent - start.percent;
    if gained <= 0.0 {
        return Err(Error::Invalid("The end should be further on than the start.".into()));
    }
    Ok(())
}

/// Works out a typed-in session. `known` is what each level takes, as the HUD showed before.
pub fn work_out(typed: &TypedSession, known: &[(u32, u64)]) -> Result<TypedOutcome> {
    check(typed)?;
    let (start, end) = (&typed.start, &typed.end);
    let hours = typed.minutes / 60.0;
    let percent_gained = (f64::from(end.level) - f64::from(start.level)) * 100.0 + end.percent - start.percent;
    let needed = |level: u32| -> Option<u64> {
        if level == start.level && let Some(n) = needed_from(start) {
            return Some(n);
        }
        if level == end.level && let Some(n) = needed_from(end) {
            return Some(n);
        }
        known.iter().find(|(l, _)| *l == level).map(|(_, n)| *n)
    };
    let exp_gained = match (start.points, end.points) {
        (Some(a), Some(b)) if start.level == end.level => (b >= a).then(|| (b - a) as i64),
        (Some(a), Some(b)) => (|| {
            let mut total = needed(start.level)?.checked_sub(a)? as i64;
            for level in start.level + 1..end.level {
                total += needed(level)? as i64;
            }
            Some(total + b as i64)
        })(),
        _ => None,
    };
    let mesos_gained = start.mesos.zip(end.mesos).map(|(a, b)| b - a);
    let percent_per_hour = percent_gained / hours;
    Ok(TypedOutcome {
        percent_gained,
        percent_per_hour,
        exp_gained,
        exp_per_hour: exp_gained.map(|e| e as f64 / hours),
        mesos_gained,
        mesos_per_hour: mesos_gained.map(|m| m as f64 / hours),
        minutes_to_level: (percent_per_hour > 0.0).then(|| (100.0 - end.percent) / percent_per_hour * 60.0),
        level_ups: end.level - start.level,
    })
}

fn known_levels(h: &dyn Host) -> Vec<(u32, u64)> {
    h.records().with(|db| db.level_exp()).unwrap_or_default().into_iter().map(|l| (l.level, l.needed)).collect()
}

/// What a typed-in session works out to, before it is saved.
pub fn typed_session_outcome(h: &dyn Host, session: TypedSession) -> Result<TypedOutcome> {
    work_out(&session, &known_levels(h))
}

/// Saves a typed-in session in the training log. Returns its id.
pub fn save_typed_session(h: &dyn Host, session: TypedSession) -> Result<i64> {
    let outcome = work_out(&session, &known_levels(h))?;
    let seconds = session.minutes * 60.0;
    let started_at = session.started_at.unwrap_or_else(|| now_f64() - seconds);
    let character = session.character.as_deref().map(str::trim).filter(|c| !c.is_empty()).map(str::to_string);
    let map = session.map.as_deref().map(str::trim).filter(|m| !m.is_empty()).map(str::to_string);
    let exp_gained = outcome.exp_gained.unwrap_or(0);
    let line = SessionLine {
        map: map.clone(),
        level: Some(session.start.level),
        active_seconds: seconds,
        exp_gained,
        percent_gained: outcome.percent_gained,
        mesos_gained: outcome.mesos_gained.unwrap_or(0),
        mesos_seconds: if outcome.mesos_gained.is_some() { seconds } else { 0.0 },
        ..SessionLine::default()
    };
    let summary = SessionSummary {
        started_at: Some(started_at),
        updated_at: Some(started_at + seconds),
        elapsed_seconds: seconds,
        active_seconds: seconds,
        real_seconds: seconds,
        level: Some(session.end.level),
        percent: Some(session.end.percent as f32),
        percent_only: outcome.exp_gained.is_none(),
        exp_needed: needed_from(&session.end),
        exp_gained,
        exp_per_hour: outcome.exp_per_hour,
        percent_per_hour: Some(outcome.percent_per_hour),
        level_ups: outcome.level_ups,
        mesos_gained: outcome.mesos_gained.unwrap_or(0),
        mesos_per_hour: outcome.mesos_per_hour,
        mesos_seconds: line.mesos_seconds,
        map: map.clone(),
        name: character,
        maps: vec![MapSummary {
            map,
            active_seconds: seconds,
            exp_gained,
            exp_per_hour: outcome.exp_per_hour,
            mesos_gained: outcome.mesos_gained.unwrap_or(0),
            mesos_per_hour: outcome.mesos_per_hour,
        }],
        lines: vec![line],
        ..SessionSummary::default()
    };
    let id = h.records().with(|db| {
        let id = db.save_session(&summary, "Typed in", crate::clock::now())?;
        if let Some(note) = session.note.as_deref().filter(|n| !n.trim().is_empty()) {
            db.set_session_note(id, note)?;
        }
        Ok(id)
    })?;
    emit(h, HISTORY_EVENT, &());
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hud(level: u32, percent: f64, points: Option<u64>) -> HudAt {
        HudAt { level, percent, points, mesos: None }
    }

    fn typed(start: HudAt, end: HudAt, minutes: f64) -> TypedSession {
        TypedSession { start, end, minutes, ..TypedSession::default() }
    }

    #[test]
    fn percent_alone_gives_percent_an_hour_and_time_to_level() {
        let out = work_out(&typed(hud(30, 10.0, None), hud(30, 40.0, None), 30.0), &[]).unwrap();
        assert_eq!(out.percent_per_hour, 60.0);
        assert_eq!(out.exp_gained, None);
        assert_eq!(out.minutes_to_level, Some(60.0));
    }

    #[test]
    fn points_within_a_level_give_exp_an_hour() {
        let out = work_out(&typed(hud(30, 10.0, Some(1_000)), hud(30, 40.0, Some(4_000)), 60.0), &[]).unwrap();
        assert_eq!(out.exp_gained, Some(3_000));
        assert_eq!(out.exp_per_hour, Some(3_000.0));
    }

    #[test]
    fn across_levels_what_each_takes_comes_from_the_points_or_the_hud_before() {
        // Level 30 takes 10,000 (1,000 at 10%); level 31 takes 12,000 as the HUD showed before.
        let session = typed(hud(30, 10.0, Some(1_000)), hud(32, 5.0, Some(700)), 60.0);
        assert_eq!(work_out(&session, &[]).unwrap().exp_gained, None);
        let out = work_out(&session, &[(31, 12_000)]).unwrap();
        assert_eq!(out.exp_gained, Some(9_000 + 12_000 + 700));
        assert_eq!(out.level_ups, 2);
        assert!((out.percent_gained - 195.0).abs() < 1e-9);
    }

    #[test]
    fn nonsense_is_refused() {
        assert!(work_out(&typed(hud(30, 40.0, None), hud(30, 10.0, None), 30.0), &[]).is_err());
        assert!(work_out(&typed(hud(30, 10.0, None), hud(30, 40.0, None), 0.0), &[]).is_err());
        assert!(work_out(&typed(hud(0, 10.0, None), hud(30, 40.0, None), 30.0), &[]).is_err());
        assert!(work_out(&typed(hud(30, 10.0, None), hud(30, 100.0, None), 30.0), &[]).is_err());
    }
}
