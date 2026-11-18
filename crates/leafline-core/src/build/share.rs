//! Moving a plan between players as a line of text. Only the player moves it, by clipboard, and
//! it holds only the plan: no character, no stats, nothing read from the game, and no link, so
//! a code from a stranger can't put one in front of the player.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};

use super::plan::{MAX_LEVEL, Plan};
use crate::error::{Error, Result};

const PREFIX: &str = "LEAFLINE-BUILD-1:";

/// Longer than any real plan, so a wrong paste fails quickly.
const MAX_CODE: usize = 64 * 1024;
/// The most goals of each kind a pasted plan may bring.
const MAX_GOALS: usize = 300;
const MAX_NAME: usize = 80;

#[derive(Serialize, Deserialize)]
struct Shared {
    name: String,
    plan: Plan,
}

pub fn export(name: &str, plan: &Plan) -> Result<String> {
    let shared = Shared { name: name.to_string(), plan: plan.clone() };
    let json = serde_json::to_vec(&shared).map_err(|err| Error::Invalid(err.to_string()))?;
    Ok(format!("{PREFIX}{}", URL_SAFE_NO_PAD.encode(json)))
}

/// Reads a code. Skills or items Leafline doesn't know are kept, and the plan reports them.
pub fn import(code: &str) -> Result<(String, Plan)> {
    let wrong = || Error::Invalid("That isn't a Leafline build code.".into());
    let code = code.trim();
    if code.len() > MAX_CODE {
        return Err(wrong());
    }
    let body = code.strip_prefix(PREFIX).ok_or_else(wrong)?;
    let json = URL_SAFE_NO_PAD.decode(body).map_err(|_| wrong())?;
    let Shared { name, mut plan } = serde_json::from_slice(&json).map_err(|_| wrong())?;
    plan.skills.truncate(MAX_GOALS);
    plan.stats.truncate(MAX_GOALS);
    plan.gear.truncate(MAX_GOALS);
    plan.plan_to = plan.plan_to.clamp(1, MAX_LEVEL);
    Ok((name.chars().take(MAX_NAME).collect(), plan))
}

#[cfg(test)]
mod tests {
    use super::super::plan::{SkillLevel, Stat, StatGoal};
    use super::*;

    fn plan() -> Plan {
        Plan {
            job: 420,
            skills: vec![SkillLevel { skill_id: 999_999, level: 5 }],
            stats: vec![StatGoal { stat: Stat::Luk, target: None, from_level: Some(11) }],
            gear: vec![7],
            plan_to: 90,
        }
    }

    #[test]
    fn a_plan_survives_the_trip_with_unknown_skills_kept() {
        let code = export("Fern runner", &plan()).unwrap();
        assert!(code.starts_with(PREFIX));
        assert_eq!(import(&format!("  {code}\n")).unwrap(), ("Fern runner".to_string(), plan()));
    }

    #[test]
    fn other_text_is_refused() {
        assert!(import("hello").is_err());
        assert!(import("LEAFLINE-BUILD-1:not*base64").is_err());
        assert!(import("LEAFLINE-BUILD-1:aGVsbG8").is_err());
        assert!(import(&format!("{PREFIX}{}", "A".repeat(MAX_CODE))).is_err());
    }

    #[test]
    fn a_pasted_plan_stays_within_limits() {
        let huge = Plan {
            skills: vec![SkillLevel { skill_id: 1, level: 1 }; MAX_GOALS + 5],
            plan_to: 100_000,
            ..plan()
        };
        let (name, plan) = import(&export(&"x".repeat(500), &huge).unwrap()).unwrap();
        assert_eq!((plan.skills.len(), plan.plan_to, name.len()), (MAX_GOALS, MAX_LEVEL, MAX_NAME));
    }
}
