//! The potions the player hunts with, for mesos per hour after potion costs. The player names
//! their potion slots and sets what each potion costs them; nothing about potions is bundled or
//! read from the game. For a session they type how many they had at the start and how many they
//! have now, and the difference is what the hunt cost.
//!
//! Counts are kept by the session's start time rather than its id, because a session is only
//! saved after a few minutes of play and the player may type the start count before that.

use std::collections::HashMap;

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use super::UserData;
use crate::error::{Error, Result};

/// More slots than a hotkey bar holds potions on.
pub const MAX_SLOTS: usize = 8;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PotionSlot {
    pub slot: u32,
    pub name: String,
    /// What one costs the player, in mesos.
    pub price: i64,
}

/// One slot's counts for a session, with the name and price as they were when last typed.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPotion {
    pub slot: u32,
    pub name: String,
    pub price: i64,
    pub start: Option<i64>,
    pub now: Option<i64>,
    /// Start minus now, once both are typed; never below zero.
    pub used: i64,
}

impl SessionPotion {
    pub fn cost(&self) -> i64 {
        self.used * self.price
    }
}

fn used(start: Option<i64>, now: Option<i64>) -> i64 {
    match (start, now) {
        (Some(start), Some(now)) => (start - now).max(0),
        _ => 0,
    }
}

/// A session's start time as a key: the same bits the tracker gave it.
fn key(started_at: f64) -> i64 {
    started_at.to_bits() as i64
}

