//! Each saved session's loot, as the Pickup Log showed it: the items and how many, and the
//! mesos picked up. Kept by the session's start time, as its potion counts are.

use std::collections::HashMap;

use rusqlite::params;
use serde::Serialize;

use super::UserData;
use super::potions::session_key;
use crate::error::Result;
use crate::loot::{LootItem, LootSummary};

/// A saved session's loot.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionLoot {
    pub mesos: u64,
    /// Active seconds the log was watched.
    pub seconds: f64,
    pub lines: u32,
    pub unread: u32,
    pub unnamed: u32,
    /// Most picked up first.
    pub items: Vec<LootItem>,
}

/// Writes `loot` for the session that started at `started_at`, replacing what was kept.
pub(super) fn write_loot(tx: &rusqlite::Transaction, started_at: Option<f64>, loot: &LootSummary) -> Result<()> {
    let Some(at) = started_at else { return Ok(()) };
    if !loot.watched {
        return Ok(());
    }
    let key = session_key(at);
    tx.execute(
        "INSERT INTO session_pickups (started_at, mesos, seconds, lines, unread, unnamed) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT (started_at) DO UPDATE SET mesos = excluded.mesos, seconds = excluded.seconds, lines = excluded.lines,
             unread = excluded.unread, unnamed = excluded.unnamed",
        params![key, loot.mesos as i64, loot.seconds, loot.lines, loot.unread, loot.unnamed],
    )?;
    tx.execute("DELETE FROM session_loot WHERE started_at = ?1", [key])?;
    for item in &loot.items {
        tx.execute(
            "INSERT INTO session_loot (started_at, item_id, name, count) VALUES (?1, ?2, ?3, ?4)",
            params![key, item.id, item.name, item.count as i64],
        )?;
    }
    Ok(())
}

/// Drops a session's loot, when the session itself is deleted.
pub(super) fn delete_loot(tx: &rusqlite::Transaction, started_at: Option<f64>) -> Result<()> {
    if let Some(at) = started_at {
        tx.execute("DELETE FROM session_pickups WHERE started_at = ?1", [session_key(at)])?;
        tx.execute("DELETE FROM session_loot WHERE started_at = ?1", [session_key(at)])?;
    }
    Ok(())
}

impl UserData {
    /// Every session's loot, by its start time's key.
    pub(super) fn all_session_loot(&self) -> Result<HashMap<i64, SessionLoot>> {
        let mut out: HashMap<i64, SessionLoot> = HashMap::new();
        let mut stmt = self.conn.prepare("SELECT started_at, mesos, seconds, lines, unread, unnamed FROM session_pickups")?;
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                SessionLoot {
                    mesos: row.get::<_, i64>(1)?.max(0) as u64,
                    seconds: row.get(2)?,
                    lines: row.get(3)?,
                    unread: row.get(4)?,
                    unnamed: row.get(5)?,
                    items: Vec::new(),
                },
            ))
        })?;
        for row in rows {
            let (key, loot) = row?;
            out.insert(key, loot);
        }
        let mut stmt = self.conn.prepare("SELECT started_at, item_id, name, count FROM session_loot ORDER BY count DESC, name")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, LootItem { id: row.get(1)?, name: row.get(2)?, count: row.get::<_, i64>(3)?.max(0) as u64 }))
        })?;
        for row in rows {
            let (key, item) = row?;
            if let Some(loot) = out.get_mut(&key) {
                loot.items.push(item);
            }
        }
        Ok(out)
    }
}
