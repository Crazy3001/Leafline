//! The local SQLite cache of downloaded map data.
//!
//! Each OSMS file is kept with the checksum it was downloaded at, so a later check can skip
//! files that have not changed. The wiki's drop table is kept whole, with the time it was
//! fetched.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;

use super::drops::{Drop, Dropper, MonsterDrops};
use super::osms::{
    Dataset, Effects, FILES, FileMeta, Height, Ingredient, ItemKind, ItemNeed, ItemRecord, MapPoint, MapRecord, Meta, MobRequirement,
    MonsterDetails, MonsterRecord, NpcRecord, PointKind, Portal, QuestRecord, Records, RecipeRecord, RewardItem, Side, SkillNeed,
    SkillRecord, Spawn,
};
use super::layout::{Layouts, MapLayout};
use super::turnins::TurnIn;
use crate::error::{Error, Result};

/// Bump when the tables change; an older cache is then dropped and downloaded again.
const CACHE_VERSION: i32 = 15;

const SCHEMA: &str = "
CREATE TABLE source_files (
    name TEXT PRIMARY KEY, sha256 TEXT NOT NULL, records INTEGER NOT NULL,
    updated_at TEXT NOT NULL, fetched_at INTEGER NOT NULL);
CREATE TABLE source_info (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE maps (
    id INTEGER PRIMARY KEY, name TEXT NOT NULL, street TEXT, region TEXT,
    is_town INTEGER NOT NULL, return_map_id INTEGER, map_mark TEXT, minimap_url TEXT, render_url TEXT, map_mark_url TEXT);
CREATE TABLE map_exits (map_id INTEGER NOT NULL, dest_map_id INTEGER NOT NULL, PRIMARY KEY (map_id, dest_map_id));
CREATE TABLE map_points (
    map_id INTEGER NOT NULL, kind INTEGER NOT NULL, ref_id INTEGER NOT NULL, name TEXT, to_portal TEXT,
    x INTEGER NOT NULL, y INTEGER NOT NULL);
CREATE INDEX map_points_by_map ON map_points (map_id);
CREATE TABLE portals (
    map_id INTEGER NOT NULL, seq INTEGER NOT NULL, name TEXT NOT NULL,
    dest_map_id INTEGER, dest_portal TEXT, side INTEGER, height INTEGER, PRIMARY KEY (map_id, seq));
CREATE TABLE monsters (
    id INTEGER PRIMARY KEY, name TEXT NOT NULL, level INTEGER NOT NULL, hp INTEGER NOT NULL,
    exp INTEGER NOT NULL, is_boss INTEGER NOT NULL, undead INTEGER NOT NULL, details TEXT NOT NULL, icon_url TEXT);
CREATE TABLE spawns (
    mob_id INTEGER NOT NULL, map_id INTEGER NOT NULL, count INTEGER NOT NULL, respawn INTEGER, PRIMARY KEY (mob_id, map_id));
CREATE TABLE npcs (id INTEGER PRIMARY KEY, name TEXT NOT NULL, icon_url TEXT);
CREATE TABLE npc_maps (npc_id INTEGER NOT NULL, map_id INTEGER NOT NULL, PRIMARY KEY (npc_id, map_id));
CREATE TABLE quests (
    id INTEGER PRIMARY KEY, name TEXT NOT NULL, npc_id INTEGER, level_min INTEGER, next_quest_id INTEGER, region TEXT,
    description TEXT, chain TEXT, repeat TEXT, reward_exp INTEGER NOT NULL, reward_mesos INTEGER NOT NULL);
CREATE TABLE quest_items (quest_id INTEGER NOT NULL, item_id INTEGER NOT NULL, count INTEGER NOT NULL);
CREATE TABLE quest_follows (quest_id INTEGER NOT NULL, needs_id INTEGER NOT NULL);
CREATE TABLE quest_mobs (quest_id INTEGER NOT NULL, mob_id INTEGER NOT NULL, count INTEGER NOT NULL);
CREATE TABLE quest_rewards (quest_id INTEGER NOT NULL, item_id INTEGER NOT NULL, count INTEGER NOT NULL, sure INTEGER NOT NULL);
CREATE TABLE skills (
    id INTEGER PRIMARY KEY, name TEXT NOT NULL, job_id INTEGER NOT NULL, class_name TEXT NOT NULL,
    job_tier TEXT NOT NULL, max_level INTEGER NOT NULL, passive INTEGER NOT NULL, description TEXT, icon_url TEXT,
    buff INTEGER NOT NULL, effects TEXT NOT NULL);
CREATE TABLE skill_needs (skill_id INTEGER NOT NULL, needs_id INTEGER NOT NULL, level INTEGER NOT NULL);
CREATE TABLE skill_levels (skill_id INTEGER NOT NULL, level INTEGER NOT NULL, description TEXT NOT NULL, PRIMARY KEY (skill_id, level));
CREATE TABLE items (
    id INTEGER PRIMARY KEY, name TEXT NOT NULL, kind TEXT NOT NULL, slot TEXT NOT NULL, gender TEXT, classes TEXT NOT NULL,
    weapon_type TEXT, req_level INTEGER NOT NULL, req_str INTEGER NOT NULL, req_dex INTEGER NOT NULL,
    req_int INTEGER NOT NULL, req_luk INTEGER NOT NULL, req_fame INTEGER NOT NULL, attack_speed INTEGER,
    knockback INTEGER, upgrade_slots INTEGER, quest_item INTEGER NOT NULL, untradable INTEGER NOT NULL, stats TEXT NOT NULL,
    price INTEGER, description TEXT, scroll TEXT, icon_url TEXT);
CREATE TABLE recipes (
    id INTEGER PRIMARY KEY, discipline TEXT NOT NULL, skill_level INTEGER NOT NULL, craft_exp INTEGER NOT NULL,
    meso_cost INTEGER NOT NULL, output_item_id INTEGER NOT NULL, output_count INTEGER NOT NULL);
CREATE TABLE recipe_ingredients (recipe_id INTEGER NOT NULL, seq INTEGER NOT NULL, item_id INTEGER NOT NULL,
    count INTEGER NOT NULL, PRIMARY KEY (recipe_id, seq));
CREATE TABLE drops (
    monster TEXT NOT NULL, item TEXT NOT NULL, category TEXT, rate REAL, rate_low REAL, rate_high REAL,
    min_quantity INTEGER, max_quantity INTEGER, sample_kills INTEGER, sample_drops INTEGER,
    game_version TEXT, quest INTEGER NOT NULL);
CREATE INDEX drops_by_monster ON drops (monster);
CREATE TABLE drop_fetches (monster TEXT PRIMARY KEY, fetched_at INTEGER NOT NULL, sample_kills INTEGER, game_version TEXT);
";

/// Tables added since the last `CACHE_VERSION`, which hold nothing the player would have to
/// download again.
const ADDED: &str = "
CREATE TABLE IF NOT EXISTS quest_turn_in_npcs (quest_id INTEGER PRIMARY KEY, npc_id INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS map_layouts (
    map_id INTEGER PRIMARY KEY, width INTEGER, height INTEGER, walkable INTEGER NOT NULL,
    platforms INTEGER NOT NULL, spawn_points INTEGER NOT NULL, spawn_platforms INTEGER NOT NULL,
    ropes INTEGER NOT NULL, ladders INTEGER NOT NULL
);
";

/// Tables from when items were asked about one at a time, which the whole drop table replaced,
/// and the wiki's turn-in names, which NPC ids from Leafline's own quest file replaced
/// (2026-11-11). Removed from an existing cache without forcing a new download.
const RETIRED: &str = "
DROP TABLE IF EXISTS item_lookups;
DROP TABLE IF EXISTS item_droppers;
DROP TABLE IF EXISTS quest_turn_ins;
DELETE FROM source_info WHERE key IN ('turn_ins_fetched_at', 'turn_ins_failed');
";

/// Where the time the drop table was downloaded is kept in `source_info`.
const DROPS_FETCHED_AT: &str = "drops_fetched_at";
/// Leafline's shop file, kept whole as downloaded, and when.
const SHOP_FILE: &str = "shop_file";
const SHOPS_FETCHED_AT: &str = "shops_fetched_at";
/// Likewise for Leafline's file of who takes each quest in.
const TURN_INS_FETCHED_AT: &str = "turn_in_npcs_fetched_at";

/// When a download of one of the wiki's tables last failed, and why, as `<time> <reason>`, so a
/// refusal is not asked again on every lookup. A download that works removes it.
pub const DROPS_FAILED: &str = "drops_failed";
pub const TURN_INS_FAILED: &str = "turn_in_npcs_failed";
/// Likewise for Leafline's file of map layouts.
const LAYOUTS_FETCHED_AT: &str = "layouts_fetched_at";
pub const LAYOUTS_FAILED: &str = "layouts_failed";

/// Kept in `source_info` when a new version of Leafline replaces the cache: the player had
/// downloaded the map data or drop table, so each comes back without asking again.
const WANTS_MAPS: &str = "wants_maps";
const WANTS_DROPS: &str = "wants_drops";

/// Between the classes that can use a piece of equipment, in `items.classes`.
const CLASS_SEPARATOR: &str = ",";

/// The tables that hold each OSMS file, cleared before the file is stored again.
fn tables_of(file: &str) -> &'static [&'static str] {
    match file {
        "maps.json" => &["maps", "map_exits", "portals", "map_points"],
        "monsters.json" => &["monsters", "spawns"],
        "npcs.json" => &["npcs", "npc_maps"],
        "quests.json" => &["quests", "quest_mobs", "quest_rewards", "quest_items", "quest_follows"],
        "skills.json" => &["skills", "skill_needs", "skill_levels"],
        "items.json" => &["items"],
        "recipes.json" => &["recipes", "recipe_ingredients"],
        _ => &[],
    }
}

/// What has been downloaded, for the settings panel and the card's footer.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataStatus {
    /// Every file the card needs is cached.
    pub ready: bool,
    pub client_build: Option<String>,
    pub data_updated_at: Option<String>,
    /// When OSMS was last asked for changes, in Unix seconds.
    pub checked_at: Option<i64>,
    pub files: Vec<FileStatus>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileStatus {
    pub name: String,
    pub records: u32,
    pub updated_at: String,
    pub fetched_at: i64,
}

/// A downloaded file waiting to be stored.
pub struct Download {
    pub name: String,
    pub meta: FileMeta,
    pub records: Records,
}

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        let version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if version != CACHE_VERSION {
            // A cache from another version of Leafline: start over rather than guess. Tables an
            // older cache lacks count as empty.
            let held = |sql: &str| conn.query_row(sql, [], |row| row.get::<_, i64>(0)).unwrap_or(0) > 0;
            let maps = held("SELECT COUNT(*) FROM source_files");
            let drops = held("SELECT COUNT(*) FROM source_info WHERE key = 'drops_fetched_at'");
            let tables: Vec<String> = conn
                .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")?
                .query_map([], |row| row.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            for table in tables {
                conn.execute(&format!("DROP TABLE \"{table}\""), [])?;
            }
            conn.execute_batch(SCHEMA)?;
            conn.pragma_update(None, "user_version", CACHE_VERSION)?;
            for (key, wanted) in [(WANTS_MAPS, maps), (WANTS_DROPS, drops)] {
                if wanted {
                    conn.execute("INSERT INTO source_info (key, value) VALUES (?1, '1')", [key])?;
                }
            }
        }
        conn.execute_batch(ADDED)?;
        conn.execute_batch(RETIRED)?;
        Ok(Self { conn })
    }

    /// Whether the player downloaded the map data before this cache replaced an older one.
    pub fn wants_maps(&self) -> Result<bool> {
        Ok(self.info(WANTS_MAPS)?.is_some())
    }

    /// Whether the drop table should be kept up to date.
    pub fn wants_drops(&self) -> Result<bool> {
        Ok(self.drops_fetched_at()?.is_some() || self.info(WANTS_DROPS)?.is_some())
    }

    /// The checksum each cached file was downloaded at.
    pub fn file_hashes(&self) -> Result<HashMap<String, String>> {
        let mut stmt = self.conn.prepare("SELECT name, sha256 FROM source_files")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn checked_at(&self) -> Result<Option<i64>> {
        Ok(self.info("checked_at")?.and_then(|v| v.parse().ok()))
    }

    fn info(&self, key: &str) -> Result<Option<String>> {
        Ok(self.conn.query_row("SELECT value FROM source_info WHERE key = ?1", [key], |row| row.get(0)).optional()?)
    }

    /// Stores the changed files and the new `meta.json` details in one transaction, so the
    /// cache never mixes half of one update with the rest.
    pub fn apply(&mut self, meta: &Meta, downloads: &[Download], now: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        for download in downloads {
            for table in tables_of(&download.name) {
                tx.execute(&format!("DELETE FROM {table}"), [])?;
            }
            match &download.records {
                Records::Maps(maps) => insert_maps(&tx, maps)?,
                Records::Monsters(monsters) => insert_monsters(&tx, monsters)?,
                Records::Npcs(npcs) => insert_npcs(&tx, npcs)?,
                Records::Quests(quests) => insert_quests(&tx, quests)?,
                Records::Skills(skills) => insert_skills(&tx, skills)?,
                Records::Items(items) => insert_items(&tx, items)?,
                Records::Recipes(recipes) => insert_recipes(&tx, recipes)?,
            }
            tx.execute(
                "INSERT OR REPLACE INTO source_files (name, sha256, records, updated_at, fetched_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![download.name, download.meta.sha256, download.meta.records, download.meta.updated_at, now],
            )?;
        }
        let set = "INSERT OR REPLACE INTO source_info (key, value) VALUES (?1, ?2)";
        tx.execute(set, ["client_build", &meta.client_build])?;
        tx.execute(set, ["data_updated_at", &meta.data_updated_at])?;
        tx.execute(set, ["checked_at", &now.to_string()])?;
        tx.commit()?;
        Ok(())
    }

    pub fn status(&self) -> Result<DataStatus> {
        let mut stmt =
            self.conn.prepare("SELECT name, records, updated_at, fetched_at FROM source_files ORDER BY name")?;
        let files: Vec<FileStatus> = stmt
            .query_map([], |row| {
                Ok(FileStatus {
                    name: row.get(0)?,
                    records: row.get(1)?,
                    updated_at: row.get(2)?,
                    fetched_at: row.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        Ok(DataStatus {
            ready: FILES.iter().all(|f| files.iter().any(|s| s.name == *f)),
            client_build: self.info("client_build")?,
            data_updated_at: self.info("data_updated_at")?,
            checked_at: self.checked_at()?,
            files,
        })
    }

    /// The cached dataset, or `None` until every file has been downloaded.
    pub fn load(&self) -> Result<Option<Dataset>> {
        if !self.status()?.ready {
            return Ok(None);
        }
        Ok(Some(Dataset {
            turn_ins: self.turn_ins()?,
            shops: self.shops()?,
            maps: self.load_maps()?,
            monsters: self.load_monsters()?,
            npcs: self.load_npcs()?,
            quests: self.load_quests()?,
            skills: self.load_skills()?,
            items: self.load_items()?,
            recipes: self.load_recipes()?,
        }))
    }

    fn load_recipes(&self) -> Result<Vec<RecipeRecord>> {
        let mut ingredients: HashMap<u32, Vec<Ingredient>> = HashMap::new();
        let mut stmt = self.conn.prepare("SELECT recipe_id, item_id, count FROM recipe_ingredients ORDER BY recipe_id, seq")?;
        for row in stmt.query_map([], |row| Ok((row.get::<_, u32>(0)?, Ingredient { item_id: row.get(1)?, count: row.get(2)? })))? {
            let (recipe, ingredient) = row?;
            ingredients.entry(recipe).or_default().push(ingredient);
        }
        let mut stmt = self.conn.prepare(
            "SELECT id, discipline, skill_level, craft_exp, meso_cost, output_item_id, output_count FROM recipes ORDER BY id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(RecipeRecord {
                id: row.get(0)?,
                discipline: row.get(1)?,
                skill_level: row.get(2)?,
                craft_exp: row.get(3)?,
                meso_cost: row.get::<_, i64>(4)? as u64,
                output_item_id: row.get(5)?,
                output_count: row.get(6)?,
                ingredients: Vec::new(),
            })
        })?;
        let mut recipes = Vec::new();
        for row in rows {
            let mut recipe = row?;
            recipe.ingredients = ingredients.remove(&recipe.id).unwrap_or_default();
            recipes.push(recipe);
        }
        Ok(recipes)
    }

    fn load_skills(&self) -> Result<Vec<SkillRecord>> {
        let mut needs: HashMap<u32, Vec<SkillNeed>> = HashMap::new();
        let mut stmt = self.conn.prepare("SELECT skill_id, needs_id, level FROM skill_needs ORDER BY rowid")?;
        for row in stmt.query_map([], |row| Ok((row.get(0)?, SkillNeed { skill_id: row.get(1)?, level: row.get(2)? })))?
        {
            let (skill, need) = row?;
            needs.entry(skill).or_default().push(need);
        }
        let mut levels: HashMap<u32, Vec<String>> = HashMap::new();
        let mut stmt = self.conn.prepare("SELECT skill_id, description FROM skill_levels ORDER BY skill_id, level")?;
        for row in stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))? {
            let (skill, description) = row?;
            levels.entry(skill).or_default().push(description);
        }
        let mut stmt = self.conn.prepare(
            "SELECT id, name, job_id, class_name, job_tier, max_level, passive, description, icon_url, buff, effects FROM skills ORDER BY id",
        )?;
        let skills = stmt.query_map([], |row| {
            let id: u32 = row.get(0)?;
            let effects: String = row.get(10)?;
            Ok(SkillRecord {
                id,
                name: row.get(1)?,
                job_id: row.get(2)?,
                class_name: row.get(3)?,
                job_tier: row.get(4)?,
                max_level: row.get(5)?,
                passive: row.get(6)?,
                description: row.get(7)?,
                required_skills: needs.remove(&id).unwrap_or_default(),
                level_descriptions: levels.remove(&id).unwrap_or_default(),
                icon_url: row.get(8)?,
                buff: row.get(9)?,
                effects: serde_json::from_str(&effects).unwrap_or_default(),
            })
        })?;
        Ok(skills.collect::<rusqlite::Result<_>>()?)
    }

    fn load_items(&self) -> Result<Vec<ItemRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, kind, slot, gender, classes, weapon_type, req_level, req_str, req_dex, req_int, req_luk,
                    req_fame, attack_speed, upgrade_slots, quest_item, untradable, stats, price, description, scroll, knockback, icon_url
             FROM items ORDER BY id",
        )?;
        let rows = stmt.query_map([], |row| {
            let kind: String = row.get(2)?;
            let classes: String = row.get(5)?;
            let stats: String = row.get(17)?;
            let scroll: Option<String> = row.get(20)?;
            let item = ItemRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                kind: ItemKind::named(&kind),
                slot: row.get(3)?,
                gender: row.get(4)?,
                classes: classes.split(CLASS_SEPARATOR).filter(|c| !c.is_empty()).map(str::to_string).collect(),
                weapon_type: row.get(6)?,
                req_level: row.get(7)?,
                req_str: row.get(8)?,
                req_dex: row.get(9)?,
                req_int: row.get(10)?,
                req_luk: row.get(11)?,
                req_fame: row.get(12)?,
                attack_speed: row.get(13)?,
                upgrade_slots: row.get(14)?,
                quest_item: row.get(15)?,
                untradable: row.get(16)?,
                stats: Vec::new(),
                price: row.get(18)?,
                description: row.get(19)?,
                scroll: None,
                knockback: row.get(21)?,
                icon_url: row.get(22)?,
            };
            Ok((item, stats, scroll))
        })?;
        let mut items = Vec::new();
        for row in rows {
            let (mut item, stats, scroll) = row?;
            item.stats = serde_json::from_str(&stats).map_err(|err| Error::Invalid(err.to_string()))?;
            item.scroll =
                scroll.map(|s| serde_json::from_str(&s)).transpose().map_err(|err| Error::Invalid(err.to_string()))?;
            items.push(item);
        }
        Ok(items)
    }

    fn load_maps(&self) -> Result<Vec<MapRecord>> {
        let mut exits: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut stmt = self.conn.prepare("SELECT map_id, dest_map_id FROM map_exits ORDER BY rowid")?;
        for row in stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))? {
            let (map, dest) = row?;
            exits.entry(map).or_default().push(dest);
        }
        let mut portals: HashMap<u32, Vec<Portal>> = HashMap::new();
        let mut stmt = self
            .conn
            .prepare("SELECT map_id, name, dest_map_id, dest_portal, side, height FROM portals ORDER BY map_id, seq")?;
        for row in stmt.query_map([], |row| {
            let (side, height): (Option<u32>, Option<u32>) = (row.get(4)?, row.get(5)?);
            let portal = Portal {
                name: row.get(1)?,
                dest_map_id: row.get(2)?,
                dest_portal: row.get(3)?,
                side: side.and_then(|i| Side::ALL.get(i as usize).copied()),
                height: height.and_then(|i| Height::ALL.get(i as usize).copied()),
            };
            Ok((row.get(0)?, portal))
        })? {
            let (map, portal) = row?;
            portals.entry(map).or_default().push(portal);
        }
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, street, region, is_town, return_map_id, map_mark, minimap_url, render_url, map_mark_url FROM maps ORDER BY id")?;
        let maps = stmt.query_map([], |row| {
            let id: u32 = row.get(0)?;
            Ok(MapRecord {
                id,
                name: row.get(1)?,
                street_name: row.get(2)?,
                region: row.get(3)?,
                is_town: row.get(4)?,
                return_map_id: row.get(5)?,
                map_mark: row.get(6)?,
                exits: exits.remove(&id).unwrap_or_default(),
                portals: portals.remove(&id).unwrap_or_default(),
                minimap_url: row.get(7)?,
                render_url: row.get(8)?,
                map_mark_url: row.get(9)?,
                points: Vec::new(),
            })
        })?;
        Ok(maps.collect::<rusqlite::Result<_>>()?)
    }

    /// A map's portals, NPCs and spawn points at their places on its picture, in file order.
    pub fn map_points(&self, map: u32) -> Result<Vec<MapPoint>> {
        let mut stmt =
            self.conn.prepare("SELECT kind, ref_id, name, to_portal, x, y FROM map_points WHERE map_id = ?1 ORDER BY rowid")?;
        let rows = stmt.query_map([map], |row| {
            let kind: u32 = row.get(0)?;
            let point = MapPoint {
                kind: PointKind::Portal,
                id: row.get(1)?,
                name: row.get(2)?,
                to_portal: row.get(3)?,
                x: row.get(4)?,
                y: row.get(5)?,
            };
            Ok((kind, point))
        })?;
        let mut points = Vec::new();
        for row in rows {
            let (kind, mut point) = row?;
            let Some(kind) = PointKind::ALL.get(kind as usize) else { continue };
            point.kind = *kind;
            points.push(point);
        }
        Ok(points)
    }

    fn load_monsters(&self) -> Result<Vec<MonsterRecord>> {
        let mut spawns: HashMap<u32, Vec<Spawn>> = HashMap::new();
        let mut stmt = self.conn.prepare("SELECT mob_id, map_id, count, respawn FROM spawns ORDER BY mob_id, map_id")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, Spawn { map_id: row.get(1)?, count: row.get(2)?, respawn: row.get(3)? }))
        })?;
        for row in rows {
            let (mob, spawn) = row?;
            spawns.entry(mob).or_default().push(spawn);
        }
        let mut stmt =
            self.conn.prepare("SELECT id, name, level, hp, exp, is_boss, undead, details, icon_url FROM monsters ORDER BY id")?;
        let rows = stmt.query_map([], |row| {
            let id: u32 = row.get(0)?;
            let details: String = row.get(7)?;
            let monster = MonsterRecord {
                id,
                name: row.get(1)?,
                level: row.get(2)?,
                hp: row.get(3)?,
                exp: row.get(4)?,
                is_boss: row.get(5)?,
                undead: row.get(6)?,
                spawns: spawns.remove(&id).unwrap_or_default(),
                details: MonsterDetails::default(),
                icon_url: row.get(8)?,
            };
            Ok((monster, details))
        })?;
        let mut monsters = Vec::new();
        for row in rows {
            let (mut monster, details) = row?;
            monster.details = serde_json::from_str(&details).map_err(|err| Error::Invalid(err.to_string()))?;
            monsters.push(monster);
        }
        Ok(monsters)
    }

    fn load_npcs(&self) -> Result<Vec<NpcRecord>> {
        let mut maps: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut stmt = self.conn.prepare("SELECT npc_id, map_id FROM npc_maps ORDER BY rowid")?;
        for row in stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))? {
            let (npc, map) = row?;
            maps.entry(npc).or_default().push(map);
        }
        let mut stmt = self.conn.prepare("SELECT id, name, icon_url FROM npcs ORDER BY id")?;
        let npcs = stmt.query_map([], |row| {
            let id: u32 = row.get(0)?;
            Ok(NpcRecord { id, name: row.get(1)?, maps: maps.remove(&id).unwrap_or_default(), icon_url: row.get(2)? })
        })?;
        Ok(npcs.collect::<rusqlite::Result<_>>()?)
    }

    fn load_quests(&self) -> Result<Vec<QuestRecord>> {
        let mut needs: HashMap<u32, Vec<MobRequirement>> = HashMap::new();
        let mut stmt = self.conn.prepare("SELECT quest_id, mob_id, count FROM quest_mobs ORDER BY rowid")?;
        for row in
            stmt.query_map([], |row| Ok((row.get(0)?, MobRequirement { mob_id: row.get(1)?, count: row.get(2)? })))?
        {
            let (quest, need) = row?;
            needs.entry(quest).or_default().push(need);
        }
        let mut rewards: HashMap<u32, Vec<RewardItem>> = HashMap::new();
        let mut stmt = self.conn.prepare("SELECT quest_id, item_id, count, sure FROM quest_rewards ORDER BY rowid")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get(0)?, RewardItem { item_id: row.get(1)?, count: row.get(2)?, sure: row.get(3)? }))
        })?;
        for row in rows {
            let (quest, item) = row?;
            rewards.entry(quest).or_default().push(item);
        }
        let mut items: HashMap<u32, Vec<ItemNeed>> = HashMap::new();
        let mut stmt = self.conn.prepare("SELECT quest_id, item_id, count FROM quest_items ORDER BY rowid")?;
        for row in stmt.query_map([], |row| Ok((row.get(0)?, ItemNeed { item_id: row.get(1)?, count: row.get(2)? })))? {
            let (quest, need) = row?;
            items.entry(quest).or_default().push(need);
        }
        let mut follows: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut stmt = self.conn.prepare("SELECT quest_id, needs_id FROM quest_follows ORDER BY rowid")?;
        for row in stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))? {
            let (quest, needs) = row?;
            follows.entry(quest).or_default().push(needs);
        }
        let mut stmt = self.conn.prepare(
            "SELECT id, name, npc_id, level_min, next_quest_id, region, description, chain, repeat, reward_exp, reward_mesos
             FROM quests ORDER BY id",
        )?;
        let quests = stmt.query_map([], |row| {
            let id: u32 = row.get(0)?;
            Ok(QuestRecord {
                id,
                name: row.get(1)?,
                npc_id: row.get(2)?,
                level_min: row.get(3)?,
                next_quest_id: row.get(4)?,
                requirements: needs.remove(&id).unwrap_or_default(),
                reward_items: rewards.remove(&id).unwrap_or_default(),
                region: row.get(5)?,
                description: row.get(6)?,
                chain: row.get(7)?,
                repeat: row.get(8)?,
                items: items.remove(&id).unwrap_or_default(),
                follows: follows.remove(&id).unwrap_or_default(),
                reward_exp: row.get(9)?,
                reward_mesos: row.get(10)?,
            })
        })?;
        Ok(quests.collect::<rusqlite::Result<_>>()?)
    }

    /// When the turn-in table was downloaded, or `None` if it has not been.
    pub fn shops_fetched_at(&self) -> Result<Option<i64>> {
        Ok(self.info(SHOPS_FETCHED_AT)?.and_then(|v| v.parse().ok()))
    }

    /// The shop file as downloaded, if it ever was.
    pub fn shop_file(&self) -> Result<Option<Vec<u8>>> {
        Ok(self.info(SHOP_FILE)?.map(String::into_bytes))
    }

    /// The downloaded shops, or none when nothing was downloaded or the download no longer reads.
    pub fn shops(&self) -> Result<Vec<super::shops::ShopRecord>> {
        let downloaded = self.shop_file()?.and_then(|body| {
            super::shops::parse(&body).map_err(|err| eprintln!("cached shop file unreadable: {err}")).ok()
        });
        Ok(match downloaded {
            Some(file) => file.shops,
            None => Vec::new(),
        })
    }

    pub fn replace_shop_file(&mut self, body: &[u8], now: i64) -> Result<()> {
        let text = std::str::from_utf8(body).map_err(|err| Error::Invalid(format!("The shop file isn't text: {err}")))?;
        let tx = self.conn.transaction()?;
        let set = "INSERT OR REPLACE INTO source_info (key, value) VALUES (?1, ?2)";
        tx.execute(set, [SHOP_FILE, text])?;
        tx.execute(set, [SHOPS_FETCHED_AT, &now.to_string()])?;
        tx.commit()?;
        Ok(())
    }

    pub fn turn_ins_fetched_at(&self) -> Result<Option<i64>> {
        Ok(self.info(TURN_INS_FETCHED_AT)?.and_then(|v| v.parse().ok()))
    }

    /// Who takes each quest in, as downloaded.
    pub fn turn_ins(&self) -> Result<Vec<TurnIn>> {
        let mut stmt = self.conn.prepare("SELECT quest_id, npc_id FROM quest_turn_in_npcs ORDER BY quest_id")?;
        let rows = stmt.query_map([], |row| Ok(TurnIn { quest: row.get(0)?, npc: row.get(1)? }))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Replaces the whole turn-in table.
    pub fn replace_turn_ins(&mut self, rows: &[TurnIn], now: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM quest_turn_in_npcs", [])?;
        for row in rows {
            tx.execute("INSERT OR REPLACE INTO quest_turn_in_npcs (quest_id, npc_id) VALUES (?1, ?2)", params![row.quest, row.npc])?;
        }
        tx.execute(
            "INSERT OR REPLACE INTO source_info (key, value) VALUES (?1, ?2)",
            [TURN_INS_FETCHED_AT, &now.to_string()],
        )?;
        tx.execute("DELETE FROM source_info WHERE key = ?1", [TURN_INS_FAILED])?;
        tx.commit()?;
        Ok(())
    }

    pub fn layouts_fetched_at(&self) -> Result<Option<i64>> {
        Ok(self.info(LAYOUTS_FETCHED_AT)?.and_then(|v| v.parse().ok()))
    }

    /// Every map's layout, as downloaded.
    pub fn layouts(&self) -> Result<Layouts> {
        let mut stmt = self.conn.prepare(
            "SELECT map_id, width, height, walkable, platforms, spawn_points, spawn_platforms, ropes, ladders
             FROM map_layouts",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(MapLayout {
                map: row.get(0)?,
                width: row.get(1)?,
                height: row.get(2)?,
                walkable: row.get(3)?,
                platforms: row.get(4)?,
                spawn_points: row.get(5)?,
                spawn_platforms: row.get(6)?,
                ropes: row.get(7)?,
                ladders: row.get(8)?,
            })
        })?;
        Ok(rows.map(|r| r.map(|l| (l.map, l))).collect::<rusqlite::Result<_>>()?)
    }

    /// Replaces every map's layout.
    pub fn replace_layouts(&mut self, rows: &[MapLayout], now: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM map_layouts", [])?;
        for l in rows {
            tx.execute(
                "INSERT OR REPLACE INTO map_layouts
                 (map_id, width, height, walkable, platforms, spawn_points, spawn_platforms, ropes, ladders)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![l.map, l.width, l.height, l.walkable, l.platforms, l.spawn_points, l.spawn_platforms, l.ropes, l.ladders],
            )?;
        }
        tx.execute("INSERT OR REPLACE INTO source_info (key, value) VALUES (?1, ?2)", [LAYOUTS_FETCHED_AT, &now.to_string()])?;
        tx.execute("DELETE FROM source_info WHERE key = ?1", [LAYOUTS_FAILED])?;
        tx.commit()?;
        Ok(())
    }

    /// When the drop table was downloaded, or `None` if it has not been.
    pub fn drops_fetched_at(&self) -> Result<Option<i64>> {
        Ok(self.info(DROPS_FETCHED_AT)?.and_then(|v| v.parse().ok()))
    }

    /// The downloaded drops of each of `monsters`. One the table doesn't list comes back with
    /// no drops.
    pub fn drops(&self, monsters: &[String]) -> Result<Vec<MonsterDrops>> {
        let fetched_at = self.drops_fetched_at()?.unwrap_or_default();
        let mut found = Vec::with_capacity(monsters.len());
        for monster in monsters {
            let summary: Option<(Option<u32>, Option<String>)> = self
                .conn
                .query_row("SELECT sample_kills, game_version FROM drop_fetches WHERE monster = ?1", [monster], |row| {
                    Ok((row.get(0)?, row.get(1)?))
                })
                .optional()?;
            let (sample_kills, game_version) = summary.unwrap_or_default();
            found.push(MonsterDrops {
                monster: monster.clone(),
                sample_kills,
                game_version,
                fetched_at,
                drops: self.drops_of(monster)?,
            });
        }
        Ok(found)
    }

    fn drops_of(&self, monster: &str) -> Result<Vec<Drop>> {
        let mut stmt = self.conn.prepare(
            "SELECT item, category, rate, rate_low, rate_high, min_quantity, max_quantity, sample_kills,
                    sample_drops, game_version, quest
             FROM drops WHERE monster = ?1 ORDER BY rowid",
        )?;
        let drops = stmt.query_map([monster], |row| {
            Ok(Drop {
                item: row.get(0)?,
                category: row.get(1)?,
                rate: row.get(2)?,
                rate_low: row.get(3)?,
                rate_high: row.get(4)?,
                min_quantity: row.get(5)?,
                max_quantity: row.get(6)?,
                sample_kills: row.get(7)?,
                sample_drops: row.get(8)?,
                game_version: row.get(9)?,
                quest: row.get(10)?,
                item_id: None,
                item_type: None,
                req_level: None,
            })
        })?;
        Ok(drops.collect::<rusqlite::Result<_>>()?)
    }

    /// Every monster and item pair the table lists, for finding monsters by what they drop.
    pub fn dropped_pairs(&self) -> Result<Vec<(String, String)>> {
        let mut stmt = self.conn.prepare("SELECT DISTINCT monster, item FROM drops ORDER BY monster, item")?;
        let pairs = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(pairs.collect::<rusqlite::Result<_>>()?)
    }

    /// Every item the table lists, for matching what was read against.
    pub fn dropped_items(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare("SELECT DISTINCT item FROM drops ORDER BY item")?;
        let items = stmt.query_map([], |row| row.get(0))?;
        Ok(items.collect::<rusqlite::Result<_>>()?)
    }

    /// The monsters the table lists for `item`, with their rates.
    pub fn droppers(&self, item: &str) -> Result<Vec<Dropper>> {
        let mut stmt = self.conn.prepare(
            "SELECT monster, rate, rate_low, rate_high, sample_kills, game_version, quest
             FROM drops WHERE item = ?1 ORDER BY rowid",
        )?;
        let droppers = stmt.query_map([item], |row| {
            Ok(Dropper {
                monster: row.get(0)?,
                rate: row.get(1)?,
                rate_low: row.get(2)?,
                rate_high: row.get(3)?,
                sample_kills: row.get(4)?,
                game_version: row.get(5)?,
                quest: row.get(6)?,
            })
        })?;
        Ok(droppers.collect::<rusqlite::Result<_>>()?)
    }

    /// Replaces the whole drop table, so a monster the wiki no longer lists goes with it.
    pub fn replace_drops(&mut self, table: &BTreeMap<String, MonsterDrops>, now: i64) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM drops", [])?;
        tx.execute("DELETE FROM drop_fetches", [])?;
        for (monster, entry) in table {
            for d in &entry.drops {
                tx.execute(
                    "INSERT INTO drops (monster, item, category, rate, rate_low, rate_high, min_quantity,
                                        max_quantity, sample_kills, sample_drops, game_version, quest)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        monster,
                        d.item,
                        d.category,
                        d.rate,
                        d.rate_low,
                        d.rate_high,
                        d.min_quantity,
                        d.max_quantity,
                        d.sample_kills,
                        d.sample_drops,
                        d.game_version,
                        d.quest
                    ],
                )?;
            }
            tx.execute(
                "INSERT OR REPLACE INTO drop_fetches (monster, fetched_at, sample_kills, game_version) VALUES (?1, ?2, ?3, ?4)",
                params![monster, entry.fetched_at, entry.sample_kills, entry.game_version],
            )?;
        }
        tx.execute(
            "INSERT OR REPLACE INTO source_info (key, value) VALUES (?1, ?2)",
            [DROPS_FETCHED_AT, &now.to_string()],
        )?;
        tx.execute("DELETE FROM source_info WHERE key = ?1", [DROPS_FAILED])?;
        tx.commit()?;
        Ok(())
    }

    /// When a download kept under `key` ([`DROPS_FAILED`] or [`TURN_INS_FAILED`]) last failed,
    /// and why.
    pub fn failure(&self, key: &str) -> Result<Option<(i64, String)>> {
        Ok(self.info(key)?.and_then(|value| {
            let (at, reason) = value.split_once(' ').unwrap_or((&value, ""));
            Some((at.parse().ok()?, reason.to_string()))
        }))
    }

    /// Records a failed download under `key`, or with `None` forgets it so the next lookup asks again.
    pub fn set_failure(&self, key: &str, failure: Option<(i64, &str)>) -> Result<()> {
        match failure {
            Some((at, reason)) => self.conn.execute(
                "INSERT OR REPLACE INTO source_info (key, value) VALUES (?1, ?2)",
                [key, &format!("{at} {reason}")],
            )?,
            None => self.conn.execute("DELETE FROM source_info WHERE key = ?1", [key])?,
        };
        Ok(())
    }
}

