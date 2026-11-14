//! Looking quests up: a search of every quest in the data, and all it says about one of them.
//! Nothing here says which quests the character has done: the game keeps that to itself.

use serde::{Deserialize, Serialize};

use super::atlas::{Atlas, QuestRef};
use super::drops::{ItemSources, item_key};
use super::guide::{ObjectiveGuide, QuestInfo, QuestQuery};
use super::osms::QuestRecord;
use super::travel::Travel;

/// Quests in one page of search results.
const QUEST_PAGE: usize = 40;
/// A quest's description tells its start, its middle and its end. The end is left out.
const STORY_LINES: usize = 2;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct QuestSearch {
    pub text: String,
    pub region: Option<String>,
    /// `daily`, `weekly` or `repeatable`.
    pub repeat: Option<String>,
    /// Only quests a character of this level can start.
    pub at_level: Option<u32>,
    /// Only quests that need no other quest finished first.
    pub no_prior: bool,
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestRow {
    pub id: u32,
    pub name: String,
    pub region: Option<String>,
    pub level_min: Option<u32>,
    pub giver: Option<String>,
    pub repeat: Option<String>,
    pub exp: u32,
    pub mesos: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestPage {
    pub quests: Vec<QuestRow>,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetObjective {
    pub count: u32,
    pub guide: ObjectiveGuide,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetItem {
    pub id: u32,
    pub name: String,
    pub count: u32,
    /// Always given. Otherwise it is one of a choice, or down to chance.
    pub sure: bool,
}

/// All the data says about one quest.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestSheet {
    pub info: QuestInfo,
    pub row: QuestRow,
    pub chain: Option<String>,
    pub story: Vec<String>,
    /// Quests to finish first.
    pub after: Vec<QuestRef>,
    pub next: Option<QuestRef>,
    pub objectives: Vec<SheetObjective>,
    pub rewards: Vec<SheetItem>,
    /// What the quests after this one in its chain pay, when it leads on.
    pub chain_rest: Option<ChainRest>,
}

/// The quests a chain still has after one of them, each leading to the next, and what they pay
/// together. Only EXP and mesos add up; item rewards stay on each quest's own page.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainRest {
    pub quests: Vec<QuestRef>,
    pub exp: u64,
    pub mesos: u64,
    /// The highest level any of them asks for.
    pub level_max: Option<u32>,
}

/// A chain longer than this is cut short: no real one is, so a loop in the data stops here.
const CHAIN_LIMIT: usize = 40;

/// Whether every word starts a word of `text`, which is already folded by [`item_key`].
pub(super) fn has_words(text: &str, words: &[String]) -> bool {
    words.iter().all(|word| text.split_whitespace().any(|w| w.starts_with(word.as_str())))
}

impl Atlas {
    pub(super) fn quest_row(&self, quest: &QuestRecord) -> QuestRow {
        QuestRow {
            id: quest.id,
            name: quest.name.clone(),
            region: quest.region.clone(),
            level_min: quest.level_min,
            giver: quest.npc_id.and_then(|npc| self.npc_names.get(&npc).cloned()),
            repeat: quest.repeat.clone(),
            exp: quest.reward_exp,
            mesos: quest.reward_mesos,
        }
    }

    /// How well `words` match a quest, its name first, or `None` if they don't.
    fn quest_rank(&self, quest: &QuestRecord, words: &[String]) -> Option<u8> {
        if words.is_empty() {
            return Some(0);
        }
        if has_words(&item_key(&quest.name), words) {
            return Some(0);
        }
        let giver = quest.npc_id.and_then(|npc| self.npc_names.get(&npc)).map(String::as_str);
        let monsters = quest.requirements.iter().filter_map(|need| self.monsters.get(&need.mob_id)).map(|m| m.name.as_str());
        let about: Vec<&str> =
            giver.into_iter().chain(quest.region.as_deref()).chain(quest.chain.as_deref()).chain(monsters).collect();
        if has_words(&item_key(&about.join(" ")), words) {
            return Some(1);
        }
        quest.description.as_deref().is_some_and(|d| has_words(&item_key(d), words)).then_some(2)
    }

    /// One page of the quests `search` finds: the best match first, then the lowest level, or
    /// with a level to play at, the nearest to it.
    pub fn search_quests(&self, search: &QuestSearch) -> QuestPage {
        let words: Vec<String> = item_key(&search.text).split_whitespace().map(str::to_string).collect();
        let fits = |q: &&QuestRecord| {
            search.region.as_ref().is_none_or(|r| q.region.as_ref() == Some(r))
                && search.repeat.as_ref().is_none_or(|r| q.repeat.as_ref() == Some(r))
                && search.at_level.is_none_or(|level| q.level_min.unwrap_or(0) <= level)
                && (!search.no_prior || q.follows.is_empty())
        };
        let mut found: Vec<(u8, &QuestRecord)> =
            self.quests.values().filter(fits).filter_map(|q| Some((self.quest_rank(q, &words)?, q))).collect();
        found.sort_by_key(|(rank, q)| {
            let level = q.level_min.unwrap_or(0) as i64;
            (*rank, if search.at_level.is_some() { -level } else { level }, q.name.clone())
        });
        QuestPage {
            total: found.len(),
            quests: found.into_iter().skip(search.offset).take(QUEST_PAGE).map(|(_, q)| self.quest_row(q)).collect(),
        }
    }

    /// Every region quests are filed under.
    pub fn quest_regions(&self) -> Vec<String> {
        let mut regions: Vec<String> = self.quests.values().filter_map(|q| q.region.clone()).collect();
        regions.sort();
        regions.dedup();
        regions
    }

    /// The names of the items a quest asks you to bring.
    pub fn quest_items(&self, id: u32, item_name: impl Fn(u32) -> Option<String>) -> Vec<String> {
        self.quests.get(&id).into_iter().flat_map(|q| &q.items).filter_map(|need| item_name(need.item_id)).collect()
    }

    /// All the data says about a quest. `sources` holds the drop table's answers for its items.
    pub fn quest_sheet(
        &self,
        id: u32,
        item_name: impl Fn(u32) -> Option<String>,
        sources: &[ItemSources],
        from: Option<u32>,
        travel: &Travel,
    ) -> Option<QuestSheet> {
        let quest = self.quests.get(&id)?;
        let mobs = quest.requirements.iter().filter_map(|need| Some((self.monsters.get(&need.mob_id)?.name.clone(), need.count)));
        let items = quest.items.iter().filter_map(|need| Some((item_name(need.item_id)?, need.count)));
        let (objectives, counts): (Vec<String>, Vec<u32>) = mobs.chain(items).unzip();
        let query = QuestQuery { title: quest.name.clone(), objectives };
        let guide = self.quest_guide(&[query], from, sources, &[], travel).pop()?;
        let objectives = guide.objectives.into_iter().zip(counts).map(|(guide, count)| SheetObjective { count, guide }).collect();
        let lines = quest.description.iter().flat_map(|d| d.lines());
        Some(QuestSheet {
            info: self.quest_info(id)?,
            row: self.quest_row(quest),
            chain: quest.chain.clone(),
            story: lines.map(str::trim).filter(|l| !l.is_empty()).take(STORY_LINES).map(str::to_string).collect(),
            after: quest.follows.iter().filter_map(|id| self.quest_ref(*id)).collect(),
            next: quest.next_quest_id.and_then(|id| self.quest_ref(id)),
            objectives,
            rewards: quest
                .reward_items
                .iter()
                .filter_map(|r| Some(SheetItem { id: r.item_id, name: item_name(r.item_id)?, count: r.count, sure: r.sure }))
                .collect(),
            chain_rest: self.chain_rest(id),
        })
    }

    /// The quests after `id` in its chain, following each one's next quest, and what they pay.
    pub fn chain_rest(&self, id: u32) -> Option<ChainRest> {
        let mut seen = vec![id];
        let mut rest = ChainRest { quests: Vec::new(), exp: 0, mesos: 0, level_max: None };
        let mut at = self.quests.get(&id)?.next_quest_id;
        while let Some(next) = at.filter(|n| !seen.contains(n) && seen.len() <= CHAIN_LIMIT) {
            let Some(quest) = self.quests.get(&next) else { break };
            seen.push(next);
            rest.exp += u64::from(quest.reward_exp);
            rest.mesos += u64::from(quest.reward_mesos);
            rest.level_max = rest.level_max.max(quest.level_min);
            rest.quests.extend(self.quest_ref(next));
            at = quest.next_quest_id;
        }
        (!rest.quests.is_empty()).then_some(rest)
    }
}

#[cfg(test)]
mod tests {
    use super::super::atlas::tests::world;
    use super::*;

    fn search(text: &str) -> Vec<String> {
        let query = QuestSearch { text: text.into(), ..QuestSearch::default() };
        world().search_quests(&query).quests.into_iter().map(|q| q.name).collect()
    }

    #[test]
    fn finds_quests_by_name_person_and_monster() {
        assert_eq!(search("shell"), ["Shell Sorting"]);
        assert_eq!(search("ilse"), ["Shell Sorting"], "the one who gives it");
        assert_eq!(search("moss"), ["Slime Samples"], "the monster it wants");
        assert_eq!(search("nothing like this"), Vec::<String>::new());
    }

    #[test]
    fn the_lowest_level_comes_first_or_the_nearest_with_a_level_to_play_at() {
        assert_eq!(search(""), ["Slime Samples", "Shell Sorting"]);
        let at = |level| {
            let query = QuestSearch { at_level: Some(level), ..QuestSearch::default() };
            world().search_quests(&query).quests.into_iter().map(|q| q.name).collect::<Vec<_>>()
        };
        assert_eq!(at(5), ["Shell Sorting", "Slime Samples"]);
        assert_eq!(at(2), ["Slime Samples"], "not yet the Lv 3 one");
    }

    #[test]
    fn the_rest_of_a_chain_adds_up_what_it_pays() {
        use crate::mapdata::osms::{Dataset, QuestRecord};
        let quest = |id: u32, next: Option<u32>, exp: u32, level: u32| QuestRecord {
            id,
            name: format!("Part {id}"),
            level_min: Some(level),
            next_quest_id: next,
            reward_exp: exp,
            reward_mesos: 10,
            follows: if id > 1 { vec![id - 1] } else { vec![] },
            ..QuestRecord::default()
        };
        let quests = vec![quest(1, Some(2), 100, 5), quest(2, Some(3), 200, 8), quest(3, Some(2), 300, 12), quest(4, None, 9, 1)];
        let atlas = Atlas::new(Dataset { quests, ..Dataset::default() });
        let rest = atlas.chain_rest(1).unwrap();
        assert_eq!(rest.quests.iter().map(|q| q.id).collect::<Vec<_>>(), [2, 3], "a loop back to 2 stops there");
        assert_eq!((rest.exp, rest.mesos, rest.level_max), (500, 20, Some(12)));
        assert!(atlas.chain_rest(4).is_none(), "a quest that leads nowhere has no rest");

        let starts = QuestSearch { no_prior: true, ..QuestSearch::default() };
        let names: Vec<String> = atlas.search_quests(&starts).quests.into_iter().map(|q| q.name).collect();
        assert_eq!(names, ["Part 1"], "the others need the one before them");
    }

    #[test]
    fn a_sheet_has_the_objectives_counts_and_rewards() {
        let sheet = world().quest_sheet(70, |id| (id == 90).then(|| "Fern Hat".to_string()), &[], Some(5), &Travel::none()).unwrap();
        assert_eq!(sheet.row.giver.as_deref(), Some("Warden Ilse"));
        assert_eq!(sheet.objectives.iter().map(|o| o.count).collect::<Vec<_>>(), [8, 2]);
        assert_eq!(sheet.objectives[0].guide.target, "Pebble Snail");
        assert_eq!(sheet.rewards, [SheetItem { id: 90, name: "Fern Hat".into(), count: 2, sure: true }]);
    }
}
