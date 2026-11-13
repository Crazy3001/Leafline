//! The potions the player hunts with, for mesos per hour after potion costs. The player names
//! their potion slots and sets what each potion costs them; nothing about potions is bundled or
//! read from the game. For a session they type how many they had at the start and how many they
//! have now, and the difference is what the hunt cost.
//!
//! Counts are kept by the session's start time rather than its id, because a session is only
//! saved after a few minutes of play and the player may type the start count before that.

use std::collections::HashMap;

use rusqlite::params;
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

/// Who a set of slots belongs to: a character's name, or empty for the shared set.
fn owner(character: Option<&str>) -> String {
    character.map(str::trim).unwrap_or_default().to_string()
}

/// A session's start time as a key: the same bits the tracker gave it.
fn key(started_at: f64) -> i64 {
    started_at.to_bits() as i64
}

impl UserData {
    /// The potion slots for a character, in slot order. A character with none of its own uses the
    /// shared ones (from before slots were kept per character, or set with no character known)
    /// until it sets its own.
    pub fn potion_slots(&self, character: Option<&str>) -> Result<Vec<PotionSlot>> {
        let own = self.slots_of(&owner(character))?;
        if own.is_empty() && !owner(character).is_empty() {
            return self.slots_of("");
        }
        Ok(own)
    }