fn insert_maps(tx: &Transaction, maps: &[MapRecord]) -> Result<()> {
    let mut map = tx.prepare(
        "INSERT OR REPLACE INTO maps (id, name, street, region, is_town, return_map_id, map_mark, minimap_url, render_url, map_mark_url)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
    )?;
    let mut exit = tx.prepare("INSERT OR IGNORE INTO map_exits (map_id, dest_map_id) VALUES (?1, ?2)")?;
    let mut point =
        tx.prepare("INSERT INTO map_points (map_id, kind, ref_id, name, to_portal, x, y) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")?;
    let mut portal = tx.prepare(
        "INSERT INTO portals (map_id, seq, name, dest_map_id, dest_portal, side, height)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    for m in maps {
        map.execute(params![m.id, m.name, m.street_name, m.region, m.is_town, m.return_map_id, m.map_mark, m.minimap_url, m.render_url, m.map_mark_url])?;
        for dest in &m.exits {
            exit.execute(params![m.id, dest])?;
        }
        for p in &m.points {
            point.execute(params![m.id, p.kind as i64, p.id, p.name, p.to_portal, p.x, p.y])?;
        }
        // Portals that lead nowhere (spawn points, scripted ones) say nothing about the way out.
        for (seq, p) in m.portals.iter().filter(|p| p.dest_map_id.is_some()).enumerate() {
            let (side, height) = (p.side.map(|s| s as i64), p.height.map(|h| h as i64));
            portal.execute(params![m.id, seq as i64, p.name, p.dest_map_id, p.dest_portal, side, height])?;
        }
    }
    Ok(())
}

