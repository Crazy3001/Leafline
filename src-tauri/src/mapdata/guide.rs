//! Where to go for the quests the Quest Helper shows: which monster each objective needs, the
//! maps with the most of it, and how many portals away each map is.
//!
//! An objective names either a monster to defeat or an item to collect. It is taken for a
//! monster when it reads like one in the data; otherwise it is an item, and the monsters that
//! drop it come from players' logs on mapleclassic.wiki, looked up separately.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::atlas::{Atlas, MapRef, QuestRef, Shortcut, fold, similarity};
use super::drops::{ItemSources, item_key};
use super::osms::{MonsterRecord, QuestRecord};
use super::training::{Measured, MeasuredRate};
use super::travel::Travel;
use crate::readers::quest::{Objective, Quest, QuestReading};

/// How closely an objective must match a monster's name to be taken for that monster.
const MIN_MONSTER_SCORE: f64 = 0.85;
/// Looser when the quest the title matched asks for that monster.
const MIN_QUEST_MONSTER_SCORE: f64 = 0.7;
/// How closely a dropper's name on the wiki must match a monster in the map data.
pub(super) const MIN_DROPPER_SCORE: f64 = 0.9;
const MIN_QUEST_SCORE: f64 = 0.8;
/// Below this a title is another quest, not a bad reading of one already known.
const MIN_HELD_SCORE: f64 = 0.6;
/// Words of a title shorter than this say too little to look a quest up by.
const MIN_TITLE_WORD: usize = 4;
const MAPS_PER_MONSTER: usize = 4;
const DROPPERS_PER_ITEM: usize = 4;
/// Some scrolls are given by dozens of quests. The first few, by level, say enough.
const QUESTS_PER_ITEM: usize = 6;

/// How many readings in a row a known quest can go unread before it counts as gone, or the
/// panel as closed. A few, so a bad moment shows nothing, but closing it soon shows.
const HELD_READINGS: u32 = 4;

/// A quest matched in this live run and what it last counted.
struct Known {
    name: String,
    objectives: Vec<Objective>,
    unread: u32,
}

impl Known {
    fn targets(&self) -> impl Iterator<Item = &str> {
        self.objectives.iter().map(|o| o.target.as_str())
    }
}

/// The quests matched so far in a live run. OCR can read one title differently from one moment
/// to the next as the scenery moves behind the panel, so once a quest is known it stays: a
/// reading too rough to match keeps the quest it most resembles, one with no counter keeps the
/// last counter, and one that loses the quest altogether shows it a little longer.
#[derive(Default)]
pub struct QuestTitles {
    known: Vec<Known>,
    last: Option<QuestReading>,
    missed: u32,
}

impl QuestTitles {
    /// Steadies one live reading of the panel. `None` when the panel wasn't found this time.
    pub fn hold(&mut self, atlas: &Atlas, reading: &mut Option<QuestReading>) {
        let Some(read) = reading else {
            self.missed += 1;
            if self.missed <= HELD_READINGS {
                reading.clone_from(&self.last);
            } else {
                *self = Self::default();
            }
            return;
        };
        self.missed = 0;
        if read.page.is_some_and(|(shown, _)| shown == 0) {
            *self = Self::default();
            return;
        }
        self.settle(atlas, &mut read.quests);
        self.keep_known(read);
        self.last = Some(read.clone());
    }

    /// Takes out rows that are no quest, and puts back known quests that went unread.
    fn keep_known(&mut self, read: &mut QuestReading) {
        let is_known = |known: &[Known], title: &str| known.iter().any(|k| k.name == title);
        if !self.known.is_empty() {
            read.quests.retain(|q| !q.objectives.is_empty() || is_known(&self.known, &q.title));
        }
        let mut seen: Vec<String> = Vec::new();
        read.quests.retain(|q| {
            let new = !seen.contains(&q.title);
            seen.push(q.title.clone());
            new
        });
        for known in &mut self.known {
            known.unread = if seen.contains(&known.name) { 0 } else { known.unread + 1 };
        }
        self.known.retain(|k| k.unread <= HELD_READINGS);
        let room = read.page.map_or(usize::MAX, |(shown, _)| shown as usize);
        for known in &self.known {
            if known.unread > 0 && read.quests.len() < room {
                read.quests.push(Quest { title: known.name.clone(), objectives: known.objectives.clone() });
            }
        }
        let place = |q: &Quest| self.known.iter().position(|k| k.name == q.title).unwrap_or(usize::MAX);
        read.quests.sort_by_key(place);
    }

