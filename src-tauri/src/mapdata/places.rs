//! Looking places and people up: a search of every map and NPC in the data, and where an NPC
//! stands with the quests they give.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::atlas::{Atlas, MapRef, QuestRef, Shortcut, fold};
use super::bestiary::name_score;
use super::drops::item_key;
use super::osms::MapRecord;
use super::questbook::{QuestRow, has_words};
use super::travel::Travel;

/// Rows in one page of search results.
const PAGE: usize = 40;
/// Maps the game's makers left in the data, which only a search by name brings up.
const HIDDEN_REGION: &str = "Dev";
/// How close a word must be to a name to be offered when nothing matched outright.
const MIN_NEAR_NAME: f64 = 0.7;
/// World-map marks that aren't a place: events, quest rooms and maps with none.
const NO_AREA: &[&str] = &["", "None", "Event", "Quest"];
/// Marks whose own spelling isn't the place's name, or that make up one place together.
const AREA_NAMES: &[(&str, &str)] = &[
    ("Amherst", "Maple Island"),
    ("MushroomVillage", "Maple Island"),
    ("SouthPerry", "Maple Island"),
    ("Rith", "Lith Harbor"),
    ("Dungeon", "Sleepywood"),
];
/// The areas in the order a new character meets them. Any others follow by name.
const AREA_ORDER: &[&str] = &["Maple Island", "Lith Harbor", "Henesys", "Ellinia", "Perion", "Kerning City", "Sleepywood"];
/// Fewer maps than this under a mark is a stray, not a place to offer.
const MIN_AREA_MAPS: usize = 3;

