//! How many points a character earns and when. The defaults come from mapleclassic.wiki, whose
//! pages were last checked before launch and disagree in places, so the player can change each.

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
    pub sp_per_level: u32,
    /// AP before this level goes in by itself; from it the player chooses.
    pub manual_ap_level: u32,
    /// The first level that gives SP, by branch. The level before it is the 1st job advancement.
    pub first_sp: FirstSp,
    pub second_job_level: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FirstSp {
    pub warrior: u32,
    pub magician: u32,
    pub bowman: u32,
    pub thief: u32,
}

impl Default for FirstSp {
    fn default() -> Self {
        Self { warrior: 11, magician: 9, bowman: 11, thief: 11 }
    }
}

impl Default for Rules {
    fn default() -> Self {
        Self { ap_per_level: 5, sp_per_level: 3, manual_ap_level: 11, first_sp: FirstSp::default(), second_job_level: 30 }
    }
}

impl Rules {
    pub fn first_sp_level(&self, branch: Branch) -> u32 {
        match branch {
            Branch::Warrior => self.first_sp.warrior,
            Branch::Magician => self.first_sp.magician,
            Branch::Bowman => self.first_sp.bowman,
            Branch::Thief => self.first_sp.thief,
        }
    }

    /// The same rules within what a hand-edited file or a mistyped field can sensibly mean.
    pub fn clamped(self) -> Self {
        let level = |v: u32| v.clamp(2, 200);
        Self {
            ap_per_level: self.ap_per_level.min(50),
            sp_per_level: self.sp_per_level.min(50),
            manual_ap_level: self.manual_ap_level.clamp(1, 200),
            first_sp: FirstSp {
                warrior: level(self.first_sp.warrior),
                magician: level(self.first_sp.magician),
                bowman: level(self.first_sp.bowman),
                thief: level(self.first_sp.thief),
            },
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
        let rules: Rules = serde_json::from_str(r#"{"apPerLevel": 4, "firstSp": {"magician": 10}}"#).unwrap();
        assert_eq!(rules.ap_per_level, 4);
        assert_eq!(rules.sp_per_level, 3);
        assert_eq!(rules.first_sp_level(Branch::Magician), 10);
        assert_eq!(rules.first_sp_level(Branch::Thief), 11);
    }

    #[test]
    fn mistyped_rules_stay_within_reason() {
        let rules = Rules { ap_per_level: 5000, second_job_level: 0, ..Rules::default() }.clamped();
        assert_eq!((rules.ap_per_level, rules.second_job_level), (50, 2));
    }
}
