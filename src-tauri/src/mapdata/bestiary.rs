//! Looking monsters up: a search of every monster in the data, and all it says about one of them.
//! The numbers are the data's base values, plus the drops players logged. Nothing here works out
//! damage, hits to kill or hit chances: Classic's formulas aren't confirmed.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::atlas::{Atlas, MapRef, QuestRef, fold, similarity};
use super::drops::{MonsterDrops, item_key};
use super::guide::{HuntingGround, MIN_DROPPER_SCORE, main_first};
use super::osms::{MapRecord, MonsterDetails, MonsterRecord};
use super::places::area_of;
use super::questbook::has_words;
use super::training::MeasuredRate;
use super::travel::Travel;

/// Monsters in one page of search results.
const MONSTER_PAGE: usize = 40;
/// How close a word must be to a monster's name to be offered when nothing matched outright.
const MIN_NEAR_NAME: f64 = 0.7;

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MonsterQuery {
    /// A monster, an item it drops, or a map it lives on.
    pub text: String,
    /// `field`, `boss` or `undead`.
    pub kind: Option<String>,
    pub weak_to: Option<String>,
    pub region: Option<String>,
    /// Monsters living on a map in one of [`Atlas::map_areas`].
    pub area: Option<String>,
    pub level_min: Option<u32>,
    pub level_max: Option<u32>,
    /// `name`, `hp`, `exp` or `efficiency`. By level otherwise.
    pub sort: Option<String>,
    /// With no `sort`, the monsters nearest this level first.
    pub near: Option<u32>,
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonsterRow {
    pub id: u32,
    pub name: String,
    pub level: u32,
    pub hp: u32,
    pub exp: u32,
    pub boss: bool,
    pub undead: bool,
    /// Why a search found it, when that isn't its name.
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonsterPage {
    pub monsters: Vec<MonsterRow>,
    pub total: usize,
    /// Nothing matched outright, so these are the closest names.
    pub closest: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonsterFilters {
    pub regions: Vec<String>,
    /// The elements some monster is weak to.
    pub elements: Vec<String>,
}

/// Another monster named on a sheet.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonsterSplit {
    pub id: u32,
    pub name: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonsterQuest {
    pub quest: QuestRef,
    pub giver: Option<String>,
    /// How many the quest wants.
    pub count: u32,
}

/// Another record with the same name but its own numbers, such as a party quest's copy.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonsterVersion {
    pub id: u32,
    pub level: u32,
    pub hp: u32,
    pub exp: u32,
    pub maps: Vec<MapRef>,
}

/// All the data says about one monster.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonsterSheet {
    pub id: u32,
    pub name: String,
    pub level: u32,
    pub hp: u32,
    pub exp: u32,
    pub boss: bool,
    pub undead: bool,
    pub details: MonsterDetails,
    /// A boss's longest respawn time as the file lists it, in seconds. Only a rough guide.
    pub respawn: Option<u32>,
    /// The best maps first.
    pub maps: Vec<HuntingGround>,
    pub quests: Vec<MonsterQuest>,
    pub versions: Vec<MonsterVersion>,
    pub splits: Vec<MonsterSplit>,
    /// What players logged. `None` when the drop table is empty or not downloaded.
    pub drops: Option<MonsterDrops>,
}

/// Whether `kind` takes in this monster.
fn of_kind(monster: &MonsterRecord, kind: &str) -> bool {
    match kind {
        "boss" => monster.is_boss,
        "undead" => monster.undead,
        "field" => !monster.is_boss,
        _ => true,
    }
}

fn row(monster: &MonsterRecord, note: Option<String>) -> MonsterRow {
    MonsterRow {
        id: monster.id,
        name: monster.name.clone(),
        level: monster.level,
        hp: monster.hp,
        exp: monster.exp,
        boss: monster.is_boss,
        undead: monster.undead,
        note,
    }
}

/// How close `query` comes to a name, whole or one of its words.
pub(super) fn name_score(query: &str, name: &str) -> f64 {
    let key = fold(name);
    key.split_whitespace().map(|word| similarity(query, word)).fold(similarity(query, &key), f64::max)
}

impl Atlas {
    /// Every monster with its versions, the one players meet first. The data lists one name
    /// several times, for party quests and for bosses that are summoned.
    fn families(&self) -> Vec<Vec<&MonsterRecord>> {
        let mut by_name: HashMap<String, Vec<&MonsterRecord>> = HashMap::new();
        for monster in self.monsters.values() {
            by_name.entry(fold(&monster.name)).or_default().push(monster);
        }
        let mut families: Vec<Vec<&MonsterRecord>> = by_name.into_values().collect();
        for family in &mut families {
            family.sort_by_key(|m| main_first(m));
        }
        families
    }

    fn placed(&self, monster: &MonsterRecord) -> bool {
        monster.spawns.iter().any(|s| s.count > 0 && self.maps.contains_key(&s.map_id))
    }

    /// Whether anything shows that players meet this monster. The data also holds test records
    /// and monsters no map has yet, which only a search by name brings up.
    fn met(&self, family: &[&MonsterRecord], logged: bool) -> bool {
        logged || family.iter().any(|m| self.placed(m) || self.quests_for_mob.contains_key(&m.id))
    }

    /// How `words` find a monster, the best first, and why if it isn't by name.
    fn monster_rank(&self, family: &[&MonsterRecord], words: &[String], items: &[&str]) -> Option<(u8, Option<String>)> {
        if words.is_empty() {
            return Some((0, None));
        }
        let name = item_key(&family[0].name);
        if has_words(&name, words) {
            return Some((0, None));
        }
        if name.contains(&words.join(" ")) {
            return Some((1, None));
        }
        if let Some(item) = items.iter().find(|item| has_words(&item_key(item), words)) {
            return Some((2, Some(format!("Drops {item}"))));
        }
        let mut maps = family.iter().flat_map(|m| &m.spawns).filter_map(|s| self.maps.get(&s.map_id));
        maps.find(|map| has_words(&item_key(&map.name), words)).map(|map| (3, Some(format!("On {}", map.name))))
    }

    /// One page of the monsters `query` finds, one row to a name. `dropped` holds the drop
    /// table's monster and item pairs, which is empty until it has been downloaded.
    pub fn search_monsters(&self, query: &MonsterQuery, dropped: &[(String, String)]) -> MonsterPage {
        let words: Vec<String> = item_key(&query.text).split_whitespace().map(str::to_string).collect();
        let mut items_of: HashMap<String, Vec<&str>> = HashMap::new();
        for (monster, item) in dropped {
            items_of.entry(fold(monster)).or_default().push(item);
        }
        let admits = |family: &&Vec<&MonsterRecord>| {
            let main = family[0];
            let lives = |on: &dyn Fn(&MapRecord) -> bool| {
                family.iter().flat_map(|m| &m.spawns).any(|s| self.maps.get(&s.map_id).is_some_and(on))
            };
            let in_region = |region: &String| lives(&|map| map.region.as_ref() == Some(region));
            let in_area = |area: &String| lives(&|map| area_of(map).as_ref() == Some(area));
            let weak = |element: &String| main.details.elements.iter().any(|e| e.effect == "Weak" && e.element == *element);
            query.kind.as_ref().is_none_or(|kind| of_kind(main, kind))
                && query.level_min.is_none_or(|level| main.level >= level)
                && query.level_max.is_none_or(|level| main.level <= level)
                && query.weak_to.as_ref().is_none_or(weak)
                && query.region.as_ref().is_none_or(in_region)
                && query.area.as_ref().is_none_or(in_area)
                && (!words.is_empty() || self.met(family, items_of.contains_key(&fold(&main.name))))
        };
        let families = self.families();
        let candidates: Vec<&Vec<&MonsterRecord>> = families.iter().filter(admits).collect();
        let mut found: Vec<(u8, &MonsterRecord, Option<String>)> = candidates
            .iter()
            .filter_map(|family| {
                let items = items_of.get(&fold(&family[0].name)).map_or(&[][..], Vec::as_slice);
                let (rank, note) = self.monster_rank(family, &words, items)?;
                Some((rank, family[0], note))
            })
            .collect();
        let closest = found.is_empty() && !words.is_empty();
        if closest {
            let text = words.join(" ");
            let mut near: Vec<(f64, &MonsterRecord)> = candidates
                .iter()
                .map(|family| (name_score(&text, &family[0].name), family[0]))
                .filter(|(score, _)| *score >= MIN_NEAR_NAME)
                .collect();
            near.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.name.cmp(&b.1.name)));
            found = near.into_iter().map(|(_, monster)| (0, monster, None)).collect();
        }
        let efficiency = |m: &MonsterRecord| f64::from(m.exp) / f64::from(m.hp.max(1));
        match query.sort.as_deref() {
            Some("name") => found.sort_by(|a, b| a.1.name.cmp(&b.1.name)),
            Some("hp") => found.sort_by_key(|(_, m, _)| std::cmp::Reverse(m.hp)),
            Some("exp") => found.sort_by_key(|(_, m, _)| std::cmp::Reverse(m.exp)),
            Some("efficiency") => found.sort_by(|a, b| efficiency(b.1).total_cmp(&efficiency(a.1))),
            _ if closest => {}
            _ => found.sort_by_key(|(rank, m, _)| {
                (*rank, query.near.map_or(0, |level| m.level.abs_diff(level)), m.level, m.name.clone())
            }),
        }
        MonsterPage {
            total: found.len(),
            monsters: found.into_iter().skip(query.offset).take(MONSTER_PAGE).map(|(_, m, note)| row(m, note)).collect(),
            closest,
        }
    }

    /// The regions monsters live in and the elements some monster is weak to.
    pub fn monster_filters(&self) -> MonsterFilters {
        let spawned = self.monsters.values().flat_map(|m| &m.spawns).filter_map(|s| self.maps.get(&s.map_id));
        let mut regions: Vec<String> = spawned.filter_map(|map| map.region.clone()).collect();
        regions.sort();
        regions.dedup();
        let effects = self.monsters.values().flat_map(|m| &m.details.elements);
        let mut elements: Vec<String> = effects.filter(|e| e.effect == "Weak").map(|e| e.element.clone()).collect();
        elements.sort();
        elements.dedup();
        MonsterFilters { regions, elements }
    }

    /// The monster a name means, the version players meet.
    pub fn monster_like(&self, name: &str) -> Option<u32> {
        let (score, ids) = self.monsters_named(name, None)?;
        let versions = ids.iter().filter_map(|id| self.monsters.get(id));
        (score >= MIN_DROPPER_SCORE).then(|| versions.min_by_key(|m| main_first(m)).map(|m| m.id))?
    }

    pub fn monster_name(&self, id: u32) -> Option<&str> {
        self.monsters.get(&id).map(|m| m.name.as_str())
    }

    /// The monsters sharing this one's name, itself included.
    fn versions_of(&self, monster: &MonsterRecord) -> Vec<&MonsterRecord> {
        let key = fold(&monster.name);
        let mut family: Vec<&MonsterRecord> = self.monsters.values().filter(|m| fold(&m.name) == key).collect();
        family.sort_by_key(|m| main_first(m));
        family
    }

    /// Quests that want this monster, in any of its versions, the lowest level first.
    fn quests_wanting(&self, family: &[&MonsterRecord]) -> Vec<MonsterQuest> {
        let ids: Vec<u32> = family.iter().map(|m| m.id).collect();
        let mut wanting: Vec<u32> = ids.iter().filter_map(|id| self.quests_for_mob.get(id)).flatten().copied().collect();
        wanting.sort_unstable();
        wanting.dedup();
        let mut quests: Vec<MonsterQuest> = wanting
            .into_iter()
            .filter_map(|id| {
                let quest = self.quests.get(&id)?;
                Some(MonsterQuest {
                    quest: self.quest_ref(id)?,
                    giver: quest.npc_id.and_then(|npc| self.npc_names.get(&npc).cloned()),
                    count: quest.requirements.iter().filter(|r| ids.contains(&r.mob_id)).map(|r| r.count).sum(),
                })
            })
            .collect();
        quests.sort_by(|a, b| (a.quest.level_min, &a.quest.name).cmp(&(b.quest.level_min, &b.quest.name)));
        quests
    }

    /// The other records of a name, one to a set of numbers. One with the same numbers as the
    /// monster shown and no map of its own says nothing.
    fn other_versions(&self, monster: &MonsterRecord, family: &[&MonsterRecord]) -> Vec<MonsterVersion> {
        let numbers = |m: &MonsterVersion| (m.level, m.hp, m.exp);
        let shown = (monster.level, monster.hp, monster.exp);
        let mut versions: Vec<MonsterVersion> = Vec::new();
        for other in family.iter().filter(|m| m.id != monster.id) {
            let placed = other.spawns.iter().filter(|s| s.count > 0 && self.maps.contains_key(&s.map_id));
            let maps: Vec<MapRef> = placed.map(|s| self.map_ref(s.map_id)).collect();
            let version = MonsterVersion { id: other.id, level: other.level, hp: other.hp, exp: other.exp, maps };
            if version.maps.is_empty() && numbers(&version) == shown {
                continue;
            }
            match versions.iter_mut().find(|v| numbers(v) == numbers(&version)) {
                Some(same) => same.maps.extend(version.maps),
                None => versions.push(version),
            }
        }
        versions
    }

    /// All the data says about a monster. `from` is the map the player is on, for portal counts;
    /// `measured` the EXP rates the player recorded; `travel` the rides and scrolls turned on.
    pub fn monster_sheet(
        &self,
        id: u32,
        drops: Option<MonsterDrops>,
        from: Option<u32>,
        measured: &[MeasuredRate],
        travel: &Travel,
    ) -> Option<MonsterSheet> {
        let monster = self.monsters.get(&id)?;
        let family = self.versions_of(monster);
        let context = self.context(from, measured, travel);
        let splits = monster.details.splits_into.iter().filter_map(|split| {
            let into = self.monsters.get(&split.mob_id)?;
            Some(MonsterSplit { id: into.id, name: into.name.clone(), count: split.count })
        });
        Some(MonsterSheet {
            id,
            name: monster.name.clone(),
            level: monster.level,
            hp: monster.hp,
            exp: monster.exp,
            boss: monster.is_boss,
            undead: monster.undead,
            details: monster.details.clone(),
            respawn: monster.spawns.iter().filter_map(|s| s.respawn).max().filter(|_| monster.is_boss),
            maps: self.hunting_grounds(&[monster], &context),
            quests: self.quests_wanting(&family),
            versions: self.other_versions(monster, &family),
            splits: splits.collect(),
            drops,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::atlas::tests::{map, mob, world};
    use super::super::osms::{Dataset, ElementEffect, MobRequirement, Portal, QuestRecord, Spawn, Split};
    use super::*;

    fn portal(name: &str, dest: u32) -> Portal {
        Portal { name: name.into(), dest_map_id: Some(dest), dest_portal: None, side: None, height: None }
    }

    // A small made-up bestiary. Fern Isle has the meadow and the den; Ashlands has the caldera.
    fn bestiary() -> Atlas {
        let weak_to = |element: &str| ElementEffect { element: element.into(), effect: "Weak".into() };
        let mut slime = mob(10, "Moss Slime", 5, 9, &[(2, 6)]);
        slime.details.elements = vec![weak_to("Fire")];
        slime.details.splits_into = vec![];
        let mut golem = mob(20, "Ember Golem", 40, 300, &[(3, 1)]);
        golem.is_boss = true;
        golem.spawns[0].respawn = Some(3600);
        golem.details.splits_into = vec![Split { mob_id: 10, count: 3 }];
        let mut ghoul = mob(30, "Bog Ghoul", 12, 30, &[(2, 2)]);
        ghoul.undead = true;
        ghoul.details.elements = vec![weak_to("Holy")];
        let mut ashlands = map(3, "Ember Caldera", "Ash Road", vec![]);
        ashlands.region = Some("Ashlands".into());
        Atlas::new(Dataset {
            maps: vec![
                map(1, "Mossbank", "Fern Road", vec![portal("east00", 2)]),
                map(2, "Mossbank Meadow", "Fern Road", vec![portal("west00", 1), portal("in00", 4)]),
                ashlands,
                map(4, "Party Den", "Fern Road", vec![portal("out00", 2)]),
            ],
            monsters: vec![
                slime,
                golem,
                ghoul,
                // The same name for a party quest, with its own numbers.
                mob(800, "Moss Slime", 3, 2, &[(4, 2)]),
                // A summoned copy of the boss, with nothing to add.
                mob(810, "Ember Golem", 40, 300, &[]),
                // Records no map has: a test leftover and a monster not in the game yet.
                mob(990, "Test(Fire)", 30, 1, &[]),
                mob(40, "Thorn Vine", 18, 40, &[]),
            ],
            quests: vec![
                QuestRecord {
                    id: 70,
                    name: "Slime Samples".into(),
                    level_min: Some(4),
                    requirements: vec![MobRequirement { mob_id: 10, count: 5 }, MobRequirement { mob_id: 800, count: 2 }],
                    ..QuestRecord::default()
                },
                QuestRecord {
                    id: 71,
                    name: "Early Samples".into(),
                    level_min: Some(1),
                    requirements: vec![MobRequirement { mob_id: 10, count: 1 }],
                    ..QuestRecord::default()
                },
            ],
            ..Dataset::default()
        })
    }

    fn found(atlas: &Atlas, query: MonsterQuery, dropped: &[(String, String)]) -> Vec<String> {
        atlas.search_monsters(&query, dropped).monsters.into_iter().map(|m| m.name).collect()
    }

    fn named(text: &str) -> MonsterQuery {
        MonsterQuery { text: text.into(), ..MonsterQuery::default() }
    }

    #[test]
    fn lists_one_row_to_a_name_by_level_and_only_monsters_players_meet() {
        let atlas = bestiary();
        let page = atlas.search_monsters(&MonsterQuery::default(), &[]);
        let rows: Vec<(u32, &str)> = page.monsters.iter().map(|m| (m.id, m.name.as_str())).collect();
        assert_eq!(rows, [(10, "Moss Slime"), (30, "Bog Ghoul"), (20, "Ember Golem")]);
        assert_eq!((page.total, page.closest), (3, false));
    }

    #[test]
    fn a_name_brings_up_a_monster_no_map_has() {
        let atlas = bestiary();
        assert_eq!(found(&atlas, named("thorn"), &[]), ["Thorn Vine"]);
        assert_eq!(found(&atlas, named("test"), &[]), ["Test(Fire)"]);
    }

    #[test]
    fn finds_monsters_by_part_of_a_name_a_drop_or_a_map() {
        let atlas = bestiary();
        let dropped = [("Bog Ghoul".to_string(), "Ember Dust".to_string()), ("Moss Slime".to_string(), "Sticky Goo".to_string())];
        assert_eq!(found(&atlas, named("lime"), &dropped), ["Moss Slime"], "inside a word");
        let page = atlas.search_monsters(&named("ember"), &dropped);
        let rows: Vec<(&str, Option<&str>)> = page.monsters.iter().map(|m| (m.name.as_str(), m.note.as_deref())).collect();
        assert_eq!(rows, [("Ember Golem", None), ("Bog Ghoul", Some("Drops Ember Dust"))], "its name counts first");
        let page = atlas.search_monsters(&named("meadow"), &[]);
        assert_eq!(page.monsters[0].note.as_deref(), Some("On Mossbank Meadow"));
        assert_eq!(page.total, 2);
    }

    #[test]
    fn a_typo_finds_the_closest_names_and_says_so() {
        let atlas = bestiary();
        let page = atlas.search_monsters(&named("golm"), &[]);
        assert!(page.closest);
        assert_eq!(page.monsters[0].name, "Ember Golem");
        assert!(!atlas.search_monsters(&named("golem"), &[]).closest);
        assert_eq!(atlas.search_monsters(&named("qqqq"), &[]).total, 0);
    }

    #[test]
    fn filters_narrow_the_list() {
        let atlas = bestiary();
        let with = |query: MonsterQuery| found(&atlas, query, &[]);
        assert_eq!(with(MonsterQuery { kind: Some("boss".into()), ..MonsterQuery::default() }), ["Ember Golem"]);
        assert_eq!(with(MonsterQuery { kind: Some("undead".into()), ..MonsterQuery::default() }), ["Bog Ghoul"]);
        assert_eq!(with(MonsterQuery { kind: Some("field".into()), ..MonsterQuery::default() }), ["Moss Slime", "Bog Ghoul"]);
        assert_eq!(with(MonsterQuery { weak_to: Some("Holy".into()), ..MonsterQuery::default() }), ["Bog Ghoul"]);
        assert_eq!(with(MonsterQuery { region: Some("Ashlands".into()), ..MonsterQuery::default() }), ["Ember Golem"]);
        assert_eq!(with(MonsterQuery { level_min: Some(6), level_max: Some(20), ..MonsterQuery::default() }), ["Bog Ghoul"]);
        assert_eq!(atlas.monster_filters().regions, ["Ashlands", "Fern Isle"]);
        assert_eq!(atlas.monster_filters().elements, ["Fire", "Holy"]);
    }

    #[test]
    fn sorts_by_what_the_player_asks() {
        let atlas = bestiary();
        let sorted = |sort: &str| found(&atlas, MonsterQuery { sort: Some(sort.into()), ..MonsterQuery::default() }, &[]);
        assert_eq!(sorted("name"), ["Bog Ghoul", "Ember Golem", "Moss Slime"]);
        assert_eq!(sorted("hp"), ["Ember Golem", "Bog Ghoul", "Moss Slime"]);
        assert_eq!(sorted("exp"), ["Ember Golem", "Bog Ghoul", "Moss Slime"]);
        // 9 EXP for 50 HP, 300 for 400, 30 for 120.
        assert_eq!(sorted("efficiency"), ["Ember Golem", "Bog Ghoul", "Moss Slime"]);
        let near = found(&atlas, MonsterQuery { near: Some(9), ..MonsterQuery::default() }, &[]);
        assert_eq!(near, ["Bog Ghoul", "Moss Slime", "Ember Golem"]);
    }

    #[test]
    fn a_page_is_a_slice_of_the_whole_list() {
        let monsters = (1..=45).map(|i| mob(i, &format!("Pest {i}"), i, 1, &[(1, 1)])).collect();
        let atlas = Atlas::new(Dataset { maps: vec![map(1, "Mossbank", "Fern Road", vec![])], monsters, ..Dataset::default() });
        let page = atlas.search_monsters(&MonsterQuery::default(), &[]);
        assert_eq!((page.monsters.len(), page.total), (MONSTER_PAGE, 45));
        let next = atlas.search_monsters(&MonsterQuery { offset: MONSTER_PAGE, ..MonsterQuery::default() }, &[]);
        assert_eq!(next.monsters.len(), 5);
    }

    #[test]
    fn a_name_means_the_version_players_meet() {
        let atlas = bestiary();
        assert_eq!(atlas.monster_like("Moss Slime"), Some(10));
        assert_eq!(atlas.monster_like("Ember Golem"), Some(20));
        assert_eq!(atlas.monster_like("Nothing Like It"), None);
    }

    #[test]
    fn a_sheet_gathers_maps_quests_and_versions() {
        let atlas = bestiary();
        let sheet = atlas.monster_sheet(10, None, Some(1), &[], &Travel::none()).unwrap();
        assert_eq!((sheet.level, sheet.hp, sheet.exp), (5, 50, 9));

        let maps: Vec<(&str, u32, u32, Option<u32>)> =
            sheet.maps.iter().map(|m| (m.map.name.as_str(), m.count, m.map_total, m.hops)).collect();
        assert_eq!(maps, [("Mossbank Meadow", 6, 8, Some(1))], "only this version's maps");

        let quests: Vec<(&str, u32)> = sheet.quests.iter().map(|q| (q.quest.name.as_str(), q.count)).collect();
        assert_eq!(quests, [("Early Samples", 1), ("Slime Samples", 7)], "both versions count");

        let versions: Vec<(u32, u32, Vec<&str>)> =
            sheet.versions.iter().map(|v| (v.id, v.level, v.maps.iter().map(|m| m.name.as_str()).collect())).collect();
        assert_eq!(versions, [(800, 3, vec!["Party Den"])]);
    }

    #[test]
    fn a_copy_with_the_same_numbers_and_no_map_is_left_out() {
        let atlas = bestiary();
        assert!(atlas.monster_sheet(20, None, None, &[], &Travel::none()).unwrap().versions.is_empty());
        let copy = atlas.monster_sheet(810, None, None, &[], &Travel::none()).unwrap();
        assert_eq!(copy.versions.iter().map(|v| v.id).collect::<Vec<_>>(), [20], "its copy has the map");
    }

    #[test]
    fn a_boss_says_what_it_turns_into_and_its_listed_respawn() {
        let atlas = bestiary();
        let golem = atlas.monster_sheet(20, None, None, &[], &Travel::none()).unwrap();
        assert_eq!(golem.respawn, Some(3600));
        assert_eq!(golem.splits, [MonsterSplit { id: 10, name: "Moss Slime".into(), count: 3 }]);

        let mut plain = mob(1, "Pebble Snail", 1, 1, &[(1, 1)]);
        plain.spawns[0] = Spawn { map_id: 1, count: 1, respawn: Some(300) };
        let atlas = Atlas::new(Dataset { maps: vec![map(1, "Mossbank", "Fern Road", vec![])], monsters: vec![plain], ..Dataset::default() });
        assert_eq!(atlas.monster_sheet(1, None, None, &[], &Travel::none()).unwrap().respawn, None, "only bosses");
    }

    #[test]
    fn a_monster_on_no_map_still_has_a_sheet() {
        let sheet = bestiary().monster_sheet(40, None, Some(1), &[], &Travel::none()).unwrap();
        assert!(sheet.maps.is_empty() && sheet.quests.is_empty() && sheet.versions.is_empty());
        assert!(world().monster_sheet(999, None, None, &[], &Travel::none()).is_none());
    }

    #[test]
    fn a_map_the_player_measured_gives_a_pace() {
        let measured = [MeasuredRate {
            label: "Fern Road : Mossbank Meadow".into(),
            exp_per_hour: 1000.0,
            active_seconds: 600.0,
            ..MeasuredRate::default()
        }];
        let sheet = bestiary().monster_sheet(10, None, None, &measured, &Travel::none()).unwrap();
        let pace = sheet.maps[0].pace.as_ref().unwrap();
        // A clear is 6 × 9 + 2 × 30 = 114 EXP, with 6 of the slimes in it.
        assert!((pace.kills_per_hour - 1000.0 * 6.0 / 114.0).abs() < 1e-9);
    }
}