fn insert_monsters(tx: &Transaction, monsters: &[MonsterRecord]) -> Result<()> {
    let mut monster = tx.prepare(
        "INSERT OR REPLACE INTO monsters (id, name, level, hp, exp, is_boss, undead, details, icon_url)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
    )?;
    let mut spawn = tx.prepare(
        "INSERT INTO spawns (mob_id, map_id, count, respawn) VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT (mob_id, map_id) DO UPDATE SET count = count + excluded.count, respawn = MAX(respawn, excluded.respawn)",
    )?;
    for m in monsters {
        let details = serde_json::to_string(&m.details).map_err(|err| Error::Invalid(err.to_string()))?;
        monster.execute(params![m.id, m.name, m.level, m.hp, m.exp, m.is_boss, m.undead, details, m.icon_url])?;
        for s in &m.spawns {
            spawn.execute(params![m.id, s.map_id, s.count, s.respawn])?;
        }
    }
    Ok(())
}

fn insert_npcs(tx: &Transaction, npcs: &[NpcRecord]) -> Result<()> {
    let mut npc = tx.prepare("INSERT OR REPLACE INTO npcs (id, name, icon_url) VALUES (?1, ?2, ?3)")?;
    let mut on_map = tx.prepare("INSERT OR IGNORE INTO npc_maps (npc_id, map_id) VALUES (?1, ?2)")?;
    for n in npcs {
        npc.execute(params![n.id, n.name, n.icon_url])?;
        for map in &n.maps {
            on_map.execute(params![n.id, map])?;
        }
    }
    Ok(())
}

