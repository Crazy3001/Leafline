//! Matching what the game's Quest window lists to the quests in the data. The window shows
//! titles, some cut short with dots, under a region bar; the data files quests under regions of
//! the same names. Some quests the game lists aren't in the data at all, so no match is an answer
//! too: the quest is kept by its title alone.

use serde::Serialize;
use strsim::normalized_levenshtein;

use super::atlas::{Atlas, fold};

/// A whole title read this close to one in the data is that quest.
const SAME_TITLE: f64 = 0.86;
/// A title cut short matches the start of a title in the data this closely.
const SAME_START: f64 = 0.88;
/// A region bar read this close to a region in the data is that region.
const SAME_REGION: f64 = 0.8;
/// A cut title this short can't say which quest it is.
const SHORTEST_CUT: usize = 8;

/// How many quests the data files under a region.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionTotal {
    pub region: String,
    pub quests: usize,
}

impl Atlas {
    /// The region in the data a bar read as `read` names, if it is near enough to one.
    pub fn quest_region_like(&self, read: &str) -> Option<String> {
        let read = fold(read);
        let mut regions: Vec<&String> = self.quests.values().filter_map(|q| q.region.as_ref()).collect();
        regions.sort();
        regions.dedup();
        regions
            .into_iter()
            .map(|r| (normalized_levenshtein(&fold(r), &read), r))
            .filter(|(score, _)| *score >= SAME_REGION)
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, r)| r.clone())
    }

    /// The quest a row of the Quest window is: `readings` are what OCR made of its title, the best
    /// first, `cut` that the game cut it short, and `region` the bar it stands under, when known.
    /// `None` when no quest in the data fits, or more than one fits a cut title equally.
    pub fn log_quest(&self, readings: &[&str], cut: bool, region: Option<&str>) -> Option<u32> {
        let region = region.and_then(|r| self.quest_region_like(r));
        let in_region = |id: &u32| region.is_none() || self.quests.get(id).and_then(|q| q.region.as_ref()) == region.as_ref();
        readings.iter().find_map(|reading| {
            let read = fold(reading);
            if read.is_empty() {
                return None;
            }
            let mut scored: Vec<(f64, u32)> = self
                .quests
                .iter()
                .filter(|(id, _)| in_region(id))
                .map(|(id, quest)| {
                    let name = fold(&quest.name);
                    let score = if cut {
                        let start: String = name.chars().take(read.chars().count()).collect();
                        normalized_levenshtein(&start, &read)
                    } else {
                        normalized_levenshtein(&name, &read)
                    };
                    (score, *id)
                })
                .filter(|(score, _)| *score >= if cut { SAME_START } else { SAME_TITLE })
                .collect();
            if cut && read.chars().count() < SHORTEST_CUT {
                return None;
            }
            scored.sort_by(|a, b| b.0.total_cmp(&a.0));
            match scored[..] {
                [] => None,
                [(_, id)] => Some(id),
                [(best, id), (next, _), ..] if best > next + 1e-9 => Some(id),
                _ => None,
            }
        })
    }

    /// A quest's name as the data has it.
    pub fn quest_name(&self, id: u32) -> Option<String> {
        self.quests.get(&id).map(|q| q.name.clone())
    }

    /// How many quests the data files under each region, for "N of M done".
    pub fn quest_region_totals(&self) -> Vec<RegionTotal> {
        let mut totals: Vec<RegionTotal> = Vec::new();
        for quest in self.quests.values() {
            let Some(region) = &quest.region else { continue };
            match totals.iter_mut().find(|t| &t.region == region) {
                Some(total) => total.quests += 1,
                None => totals.push(RegionTotal { region: region.clone(), quests: 1 }),
            }
        }
        totals.sort_by(|a, b| a.region.cmp(&b.region));
        totals
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapdata::osms::{Dataset, QuestRecord};

    fn quest(id: u32, name: &str, region: &str) -> QuestRecord {
        QuestRecord { id, name: name.into(), region: Some(region.into()), ..QuestRecord::default() }
    }

    fn atlas() -> Atlas {
        Atlas::new(Dataset {
            quests: vec![
                quest(1, "Pebble Patrol", "Fern Isle"),
                quest(2, "Asking After Brin, the Lamplighter", "Townsfolk"),
                quest(3, "Asking After Brin, the Baker", "Townsfolk"),
                quest(4, "Asking After Mossa, the Lamplighter", "Townsfolk"),
                quest(5, "[Festival] The First Lantern", "Festival"),
                quest(6, "[Festival] The Final Lantern", "Festival"),
                quest(7, "Owl Feathers", "Duskwood"),
                quest(8, "Owl Feathers", "Fern Isle"),
            ],
            ..Dataset::default()
        })
    }

    #[test]
    fn a_whole_title_matches_even_a_letter_off() {
        let atlas = atlas();
        assert_eq!(atlas.log_quest(&["Pebble Patrol"], false, None), Some(1));
        assert_eq!(atlas.log_quest(&["Pebble Patroi"], false, None), Some(1));
        assert_eq!(atlas.log_quest(&["[Festivall The Final Lantern"], false, Some("Festival")), Some(6));
        assert_eq!(atlas.log_quest(&["A Quest Not In The Data"], false, None), None);
    }

    #[test]
    fn a_cut_title_matches_by_its_start_when_only_one_quest_starts_so() {
        let atlas = atlas();
        assert_eq!(atlas.log_quest(&["Asking After Brin, the Lamp"], true, None), Some(2));
        assert_eq!(atlas.log_quest(&["Asking After Brin, the"], true, None), None, "two quests start so");
        assert_eq!(atlas.log_quest(&["Asking"], true, None), None, "too short to tell");
    }

    #[test]
    fn the_region_bar_tells_two_quests_of_one_name_apart_and_a_later_reading_can_match() {
        let atlas = atlas();
        assert_eq!(atlas.log_quest(&["Owl Feathers"], false, None), None, "two quests of that name");
        assert_eq!(atlas.log_quest(&["Owl Feathers"], false, Some("Duskwood")), Some(7));
        assert_eq!(atlas.log_quest(&["Owl Feathers"], false, Some("Fern lsle")), Some(8));
        assert_eq!(atlas.log_quest(&["0wl Fethrs Xq", "Owl Feathers"], false, Some("Duskwood")), Some(7));
    }

    #[test]
    fn regions_are_counted_for_the_done_view() {
        let totals = atlas().quest_region_totals();
        assert_eq!(totals[0], RegionTotal { region: "Duskwood".into(), quests: 1 });
        assert_eq!(totals.iter().find(|t| t.region == "Townsfolk").unwrap().quests, 3);
    }
}
