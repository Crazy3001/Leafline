//! How many points a character earns and when. No source is official: AP and Beginner SP at
//! Lv 2 were seen in game, and the rest come from community guides, so the player can change each.

use serde::{Deserialize, Serialize};

use super::plan::Stat;

/// A job's family, the hundreds of its id. Tooltips and item data name them this way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Branch {
    Warrior,
    Magician,
    Bowman,
    Thief,
}

impl Branch {
    pub fn of_job(job: u32) -> Option<Self> {
        match job / 100 {
            1 => Some(Self::Warrior),
            2 => Some(Self::Magician),
            3 => Some(Self::Bowman),
            4 => Some(Self::Thief),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Warrior => "Warrior",
            Self::Magician => "Magician",
            Self::Bowman => "Bowman",
            Self::Thief => "Thief",
        }
    }

    /// The stat the wiki's Jobs page names for the branch, used to start a new build.
    pub fn primary(self) -> Stat {
        match self {
            Self::Warrior => Stat::Str,
            Self::Magician => Stat::Int,
            Self::Bowman => Stat::Dex,
            Self::Thief => Stat::Luk,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Rules {
    pub ap_per_level: u32,
    /// SP each level as a first or second job. The level of an advancement counts as the job before.
    pub sp_per_level: u32,
    /// SP each level as a Beginner, for Beginner skills only.
    pub beginner_sp_per_level: u32,
    /// SP from each job advancement, for that job's skills.
    pub advancement_sp: u32,
    pub first_job_level: u32,
    pub second_job_level: u32,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            ap_per_level: 5,
            sp_per_level: 3,
            beginner_sp_per_level: 1,
            advancement_sp: 1,
            first_job_level: 10,
            second_job_level: 30,
        }
    }
}

impl Rules {
    /// The SP levels `from + 1` to `to` bring, as [`super::plan::expand`] counts them.
    pub fn sp_earned(&self, from: u32, to: u32, has_job: bool) -> u32 {
        let per_level = |level: u32| {
            if level <= self.first_job_level {
                self.beginner_sp_per_level
            } else if has_job {
                self.sp_per_level
            } else {
                0
            }
        };
        let advanced = |level: u32| if has_job && (level == self.first_job_level || level == self.second_job_level) { self.advancement_sp } else { 0 };
        (from + 1..=to).map(|level| per_level(level) + advanced(level)).sum()
    }

    /// The same rules within what a hand-edited file or a mistyped field can sensibly mean.
    pub fn clamped(self) -> Self {
        let level = |v: u32| v.clamp(2, 200);
        Self {
            ap_per_level: self.ap_per_level.min(50),
            sp_per_level: self.sp_per_level.min(50),
            beginner_sp_per_level: self.beginner_sp_per_level.min(50),
            advancement_sp: self.advancement_sp.min(50),
            first_job_level: level(self.first_job_level),
            second_job_level: level(self.second_job_level),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jobs_belong_to_the_branch_of_their_hundreds() {
        assert_eq!(Branch::of_job(0), None);
        assert_eq!(Branch::of_job(120), Some(Branch::Warrior));
        assert_eq!(Branch::of_job(200), Some(Branch::Magician));
        assert_eq!(Branch::of_job(320), Some(Branch::Bowman));
        assert_eq!(Branch::of_job(420), Some(Branch::Thief));
        assert_eq!(Branch::of_job(9200), None);
    }

    #[test]
    fn a_saved_file_missing_new_rules_keeps_the_defaults() {
        // `gearCounts` and `secondSpBuysFirst` were switches once; old files still carry them.
        let rules: Rules =
            serde_json::from_str(r#"{"apPerLevel": 4, "manualApLevel": 11, "gearCounts": false, "secondSpBuysFirst": true}"#)
                .unwrap();
        assert_eq!(rules.ap_per_level, 4);
        assert_eq!(rules.sp_per_level, 3);
        assert_eq!(rules.first_job_level, 10);
    }

    #[test]
    fn mistyped_rules_stay_within_reason() {
        let rules = Rules { ap_per_level: 5000, second_job_level: 0, ..Rules::default() }.clamped();
        assert_eq!((rules.ap_per_level, rules.second_job_level), (50, 2));
    }
}