fn insert_quests(tx: &Transaction, quests: &[QuestRecord]) -> Result<()> {
    let mut quest = tx.prepare(
        "INSERT OR REPLACE INTO quests (id, name, npc_id, level_min, next_quest_id, region, description, chain, repeat,
                                        reward_exp, reward_mesos)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
    )?;
    let mut need = tx.prepare("INSERT INTO quest_mobs (quest_id, mob_id, count) VALUES (?1, ?2, ?3)")?;
    let mut reward = tx.prepare("INSERT INTO quest_rewards (quest_id, item_id, count, sure) VALUES (?1, ?2, ?3, ?4)")?;
    let mut item = tx.prepare("INSERT INTO quest_items (quest_id, item_id, count) VALUES (?1, ?2, ?3)")?;
    let mut follows = tx.prepare("INSERT INTO quest_follows (quest_id, needs_id) VALUES (?1, ?2)")?;
    for q in quests {
        quest.execute(params![
            q.id,
            q.name,
            q.npc_id,
            q.level_min,
            q.next_quest_id,
            q.region,
            q.description,
            q.chain,
            q.repeat,
            q.reward_exp,
            q.reward_mesos
        ])?;
        for need in &q.items {
            item.execute(params![q.id, need.item_id, need.count])?;
        }
        for needs in &q.follows {
            follows.execute(params![q.id, needs])?;
        }
        for r in &q.requirements {
            need.execute(params![q.id, r.mob_id, r.count])?;
        }
        for item in &q.reward_items {
            reward.execute(params![q.id, item.item_id, item.count, item.sure])?;
        }
    }
    Ok(())
}