impl UserData {
    /// The player's potion slots, in slot order.
    pub fn potion_slots(&self) -> Result<Vec<PotionSlot>> {
        let mut stmt = self.conn.prepare("SELECT slot, name, price FROM potion_slots ORDER BY slot")?;
        let rows = stmt.query_map([], |row| Ok(PotionSlot { slot: row.get(0)?, name: row.get(1)?, price: row.get(2)? }))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Replaces the potion slots. A slot keeps its number, so a session's counts stay with it;
    /// a new slot (number 0) takes the next free one. Counts already typed for sessions keep the
    /// name and price they were typed with.
    pub fn set_potion_slots(&mut self, slots: &[PotionSlot]) -> Result<()> {
        if slots.len() > MAX_SLOTS {
            return Err(Error::Invalid(format!("Up to {MAX_SLOTS} potion slots can be set.")));
        }
        for slot in slots {
            if slot.name.trim().is_empty() {
                return Err(Error::Invalid("Each potion slot needs a name.".into()));
            }
            if slot.price < 0 {
                return Err(Error::Invalid(format!("{}: a price can't be below zero.", slot.name.trim())));
            }
        }
        let mut taken: std::collections::HashSet<u32> = slots.iter().map(|s| s.slot).filter(|&n| n > 0).collect();
        if taken.len() != slots.iter().filter(|s| s.slot > 0).count() {
            return Err(Error::Invalid("Two potion slots share a number.".into()));
        }
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM potion_slots", [])?;
        for slot in slots {
            let number = if slot.slot > 0 {
                slot.slot
            } else {
                let next = (1..).find(|n| !taken.contains(n)).unwrap_or(1);
                taken.insert(next);
                next
            };
            tx.execute(
                "INSERT INTO potion_slots (slot, name, price) VALUES (?1, ?2, ?3)",
                params![number, slot.name.trim(), slot.price],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// The counts typed for a session, in slot order.
    pub fn session_potions(&self, started_at: f64) -> Result<Vec<SessionPotion>> {
        let mut stmt = self.conn.prepare(
            "SELECT slot, name, price, start_count, now_count FROM session_potions WHERE started_at = ?1 ORDER BY slot",
        )?;
        let rows = stmt.query_map([key(started_at)], |row| {
            let (start, now) = (row.get(3)?, row.get(4)?);
            Ok(SessionPotion { slot: row.get(0)?, name: row.get(1)?, price: row.get(2)?, start, now, used: used(start, now) })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Every session's counts, by start time.
    pub(super) fn all_session_potions(&self) -> Result<HashMap<i64, Vec<SessionPotion>>> {
        let mut stmt = self.conn.prepare(
            "SELECT started_at, slot, name, price, start_count, now_count FROM session_potions ORDER BY started_at, slot",
        )?;
        let mut by: HashMap<i64, Vec<SessionPotion>> = HashMap::new();
        let rows = stmt.query_map([], |row| {
            let (start, now) = (row.get(4)?, row.get(5)?);
            Ok((
                row.get::<_, i64>(0)?,
                SessionPotion { slot: row.get(1)?, name: row.get(2)?, price: row.get(3)?, start, now, used: used(start, now) },
            ))
        })?;
        for row in rows {
            let (at, potion) = row?;
            by.entry(at).or_default().push(potion);
        }
        Ok(by)
    }

    /// Records a slot's counts for a session, taking the slot's current name and price. With
    /// neither count, the slot's entry is removed.
    pub fn set_session_potion(&mut self, started_at: f64, slot: u32, start: Option<i64>, now: Option<i64>) -> Result<()> {
        if start.is_some_and(|n| n < 0) || now.is_some_and(|n| n < 0) {
            return Err(Error::Invalid("A potion count can't be below zero.".into()));
        }
        if start.is_none() && now.is_none() {
            self.conn
                .execute("DELETE FROM session_potions WHERE started_at = ?1 AND slot = ?2", params![key(started_at), slot])?;
            return Ok(());
        }
        let found: Option<(String, i64)> = self
            .conn
            .query_row("SELECT name, price FROM potion_slots WHERE slot = ?1", [slot], |r| Ok((r.get(0)?, r.get(1)?)))
            .optional()?;
        let Some((name, price)) = found else {
            return Err(Error::Invalid("That potion slot isn't set up any more.".into()));
        };
        self.conn.execute(
            "INSERT INTO session_potions (started_at, slot, name, price, start_count, now_count)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (started_at, slot) DO UPDATE SET
                 name = excluded.name, price = excluded.price,
                 start_count = excluded.start_count, now_count = excluded.now_count",
            params![key(started_at), slot, name, price, start, now],
        )?;
        Ok(())
    }

    /// Drops a session's counts, when the session itself is deleted.
    pub(super) fn delete_session_potions(tx: &rusqlite::Transaction, started_at: Option<f64>) -> Result<()> {
        if let Some(at) = started_at {
            tx.execute("DELETE FROM session_potions WHERE started_at = ?1", [key(at)])?;
        }
        Ok(())
    }
}

/// What a session's potions cost, by its start time.
pub(super) fn cost_of(potions: &[SessionPotion]) -> i64 {
    potions.iter().map(SessionPotion::cost).sum()
}

pub(super) fn session_key(started_at: f64) -> i64 {
    key(started_at)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(name: &str, price: i64) -> PotionSlot {
        PotionSlot { slot: 0, name: name.into(), price }
    }

    #[test]
    fn slots_are_numbered_in_the_order_given() {
        let mut db = UserData::open_in_memory().unwrap();
        db.set_potion_slots(&[slot(" White Potion ", 320), slot("Blue Potion", 200)]).unwrap();
        let slots = db.potion_slots().unwrap();
        assert_eq!(slots, [
            PotionSlot { slot: 1, name: "White Potion".into(), price: 320 },
            PotionSlot { slot: 2, name: "Blue Potion".into(), price: 200 },
        ]);
        assert!(db.set_potion_slots(&[slot("", 1)]).is_err());
        assert!(db.set_potion_slots(&[slot("Elixir", -1)]).is_err());
        assert!(db.set_potion_slots(&vec![slot("Red Potion", 50); MAX_SLOTS + 1]).is_err());
        assert_eq!(db.potion_slots().unwrap().len(), 2, "a refused change leaves the slots alone");

        // Taking the first away leaves the second at its number; a new one fills the gap.
        let mut kept = db.potion_slots().unwrap();
        kept.remove(0);
        kept.push(slot("Mana Elixir", 620));
        db.set_potion_slots(&kept).unwrap();
        let numbers: Vec<_> = db.potion_slots().unwrap().into_iter().map(|s| (s.slot, s.name)).collect();
        assert_eq!(numbers, [(1, "Mana Elixir".to_string()), (2, "Blue Potion".to_string())]);
    }

    #[test]
    fn a_session_uses_what_it_started_with_minus_what_is_left() {
        let mut db = UserData::open_in_memory().unwrap();
        db.set_potion_slots(&[slot("White Potion", 320), slot("Blue Potion", 200)]).unwrap();
        let at = 1_794_000_000.25;
        db.set_session_potion(at, 1, Some(300), None).unwrap();
        assert_eq!(db.session_potions(at).unwrap()[0].used, 0, "nothing used until the count now is typed");
        db.set_session_potion(at, 1, Some(300), Some(212)).unwrap();
        db.set_session_potion(at, 2, Some(100), Some(140)).unwrap();
        let potions = db.session_potions(at).unwrap();
        assert_eq!(potions.iter().map(|p| p.used).collect::<Vec<_>>(), [88, 0], "more than at the start counts as none used");
        assert_eq!(cost_of(&potions), 88 * 320);

        // A new price applies to counts typed afterwards, not to the ones already kept.
        db.set_potion_slots(&[PotionSlot { slot: 1, name: "White Potion".into(), price: 400 }]).unwrap();
        assert_eq!(db.session_potions(at).unwrap()[0].price, 320);
        db.set_session_potion(at, 1, Some(300), Some(200)).unwrap();
        assert_eq!(db.session_potions(at).unwrap()[0].price, 400);
        assert!(db.set_session_potion(at, 2, Some(1), Some(0)).is_err(), "slot 2 is gone");
        db.set_session_potion(at, 2, None, None).unwrap();
        assert_eq!(db.session_potions(at).unwrap().len(), 1);
        assert!(db.session_potions(at + 1.0).unwrap().is_empty());
        assert!(db.set_session_potion(at, 1, Some(-1), None).is_err());
    }
}
