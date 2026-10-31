//! The parts of OSMS Data Explorer's v1 JSON that the map card and build planner use.
//!
//! Only the fields listed here are read. Every `*_url` image field is left out on purpose, so
//! none can reach the cache or the screen. Coordinates are read only to tell which third of its
//! map a portal stands in, and are dropped as the file is parsed: only that coarse place is
//! kept, and nothing is ever drawn at a map's real positions.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// The files Leafline needs, in the order they are fetched.
pub const FILES: [&str; 6] = ["maps.json", "monsters.json", "npcs.json", "quests.json", "skills.json", "items.json"];

/// The only `schema_version` this reader understands. OSMS ships breaking changes as `v2/`.
pub const SCHEMA_VERSION: u32 = 1;

/// A portal destination that means "no destination".
pub const NO_MAP: u32 = 999_999_999;

/// `meta.json`: what the dataset was built from, and a checksum for each file.
#[derive(Debug, Clone, Deserialize)]
pub struct Meta {
    pub schema_version: u32,
    pub client_build: String,
    pub data_updated_at: String,
    pub files: HashMap<String, FileMeta>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FileMeta {
    pub sha256: String,
    pub records: u32,
    pub updated_at: String,
}

/// Less of a spread than this across a map, in the game's pixels, says nothing about where
/// things stand in it.
const MIN_SPREAD: i32 = 150;

/// The third of its map a portal stands in, across.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Left,
    Centre,
    Right,
}

impl Side {
    /// In order across the map, so a third's number is its place here.
    pub const ALL: [Self; 3] = [Self::Left, Self::Centre, Self::Right];
}

/// The third of its map a portal stands in, down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Height {
    High,
    Middle,
    Low,
}

impl Height {
    /// In order down the map, so a third's number is its place here.
    pub const ALL: [Self; 3] = [Self::High, Self::Middle, Self::Low];
}

/// The file's fields, coordinates included, until [`MapRecord`] takes what it keeps.
#[derive(Deserialize)]
struct RawMap {
    id: u32,
    name: String,
    street_name: Option<String>,
    region: Option<String>,
    #[serde(default)]
    is_town: bool,
    return_map_id: Option<u32>,
    map_mark: Option<String>,
    #[serde(default)]
    exits: Vec<u32>,
    #[serde(default)]
    portals: Vec<RawPortal>,
    #[serde(default)]
    mobs: Vec<Point>,
    #[serde(default)]
    npcs: Vec<Point>,
}

#[derive(Deserialize)]
struct RawPortal {
    name: String,
    dest_map_id: Option<u32>,
    dest_portal: Option<String>,
    x: Option<i32>,
    y: Option<i32>,
}

#[derive(Deserialize)]
struct Point {
    x: Option<i32>,
    y: Option<i32>,
}

/// The lowest and highest of `values`, if they spread far enough to mean anything.
fn spread(values: impl Iterator<Item = i32>) -> Option<(i32, i32)> {
    let values: Vec<i32> = values.collect();
    let (low, high) = (*values.iter().min()?, *values.iter().max()?);
    (high - low >= MIN_SPREAD).then_some((low, high))
}

/// Which third of `range` `value` falls in: 0, 1 or 2.
fn third(value: i32, (low, high): (i32, i32)) -> usize {
    (i64::from(value - low) * 3 / i64::from(high - low + 1)) as usize
}