/// The area a map is in, from the mark the world map files it under: `KerningCity` is Kerning
/// City, and the dungeon mark covers Sleepywood and the caves and tunnels around it.
pub(super) fn area_of(map: &MapRecord) -> Option<String> {
    let mark = map.map_mark.as_deref()?.trim();
    if NO_AREA.contains(&mark) || map.region.as_deref() == Some(HIDDEN_REGION) {
        return None;
    }
    if let Some((_, name)) = AREA_NAMES.iter().find(|(m, _)| *m == mark) {
        return Some(name.to_string());
    }
    let mut name = String::new();
    for (i, c) in mark.char_indices() {
        if c.is_uppercase() && mark[..i].chars().last().is_some_and(char::is_lowercase) {
            name.push(' ');
        }
        name.push(c);
    }
    Some(name)
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MapQuery {
    /// A map, a street or region, a monster on it or an NPC on it.
    pub text: String,
    /// `town` or `field`.
    pub kind: Option<String>,
    pub region: Option<String>,
    /// One of [`Atlas::map_areas`].
    pub area: Option<String>,
    /// Maps with a monster in this range of levels.
    pub level_min: Option<u32>,
    pub level_max: Option<u32>,
    /// `level` for the lowest first. By name otherwise.
    pub sort: Option<String>,
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapRow {
    pub map: MapRef,
    pub region: Option<String>,
    pub is_town: bool,
    pub level_range: Option<(u32, u32)>,
    /// Why a search found it, when that isn't its name.
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapPage {
    pub maps: Vec<MapRow>,
    pub total: usize,
    /// Nothing matched outright, so these are the closest names.
    pub closest: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NpcQuery {
    /// An NPC, a quest they give or a map they stand on.
    pub text: String,
    /// `quests` for only those who give some.
    pub kind: Option<String>,
    pub region: Option<String>,
    /// Those standing on a map in one of [`Atlas::map_areas`].
    pub area: Option<String>,
    pub offset: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcRow {
    pub id: u32,
    pub name: String,
    /// The first map they stand on.
    pub map: Option<String>,
    pub maps: usize,
    pub quests: usize,
    /// Why a search found them, when that isn't their name.
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcPage {
    pub npcs: Vec<NpcRow>,
    pub total: usize,
    /// Nothing matched outright, so these are the closest names.
    pub closest: bool,
}

/// A map someone stands on, and how far it is.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcPlace {
    pub map: MapRef,
    /// Portals from the current map, when it is known and a route exists.
    pub hops: Option<u32>,
    /// Fewer steps with the travel the player turned on, when that is shorter.
    pub travel: Option<Shortcut>,
}

/// All the data says about one NPC.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NpcSheet {
    pub id: u32,
    pub name: String,
    /// The closest first.
    pub maps: Vec<NpcPlace>,
    /// The lowest level first.
    pub quests: Vec<QuestRow>,
    /// Quests someone else gives that go by what shows they are handed in here.
    pub takes: Vec<QuestRef>,
}

impl Atlas {
    /// Lowest and highest monster level on a map, and the names of what lives there.
    fn map_monsters(&self, id: u32) -> (Option<(u32, u32)>, Vec<&str>) {
        let monsters: Vec<_> = self.spawns.get(&id).into_iter().flatten().filter_map(|(mob, _)| self.monsters.get(mob)).collect();
        let levels = monsters.iter().map(|m| m.level);
        (levels.clone().min().zip(levels.max()), monsters.iter().map(|m| m.name.as_str()).collect())
    }

    /// How `words` find a map, the best first, and why if it isn't by name.
    fn map_rank(&self, map: &MapRecord, words: &[String], monsters: &[&str]) -> Option<(u8, Option<String>)> {
        if words.is_empty() {
            return (map.region.as_deref() != Some(HIDDEN_REGION)).then_some((0, None));
        }
        let name = item_key(&map.name);
        if has_words(&name, words) {
            return Some((0, None));
        }
        if name.contains(&words.join(" ")) {
            return Some((1, None));
        }
        let place = [map.street_name.as_deref(), map.region.as_deref(), map.map_mark.as_deref()];
        if has_words(&item_key(&place.iter().flatten().copied().collect::<Vec<_>>().join(" ")), words) {
            return Some((2, None));
        }
        if let Some(monster) = monsters.iter().find(|m| has_words(&item_key(m), words)) {
            return Some((3, Some(format!("Has {monster}"))));
        }
        let mut npcs = self.npcs_on.get(&map.id).into_iter().flatten().filter_map(|npc| self.npc_names.get(npc));
        npcs.find(|npc| has_words(&item_key(npc), words)).map(|npc| (4, Some(format!("{npc} is here"))))
    }

    /// One page of the maps `query` finds.
    pub fn browse_maps(&self, query: &MapQuery) -> MapPage {
        let words: Vec<String> = item_key(&query.text).split_whitespace().map(str::to_string).collect();
        let overlaps = |range: Option<(u32, u32)>| {
            let wanted = query.level_min.is_some() || query.level_max.is_some();
            !wanted
                || range.is_some_and(|(low, high)| {
                    query.level_min.is_none_or(|level| high >= level) && query.level_max.is_none_or(|level| low <= level)
                })
        };
        let admits = |map: &&MapRecord| {
            query.region.as_ref().is_none_or(|region| map.region.as_ref() == Some(region))
                && query.area.as_ref().is_none_or(|area| area_of(map).as_ref() == Some(area))
                && query.kind.as_deref().is_none_or(|kind| (kind == "town") == map.is_town)
        };
        let row = |map: &MapRecord, range, note| MapRow {
            map: self.map_ref(map.id),
            region: map.region.clone(),
            is_town: map.is_town,
            level_range: range,
            note,
        };
        let candidates: Vec<&MapRecord> = self.maps.values().filter(admits).collect();
        let mut found: Vec<(u8, MapRow)> = candidates
            .iter()
            .filter_map(|map| {
                let (range, monsters) = self.map_monsters(map.id);
                if !overlaps(range) {
                    return None;
                }
                let (rank, note) = self.map_rank(map, &words, &monsters)?;
                Some((rank, row(map, range, note)))
            })
            .collect();
        let closest = found.is_empty() && !words.is_empty();
        if closest {
            let text = words.join(" ");
            let mut near: Vec<(f64, MapRow)> = candidates
                .iter()
                .map(|map| (name_score(&text, &map.name), row(map, self.map_monsters(map.id).0, None)))
                .filter(|(score, _)| *score >= MIN_NEAR_NAME)
                .collect();
            near.sort_by(|a, b| b.0.total_cmp(&a.0));
            found = near.into_iter().map(|(_, row)| (0, row)).collect();
        } else if query.sort.as_deref() == Some("level") {
            found.sort_by_key(|(rank, row)| (*rank, row.level_range.is_none(), row.level_range, row.map.name.clone()));
        } else {
            found.sort_by_key(|(rank, row)| (*rank, row.map.name.clone(), row.map.id));
        }
        MapPage {
            total: found.len(),
            maps: found.into_iter().skip(query.offset).take(PAGE).map(|(_, row)| row).collect(),
            closest,
        }
    }

    /// The regions maps are filed under, without the makers' own.
    pub fn map_regions(&self) -> Vec<String> {
        let mut regions: Vec<String> = self.maps.values().filter_map(|m| m.region.clone()).filter(|r| r != HIDDEN_REGION).collect();
        regions.sort();
        regions.dedup();
        regions
    }

    /// The areas the Lookup tab narrows maps, monsters and NPCs to, Maple Island first.
    pub fn map_areas(&self) -> Vec<String> {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for area in self.maps.values().filter_map(area_of) {
            *counts.entry(area).or_default() += 1;
        }
        let mut areas: Vec<String> = counts.into_iter().filter(|(_, n)| *n >= MIN_AREA_MAPS).map(|(area, _)| area).collect();
        let place = |area: &String| AREA_ORDER.iter().position(|a| a == area).unwrap_or(AREA_ORDER.len());
        areas.sort_by(|a, b| place(a).cmp(&place(b)).then_with(|| a.cmp(b)));
        areas
    }

    /// How `words` find an NPC, the best first, and why if it isn't by name.
    fn npc_rank(&self, id: u32, name: &str, maps: &[&MapRecord], words: &[String]) -> Option<(u8, Option<String>)> {
        if words.is_empty() {
            return Some((0, None));
        }
        let key = item_key(name);
        if has_words(&key, words) {
            return Some((0, None));
        }
        if key.contains(&words.join(" ")) {
            return Some((1, None));
        }
        let mut gives = self.quests_from.get(&id).into_iter().flatten().filter_map(|quest| self.quests.get(quest));
        if let Some(quest) = gives.find(|quest| has_words(&item_key(&quest.name), words)) {
            return Some((2, Some(format!("Gives {}", quest.name))));
        }
        maps.iter().find(|map| has_words(&item_key(&map.name), words)).map(|map| (3, Some(format!("On {}", map.name))))
    }

    /// One page of the NPCs `query` finds. Those on no map come up only by name.
    pub fn search_npcs(&self, query: &NpcQuery) -> NpcPage {
        let words: Vec<String> = item_key(&query.text).split_whitespace().map(str::to_string).collect();
        let mut maps_of: HashMap<u32, Vec<&MapRecord>> = HashMap::new();
        for (map, npcs) in &self.npcs_on {
            let Some(record) = self.maps.get(map) else { continue };
            for npc in npcs {
                maps_of.entry(*npc).or_default().push(record);
            }
        }
        for maps in maps_of.values_mut() {
            maps.sort_by_key(|map| map.id);
        }
        let row = |id: u32, name: &str, maps: &[&MapRecord], note| NpcRow {
            id,
            name: name.to_string(),
            map: maps.first().map(|map| map.name.clone()),
            maps: maps.len(),
            quests: self.quests_from.get(&id).map_or(0, Vec::len),
            note,
        };
        let admits = |id: u32, maps: &[&MapRecord]| {
            (query.kind.as_deref() != Some("quests") || self.quests_from.contains_key(&id))
                && query.region.as_ref().is_none_or(|region| maps.iter().any(|map| map.region.as_ref() == Some(region)))
                && query.area.as_ref().is_none_or(|area| maps.iter().any(|map| area_of(map).as_ref() == Some(area)))
                && (!words.is_empty() || !maps.is_empty())
        };
        let candidates: Vec<(u32, &str, &[&MapRecord])> = self
            .npc_names
            .iter()
            .map(|(id, name)| (*id, name.as_str(), maps_of.get(id).map_or(&[][..], Vec::as_slice)))
            .filter(|(id, _, maps)| admits(*id, maps))
            .collect();
        let mut found: Vec<(u8, NpcRow)> = candidates
            .iter()
            .filter_map(|(id, name, maps)| {
                let (rank, note) = self.npc_rank(*id, name, maps, &words)?;
                Some((rank, row(*id, name, maps, note)))
            })
            .collect();
        let closest = found.is_empty() && !words.is_empty();
        if closest {
            let text = words.join(" ");
            let mut near: Vec<(f64, NpcRow)> = candidates
                .iter()
                .map(|(id, name, maps)| (name_score(&text, name), row(*id, name, maps, None)))
                .filter(|(score, _)| *score >= MIN_NEAR_NAME)
                .collect();
            near.sort_by(|a, b| b.0.total_cmp(&a.0));
            found = near.into_iter().map(|(_, row)| (0, row)).collect();
        } else {
            found.sort_by_key(|(rank, row)| (*rank, fold(&row.name), row.id));
        }
        NpcPage {
            total: found.len(),
            npcs: found.into_iter().skip(query.offset).take(PAGE).map(|(_, row)| row).collect(),
            closest,
        }
    }

    /// The NPC a name means: the one standing on a map, when several share it.
    pub fn npc_like(&self, name: &str) -> Option<u32> {
        let key = fold(name);
        let mut named: Vec<u32> = self.npc_names.iter().filter(|(_, n)| fold(n) == key).map(|(id, _)| *id).collect();
        named.sort_unstable();
        named.iter().copied().find(|id| !self.npc_maps(*id).is_empty()).or(named.first().copied())
    }

    /// All the data says about an NPC. `from` is the map the player is on, for portal counts.
    pub fn npc_sheet(&self, id: u32, from: Option<u32>, travel: &Travel) -> Option<NpcSheet> {
        let name = self.npc_names.get(&id)?.clone();
        let context = self.context(from, &[], travel);
        let mut maps: Vec<NpcPlace> = self
            .npc_maps(id)
            .into_iter()
            .map(|map| NpcPlace {
                hops: context.hops.get(&map.id).copied(),
                travel: context.shortcuts.get(&map.id).cloned(),
                map,
            })
            .collect();
        maps.sort_by_key(|place| (place.hops.is_none(), place.hops, place.map.name.clone()));
        let mut quests: Vec<QuestRow> =
            self.quests_from.get(&id).into_iter().flatten().filter_map(|q| Some(self.quest_row(self.quests.get(q)?))).collect();
        quests.sort_by_key(|q| (q.level_min, q.name.clone()));
        let mut takes: Vec<QuestRef> = self
            .quests
            .values()
            .filter(|q| q.npc_id != Some(id) && self.turn_in(q) == Some(id))
            .filter_map(|q| self.quest_ref(q.id))
            .collect();
        takes.sort_by_key(|q| (q.level_min, q.name.clone()));
        Some(NpcSheet { id, name, maps, quests, takes })
    }
}

#[cfg(test)]
mod tests {
    use super::super::atlas::tests::world;
    use super::*;

    fn maps(query: MapQuery) -> Vec<String> {
        world().browse_maps(&query).maps.into_iter().map(|m| m.map.name).collect()
    }

    fn named(text: &str) -> MapQuery {
        MapQuery { text: text.into(), ..MapQuery::default() }
    }

    fn npcs(text: &str) -> Vec<(String, Option<String>)> {
        let query = NpcQuery { text: text.into(), ..NpcQuery::default() };
        world().search_npcs(&query).npcs.into_iter().map(|n| (n.name, n.note)).collect()
    }

    #[test]
    fn finds_maps_by_name_street_monster_and_npc() {
        assert_eq!(maps(named("burrow")), ["Burrow Den"]);
        assert_eq!(maps(named("old fern")), ["Mossbank"], "the street");
        let by_monster = world().browse_maps(&named("slime"));
        assert_eq!(by_monster.maps[0].map.name, "Mossbank Meadow I");
        assert_eq!(by_monster.maps[0].note.as_deref(), Some("Has Moss Slime"));
        let by_npc = world().browse_maps(&named("ilse"));
        assert_eq!(by_npc.maps[0].note.as_deref(), Some("Warden Ilse is here"));
    }

    #[test]
    fn a_typo_finds_the_closest_maps_and_says_so() {
        let page = world().browse_maps(&named("burow"));
        assert!(page.closest);
        assert_eq!(page.maps[0].map.name, "Burrow Den");
    }

    #[test]
    fn maps_narrow_by_kind_and_level() {
        let kind = |kind: &str| MapQuery { kind: Some(kind.into()), ..MapQuery::default() };
        assert_eq!(maps(kind("town")), ["Mossbank"]);
        assert_eq!(maps(kind("field")).len(), 5);
        let levels = MapQuery { level_min: Some(6), level_max: Some(9), ..MapQuery::default() };
        assert_eq!(maps(levels), ["Burrow Den"], "towns and maps below that have nothing in range");
        let reaching = MapQuery { level_min: Some(5), level_max: Some(5), ..MapQuery::default() };
        assert_eq!(maps(reaching), ["Mossbank Meadow I"], "a range that takes in the level counts");
    }

    #[test]
    fn maps_sort_by_name_or_level() {
        let sorted = MapQuery { sort: Some("level".into()), ..MapQuery::default() };
        let first = maps(sorted);
        assert_eq!(&first[..3], ["Mossbank Meadow II", "Mossbank Meadow I", "Burrow Den"], "Lv 2, then Lv 2 to 5, then Lv 8");
        assert_eq!(maps(MapQuery::default())[0], "Burrow Den");
    }

    #[test]
    fn the_makers_own_maps_only_come_up_by_name() {
        let mut den = crate::mapdata::atlas::tests::map(9, "Test Field", "Fern Road", vec![]);
        den.region = Some(HIDDEN_REGION.into());
        let atlas = Atlas::new(crate::mapdata::osms::Dataset { maps: vec![den], ..Default::default() });
        assert_eq!(atlas.browse_maps(&MapQuery::default()).total, 0);
        assert_eq!(atlas.browse_maps(&named("test")).total, 1);
        assert!(atlas.map_regions().is_empty());
    }

    #[test]
    fn maps_are_grouped_into_areas_by_their_mark() {
        use crate::mapdata::atlas::tests::map;
        let marked = |id, mark: &str| MapRecord { map_mark: Some(mark.into()), ..map(id, &format!("Map {id}"), "Fern Road", vec![]) };
        let mut maps = vec![];
        for (id, mark) in (1..).zip(["KerningCity", "KerningCity", "KerningCity", "Rith", "Rith", "Rith", "Amherst", "SouthPerry"]) {
            maps.push(marked(id, mark));
        }
        maps.push(marked(20, "MushroomVillage"));
        maps.extend((30..34).map(|id| marked(id, "Event")));
        maps.extend((40..44).map(|id| marked(id, "ElNath")));
        maps.push(marked(50, "ForgottenHollow"));
        let mut hidden = marked(60, "Henesys");
        hidden.region = Some(HIDDEN_REGION.into());
        maps.push(hidden);
        let atlas = Atlas::new(crate::mapdata::osms::Dataset { maps, ..Default::default() });
        assert_eq!(atlas.map_areas(), ["Maple Island", "Lith Harbor", "Kerning City", "El Nath"], "strays and events left out");
        let area = |area: &str| MapQuery { area: Some(area.into()), ..MapQuery::default() };
        assert_eq!(atlas.browse_maps(&area("Maple Island")).total, 3);
        assert_eq!(atlas.browse_maps(&area("Henesys")).total, 0, "the makers' maps aren't in any area");
    }

    #[test]
    fn finds_npcs_by_name_quest_and_map() {
        assert_eq!(npcs("ilse"), [("Warden Ilse".to_string(), None)]);
        assert_eq!(npcs("shell"), [("Warden Ilse".to_string(), Some("Gives Shell Sorting".to_string()))]);
        assert_eq!(npcs("inn"), [("Innkeeper Bo".to_string(), None)]);
        assert_eq!(npcs("mossbank inn")[0].1.as_deref(), Some("On Mossbank Inn"));
    }

    #[test]
    fn narrows_npcs_to_quest_givers_and_a_region() {
        let atlas = world();
        let givers = NpcQuery { kind: Some("quests".into()), ..NpcQuery::default() };
        assert_eq!(atlas.search_npcs(&givers).total, 2);
        let elsewhere = NpcQuery { region: Some("Nowhere".into()), ..NpcQuery::default() };
        assert_eq!(atlas.search_npcs(&elsewhere).total, 0);
        let row = &atlas.search_npcs(&NpcQuery::default()).npcs[0];
        assert_eq!((row.name.as_str(), row.map.as_deref(), row.maps, row.quests), ("Innkeeper Bo", Some("Mossbank Inn"), 1, 1));
    }

    #[test]
    fn a_name_means_the_npc_standing_on_a_map() {
        let atlas = world();
        assert_eq!(atlas.npc_like("warden ilse"), Some(50));
        assert_eq!(atlas.npc_like("Nobody"), None);
    }

    #[test]
    fn a_sheet_says_where_they_stand_and_what_they_give() {
        let sheet = world().npc_sheet(50, Some(2), &Travel::none()).unwrap();
        assert_eq!(sheet.name, "Warden Ilse");
        assert_eq!(sheet.maps.iter().map(|m| (m.map.name.as_str(), m.hops)).collect::<Vec<_>>(), [("Mossbank", Some(1))]);
        assert_eq!(sheet.quests.iter().map(|q| q.name.as_str()).collect::<Vec<_>>(), ["Shell Sorting"]);
        assert!(sheet.takes.is_empty());
        assert!(world().npc_sheet(999, None, &Travel::none()).is_none());
    }
}