    /// Gives each quest the data's name and counts, from its title and what it counts, or the
    /// known quest it most resembles.
    pub fn settle(&mut self, atlas: &Atlas, quests: &mut [Quest]) {
        for quest in quests {
            let matched = atlas.quest_for(&quest.title, &quest.objectives);
            if let Some(id) = matched {
                quest.title = atlas.quests[&id].name.clone();
                atlas.fit_counts(id, &mut quest.objectives);
            } else if let Some(name) = self.closest(&quest.title, &quest.objectives) {
                quest.title = name.to_string();
            } else {
                continue;
            }
            match self.known.iter_mut().find(|k| k.name == quest.title) {
                Some(known) if quest.objectives.is_empty() => quest.objectives.clone_from(&known.objectives),
                Some(known) => known.objectives.clone_from(&quest.objectives),
                None => self.known.push(Known { name: quest.title.clone(), objectives: quest.objectives.clone(), unread: 0 }),
            }
        }
    }

    /// The known quest counting the same things, or else with the most similar name.
    fn closest(&self, title: &str, objectives: &[Objective]) -> Option<&str> {
        let query = fold(title);
        let targets: Vec<&str> = objectives.iter().map(|o| o.target.as_str()).collect();
        self.known
            .iter()
            .map(|known| {
                let same = !targets.is_empty() && known.targets().eq(targets.iter().copied());
                (if same { 1.0 } else { similarity(&query, &fold(&known.name)) }, known.name.as_str())
            })
            .filter(|(score, _)| *score >= MIN_HELD_SCORE)
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, name)| name)
    }
}

