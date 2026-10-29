//! The local SQLite cache of downloaded map data.
//!
//! Each OSMS file is kept with the checksum it was downloaded at, so a later check can skip
//! files that have not changed. Wiki drops are cached per monster with the time they were
//! fetched.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Transaction, params};
use serde::Serialize;

use super::drops::{Drop, MonsterDrops};
use super::osms::{
    Dataset, FILES, FileMeta, MapRecord, Meta, MobRequirement, MonsterRecord, NpcRecord, Portal, QuestRecord, Records,
    Spawn,
};
use crate::error::Result;

/// Bump when the tables change; an older cache is then dropped and downloaded again.
const CACHE_VERSION: i32 = 1;

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
    dest_map_id INTEGER, dest_portal TEXT, PRIMARY KEY (map_id, seq));
CREATE TABLE monsters (
    id INTEGER PRIMARY KEY, name TEXT NOT NULL, level INTEGER NOT NULL, hp INTEGER NOT NULL,
    exp INTEGER NOT NULL, is_boss INTEGER NOT NULL, undead INTEGER NOT NULL);
CREATE TABLE spawns (mob_id INTEGER NOT NULL, map_id INTEGER NOT NULL, count INTEGER NOT NULL, PRIMARY KEY (mob_id, map_id));
CREATE TABLE npcs (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
CREATE TABLE npc_maps (npc_id INTEGER NOT NULL, map_id INTEGER NOT NULL, PRIMARY KEY (npc_id, map_id));
CREATE TABLE quests (id INTEGER PRIMARY KEY, name TEXT NOT NULL, npc_id INTEGER, level_min INTEGER);
CREATE TABLE quest_mobs (quest_id INTEGER NOT NULL, mob_id INTEGER NOT NULL, count INTEGER NOT NULL);
CREATE TABLE drops (
    monster TEXT NOT NULL, item TEXT NOT NULL, category TEXT, rate REAL, rate_low REAL, rate_high REAL,
    min_quantity INTEGER, max_quantity INTEGER, sample_kills INTEGER, sample_drops INTEGER,
    game_version TEXT, quest INTEGER NOT NULL);
CREATE INDEX drops_by_monster ON drops (monster);
CREATE TABLE drop_fetches (monster TEXT PRIMARY KEY, fetched_at INTEGER NOT NULL, sample_kills INTEGER, game_version TEXT);
";

/// The tables that hold each OSMS file, cleared before the file is stored again.
fn tables_of(file: &str) -> &'static [&'static str] {
    match file {
        "maps.json" => &["maps", "map_exits", "portals"],
        "monsters.json" => &["monsters", "spawns"],
        "npcs.json" => &["npcs", "npc_maps"],
        "quests.json" => &["quests", "quest_mobs"],
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
            // A cache from another version of Leafline: start over rather than guess.
            let tables: Vec<String> = conn
                .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")?
                .query_map([], |row| row.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            for table in tables {
                conn.execute(&format!("DROP TABLE \"{table}\""), [])?;
            }
            conn.execute_batch(SCHEMA)?;
            conn.pragma_update(None, "user_version", CACHE_VERSION)?;
        }
        Ok(Self { conn })
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
        }))
    }

    fn load_maps(&self) -> Result<Vec<MapRecord>> {
        let mut exits: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut stmt = self.conn.prepare("SELECT map_id, dest_map_id FROM map_exits ORDER BY rowid")?;
        for row in stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))? {
            let (map, dest) = row?;
            exits.entry(map).or_default().push(dest);
        }
        let mut portals: HashMap<u32, Vec<Portal>> = HashMap::new();
        let mut stmt =
            self.conn.prepare("SELECT map_id, name, dest_map_id, dest_portal FROM portals ORDER BY map_id, seq")?;
        for row in stmt.query_map([], |row| {
            Ok((row.get(0)?, Portal { name: row.get(1)?, dest_map_id: row.get(2)?, dest_portal: row.get(3)? }))
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
        let mut stmt = self.conn.prepare("SELECT id, name, npc_id, level_min FROM quests ORDER BY id")?;
        let quests = stmt.query_map([], |row| {
            let id: u32 = row.get(0)?;
            Ok(QuestRecord {
                id,
                name: row.get(1)?,
                npc_id: row.get(2)?,
                level_min: row.get(3)?,
                requirements: needs.remove(&id).unwrap_or_default(),
            })
        })?;
        Ok(quests.collect::<rusqlite::Result<_>>()?)
    }

    /// Cached drops for `monsters` fetched after `since`, and the monsters that need fetching.
    pub fn cached_drops(&self, monsters: &[String], since: i64) -> Result<(Vec<MonsterDrops>, Vec<String>)> {
        let mut found = Vec::new();
        let mut missing = Vec::new();
        for monster in monsters {
            let fetch: Option<(i64, Option<u32>, Option<String>)> = self
                .conn
                .query_row(
                    "SELECT fetched_at, sample_kills, game_version FROM drop_fetches WHERE monster = ?1",
                    [monster],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()?;
            match fetch {
                Some((fetched_at, sample_kills, game_version)) if fetched_at > since => found.push(MonsterDrops {
                    monster: monster.clone(),
                    sample_kills,
                    game_version,
                    fetched_at,
                    drops: self.drops_of(monster)?,
                }),
                _ => missing.push(monster.clone()),
            }
        }
        Ok((found, missing))
    }

    fn drops_of(&self, monster: &str) -> Result<Vec<Drop>> {
        let mut stmt = self.conn.prepare(
            "SELECT item, category, rate, rate_low, rate_high, min_quantity, max_quantity, sample_kills,
                    sample_drops, game_version, quest
             FROM drops WHERE monster = ?1 ORDER BY rowid",
        )?;
        let drops = stmt.query_map([monster], |row| {
            Ok(Drop {
                // Drops cached before names were unescaped still hold `&#039;`.
                item: super::drops::unescape(&row.get::<_, String>(0)?),
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

    /// Replaces the cached drops of every monster in `fetched`, including ones the wiki had
    /// nothing for, so they are not asked about again until the cache expires.
    pub fn save_drops(&mut self, fetched: &BTreeMap<String, MonsterDrops>) -> Result<()> {
        let tx = self.conn.transaction()?;
        for (monster, entry) in fetched {
            tx.execute("DELETE FROM drops WHERE monster = ?1", [monster])?;
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
    let mut portal =
        tx.prepare("INSERT INTO portals (map_id, seq, name, dest_map_id, dest_portal) VALUES (?1, ?2, ?3, ?4, ?5)")?;
    for m in maps {
        map.execute(params![m.id, m.name, m.street_name, m.region, m.is_town, m.return_map_id, m.map_mark])?;
        for dest in &m.exits {
            exit.execute(params![m.id, dest])?;
        }
        // Portals that lead nowhere (spawn points, scripted ones) say nothing about the way out.
        for (seq, p) in m.portals.iter().filter(|p| p.dest_map_id.is_some()).enumerate() {
            portal.execute(params![m.id, seq as i64, p.name, p.dest_map_id, p.dest_portal])?;
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
    let mut quest =
        tx.prepare("INSERT OR REPLACE INTO quests (id, name, npc_id, level_min) VALUES (?1, ?2, ?3, ?4)")?;
    let mut need = tx.prepare("INSERT INTO quest_mobs (quest_id, mob_id, count) VALUES (?1, ?2, ?3)")?;
    for q in quests {
        quest.execute(params![q.id, q.name, q.npc_id, q.level_min])?;
        for r in &q.requirements {
            need.execute(params![q.id, r.mob_id, r.count])?;
        }
    }
    Ok(())
}
