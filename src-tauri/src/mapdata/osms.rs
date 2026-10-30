//! The parts of OSMS Data Explorer's v1 JSON that the map card uses.
//!
//! Only the fields listed here are read. Every `*_url` image field and every map coordinate
//! is left out on purpose, so neither can reach the cache or the screen.

use std::collections::HashMap;

use serde::Deserialize;

/// The files the map card needs, in the order they are fetched.
pub const FILES: [&str; 4] = ["maps.json", "monsters.json", "npcs.json", "quests.json"];

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

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MapRecord {
    pub id: u32,
    pub name: String,
    pub street_name: Option<String>,
    pub region: Option<String>,
    #[serde(default)]
    pub is_town: bool,
    pub return_map_id: Option<u32>,
    pub map_mark: Option<String>,
    /// Maps reachable from this one.
    #[serde(default)]
    pub exits: Vec<u32>,
    #[serde(default)]
    pub portals: Vec<Portal>,
}

/// A portal's name and where it leads. Its position on the map is not read.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Portal {
    pub name: String,
    pub dest_map_id: Option<u32>,
    pub dest_portal: Option<String>,
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

/// Everything the card is built from.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Dataset {
    pub maps: Vec<MapRecord>,
    pub monsters: Vec<MonsterRecord>,
    pub npcs: Vec<NpcRecord>,
    pub quests: Vec<QuestRecord>,
}

/// One downloaded file, parsed.
#[derive(Debug, Clone, PartialEq)]
pub enum Records {
    Maps(Vec<MapRecord>),
    Monsters(Vec<MonsterRecord>),
    Npcs(Vec<NpcRecord>),
    Quests(Vec<QuestRecord>),
}

impl Records {
    /// Parses the file called `name`, one of [`FILES`].
    pub fn parse(name: &str, body: &[u8]) -> serde_json::Result<Self> {
        Ok(match name {
            "maps.json" => Self::Maps(serde_json::from_slice(body)?),
            "monsters.json" => Self::Monsters(serde_json::from_slice(body)?),
            "npcs.json" => Self::Npcs(serde_json::from_slice(body)?),
            "quests.json" => Self::Quests(serde_json::from_slice(body)?),
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

    #[test]
    fn keeps_only_mob_requirements() {
        let json = br#"[{"id": 1, "name": "Pebble Patrol", "npc_id": 3, "requirements": [
            {"type": "mob", "id": 11, "count": 20}, {"type": "item", "id": 400, "count": 5},
            {"type": "quest", "id": 9, "state": 2}]}]"#;
        let Records::Quests(quests) = Records::parse("quests.json", json).unwrap() else { panic!() };
        assert_eq!(quests[0].requirements, vec![MobRequirement { mob_id: 11, count: 20 }]);
    }
}