fn insert_skills(tx: &Transaction, skills: &[SkillRecord]) -> Result<()> {
    let mut skill = tx.prepare(
        "INSERT OR REPLACE INTO skills (id, name, job_id, class_name, job_tier, max_level, passive, description, icon_url, buff, effects)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
    )?;
    let mut need = tx.prepare("INSERT INTO skill_needs (skill_id, needs_id, level) VALUES (?1, ?2, ?3)")?;
    let mut level = tx.prepare("INSERT OR REPLACE INTO skill_levels (skill_id, level, description) VALUES (?1, ?2, ?3)")?;
    for s in skills {
        // Only what a buff changes is worth keeping; other skills' levels are all zeros.
        let effects = if s.effects.iter().any(Effects::any) { serde_json::to_string(&s.effects).unwrap_or_default() } else { "[]".into() };
        skill.execute(params![
            s.id,
            s.name,
            s.job_id,
            s.class_name,
            s.job_tier,
            s.max_level,
            s.passive,
            s.description,
            s.icon_url,
            s.buff,
            effects
        ])?;
        for n in &s.required_skills {
            need.execute(params![s.id, n.skill_id, n.level])?;
        }
        for (i, text) in s.level_descriptions.iter().enumerate() {
            level.execute(params![s.id, i as i64 + 1, text])?;
        }
    }
    Ok(())
}

