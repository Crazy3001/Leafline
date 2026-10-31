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
    Dataset, FILES, FileMeta, Height, ItemRecord, MapRecord, Meta, MobRequirement, MonsterRecord, NpcRecord, Portal,
    QuestRecord, Records, Side, SkillNeed, SkillRecord, Spawn,
};
use crate::error::{Error, Result};

/// Bump when the tables change; an older cache is then dropped and downloaded again.
const CACHE_VERSION: i32 = 4;

const SCHEMA: &str = "
CREATE TABLE source_files (
    name TEXT PRIMARY KEY, sha256 TEXT NOT NULL, records INTEGER NOT NULL,
    updated_at TEXT NOT NULL, fetched_at INTEGER NOT NULL);
CREATE TABLE source_info (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE maps (
    id INTEGER PRIMARY KEY, name TEXT NOT NULL, street TEXT, region TEXT,
    is_town INTEGER NOT NULL, return_map_id INTEGER, map_mark TEXT);
CREATE TABLE map_exits (map_id INTEGER NOT NULL, dest_map_id INTEGER NOT NULL, PRIMARY KEY (map_id, dest_map_id));
CREATE TABLE portals (
    map_id INTEGER NOT NULL, seq INTEGER NOT NULL, name TEXT NOT NULL,
    dest_map_id INTEGER, dest_portal TEXT, side INTEGER, height INTEGER, PRIMARY KEY (map_id, seq));
CREATE TABLE monsters (
    id INTEGER PRIMARY KEY, name TEXT NOT NULL, level INTEGER NOT NULL, hp INTEGER NOT NULL,
    exp INTEGER NOT NULL, is_boss INTEGER NOT NULL, undead INTEGER NOT NULL);
CREATE TABLE spawns (mob_id INTEGER NOT NULL, map_id INTEGER NOT NULL, count INTEGER NOT NULL, PRIMARY KEY (mob_id, map_id));
CREATE TABLE npcs (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
CREATE TABLE npc_maps (npc_id INTEGER NOT NULL, map_id INTEGER NOT NULL, PRIMARY KEY (npc_id, map_id));
CREATE TABLE quests (id INTEGER PRIMARY KEY, name TEXT NOT NULL, npc_id INTEGER, level_min INTEGER, next_quest_id INTEGER);
CREATE TABLE quest_mobs (quest_id INTEGER NOT NULL, mob_id INTEGER NOT NULL, count INTEGER NOT NULL);
CREATE TABLE skills (
    id INTEGER PRIMARY KEY, name TEXT NOT NULL, job_id INTEGER NOT NULL, class_name TEXT NOT NULL,
    job_tier TEXT NOT NULL, max_level INTEGER NOT NULL, passive INTEGER NOT NULL, description TEXT);
CREATE TABLE skill_needs (skill_id INTEGER NOT NULL, needs_id INTEGER NOT NULL, level INTEGER NOT NULL);
CREATE TABLE skill_levels (skill_id INTEGER NOT NULL, level INTEGER NOT NULL, description TEXT NOT NULL, PRIMARY KEY (skill_id, level));
CREATE TABLE equipment (
    id INTEGER PRIMARY KEY, name TEXT NOT NULL, slot TEXT NOT NULL, gender TEXT, classes TEXT NOT NULL, weapon_type TEXT,
    req_level INTEGER NOT NULL, req_str INTEGER NOT NULL, req_dex INTEGER NOT NULL, req_int INTEGER NOT NULL,
    req_luk INTEGER NOT NULL, stats TEXT NOT NULL);
CREATE TABLE drops (
    monster TEXT NOT NULL, item TEXT NOT NULL, category TEXT, rate REAL, rate_low REAL, rate_high REAL,
    min_quantity INTEGER, max_quantity INTEGER, sample_kills INTEGER, sample_drops INTEGER,
    game_version TEXT, quest INTEGER NOT NULL);
CREATE INDEX drops_by_monster ON drops (monster);
CREATE TABLE drop_fetches (monster TEXT PRIMARY KEY, fetched_at INTEGER NOT NULL, sample_kills INTEGER, game_version TEXT);
";

/// Tables from when items were asked about one at a time. The whole drop table replaces them,
/// so they are removed from an existing cache without forcing a new download.
const RETIRED: &str = "
DROP TABLE IF EXISTS item_lookups;
DROP TABLE IF EXISTS item_droppers;
";

/// Where the time the drop table was downloaded is kept in `source_info`.
const DROPS_FETCHED_AT: &str = "drops_fetched_at";

/// Kept in `source_info` when a new version of Leafline replaces the cache: the player had
/// downloaded the map data or drop table, so each comes back without asking again.
const WANTS_MAPS: &str = "wants_maps";
const WANTS_DROPS: &str = "wants_drops";

/// Between the classes that can use a piece of equipment, in `equipment.classes`.
const CLASS_SEPARATOR: &str = ",";

/// The tables that hold each OSMS file, cleared before the file is stored again.
fn tables_of(file: &str) -> &'static [&'static str] {
    match file {
        "maps.json" => &["maps", "map_exits", "portals"],
        "monsters.json" => &["monsters", "spawns"],
        "npcs.json" => &["npcs", "npc_maps"],
        "quests.json" => &["quests", "quest_mobs"],
        "skills.json" => &["skills", "skill_needs", "skill_levels"],
        "items.json" => &["equipment"],
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
            maps: self.load_maps()?,
            monsters: self.load_monsters()?,
            npcs: self.load_npcs()?,
            quests: self.load_quests()?,
            skills: self.load_skills()?,
            items: self.load_items()?,
        }))
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
            "SELECT id, name, job_id, class_name, job_tier, max_level, passive, description FROM skills ORDER BY id",
        )?;
        let skills = stmt.query_map([], |row| {
            let id: u32 = row.get(0)?;
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
            })
        })?;
        Ok(skills.collect::<rusqlite::Result<_>>()?)
    }

    fn load_items(&self) -> Result<Vec<ItemRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, slot, gender, classes, weapon_type, req_level, req_str, req_dex, req_int, req_luk, stats
             FROM equipment ORDER BY id",
        )?;
        let rows = stmt.query_map([], |row| {
            let classes: String = row.get(4)?;
            let stats: String = row.get(11)?;
            let item = ItemRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                category: "Equipment".into(),
                slot: row.get(2)?,
                gender: row.get(3)?,
                classes: classes.split(CLASS_SEPARATOR).filter(|c| !c.is_empty()).map(str::to_string).collect(),
                weapon_type: row.get(5)?,
                req_level: row.get(6)?,
                req_str: row.get(7)?,
                req_dex: row.get(8)?,
                req_int: row.get(9)?,
                req_luk: row.get(10)?,
                stats: Vec::new(),
            };
            Ok((item, stats))
        })?;
        let mut items = Vec::new();
        for row in rows {
            let (mut item, stats) = row?;
            item.stats = serde_json::from_str(&stats).map_err(|err| Error::Invalid(err.to_string()))?;
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
            .prepare("SELECT id, name, street, region, is_town, return_map_id, map_mark FROM maps ORDER BY id")?;
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
            })
        })?;
        Ok(maps.collect::<rusqlite::Result<_>>()?)
    }

    fn load_monsters(&self) -> Result<Vec<MonsterRecord>> {
        let mut spawns: HashMap<u32, Vec<Spawn>> = HashMap::new();
        let mut stmt = self.conn.prepare("SELECT mob_id, map_id, count FROM spawns ORDER BY mob_id, map_id")?;
        for row in stmt.query_map([], |row| Ok((row.get(0)?, Spawn { map_id: row.get(1)?, count: row.get(2)? })))? {
            let (mob, spawn) = row?;
            spawns.entry(mob).or_default().push(spawn);
        }
        let mut stmt =
            self.conn.prepare("SELECT id, name, level, hp, exp, is_boss, undead FROM monsters ORDER BY id")?;
        let monsters = stmt.query_map([], |row| {
            let id: u32 = row.get(0)?;
            Ok(MonsterRecord {
                id,
                name: row.get(1)?,
                level: row.get(2)?,
                hp: row.get(3)?,
                exp: row.get(4)?,
                is_boss: row.get(5)?,
                undead: row.get(6)?,
                spawns: spawns.remove(&id).unwrap_or_default(),
            })
        })?;
        Ok(monsters.collect::<rusqlite::Result<_>>()?)
    }

    fn load_npcs(&self) -> Result<Vec<NpcRecord>> {
        let mut maps: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut stmt = self.conn.prepare("SELECT npc_id, map_id FROM npc_maps ORDER BY rowid")?;
        for row in stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))? {
            let (npc, map) = row?;
            maps.entry(npc).or_default().push(map);
        }
        let mut stmt = self.conn.prepare("SELECT id, name FROM npcs ORDER BY id")?;
        let npcs = stmt.query_map([], |row| {
            let id: u32 = row.get(0)?;
            Ok(NpcRecord { id, name: row.get(1)?, maps: maps.remove(&id).unwrap_or_default() })
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
        let mut stmt =
            self.conn.prepare("SELECT id, name, npc_id, level_min, next_quest_id FROM quests ORDER BY id")?;
        let quests = stmt.query_map([], |row| {
            let id: u32 = row.get(0)?;
            Ok(QuestRecord {
                id,
                name: row.get(1)?,
                npc_id: row.get(2)?,
                level_min: row.get(3)?,
                next_quest_id: row.get(4)?,
                requirements: needs.remove(&id).unwrap_or_default(),
            })
        })?;
        Ok(quests.collect::<rusqlite::Result<_>>()?)
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
            })
        })?;
        Ok(drops.collect::<rusqlite::Result<_>>()?)
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
        tx.commit()?;
        Ok(())
    }
}

