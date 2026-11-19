//! What a session picked up, from the Pickup Log: each item and how many, the mesos, and the
//! EXP lines whose colour tells the player's own kills from a partymate's share.
//!
//! The log is only counted while it can be seen, so the rates are over the active time since
//! it was first found. Lines that arrived but didn't read, and items no item in the data
//! matches, are counted apart and add nothing: loot may come out short, never over.

use serde::{Deserialize, Serialize};

/// One line of the log, as the session takes it in.
#[derive(Debug, Clone, PartialEq)]
pub enum Pickup {
    Mesos(u64),
    /// An item, named from the item data. `id` is `None` when no item matched the name read.
    Item { id: Option<u32>, name: String, count: u32 },
    Exp { amount: u64, party: bool },
    PartyBonus(u64),
    /// A line that arrived but didn't read.
    Unread,
}

/// The running totals, kept with the session tracker's state.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LootTally {
    /// The active clock when the log was first seen this session.
    watched_from: Option<f64>,
    mesos: u64,
    items: Vec<LootItem>,
    lines: u32,
    unread: u32,
    unnamed: u32,
    own_exp: u64,
    party_exp: u64,
    party_bonus: u64,
    /// The active clock at the last line that showed party play.
    party_at: Option<f64>,
}

/// One item and how many were picked up.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LootItem {
    pub id: u32,
    pub name: String,
    pub count: u64,
}

/// What the session shows of its loot.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LootSummary {
    /// The log has been seen this session, so the totals mean something.
    pub watched: bool,
    /// Active seconds since it was first seen: what the rates are over.
    pub seconds: f64,
    pub mesos: u64,
    pub mesos_per_hour: Option<f64>,
    /// Most picked up first.
    pub items: Vec<LootItem>,
    /// Lines counted, read or not.
    pub lines: u32,
    pub unread: u32,
    /// Items whose name matched nothing in the item data.
    pub unnamed: u32,
    pub own_exp: u64,
    pub party_exp: u64,
    pub party_bonus: u64,
    /// A partymate's share or party bonus showed in the last five minutes of play.
    pub party_now: bool,
}

/// Seconds of play after a party line that the player still counts as partied.
const PARTY_FOR: f64 = 300.0;

impl LootTally {
    /// The log was seen at active clock `clock`.
    pub fn watching(&mut self, clock: f64) {
        self.watched_from.get_or_insert(clock);
    }

    /// Adds a line that arrived at active clock `clock`.
    pub fn add(&mut self, pickup: &Pickup, clock: f64) {
        self.watching(clock);
        self.lines += 1;
        match pickup {
            Pickup::Mesos(amount) => self.mesos += amount,
            Pickup::Item { id: Some(id), name, count } => match self.items.iter_mut().find(|i| i.id == *id) {
                Some(item) => item.count += u64::from(*count),
                None => self.items.push(LootItem { id: *id, name: name.clone(), count: u64::from(*count) }),
            },
            Pickup::Item { id: None, .. } => self.unnamed += 1,
            Pickup::Exp { amount, party: false } => self.own_exp += amount,
            Pickup::Exp { amount, party: true } => {
                self.party_exp += amount;
                self.party_at = Some(clock);
            }
            Pickup::PartyBonus(amount) => {
                self.party_bonus += amount;
                self.party_at = Some(clock);
            }
            Pickup::Unread => self.unread += 1,
        }
    }

    /// The summary at active clock `clock`, rates shown after `min_seconds` of watching.
    pub fn summary(&self, clock: f64, min_seconds: f64) -> LootSummary {
        let seconds = self.watched_from.map_or(0.0, |from| (clock - from).max(0.0));
        let mut items = self.items.clone();
        items.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
        LootSummary {
            watched: self.watched_from.is_some(),
            seconds,
            mesos: self.mesos,
            mesos_per_hour: (seconds >= min_seconds).then(|| self.mesos as f64 * 3600.0 / seconds),
            items,
            lines: self.lines,
            unread: self.unread,
            unnamed: self.unnamed,
            own_exp: self.own_exp,
            party_exp: self.party_exp,
            party_bonus: self.party_bonus,
            party_now: self.party_at.is_some_and(|at| clock - at <= PARTY_FOR),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: u32, name: &str, count: u32) -> Pickup {
        Pickup::Item { id: Some(id), name: name.into(), count }
    }

    #[test]
    fn items_add_up_by_kind_and_mesos_by_amount() {
        let mut tally = LootTally::default();
        tally.add(&Pickup::Mesos(27), 10.0);
        tally.add(&item(4000000, "Snail Shell", 1), 12.0);
        tally.add(&item(4000000, "Snail Shell", 2), 14.0);
        tally.add(&item(2000000, "Red Potion", 1), 15.0);
        tally.add(&Pickup::Mesos(11), 20.0);
        let summary = tally.summary(10.0 + 1800.0, 30.0);
        assert_eq!(summary.mesos, 38);
        assert_eq!(summary.mesos_per_hour, Some(76.0), "over the half hour since the log was first seen");
        let items: Vec<(&str, u64)> = summary.items.iter().map(|i| (i.name.as_str(), i.count)).collect();
        assert_eq!(items, vec![("Snail Shell", 3), ("Red Potion", 1)]);
        assert_eq!(summary.lines, 5);
    }

    #[test]
    fn unread_lines_and_unknown_items_add_nothing() {
        let mut tally = LootTally::default();
        tally.add(&Pickup::Unread, 1.0);
        tally.add(&Pickup::Item { id: None, name: "Lcaf Xyz".into(), count: 1 }, 2.0);
        let summary = tally.summary(100.0, 30.0);
        assert!(summary.items.is_empty());
        assert_eq!((summary.lines, summary.unread, summary.unnamed), (2, 1, 1));
    }

    #[test]
    fn party_lines_mark_party_play_for_a_while() {
        let mut tally = LootTally::default();
        tally.add(&Pickup::Exp { amount: 20, party: false }, 1.0);
        assert!(!tally.summary(2.0, 30.0).party_now);
        tally.add(&Pickup::Exp { amount: 4, party: true }, 3.0);
        tally.add(&Pickup::PartyBonus(2), 3.0);
        let summary = tally.summary(4.0, 30.0);
        assert!(summary.party_now);
        assert_eq!((summary.own_exp, summary.party_exp, summary.party_bonus), (20, 4, 2));
        assert!(!tally.summary(3.0 + PARTY_FOR + 1.0, 30.0).party_now);
    }

    #[test]
    fn nothing_seen_shows_no_rate() {
        let summary = LootTally::default().summary(500.0, 30.0);
        assert!(!summary.watched);
        assert_eq!(summary.mesos_per_hour, None);
    }
}