fn insert_recipes(tx: &Transaction, recipes: &[RecipeRecord]) -> Result<()> {
    let mut recipe = tx.prepare(
        "INSERT OR REPLACE INTO recipes (id, discipline, skill_level, craft_exp, meso_cost, output_item_id, output_count)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    let mut ingredient =
        tx.prepare("INSERT OR REPLACE INTO recipe_ingredients (recipe_id, seq, item_id, count) VALUES (?1, ?2, ?3, ?4)")?;
    for r in recipes {
        recipe.execute(params![
            r.id,
            r.discipline,
            r.skill_level,
            r.craft_exp,
            r.meso_cost as i64,
            r.output_item_id,
            r.output_count
        ])?;
        for (i, g) in r.ingredients.iter().enumerate() {
            ingredient.execute(params![r.id, i as i64, g.item_id, g.count])?;
        }
    }
    Ok(())
}

fn insert_items(tx: &Transaction, items: &[ItemRecord]) -> Result<()> {
    let mut item = tx.prepare(
        "INSERT OR REPLACE INTO items (id, name, kind, slot, gender, classes, weapon_type, req_level, req_str, req_dex,
                                       req_int, req_luk, req_fame, attack_speed, upgrade_slots, quest_item, untradable,
                                       stats, price, description, scroll, knockback, icon_url)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)",
    )?;
    for i in items {
        let json = |err: serde_json::Error| Error::Invalid(err.to_string());
        let stats = serde_json::to_string(&i.stats).map_err(json)?;
        let scroll = i.scroll.as_ref().map(serde_json::to_string).transpose().map_err(json)?;
        item.execute(params![
            i.id,
            i.name,
            i.kind.name(),
            i.slot,
            i.gender,
            i.classes.join(CLASS_SEPARATOR),
            i.weapon_type,
            i.req_level,
            i.req_str,
            i.req_dex,
            i.req_int,
            i.req_luk,
            i.req_fame,
            i.attack_speed,
            i.upgrade_slots,
            i.quest_item,
            i.untradable,
            stats,
            i.price,
            i.description,
            scroll,
            i.knockback,
            i.icon_url
        ])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cache left by the version of Leafline before this one.
    fn older_cache(setup: &str) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();
        conn.execute_batch(setup).unwrap();
        conn.pragma_update(None, "user_version", CACHE_VERSION - 1).unwrap();
        conn
    }

    const A_FILE: &str = "INSERT INTO source_files VALUES ('maps.json', 'abc', 1, 'today', 0);";
    const DROPS: &str = "INSERT INTO source_info VALUES ('drops_fetched_at', '5');";

    #[test]
    fn a_replaced_cache_remembers_what_the_player_downloaded() {
        let store = Store::init(older_cache(&format!("{A_FILE}{DROPS}"))).unwrap();
        assert!(!store.status().unwrap().ready, "the old data is gone");
        assert!(store.wants_maps().unwrap());
        assert!(store.wants_drops().unwrap());
        assert_eq!(store.drops_fetched_at().unwrap(), None);

        let store = Store::init(older_cache(A_FILE)).unwrap();
        assert!(store.wants_maps().unwrap());
        assert!(!store.wants_drops().unwrap(), "the drop table was never asked for");
    }

    #[test]
    fn keeps_a_maps_points_apart_and_gives_them_back_in_order() {
        // A made-up map in OSMS's shape: a portal that leads somewhere, a spawn point, an NPC, a monster.
        let json = br#"[{"id": 7, "name": "Fernway Path", "portals": [
            {"name": "h_east00", "x": 10, "y": 20, "dest_map_id": 8}, {"name": "sp", "x": 1, "y": 2}],
            "npcs": [{"npc_id": 50, "x": 30, "y": 40}], "mobs": [{"mob_id": 3, "x": 50, "y": 60}]}]"#;
        let json = String::from_utf8_lossy(json).replace(r#""dest_map_id": 8}"#, r#""dest_map_id": 8, "dest_portal": "west00"}"#);
        let json = json.as_bytes();
        let Records::Maps(maps) = Records::parse("maps.json", json).unwrap() else { panic!() };
        let mut store = Store::open_in_memory().unwrap();
        let tx = store.conn.transaction().unwrap();
        insert_maps(&tx, &maps).unwrap();
        tx.commit().unwrap();
        let points: Vec<_> = store.map_points(7).unwrap().into_iter().map(|p| (p.kind, p.id, p.name, p.x, p.y)).collect();
        assert_eq!(
            points,
            [
                (PointKind::Portal, 8, Some("h_east00".to_string()), 10, 20),
                (PointKind::Npc, 50, None, 30, 40),
                (PointKind::Mob, 3, None, 50, 60),
            ]
        );
        assert!(store.map_points(8).unwrap().is_empty());
        assert_eq!(store.map_points(7).unwrap()[0].to_portal.as_deref(), Some("west00"));
    }

    #[test]
    fn a_cache_from_before_the_turn_in_table_keeps_what_it_holds() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();
        conn.execute_batch(A_FILE).unwrap();
        conn.pragma_update(None, "user_version", CACHE_VERSION).unwrap();
        let store = Store::init(conn).unwrap();
        assert_eq!(store.status().unwrap().files.len(), 1, "nothing to download again");
        assert!(store.turn_ins().unwrap().is_empty());
    }

    #[test]
    fn the_wiki_turn_in_names_give_way_without_a_new_download() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA).unwrap();
        conn.execute_batch(A_FILE).unwrap();
        conn.execute_batch(
            "CREATE TABLE quest_turn_ins (quest_id INTEGER PRIMARY KEY, npc TEXT NOT NULL);
             INSERT INTO quest_turn_ins VALUES (70, 'Warden Ilse');
             INSERT INTO source_info VALUES ('turn_ins_fetched_at', '5');",
        )
        .unwrap();
        conn.pragma_update(None, "user_version", CACHE_VERSION).unwrap();
        let store = Store::init(conn).unwrap();
        assert_eq!(store.status().unwrap().files.len(), 1, "the map data stays");
        assert!(store.turn_ins().unwrap().is_empty());
        assert_eq!(store.turn_ins_fetched_at().unwrap(), None, "so the new file is downloaded at once");
        let gone: i64 = store
            .conn
            .query_row("SELECT COUNT(*) FROM sqlite_master WHERE name = 'quest_turn_ins'", [], |row| row.get(0))
            .unwrap();
        assert_eq!(gone, 0);
    }

    #[test]
    fn nothing_comes_down_until_the_player_asks() {
        assert!(!Store::open_in_memory().unwrap().wants_maps().unwrap());
        assert!(!Store::init(older_cache("")).unwrap().wants_maps().unwrap());
    }
}