impl From<RawMap> for MapRecord {
    /// Everything on the map marks how far it reaches, so a lone portal still has a place.
    fn from(raw: RawMap) -> Self {
        let places = || {
            let portals = raw.portals.iter().map(|p| (p.x, p.y));
            portals.chain(raw.mobs.iter().chain(&raw.npcs).map(|p| (p.x, p.y)))
        };
        let across = spread(places().filter_map(|(x, _)| x));
        let down = spread(places().filter_map(|(_, y)| y));
        let portals = raw
            .portals
            .iter()
            .map(|p| Portal {
                name: p.name.clone(),
                dest_map_id: p.dest_map_id,
                dest_portal: p.dest_portal.clone(),
                side: p.x.zip(across).map(|(x, range)| Side::ALL[third(x, range)]),
                height: p.y.zip(down).map(|(y, range)| Height::ALL[third(y, range)]),
            })
            .collect();
        Self {
            id: raw.id,
            name: raw.name,
            street_name: raw.street_name,
            region: raw.region,
            is_town: raw.is_town,
            return_map_id: raw.return_map_id,
            map_mark: raw.map_mark,
            exits: raw.exits,
            portals,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(from = "RawMap")]
pub struct MapRecord {
    pub id: u32,
    pub name: String,
    pub street_name: Option<String>,
    pub region: Option<String>,
    pub is_town: bool,
    pub return_map_id: Option<u32>,
    pub map_mark: Option<String>,
    /// Maps reachable from this one.
    pub exits: Vec<u32>,
    pub portals: Vec<Portal>,
}

/// A portal's name, where it leads, and roughly where it stands: which third of the map across
/// and down, when the map's data can tell.
#[derive(Debug, Clone, PartialEq)]
pub struct Portal {
    pub name: String,
    pub dest_map_id: Option<u32>,
    pub dest_portal: Option<String>,
    pub side: Option<Side>,
    pub height: Option<Height>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MonsterRecord {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub level: u32,
    #[serde(default)]
    pub hp: u32,
    #[serde(default)]
    pub exp: u32,
    #[serde(default)]
    pub is_boss: bool,
    #[serde(default)]
    pub undead: bool,
    #[serde(default)]
    pub spawns: Vec<Spawn>,
}

/// How many of a monster a map spawns.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Spawn {
    pub map_id: u32,
    #[serde(default)]
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct NpcRecord {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub maps: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct QuestRecord {
    pub id: u32,
    pub name: String,
    /// The NPC who gives the quest.
    pub npc_id: Option<u32>,
    pub level_min: Option<u32>,
    /// The quest this one leads to.
    pub next_quest_id: Option<u32>,
    /// Only the monsters to hunt are kept; other requirement types are skipped.
    #[serde(default, deserialize_with = "mob_requirements")]
    pub requirements: Vec<MobRequirement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MobRequirement {
    pub mob_id: u32,
    pub count: u32,
}

fn mob_requirements<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<MobRequirement>, D::Error> {
    #[derive(Deserialize)]
    struct Requirement {
        #[serde(rename = "type")]
        kind: String,
        id: Option<u32>,
        count: Option<u32>,
    }
    let all = Vec::<Requirement>::deserialize(deserializer)?;
    Ok(all
        .into_iter()
        .filter(|r| r.kind == "mob")
        .filter_map(|r| Some(MobRequirement { mob_id: r.id?, count: r.count.unwrap_or(1) }))
        .collect())
}

/// The tier of the crafting disciplines, which share the file but use no skill points.
const CRAFTING: &str = "Crafting";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct SkillRecord {
    pub id: u32,
    pub name: String,
    pub job_id: u32,
    #[serde(default)]
    pub class_name: String,
    pub job_tier: String,
    #[serde(default)]
    pub max_level: u32,
    #[serde(default)]
    pub passive: bool,
    pub description: Option<String>,
    #[serde(default)]
    pub required_skills: Vec<SkillNeed>,
    /// What the skill does at each level, from level 1.
    #[serde(default)]
    pub level_descriptions: Vec<String>,
}

/// A skill that must reach `level` first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct SkillNeed {
    pub skill_id: u32,
    pub level: u32,
}

/// The file's equipment stats and the name each goes by on screen, in the order shown.
const ITEM_STATS: [(&str, &str); 16] = [
    ("incPAD", "Attack"),
    ("incMAD", "Magic"),
    ("incPDD", "Weapon Def."),
    ("incMDD", "Magic Def."),
    ("incSTR", "STR"),
    ("incDEX", "DEX"),
    ("incINT", "INT"),
    ("incLUK", "LUK"),
    ("incMHP", "MaxHP"),
    ("incMMP", "MaxMP"),
    ("incACC", "Accuracy"),
    ("incEVA", "Evasion"),
    ("incCRT", "Crit. Rate"),
    ("incCRD", "Crit. Damage"),
    ("incSpeed", "Speed"),
    ("incJump", "Jump"),
];

#[derive(Deserialize)]
struct RawItem {
    id: u32,
    name: String,
    category: String,
    sub_category: Option<String>,
    gender: Option<String>,
    #[serde(default)]
    req_classes: Vec<String>,
    #[serde(default)]
    stats: HashMap<String, serde_json::Value>,
    weapon: Option<RawWeapon>,
}

#[derive(Deserialize)]
struct RawWeapon {
    #[serde(rename = "type")]
    kind: Option<String>,
}

impl From<RawItem> for ItemRecord {
    fn from(raw: RawItem) -> Self {
        // A few values are written as text.
        let number = |key: &str| {
            let value = raw.stats.get(key)?;
            value.as_i64().or_else(|| value.as_str()?.trim().parse().ok())
        };
        let need = |key: &str| number(key).map_or(0, |v| v.max(0) as u32);
        let stats = ITEM_STATS
            .iter()
            .filter_map(|(key, label)| Some(ItemStat { label: (*label).to_string(), value: number(key)? }))
            .collect();
        Self {
            id: raw.id,
            slot: raw.sub_category.clone().unwrap_or_default(),
            gender: raw.gender.clone(),
            req_level: need("reqLevel"),
            req_str: need("reqSTR"),
            req_dex: need("reqDEX"),
            req_int: need("reqINT"),
            req_luk: need("reqLUK"),
            weapon_type: raw.weapon.as_ref().and_then(|w| w.kind.clone()),
            stats,
            classes: raw.req_classes,
            category: raw.category,
            name: raw.name,
        }
    }
}

/// A piece of equipment. Other items in the file are dropped as it is parsed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "RawItem", rename_all(serialize = "camelCase"))]
pub struct ItemRecord {
    pub id: u32,
    pub name: String,
    #[serde(skip_serializing)]
    pub category: String,
    pub slot: String,
    pub gender: Option<String>,
    /// Who can use it, as the tooltip's job flags name them. Empty means anyone.
    pub classes: Vec<String>,
    pub weapon_type: Option<String>,
    pub req_level: u32,
    pub req_str: u32,
    pub req_dex: u32,
    pub req_int: u32,
    pub req_luk: u32,
    pub stats: Vec<ItemStat>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemStat {
    pub label: String,
    pub value: i64,
}

/// Everything the map card and build planner are built from.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Dataset {
    pub maps: Vec<MapRecord>,
    pub monsters: Vec<MonsterRecord>,
    pub npcs: Vec<NpcRecord>,
    pub quests: Vec<QuestRecord>,
    pub skills: Vec<SkillRecord>,
    pub items: Vec<ItemRecord>,
}

/// One downloaded file, parsed.
#[derive(Debug, Clone, PartialEq)]
pub enum Records {
    Maps(Vec<MapRecord>),
    Monsters(Vec<MonsterRecord>),
    Npcs(Vec<NpcRecord>),
    Quests(Vec<QuestRecord>),
    Skills(Vec<SkillRecord>),
    Items(Vec<ItemRecord>),
}

impl Records {
    /// Parses the file called `name`, one of [`FILES`].
    pub fn parse(name: &str, body: &[u8]) -> serde_json::Result<Self> {
        Ok(match name {
            "maps.json" => Self::Maps(serde_json::from_slice(body)?),
            "monsters.json" => Self::Monsters(serde_json::from_slice(body)?),
            "npcs.json" => Self::Npcs(serde_json::from_slice(body)?),
            "quests.json" => Self::Quests(serde_json::from_slice(body)?),
            "skills.json" => {
                let mut skills: Vec<SkillRecord> = serde_json::from_slice(body)?;
                skills.retain(|s| s.job_tier != CRAFTING);
                Self::Skills(skills)
            }
            "items.json" => {
                let mut items: Vec<ItemRecord> = serde_json::from_slice(body)?;
                items.retain(|i| i.category == "Equipment");
                Self::Items(items)
            }
            other => unreachable!("not a map data file: {other}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_only_what_the_card_uses() {
        // Made-up records in OSMS's shape, image URLs and coordinates included.
        let json = br#"[{"id": 7, "name": "Fernway Path", "street_name": "Mossy Road", "is_town": false,
            "exits": [8], "portals": [{"name": "east00", "type": 2, "x": 10, "y": 20, "dest_map_id": 8,
            "dest_portal": "west00"}, {"name": "sp", "type": 0, "x": 1, "y": 2}],
            "mobs": [{"mob_id": 1, "x": 5, "y": 6}], "minimap_url": "https://example.invalid/m.png"}]"#;
        let Records::Maps(maps) = Records::parse("maps.json", json).unwrap() else { panic!() };
        assert_eq!(maps[0].portals.len(), 2);
        assert_eq!(maps[0].portals[0].dest_map_id, Some(8));
        assert_eq!(maps[0].portals[1].dest_map_id, None);
        assert_eq!(maps[0].street_name.as_deref(), Some("Mossy Road"));
    }

    fn portals_of(json: &[u8]) -> Vec<Portal> {
        let Records::Maps(maps) = Records::parse("maps.json", json).unwrap() else { panic!() };
        maps.into_iter().next().unwrap().portals
    }

    #[test]
    fn places_portals_in_coarse_thirds() {
        // Made-up map, 900 wide and 600 tall by what stands on it.
        let portals = portals_of(
            br#"[{"id": 1, "name": "Fernway", "portals": [
            {"name": "a", "x": 10, "y": 300, "dest_map_id": 2},
            {"name": "b", "x": 450, "y": 10, "dest_map_id": 3},
            {"name": "c", "x": 890, "y": 590, "dest_map_id": 4},
            {"name": "d", "x": 890, "y": 300, "dest_map_id": 5}],
            "mobs": [{"mob_id": 1, "x": 0, "y": 0}], "npcs": [{"npc_id": 1, "x": 900, "y": 600}]}]"#,
        );
        let places: Vec<_> = portals.iter().map(|p| (p.side, p.height)).collect();
        assert_eq!(
            places,
            [
                (Some(Side::Left), Some(Height::Middle)),
                (Some(Side::Centre), Some(Height::High)),
                (Some(Side::Right), Some(Height::Low)),
                (Some(Side::Right), Some(Height::Middle)),
            ]
        );
    }

    #[test]
    fn the_far_edges_of_a_map_are_in_its_outer_thirds() {
        let portals = portals_of(
            br#"[{"id": 1, "name": "Fernway", "portals": [
            {"name": "a", "x": 0, "y": 0}, {"name": "b", "x": 300, "y": 300}]}]"#,
        );
        assert_eq!((portals[0].side, portals[0].height), (Some(Side::Left), Some(Height::High)));
        assert_eq!((portals[1].side, portals[1].height), (Some(Side::Right), Some(Height::Low)));
    }

    #[test]
    fn a_map_with_too_little_on_it_places_nothing() {
        // A lone portal, and a flat map: nothing to measure against, across or down.
        let lone = portals_of(br#"[{"id": 1, "name": "Fernway", "portals": [{"name": "a", "x": 5, "y": 5}]}]"#);
        assert_eq!((lone[0].side, lone[0].height), (None, None));
        let flat = portals_of(
            br#"[{"id": 1, "name": "Fernway", "portals": [
            {"name": "a", "x": 0, "y": 50}, {"name": "b", "x": 900, "y": 60}]}]"#,
        );
        assert_eq!((flat[0].side, flat[0].height), (Some(Side::Left), None));
        let unplaced = portals_of(br#"[{"id": 1, "name": "Fernway", "portals": [{"name": "a"}]}]"#);
        assert_eq!((unplaced[0].side, unplaced[0].height), (None, None));
    }

    #[test]
    fn reads_skills_but_not_crafting_or_icons() {
        // Made-up skills in OSMS's shape. Crafting records carry no levels at all.
        let json = br#"[
            {"id": 7, "name": "Pebble Toss", "job_id": 400, "class_name": "Rogue", "job_tier": "1st Job",
             "max_level": 2, "passive": false, "description": "Throws a pebble.", "icon_url": "https://example.invalid/i.png",
             "required_skills": [{"skill_id": 6, "level": 3}], "level_descriptions": ["Damage 1", "Damage 2"]},
            {"id": 92000000, "name": "Knitting", "job_id": 9200, "job_tier": "Crafting", "max_level": 10}]"#;
        let Records::Skills(skills) = Records::parse("skills.json", json).unwrap() else { panic!() };
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].required_skills, vec![SkillNeed { skill_id: 6, level: 3 }]);
        assert_eq!(skills[0].level_descriptions.len(), 2);
    }

    #[test]
    fn keeps_only_equipment_and_reads_numbers_written_as_text() {
        let json = br#"[
            {"id": 1, "name": "Twig Wand", "category": "Equipment", "sub_category": "Weapon", "price": "5",
             "req_classes": ["Magician"], "weapon": {"type": "Wand", "swing_reach_px": 88},
             "stats": {"reqLevel": 8, "reqINT": 20, "incMAD": 12, "incPDD": "3", "tuc": 7},
             "icon_url": "https://example.invalid/w.png"},
            {"id": 2, "name": "Plain Cap", "category": "Equipment", "sub_category": "Hat", "gender": "male"},
            {"id": 3, "name": "Red Berry", "category": "Consumable", "sub_category": "Consume"}]"#;
        let Records::Items(items) = Records::parse("items.json", json).unwrap() else { panic!() };
        assert_eq!(items.len(), 2);
        let wand = &items[0];
        assert_eq!((wand.req_level, wand.req_int, wand.req_str), (8, 20, 0));
        assert_eq!(wand.weapon_type.as_deref(), Some("Wand"));
        assert_eq!(wand.classes, ["Magician"]);
        let stats: Vec<_> = wand.stats.iter().map(|s| (s.label.as_str(), s.value)).collect();
        assert_eq!(stats, [("Magic", 12), ("Weapon Def.", 3)]);
        assert!(items[1].classes.is_empty() && items[1].req_level == 0);
    }

    #[test]
    fn keeps_only_mob_requirements() {
        let json = br#"[{"id": 1, "name": "Pebble Patrol", "npc_id": 3, "requirements": [
            {"type": "mob", "id": 11, "count": 20}, {"type": "item", "id": 400, "count": 5},
            {"type": "quest", "id": 9, "state": 2}]}]"#;
        let Records::Quests(quests) = Records::parse("quests.json", json).unwrap() else { panic!() };
        assert_eq!(quests[0].requirements, vec![MobRequirement { mob_id: 11, count: 20 }]);
    }
}
