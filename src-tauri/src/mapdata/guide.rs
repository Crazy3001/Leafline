//! Where to go for the quests the Quest Helper shows: which monster each objective needs, the
//! maps with the most of it, and how many portals away each map is.
//!
//! An objective names either a monster to defeat or an item to collect. It is taken for a
//! monster when it reads like one in the data; otherwise it is an item, and the monsters that
//! drop it come from players' logs on mapleclassic.wiki, looked up separately.

use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use super::atlas::{Atlas, MapRef, QuestRef, fold, similarity};
use super::drops::{ItemSources, item_key};
use super::training::{Measured, MeasuredRate};

/// How closely an objective must match a monster's name to be taken for that monster.
const MIN_MONSTER_SCORE: f64 = 0.85;
/// Looser when the quest the title matched asks for that monster.
const MIN_QUEST_MONSTER_SCORE: f64 = 0.7;
/// How closely a dropper's name on the wiki must match a monster in the map data.
const MIN_DROPPER_SCORE: f64 = 0.9;
const MIN_QUEST_SCORE: f64 = 0.8;
const MAPS_PER_MONSTER: usize = 4;
const DROPPERS_PER_ITEM: usize = 4;

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
    /// Portals from the current map, when it is known and a route exists.
    pub hops: Option<u32>,
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

/// What the guide knows about the player, beyond the map they are on.
#[derive(Debug, Default)]
struct Context {
    hops: HashMap<u32, u32>,
    measured: HashMap<u32, Measured>,
}

impl Atlas {
    /// A guide for each quest. `from` is the map the player is on, for portal counts; `items`
    /// holds the wiki lookups made so far for item objectives; `measured` the EXP rates the
    /// player recorded, matched to maps by name.
    pub fn quest_guide(
        &self,
        quests: &[QuestQuery],
        from: Option<u32>,
        items: &[ItemSources],
        measured: &[MeasuredRate],
    ) -> Vec<QuestGuide> {
        let context = Context {
            hops: from.map(|id| self.hops_from(id)).unwrap_or_default(),
            measured: self.measured_by_map(measured),
        };
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
        let sources = items.iter().find(|s| s.key == key);
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

    /// The monsters (by ID) whose name best matches `name`, and how well. Several IDs can share
    /// a name; all of them are returned so their spawns add up. `among` limits the search.
    fn monsters_named(&self, name: &str, among: Option<&[u32]>) -> Option<(f64, Vec<u32>)> {
        let query = fold(name);
        if query.is_empty() {
            return None;
        }
        let candidates: Vec<&super::osms::MonsterRecord> = match among {
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
        let records: Vec<&super::osms::MonsterRecord> = ids.iter().filter_map(|id| self.monsters.get(id)).collect();
        // Of IDs sharing a name, the one that spawns most is the one players meet.
        let main = records
            .iter()
            .max_by_key(|m| (m.spawns.iter().map(|s| s.count).sum::<u32>(), std::cmp::Reverse(m.id)))
            .expect("hunt needs a known monster");
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
                hops: context.hops.get(&id).copied(),
                pace: self.pace(id, count, context),
            })
            .collect();
        maps.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then(a.hops.unwrap_or(u32::MAX).cmp(&b.hops.unwrap_or(u32::MAX)))
                .then_with(|| a.map.name.cmp(&b.map.name))
        });
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

    fn quest_info(&self, id: u32) -> Option<QuestInfo> {
        let quest = self.quests.get(&id)?;
        let mut giver_maps: Vec<MapRef> = match quest.npc_id {
            Some(npc) => self
                .npcs_on
                .iter()
                .filter(|(_, npcs)| npcs.contains(&npc))
                .map(|(map, _)| self.map_ref(*map))
                .filter(|m| m.known)
                .collect(),
            None => Vec::new(),
        };
        giver_maps.sort_by_key(|m| m.id);
        Some(QuestInfo {
            quest: self.quest_ref(id)?,
            giver: quest.npc_id.and_then(|n| self.npc_names.get(&n).cloned()),
            giver_maps,
        })
    }

    /// Portals from `from` to every map reachable through them.
    pub(super) fn hops_from(&self, from: u32) -> HashMap<u32, u32> {
        let mut hops = HashMap::from([(from, 0)]);
        let mut queue = VecDeque::from([from]);
        while let Some(at) = queue.pop_front() {
            let next = hops[&at] + 1;
            for exit in self.exits.get(&at).into_iter().flatten().filter(|e| e.to.known) {
                hops.entry(exit.to.id).or_insert_with(|| {
                    queue.push_back(exit.to.id);
                    next
                });
            }
        }
        hops
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapdata::atlas::tests::world;
    use crate::mapdata::drops::Dropper;

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
        let guide = atlas.quest_guide(&[query("Shell Sortng", &["Pebble Snai1"])], Some(5), &[], &[]);
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
    fn a_measured_exp_rate_becomes_a_kill_pace() {
        let atlas = world();
        let measured = [MeasuredRate {
            label: "Fern Road : Mossbank Meadow I".into(),
            exp_per_hour: 600.0,
            active_seconds: 900.0,
        }];
        let guide = atlas.quest_guide(&[query("Shell Sorting", &["Pebble Snail"])], Some(1), &[], &measured);
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

        let before = &atlas.quest_guide(&quests, None, &[], &[])[0];
        assert!(before.quest.is_none());
        let objective = &before.objectives[0];
        assert_eq!((objective.kind, objective.looked_up, objective.hunts.len()), (TargetKind::Item, false, 0));

        let after = &atlas.quest_guide(&quests, Some(1), &[wing_sources()], &[])[0].objectives[0];
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
        let guide = atlas.quest_guide(&[query("Untracked", &["Moss Slime Jelly", "Moss Slime"])], None, &[], &[]);
        let kinds: Vec<TargetKind> = guide[0].objectives.iter().map(|o| o.kind).collect();
        assert_eq!(kinds, [TargetKind::Item, TargetKind::Monster]);
    }

    #[test]
    fn truncated_titles_match_the_start_of_a_name() {
        let atlas = world();
        let guide = atlas.quest_guide(&[query("Slime Sam...", &[])], None, &[], &[]);
        assert_eq!(guide[0].quest.as_ref().map(|q| q.quest.id), Some(71));
        assert!(atlas.quest_guide(&[query("Slime", &[])], None, &[], &[])[0].quest.is_none());
    }
}