    fn slots_of(&self, owner: &str) -> Result<Vec<PotionSlot>> {
        let mut stmt =
            self.conn.prepare("SELECT slot, name, price FROM potion_slots WHERE character = ?1 ORDER BY slot")?;
        let rows = stmt.query_map([owner], |row| Ok(PotionSlot { slot: row.get(0)?, name: row.get(1)?, price: row.get(2)? }))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Replaces the potion slots. A slot keeps its number, so a session's counts stay with it;
    /// a new slot (number 0) takes the next free one. Counts already typed for sessions keep the
    /// name and price they were typed with.
    pub fn set_potion_slots(&mut self, character: Option<&str>, slots: &[PotionSlot]) -> Result<()> {
        let owner = owner(character);
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
        tx.execute("DELETE FROM potion_slots WHERE character = ?1", [&owner])?;
        for slot in slots {
            let number = if slot.slot > 0 {
                slot.slot
            } else {
                let next = (1..).find(|n| !taken.contains(n)).unwrap_or(1);
                taken.insert(next);
                next
            };
            tx.execute(
                "INSERT INTO potion_slots (character, slot, name, price) VALUES (?1, ?2, ?3, ?4)",
                params![owner, number, slot.name.trim(), slot.price],
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
    pub fn set_session_potion(
        &mut self,
        character: Option<&str>,
        started_at: f64,
        slot: u32,
        start: Option<i64>,
        now: Option<i64>,
    ) -> Result<()> {
        if start.is_some_and(|n| n < 0) || now.is_some_and(|n| n < 0) {
            return Err(Error::Invalid("A potion count can't be below zero.".into()));
        }
        if start.is_none() && now.is_none() {
            self.conn
                .execute("DELETE FROM session_potions WHERE started_at = ?1 AND slot = ?2", params![key(started_at), slot])?;
            return Ok(());
        }
        let found = self.potion_slots(character)?.into_iter().find(|s| s.slot == slot).map(|s| (s.name, s.price));
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
        db.set_potion_slots(None, &[slot(" White Potion ", 320), slot("Blue Potion", 200)]).unwrap();
        let slots = db.potion_slots(None).unwrap();
        assert_eq!(slots, [
            PotionSlot { slot: 1, name: "White Potion".into(), price: 320 },
            PotionSlot { slot: 2, name: "Blue Potion".into(), price: 200 },
        ]);
        assert!(db.set_potion_slots(None, &[slot("", 1)]).is_err());
        assert!(db.set_potion_slots(None, &[slot("Elixir", -1)]).is_err());
        assert!(db.set_potion_slots(None, &vec![slot("Red Potion", 50); MAX_SLOTS + 1]).is_err());
        assert_eq!(db.potion_slots(None).unwrap().len(), 2, "a refused change leaves the slots alone");

        // Taking the first away leaves the second at its number; a new one fills the gap.
        let mut kept = db.potion_slots(None).unwrap();
        kept.remove(0);
        kept.push(slot("Mana Elixir", 620));
        db.set_potion_slots(None, &kept).unwrap();
        let numbers: Vec<_> = db.potion_slots(None).unwrap().into_iter().map(|s| (s.slot, s.name)).collect();
        assert_eq!(numbers, [(1, "Mana Elixir".to_string()), (2, "Blue Potion".to_string())]);
    }

    #[test]
    fn a_session_uses_what_it_started_with_minus_what_is_left() {
        let mut db = UserData::open_in_memory().unwrap();
        db.set_potion_slots(None, &[slot("White Potion", 320), slot("Blue Potion", 200)]).unwrap();
        let at = 1_794_000_000.25;
        db.set_session_potion(None, at, 1, Some(300), None).unwrap();
        assert_eq!(db.session_potions(at).unwrap()[0].used, 0, "nothing used until the count now is typed");
        db.set_session_potion(None, at, 1, Some(300), Some(212)).unwrap();
        db.set_session_potion(None, at, 2, Some(100), Some(140)).unwrap();
        let potions = db.session_potions(at).unwrap();
        assert_eq!(potions.iter().map(|p| p.used).collect::<Vec<_>>(), [88, 0], "more than at the start counts as none used");
        assert_eq!(cost_of(&potions), 88 * 320);

        // A new price applies to counts typed afterwards, not to the ones already kept.
        db.set_potion_slots(None, &[PotionSlot { slot: 1, name: "White Potion".into(), price: 400 }]).unwrap();
        assert_eq!(db.session_potions(at).unwrap()[0].price, 320);
        db.set_session_potion(None, at, 1, Some(300), Some(200)).unwrap();
        assert_eq!(db.session_potions(at).unwrap()[0].price, 400);
        assert!(db.set_session_potion(None, at, 2, Some(1), Some(0)).is_err(), "slot 2 is gone");
        db.set_session_potion(None, at, 2, None, None).unwrap();
        assert_eq!(db.session_potions(at).unwrap().len(), 1);
        assert!(db.session_potions(at + 1.0).unwrap().is_empty());
        assert!(db.set_session_potion(None, at, 1, Some(-1), None).is_err());
    }

    #[test]
    fn each_character_has_its_own_slots_and_falls_back_to_the_shared_ones() {
        let mut db = UserData::open_in_memory().unwrap();
        db.set_potion_slots(None, &[slot("White Potion", 320)]).unwrap();
        // Nothing of its own yet: the shared slots show.
        assert_eq!(db.potion_slots(Some("Swiftleaf")).unwrap()[0].name, "White Potion");

        db.set_potion_slots(Some("Swiftleaf"), &[slot("Elixir", 900), slot("Blue Potion", 200)]).unwrap();
        let own: Vec<_> = db.potion_slots(Some("swiftleaf")).unwrap().into_iter().map(|s| s.name).collect();
        assert_eq!(own, ["Elixir", "Blue Potion"], "names match whatever their case");
        assert_eq!(db.potion_slots(Some("Mapleduck")).unwrap()[0].name, "White Potion");
        assert_eq!(db.potion_slots(None).unwrap().len(), 1, "the shared set is left alone");

        // A session's counts take the price of its own character's slot.
        db.set_session_potion(Some("Swiftleaf"), 5.0, 1, Some(10), Some(5)).unwrap();
        assert_eq!(db.session_potions(5.0).unwrap()[0].price, 900);
    }

    #[test]
    fn slots_from_before_characters_move_to_the_shared_set() {
        let db = UserData::open_in_memory().unwrap();
        db.conn.execute("DELETE FROM potion_slots", []).unwrap();
        // The migration copies old rows over as the shared set: the column default does the same.
        db.conn.execute("INSERT INTO potion_slots (slot, name, price) VALUES (1, 'Red Potion', 50)", []).unwrap();
        assert_eq!(db.potion_slots(None).unwrap()[0].name, "Red Potion");
        assert_eq!(db.potion_slots(Some("Anyone")).unwrap()[0].name, "Red Potion");
    }
}