fn insert_maps(tx: &Transaction, maps: &[MapRecord]) -> Result<()> {
    let mut map = tx.prepare(
        "INSERT OR REPLACE INTO maps (id, name, street, region, is_town, return_map_id, map_mark)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    let mut exit = tx.prepare("INSERT OR IGNORE INTO map_exits (map_id, dest_map_id) VALUES (?1, ?2)")?;
    let mut portal = tx.prepare(
        "INSERT INTO portals (map_id, seq, name, dest_map_id, dest_portal, side, height)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    for m in maps {
        map.execute(params![m.id, m.name, m.street_name, m.region, m.is_town, m.return_map_id, m.map_mark])?;
        for dest in &m.exits {
            exit.execute(params![m.id, dest])?;
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
        "INSERT OR REPLACE INTO monsters (id, name, level, hp, exp, is_boss, undead) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
    )?;
    let mut spawn = tx.prepare(
        "INSERT INTO spawns (mob_id, map_id, count) VALUES (?1, ?2, ?3)
         ON CONFLICT (mob_id, map_id) DO UPDATE SET count = count + excluded.count",
    )?;
    for m in monsters {
        monster.execute(params![m.id, m.name, m.level, m.hp, m.exp, m.is_boss, m.undead])?;
        for s in &m.spawns {
            spawn.execute(params![m.id, s.map_id, s.count])?;
        }
    }
    Ok(())
}

fn insert_npcs(tx: &Transaction, npcs: &[NpcRecord]) -> Result<()> {
    let mut npc = tx.prepare("INSERT OR REPLACE INTO npcs (id, name) VALUES (?1, ?2)")?;
    let mut on_map = tx.prepare("INSERT OR IGNORE INTO npc_maps (npc_id, map_id) VALUES (?1, ?2)")?;
    for n in npcs {
        npc.execute(params![n.id, n.name])?;
        for map in &n.maps {
            on_map.execute(params![n.id, map])?;
        }
    }
    Ok(())
}

fn insert_quests(tx: &Transaction, quests: &[QuestRecord]) -> Result<()> {
    let mut quest = tx.prepare(
        "INSERT OR REPLACE INTO quests (id, name, npc_id, level_min, next_quest_id) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    let mut need = tx.prepare("INSERT INTO quest_mobs (quest_id, mob_id, count) VALUES (?1, ?2, ?3)")?;
    for q in quests {
        quest.execute(params![q.id, q.name, q.npc_id, q.level_min, q.next_quest_id])?;
        for r in &q.requirements {
            need.execute(params![q.id, r.mob_id, r.count])?;
        }
    }
    Ok(())
}

fn insert_skills(tx: &Transaction, skills: &[SkillRecord]) -> Result<()> {
    let mut skill = tx.prepare(
        "INSERT OR REPLACE INTO skills (id, name, job_id, class_name, job_tier, max_level, passive, description)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )?;
    let mut need = tx.prepare("INSERT INTO skill_needs (skill_id, needs_id, level) VALUES (?1, ?2, ?3)")?;
    let mut level = tx.prepare("INSERT OR REPLACE INTO skill_levels (skill_id, level, description) VALUES (?1, ?2, ?3)")?;
    for s in skills {
        skill.execute(params![s.id, s.name, s.job_id, s.class_name, s.job_tier, s.max_level, s.passive, s.description])?;
        for n in &s.required_skills {
            need.execute(params![s.id, n.skill_id, n.level])?;
        }
        for (i, text) in s.level_descriptions.iter().enumerate() {
            level.execute(params![s.id, i as i64 + 1, text])?;
        }
    }
    Ok(())
}

fn insert_items(tx: &Transaction, items: &[ItemRecord]) -> Result<()> {
    let mut item = tx.prepare(
        "INSERT OR REPLACE INTO equipment (id, name, slot, gender, classes, weapon_type, req_level, req_str, req_dex,
                                           req_int, req_luk, stats)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
    )?;
    for i in items {
        let stats = serde_json::to_string(&i.stats).map_err(|err| Error::Invalid(err.to_string()))?;
        item.execute(params![
            i.id,
            i.name,
            i.slot,
            i.gender,
            i.classes.join(CLASS_SEPARATOR),
            i.weapon_type,
            i.req_level,
            i.req_str,
            i.req_dex,
            i.req_int,
            i.req_luk,
            stats
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
    fn nothing_comes_down_until_the_player_asks() {
        assert!(!Store::open_in_memory().unwrap().wants_maps().unwrap());
        assert!(!Store::init(older_cache("")).unwrap().wants_maps().unwrap());
    }
}