/// One quest as the Quest Helper showed it.
#[derive(Debug, Clone, Deserialize)]
pub struct QuestQuery {
    pub title: String,
    /// What each objective names, as read.
    pub objectives: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestGuide {
    pub title: String,
    /// The quest in the data whose name the title matched, if any.
    pub quest: Option<QuestInfo>,
    pub objectives: Vec<ObjectiveGuide>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestInfo {
    pub quest: QuestRef,
    pub giver: Option<String>,
    /// Where the quest giver stands.
    pub giver_maps: Vec<MapRef>,
    /// Who takes the quest in, when something shows it. Not always the giver.
    pub turn_in: Option<String>,
    /// Where they stand.
    pub turn_in_maps: Vec<MapRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetKind {
    Monster,
    Item,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveGuide {
    /// The objective as read.
    pub target: String,
    pub kind: TargetKind,
    /// For an item: the wiki's name for it, once looked up and found.
    pub item: Option<String>,
    /// For an item: whether the wiki has been asked who drops it.
    pub looked_up: bool,
    /// Monsters to hunt: the one named, or the item's droppers, highest drop rate first.
    pub hunts: Vec<Hunt>,
}

/// Where to get an item.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemOrigin {
    pub drops: ObjectiveGuide,
    /// Quests that can give it, the lowest level first.
    pub quests: Vec<QuestInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hunt {
    pub monster: MonsterRef,
    /// For an item: how often players saw this monster drop it.
    pub drop: Option<DropRate>,
    /// Maps with the most of this monster first.
    pub maps: Vec<HuntingGround>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonsterRef {
    pub name: String,
    /// False for a dropper the wiki names that the map data has no monster for.
    pub known: bool,
    pub level: u32,
    pub hp: u32,
    pub exp: u32,
    pub boss: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DropRate {
    /// Percent, with its confidence interval.
    pub rate: Option<f64>,
    pub rate_low: Option<f64>,
    pub rate_high: Option<f64>,
    pub sample_kills: Option<u32>,
    pub game_version: Option<String>,
    /// Drops only while a quest wants it.
    pub quest: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HuntingGround {
    pub map: MapRef,
    /// How many of the monster the map spawns.
    pub count: u32,
    /// How many monsters of any kind the map spawns.
    pub map_total: u32,
    /// Portals from the current map, when it is known and a route exists.
    pub hops: Option<u32>,
    /// Fewer steps with the travel the player turned on, when that is shorter.
    pub travel: Option<Shortcut>,
    /// The player's EXP rate measured here, as kills of this monster.
    pub pace: Option<Pace>,
}

/// A rough kill rate for one monster from the EXP per hour the player measured on its map. It
/// assumes the player defeats the map's monsters in proportion to how many of each spawn, and
/// earns the map data's base EXP for them, so EXP events and parties throw it off.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pace {
    pub exp_per_hour: f64,
    pub active_seconds: f64,
    pub kills_per_hour: f64,
}

/// Sorts the versions of one monster: the one that spawns most, which players meet, comes first.
pub(super) fn main_first(monster: &MonsterRecord) -> (std::cmp::Reverse<u32>, u32) {
    (std::cmp::Reverse(monster.spawns.iter().map(|s| s.count).sum()), monster.id)
}

/// What the guide knows about the player, beyond the map they are on.
#[derive(Debug, Default)]
pub(super) struct Context {
    pub(super) hops: HashMap<u32, u32>,
    pub(super) shortcuts: HashMap<u32, Shortcut>,
    measured: HashMap<u32, Measured>,
}

impl Atlas {
    /// What the guide can tell from the map the player is on, the rates they measured and the
    /// rides and scrolls they turned on.
    pub(super) fn context(&self, from: Option<u32>, measured: &[MeasuredRate], travel: &Travel) -> Context {
        let hops = from.map(|id| self.hops_from(id)).unwrap_or_default();
        let shortcuts = from.map(|id| self.shortcuts_from(id, travel, &hops)).unwrap_or_default();
        Context { hops, shortcuts, measured: self.measured_by_map(measured) }
    }

    /// A guide for each quest. `from` is the map the player is on, for portal counts; `items`
    /// holds the drop table's answers for item objectives; `measured` the EXP rates the
    /// player recorded, matched to maps by name; `travel` the rides and scrolls turned on.
    pub fn quest_guide(
        &self,
        quests: &[QuestQuery],
        from: Option<u32>,
        items: &[ItemSources],
        measured: &[MeasuredRate],
        travel: &Travel,
    ) -> Vec<QuestGuide> {
        let context = self.context(from, measured, travel);
        quests
            .iter()
            .map(|query| {
                let quest = self.find_quest(&query.title);
                let wanted: Vec<u32> = quest
                    .and_then(|id| self.quests.get(&id))
                    .map(|q| q.requirements.iter().map(|r| r.mob_id).collect())
                    .unwrap_or_default();
                let objectives =
                    query.objectives.iter().map(|target| self.objective(target, &wanted, items, &context)).collect();
                QuestGuide { title: query.title.clone(), quest: quest.and_then(|id| self.quest_info(id)), objectives }
            })
            .collect()
    }

    fn objective(&self, target: &str, wanted: &[u32], items: &[ItemSources], context: &Context) -> ObjectiveGuide {
        let named = self
            .monsters_named(target, Some(wanted))
            .filter(|(score, _)| *score >= MIN_QUEST_MONSTER_SCORE)
            .or_else(|| self.monsters_named(target, None).filter(|(score, _)| *score >= MIN_MONSTER_SCORE));
        if let Some((_, ids)) = named {
            return ObjectiveGuide {
                target: target.to_string(),
                kind: TargetKind::Monster,
                item: None,
                looked_up: false,
                hunts: vec![self.hunt(&ids, None, context)],
            };
        }

        let key = item_key(target);
        self.item_objective(target, items.iter().find(|s| s.key == key), context)
    }

    /// An item and the monsters players saw drop it. `sources` is `None` until it is looked up.
    fn item_objective(&self, target: &str, sources: Option<&ItemSources>, context: &Context) -> ObjectiveGuide {
        let hunts = sources
            .into_iter()
            .flat_map(|s| &s.droppers)
            .take(DROPPERS_PER_ITEM)
            .map(|dropper| {
                let drop = DropRate {
                    rate: dropper.rate,
                    rate_low: dropper.rate_low,
                    rate_high: dropper.rate_high,
                    sample_kills: dropper.sample_kills,
                    game_version: dropper.game_version.clone(),
                    quest: dropper.quest,
                };
                match self.monsters_named(&dropper.monster, None).filter(|(score, _)| *score >= MIN_DROPPER_SCORE) {
                    Some((_, ids)) => self.hunt(&ids, Some(drop), context),
                    None => Hunt {
                        monster: MonsterRef {
                            name: dropper.monster.clone(),
                            known: false,
                            level: 0,
                            hp: 0,
                            exp: 0,
                            boss: false,
                        },
                        drop: Some(drop),
                        maps: Vec::new(),
                    },
                }
            })
            .collect();
        ObjectiveGuide {
            target: target.to_string(),
            kind: TargetKind::Item,
            item: sources.and_then(|s| s.item.clone()),
            looked_up: sources.is_some(),
            hunts,
        }
    }

    /// Where to get an item by its data id: who drops it, as a quest item's objective would show,
    /// and the quests that can give it. `from` is the map the player is on, for portal counts.
    pub fn item_origin(
        &self,
        item: u32,
        name: &str,
        sources: Option<&ItemSources>,
        from: Option<u32>,
        travel: &Travel,
    ) -> ItemOrigin {
        let context = self.context(from, &[], travel);
        let quests = self.quests_rewarding.get(&item).into_iter().flatten();
        ItemOrigin {
            drops: self.item_objective(name, sources, &context),
            quests: quests.filter_map(|id| self.quest_info(*id)).take(QUESTS_PER_ITEM).collect(),
        }
    }

    /// The monsters (by ID) whose name best matches `name`, and how well. Several IDs can share
    /// a name; all of them are returned so their spawns add up. `among` limits the search.
    pub(super) fn monsters_named(&self, name: &str, among: Option<&[u32]>) -> Option<(f64, Vec<u32>)> {
        let query = fold(name);
        if query.is_empty() {
            return None;
        }
        let candidates: Vec<&MonsterRecord> = match among {
            Some(ids) => ids.iter().filter_map(|id| self.monsters.get(id)).collect(),
            None => self.monsters.values().collect(),
        };
        let (score, best) = candidates
            .iter()
            .map(|m| {
                let key = fold(&m.name);
                (similarity(&query, &key), key)
            })
            .max_by(|a, b| a.0.total_cmp(&b.0).then_with(|| b.1.cmp(&a.1)))?;
        let mut ids: Vec<u32> = self.monsters.values().filter(|m| fold(&m.name) == best).map(|m| m.id).collect();
        ids.sort_unstable();
        Some((score, ids))
    }

    /// Where to hunt the monster with these IDs.
    fn hunt(&self, ids: &[u32], drop: Option<DropRate>, context: &Context) -> Hunt {
        let records: Vec<&MonsterRecord> = ids.iter().filter_map(|id| self.monsters.get(id)).collect();
        // Of IDs sharing a name, the one that spawns most is the one players meet.
        let main = records.iter().copied().min_by_key(|m| main_first(m)).expect("hunt needs a known monster");
        let mut maps = self.hunting_grounds(&records, context);
        maps.truncate(MAPS_PER_MONSTER);
        Hunt {
            monster: MonsterRef {
                name: main.name.clone(),
                known: true,
                level: main.level,
                hp: main.hp,
                exp: main.exp,
                boss: main.is_boss,
            },
            drop,
            maps,
        }
    }

    /// Every map these monsters spawn on, the best first.
    pub(super) fn hunting_grounds(&self, records: &[&MonsterRecord], context: &Context) -> Vec<HuntingGround> {
        let mut counts: HashMap<u32, u32> = HashMap::new();
        for spawn in records.iter().flat_map(|m| &m.spawns).filter(|s| s.count > 0) {
            *counts.entry(spawn.map_id).or_default() += spawn.count;
        }
        let mut maps: Vec<HuntingGround> = counts
            .into_iter()
            .filter(|(id, _)| self.maps.contains_key(id))
            .map(|(id, count)| HuntingGround {
                map: self.map_ref(id),
                count,
                map_total: self.spawns.get(&id).into_iter().flatten().map(|(_, count)| count).sum(),
                hops: context.hops.get(&id).copied(),
                travel: context.shortcuts.get(&id).cloned(),
                pace: self.pace(id, count, context),
            })
            .collect();
        // Maps the player can get to come first, however rich the others are. Without a known
        // position none has a route, and count alone decides.
        let stranded = |m: &HuntingGround| m.hops.is_none() && m.travel.is_none();
        maps.sort_by(|a, b| {
            stranded(a)
                .cmp(&stranded(b))
                .then(b.count.cmp(&a.count))
                .then(a.hops.unwrap_or(u32::MAX).cmp(&b.hops.unwrap_or(u32::MAX)))
                .then_with(|| a.map.name.cmp(&b.map.name))
        });
        maps
    }

    /// Kills per hour of a monster that spawns `count` times on map `id`, from the EXP rate the
    /// player measured there: their EXP per hour is so many clears of the map, and each clear
    /// has `count` of the monster.
    fn pace(&self, id: u32, count: u32, context: &Context) -> Option<Pace> {
        let measured = context.measured.get(&id)?;
        let per_clear = self.card(id)?.exp_per_clear;
        (per_clear > 0 && measured.exp_per_hour > 0.0).then(|| Pace {
            exp_per_hour: measured.exp_per_hour,
            active_seconds: measured.active_seconds,
            kills_per_hour: measured.exp_per_hour * f64::from(count) / per_clear as f64,
        })
    }

    /// The quest a reading of the panel means: by its title, or when that lost words, by a name
    /// holding what is left of it and by what it counts, if only one quest fits.
    fn quest_for(&self, title: &str, objectives: &[Objective]) -> Option<u32> {
        if let Some(id) = self.find_quest(title) {
            return Some(id);
        }
        let query = fold(title);
        let words: Vec<&str> = query.split_whitespace().filter(|w| w.len() >= MIN_TITLE_WORD).collect();
        let named = |q: &&QuestRecord| {
            let name = fold(&q.name);
            !words.is_empty() && words.iter().all(|w| name.contains(w))
        };
        let counts = |q: &&QuestRecord| {
            !objectives.is_empty() && objectives.iter().all(|o| self.count_of(q, &o.target) == Some(o.required))
        };
        let only = |found: Vec<u32>| (found.len() == 1).then(|| found[0]);
        let quests = || self.quests.values();
        only(quests().filter(named).filter(counts).map(|q| q.id).collect())
            .or_else(|| only(quests().filter(counts).map(|q| q.id).collect()))
            .or_else(|| only(quests().filter(named).map(|q| q.id).collect()))
    }

    /// The monster called `target` that a quest asks for, and how many.
    fn wanted(&self, quest: &QuestRecord, target: &str) -> Option<(&str, u32)> {
        let target = fold(target);
        quest.requirements.iter().find_map(|need| {
            let monster = self.monsters.get(&need.mob_id).filter(|m| fold(&m.name) == target)?;
            Some((monster.name.as_str(), need.count))
        })
    }

    fn count_of(&self, quest: &QuestRecord, target: &str) -> Option<u32> {
        self.wanted(quest, target).map(|(_, count)| count)
    }

    /// Corrects what a quest counts to what the data says it asks for, spelled the data's way.
    fn fit_counts(&self, id: u32, objectives: &mut [Objective]) {
        let Some(quest) = self.quests.get(&id) else { return };
        for objective in objectives {
            if let Some((name, count)) = self.wanted(quest, &objective.target) {
                objective.target = name.to_string();
                objective.required = count;
            }
        }
    }

    /// The quest whose name the Quest Helper's title matches. Long titles are cut short with
    /// `...` there, so those are compared with the start of each name.
    fn find_quest(&self, title: &str) -> Option<u32> {
        let trimmed = title.trim_end();
        let truncated = trimmed.ends_with("...") || trimmed.ends_with('…');
        let query = fold(trimmed);
        if query.len() < 4 {
            return None;
        }
        self.quests
            .values()
            .map(|q| {
                let name = fold(&q.name);
                let name = if truncated { name.chars().take(query.chars().count()).collect() } else { name };
                (similarity(&query, &name), q.id)
            })
            .filter(|(score, _)| *score >= MIN_QUEST_SCORE)
            .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
            .map(|(_, id)| id)
    }

    pub(super) fn quest_info(&self, id: u32) -> Option<QuestInfo> {
        let quest = self.quests.get(&id)?;
        let name = |npc: Option<u32>| npc.and_then(|n| self.npc_names.get(&n).cloned());
        let maps = |npc: Option<u32>| npc.map(|n| self.npc_maps(n)).unwrap_or_default();
        let turn_in = self.turn_in(quest);
        Some(QuestInfo {
            quest: self.quest_ref(id)?,
            giver: name(quest.npc_id),
            giver_maps: maps(quest.npc_id),
            turn_in: name(turn_in),
            turn_in_maps: maps(turn_in),
        })
    }

    /// Where an NPC stands.
    pub(super) fn npc_maps(&self, npc: u32) -> Vec<MapRef> {
        let mut maps: Vec<MapRef> = self
            .npcs_on
            .iter()
            .filter(|(_, npcs)| npcs.contains(&npc))
            .map(|(map, _)| self.map_ref(*map))
            .filter(|m| m.known)
            .collect();
        maps.sort_by_key(|m| m.id);
        maps
    }

    /// The NPC the wiki names as taking a quest in. Several can share a name, as the ones in a
    /// job's instance do, so the quest's own giver wins, and then one standing on a map.
    pub(super) fn turn_in_npc(&self, quest: u32, name: &str) -> Option<u32> {
        let giver = self.quests.get(&quest).and_then(|q| q.npc_id);
        giver.filter(|id| self.npc_names.get(id).is_some_and(|n| n == name)).or_else(|| self.npc_like(name))
    }

    /// Who a quest is handed in to. The client data has no field for it, so this is the wiki's
    /// answer where it has one. Otherwise it goes by what shows it: the next quest of a chain
    /// starts with them, or the title ends `to Heena`. Failing that it stays unknown: the giver
    /// is not a safe guess, being someone else on half the quests the wiki covers.
    pub(super) fn turn_in(&self, quest: &QuestRecord) -> Option<u32> {
        let next = quest.next_quest_id.and_then(|id| self.quests.get(&id)).and_then(|q| q.npc_id);
        self.turn_ins.get(&quest.id).copied().or(next).or_else(|| {
            let (_, name) = quest.name.rsplit_once(" to ")?;
            let name = name.trim_end_matches(['.', '!', '?']);
            let mut named: Vec<u32> =
                self.npc_names.iter().filter(|(_, n)| n.as_str() == name).map(|(id, _)| *id).collect();
            named.sort_unstable();
            named.into_iter().find(|&id| !self.npc_maps(id).is_empty())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapdata::atlas::tests::{island_world, world};
    use crate::mapdata::drops::Dropper;
    use crate::readers::quest::Objective;

    fn query(title: &str, objectives: &[&str]) -> QuestQuery {
        QuestQuery { title: title.into(), objectives: objectives.iter().map(|o| o.to_string()).collect() }
    }

    fn wing_sources() -> ItemSources {
        let dropper = |monster: &str, rate: f64| Dropper {
            monster: monster.into(),
            rate: Some(rate),
            rate_low: None,
            rate_high: None,
            sample_kills: Some(200),
            game_version: None,
            quest: true,
        };
        ItemSources {
            key: "rat whisker".into(),
            item: Some("Rat Whisker".into()),
            fetched_at: 0,
            droppers: vec![dropper("Burrow Rat", 12.0), dropper("Sewer King", 50.0)],
        }
    }

    #[test]
    fn monster_objectives_list_the_best_maps_and_portals_away() {
        let atlas = world();
        // From the inn: out to Mossbank, east to Meadow I, east again to Meadow II.
        let guide = atlas.quest_guide(&[query("Shell Sortng", &["Pebble Snai1"])], Some(5), &[], &[], &Travel::none());
        let quest = guide[0].quest.as_ref().unwrap();
        assert_eq!(quest.quest.name, "Shell Sorting");
        assert_eq!(quest.giver.as_deref(), Some("Warden Ilse"));
        assert_eq!(quest.giver_maps.iter().map(|m| m.id).collect::<Vec<_>>(), [1]);

        let objective = &guide[0].objectives[0];
        assert_eq!(objective.kind, TargetKind::Monster);
        let hunt = &objective.hunts[0];
        assert_eq!((hunt.monster.name.as_str(), hunt.monster.level), ("Pebble Snail", 2));
        let maps: Vec<(&str, u32, Option<u32>)> =
            hunt.maps.iter().map(|g| (g.map.name.as_str(), g.count, g.hops)).collect();
        assert_eq!(maps, [("Mossbank Meadow I", 6, Some(2)), ("Mossbank Meadow II", 2, Some(3))]);
    }

    #[test]
    fn a_garbled_title_keeps_the_quest_read_before_it() {
        let atlas = world();
        let mut titles = QuestTitles::default();
        let mut settle = |title: &str, target: &str| {
            let objectives = vec![Objective { target: target.into(), current: 0, required: 8 }];
            let mut quests = [Quest { title: title.into(), objectives }];
            titles.settle(&atlas, &mut quests);
            quests[0].title.clone()
        };
        const GARBLED: &str = "Sheli Sor 101ng";
        assert_eq!(settle(GARBLED, "Fern Moth"), GARBLED, "nothing to hold on to yet");
        assert_eq!(settle(GARBLED, "Pebble Snail"), "Shell Sorting", "only this quest counts these");
        assert_eq!(settle("Shell Sortng", "Pebble Snail"), "Shell Sorting");
        assert_eq!(settle(GARBLED, "Pebble Snail"), "Shell Sorting");
        assert_eq!(settle(GARBLED, "Pebb1e Snail"), "Shell Sorting", "resembles it, counter or not");
        assert_eq!(settle("", "Pebble Snail"), "Shell Sorting", "the title lost, the same counter");
        assert_eq!(settle("Another Errand", "Moss Slime"), "Another Errand");
        assert_eq!(settle("", "Moss Slime"), "");
    }

    fn reading(quests: Vec<Quest>, page: Option<(u32, u32)>) -> Option<QuestReading> {
        Some(QuestReading { quests, page, ..QuestReading::default() })
    }

    fn quest(title: &str, counted: &[(&str, u32, u32)]) -> Quest {
        let objectives = counted
            .iter()
            .map(|(target, current, required)| Objective { target: (*target).into(), current: *current, required: *required })
            .collect();
        Quest { title: title.into(), objectives }
    }

    fn held(titles: &mut QuestTitles, atlas: &Atlas, mut read: Option<QuestReading>) -> Option<QuestReading> {
        titles.hold(atlas, &mut read);
        read
    }

    #[test]
    fn a_title_that_lost_a_word_finds_its_quest() {
        let atlas = world();
        assert_eq!(atlas.quest_for("Sorting", &[]), Some(70), "only one name holds it");
        let wanted = [Objective { target: "Moss Slime".into(), current: 0, required: 5 }];
        assert_eq!(atlas.quest_for("Xq", &wanted), Some(71), "only one quest counts that");
        assert_eq!(atlas.quest_for("Xq", &[]), None);
    }

    #[test]
    fn a_locked_quest_keeps_its_counter_through_a_bad_reading() {
        let (atlas, mut titles) = (world(), QuestTitles::default());
        held(&mut titles, &atlas, reading(vec![quest("Slime Samples", &[("Moss Slime", 2, 5)])], None));
        let bad = held(&mut titles, &atlas, reading(vec![quest("Slime Samples", &[])], None)).unwrap();
        assert_eq!(bad.quests, [quest("Slime Samples", &[("Moss Slime", 2, 5)])]);
    }

    #[test]
    fn the_data_corrects_what_a_quest_counts() {
        let (atlas, mut titles) = (world(), QuestTitles::default());
        let read = held(&mut titles, &atlas, reading(vec![quest("Slime Samples", &[("Moss Slime", 2, 6)])], None)).unwrap();
        assert_eq!(read.quests[0].objectives[0].required, 5);
    }

    #[test]
    fn a_row_that_is_no_quest_goes_and_a_lost_quest_comes_back_in_order() {
        let (atlas, mut titles) = (world(), QuestTitles::default());
        let both = vec![quest("Shell Sorting", &[("Pebble Snail", 1, 8)]), quest("Slime Samples", &[("Moss Slime", 2, 5)])];
        held(&mut titles, &atlas, reading(both.clone(), None));
        let rough = vec![quest("O Slime", &[]), quest("Slime Samples", &[("Moss Slime", 2, 5)])];
        assert_eq!(held(&mut titles, &atlas, reading(rough, None)).unwrap().quests, both);
    }

    #[test]
    fn a_quest_unread_for_long_is_gone() {
        let (atlas, mut titles) = (world(), QuestTitles::default());
        held(&mut titles, &atlas, reading(vec![quest("Slime Samples", &[("Moss Slime", 2, 5)])], None));
        for _ in 0..HELD_READINGS {
            assert_eq!(held(&mut titles, &atlas, reading(vec![], None)).unwrap().quests.len(), 1);
        }
        assert!(held(&mut titles, &atlas, reading(vec![], None)).unwrap().quests.is_empty());
    }

    #[test]
    fn the_panel_going_missing_shows_for_a_moment_then_clears() {
        let (atlas, mut titles) = (world(), QuestTitles::default());
        held(&mut titles, &atlas, reading(vec![quest("Slime Samples", &[("Moss Slime", 2, 5)])], None));
        for _ in 0..HELD_READINGS {
            assert!(held(&mut titles, &atlas, None).is_some());
        }
        assert!(held(&mut titles, &atlas, None).is_none());
    }

    #[test]
    fn the_panels_own_count_decides_how_many_show() {
        let (atlas, mut titles) = (world(), QuestTitles::default());
        let both = vec![quest("Shell Sorting", &[("Pebble Snail", 1, 8)]), quest("Slime Samples", &[("Moss Slime", 2, 5)])];
        held(&mut titles, &atlas, reading(both, Some((2, 5))));
        let one = held(&mut titles, &atlas, reading(vec![quest("Slime Samples", &[("Moss Slime", 2, 5)])], Some((1, 5))));
        assert_eq!(one.unwrap().quests.len(), 1);
        assert!(held(&mut titles, &atlas, reading(vec![], Some((0, 5)))).unwrap().quests.is_empty());
    }

    #[test]
    fn a_title_garbled_live_is_too_rough_to_match_but_close_enough_to_hold() {
        let score = similarity(&fold("Blinging a Mil 101 to Heena"), &fold("Bringing a Mirror to Heena"));
        assert!((MIN_HELD_SCORE..MIN_QUEST_SCORE).contains(&score), "scored {score}");
    }

    #[test]
    fn a_quest_is_turned_in_only_where_something_shows_it() {
        let mut atlas = world();
        let mut add = |id, name: &str, npc_id, next_quest_id| {
            let name = name.into();
            let quest =
                QuestRecord { id, name, npc_id: Some(npc_id), next_quest_id, ..QuestRecord::default() };
            atlas.quests.insert(id, quest);
        };
        add(80, "Borrowing a Lamp", 50, Some(81));
        add(81, "Bringing a Lamp to Warden Ilse", 51, None);
        add(82, "Lamp Oil", 51, None);
        add(83, "A Trip to Town", 51, None);

        let chained = atlas.quest_info(80).unwrap();
        assert_eq!(chained.giver.as_deref(), Some("Warden Ilse"));
        assert_eq!(chained.turn_in.as_deref(), Some("Innkeeper Bo"));
        assert_eq!(chained.turn_in_maps.iter().map(|m| m.id).collect::<Vec<_>>(), [5]);

        let titled = atlas.quest_info(81).unwrap();
        assert_eq!(titled.giver.as_deref(), Some("Innkeeper Bo"));
        assert_eq!(titled.turn_in.as_deref(), Some("Warden Ilse"));
        assert_eq!(titled.turn_in_maps.iter().map(|m| m.id).collect::<Vec<_>>(), [1]);

        assert_eq!(atlas.quest_info(82).unwrap().turn_in, None);
        assert_eq!(atlas.quest_info(83).unwrap().turn_in, None, "`to` followed by no NPC");
    }

    #[test]
    fn the_wiki_says_who_takes_a_quest_in_before_anything_is_inferred() {
        use crate::mapdata::atlas::tests::map;
        use crate::mapdata::osms::{Dataset, NpcRecord, QuestRecord};
        use crate::mapdata::turnins::TurnIn;
        let npc = |id: u32, name: &str, maps: Vec<u32>| NpcRecord { id, name: name.into(), maps };
        let quest = |id: u32, name: &str, giver: u32, next: Option<u32>| QuestRecord {
            id,
            name: name.into(),
            npc_id: Some(giver),
            next_quest_id: next,
            ..QuestRecord::default()
        };
        let turn_in = |quest: u32, npc: &str| TurnIn { quest, npc: npc.into() };
        let atlas = Atlas::new(Dataset {
            maps: vec![map(1, "Mossbank", "Fern Road", vec![]), map(2, "Split Road", "Fern Road", vec![]), map(9, "Instance", "Fern Road", vec![])],
            npcs: vec![
                npc(1, "Sam", vec![1]),
                npc(2, "Mai", vec![2]),
                npc(3, "Tutor", vec![9]),
                npc(4, "Tutor", vec![2]),
                npc(5, "Biggs", vec![1]),
            ],
            quests: vec![
                quest(10, "Sam's Suggestion", 1, None),
                quest(11, "Into a Chain", 1, Some(12)),
                quest(12, "Tutor's Test", 4, None),
                quest(13, "Off the Wiki", 5, None),
                quest(14, "Unmatched", 1, Some(12)),
            ],
            turn_ins: vec![
                turn_in(10, "Mai"),
                turn_in(11, "Biggs"),
                turn_in(12, "Tutor (Fern Road)"),
                turn_in(14, "Nobody The Data Has"),
                turn_in(99, "Mai"),
            ],
            ..Dataset::default()
        });
        let who = |id: u32| atlas.quest_info(id).unwrap().turn_in;
        assert_eq!(who(10).as_deref(), Some("Mai"), "the client data has nothing to say on this one");
        assert_eq!(atlas.quest_info(10).unwrap().turn_in_maps[0].name, "Split Road");
        assert_eq!(who(11).as_deref(), Some("Biggs"), "the wiki outranks the next quest's giver");
        assert_eq!(atlas.quest_info(12).unwrap().turn_in_maps[0].name, "Split Road", "the giver wins among shared names");
        assert_eq!(who(13), None, "nothing to go on, so nothing is guessed");
        assert_eq!(who(14).as_deref(), Some("Tutor"), "a name the data lacks falls back on the chain");
    }

    #[test]
    fn maps_the_player_can_reach_come_before_richer_ones() {
        let atlas = island_world();
        let hunting = |from| {
            let guide =
                atlas.quest_guide(&[query("Shell Sorting", &["Pebble Snail"])], from, &[], &[], &Travel::none());
            guide[0].objectives[0].hunts[0].maps.iter().map(|g| g.map.name.clone()).collect::<Vec<_>>()
        };
        assert_eq!(hunting(Some(21)), ["Cove Shore", "Harbor Field"]);
        assert_eq!(hunting(None), ["Harbor Field", "Cove Shore"]);
    }

    #[test]
    fn a_measured_exp_rate_becomes_a_kill_pace() {
        let atlas = world();
        let measured = [MeasuredRate {
            label: "Fern Road : Mossbank Meadow I".into(),
            exp_per_hour: 600.0,
            active_seconds: 900.0,
        }];
        let guide =
            atlas.quest_guide(&[query("Shell Sorting", &["Pebble Snail"])], Some(1), &[], &measured, &Travel::none());
        let grounds = &guide[0].objectives[0].hunts[0].maps;
        // A clear of Meadow I is worth 60 base EXP and has 6 Pebble Snails: 10 clears an hour.
        let pace = grounds[0].pace.as_ref().unwrap();
        assert_eq!((pace.exp_per_hour, pace.active_seconds), (600.0, 900.0));
        assert!((pace.kills_per_hour - 60.0).abs() < 1e-9);
        assert!(grounds[1].pace.is_none(), "nothing was measured on Meadow II");
    }

    #[test]
    fn items_need_a_lookup_then_list_their_droppers() {
        let atlas = world();
        let quests = [query("[Event] Whisker Hunt", &["Rat Whisker"])];

        let before = &atlas.quest_guide(&quests, None, &[], &[], &Travel::none())[0];
        assert!(before.quest.is_none());
        let objective = &before.objectives[0];
        assert_eq!((objective.kind, objective.looked_up, objective.hunts.len()), (TargetKind::Item, false, 0));

        let after = &atlas.quest_guide(&quests, Some(1), &[wing_sources()], &[], &Travel::none())[0].objectives[0];
        assert!(after.looked_up);
        assert_eq!(after.item.as_deref(), Some("Rat Whisker"));
        let rat = &after.hunts[0];
        assert_eq!(rat.monster.name, "Burrow Rat");
        assert_eq!(rat.drop.as_ref().unwrap().rate, Some(12.0));
        assert_eq!((rat.maps[0].map.name.as_str(), rat.maps[0].hops), ("Burrow Den", Some(2)));
        // The wiki names a monster the map data doesn't have: still listed, without maps.
        assert!(!after.hunts[1].monster.known && after.hunts[1].maps.is_empty());
    }

    #[test]
    fn an_item_named_after_its_monster_is_not_the_monster() {
        let atlas = world();
        let guide = atlas.quest_guide(
            &[query("Untracked", &["Moss Slime Jelly", "Moss Slime"])],
            None,
            &[],
            &[],
            &Travel::none(),
        );
        let kinds: Vec<TargetKind> = guide[0].objectives.iter().map(|o| o.kind).collect();
        assert_eq!(kinds, [TargetKind::Item, TargetKind::Monster]);
    }

    #[test]
    fn truncated_titles_match_the_start_of_a_name() {
        let atlas = world();
        let guide = atlas.quest_guide(&[query("Slime Sam...", &[])], None, &[], &[], &Travel::none());
        assert_eq!(guide[0].quest.as_ref().map(|q| q.quest.id), Some(71));
        assert!(atlas.quest_guide(&[query("Slime", &[])], None, &[], &[], &Travel::none())[0].quest.is_none());
    }
}
