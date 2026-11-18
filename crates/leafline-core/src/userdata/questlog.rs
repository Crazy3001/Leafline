//! Each character's quest log, as the game's Quest window last showed it. It is only true as of
//! the last time the window was open, so everything carries when it was seen. Reading never
//! forgets a done quest: the Completed tab only ever adds.

use rusqlite::{OptionalExtension, params};
use serde::Serialize;

use super::UserData;
use crate::error::Result;
use crate::readers::questlog::{DetailCounter, DetailReward, QuestDetail, QuestTab, RowIcon};

/// A quest's place in the log as last seen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LogState {
    /// On the Available tab.
    Available,
    /// On the In Progress tab.
    InProgress,
    /// Was in progress, and a full look at the In Progress tab no longer had it: handed in or
    /// dropped. The Completed tab tells which, once it has been looked through.
    Gone,
    /// None of these as of the last look.
    None,
}

impl LogState {
    fn from_key(key: &str) -> Self {
        match key {
            "available" => LogState::Available,
            "inProgress" => LogState::InProgress,
            "gone" => LogState::Gone,
            _ => LogState::None,
        }
    }
}

/// A row of the Quest window, ready to keep.
#[derive(Debug, Clone, PartialEq)]
pub struct SeenQuest {
    /// What the game showed, without the dots of a cut title.
    pub title: String,
    pub cut: bool,
    /// The quest in the data, when the title matched one.
    pub quest_id: Option<u32>,
    pub region: Option<String>,
    pub icon: RowIcon,
    pub on_helper: Option<bool>,
}

impl SeenQuest {
    /// The quest's key in the log: its id when matched, otherwise its title.
    pub fn key(&self) -> String {
        log_key(self.quest_id, &self.title)
    }
}

pub fn log_key(quest_id: Option<u32>, title: &str) -> String {
    match quest_id {
        Some(id) => format!("#{id}"),
        None => format!("t:{}", crate::readers::text::squash(title)),
    }
}

/// What Leafline knows of one quest in a character's log.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggedQuest {
    pub key: String,
    pub title: String,
    pub cut: bool,
    pub quest_id: Option<u32>,
    pub region: Option<String>,
    pub state: LogState,
    /// When `state` was last seen, unix seconds.
    pub state_seen_at: Option<i64>,
    pub done: bool,
    /// The day the game says it was completed, `2026-10-25`, when its detail was read.
    pub done_on: Option<String>,
    pub done_seen_at: Option<i64>,
    pub on_helper: Option<bool>,
    pub icon: Option<RowIcon>,
    /// From the detail pane, when it was open on this quest.
    pub level: Option<u32>,
    pub counters: Vec<DetailCounter>,
    pub talk_to: Option<String>,
    pub rewards: Vec<DetailReward>,
    pub reward_exp: Option<u32>,
    pub reward_mesos: Option<u32>,
    pub detail_seen_at: Option<i64>,
}

/// When a tab was last looked at, and last looked through from top to end.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TabLook {
    pub tab: QuestTab,
    pub seen_at: i64,
    pub whole_at: Option<i64>,
}

/// A region bar's count, as the game last showed it on a tab.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionCount {
    pub tab: QuestTab,
    pub region: String,
    pub count: u32,
    pub seen_at: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestLog {
    pub character: Option<String>,
    pub looks: Vec<TabLook>,
    pub quests: Vec<LoggedQuest>,
    pub regions: Vec<RegionCount>,
}

impl UserData {
    /// Keeps what a look at one tab showed. `whole` says the tab was seen from top to end, so a
    /// quest kept as on it that it no longer lists has left it.
    pub fn record_quest_tab(
        &mut self,
        character: &str,
        tab: QuestTab,
        quests: &[SeenQuest],
        regions: &[(String, Option<u32>)],
        whole: bool,
        now: i64,
    ) -> Result<()> {
        let tx = self.conn.transaction()?;
        for quest in quests {
            let key = quest.key();
            tx.execute(
                "INSERT INTO quest_log (character, key, title, cut, quest_id, region) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT (character, key) DO UPDATE SET
                     title = CASE WHEN excluded.cut AND NOT quest_log.cut THEN quest_log.title ELSE excluded.title END,
                     cut = quest_log.cut AND excluded.cut,
                     region = COALESCE(excluded.region, quest_log.region)",
                params![character, key, quest.title, quest.cut, quest.quest_id, quest.region],
            )?;
            let icon = icon_key(quest.icon);
            match tab {
                QuestTab::Completed => tx.execute(
                    "UPDATE quest_log SET done = 1, done_seen_at = ?3, icon = COALESCE(icon, ?4)
                     WHERE character = ?1 AND key = ?2",
                    params![character, key, now, icon],
                )?,
                QuestTab::InProgress => tx.execute(
                    "UPDATE quest_log SET state = 'inProgress', state_seen_at = ?3, icon = ?4, on_helper = ?5
                     WHERE character = ?1 AND key = ?2",
                    params![character, key, now, icon, quest.on_helper],
                )?,
                QuestTab::Available => tx.execute(
                    "UPDATE quest_log SET state = 'available', state_seen_at = ?3 WHERE character = ?1 AND key = ?2",
                    params![character, key, now],
                )?,
            };
        }
        if whole {
            let keys: Vec<String> = quests.iter().map(SeenQuest::key).collect();
            let listed = serde_json::to_string(&keys).unwrap_or_default();
            // Gone from In Progress: handed in or dropped. No longer offered: taken, or out of reach.
            let (from, to) = match tab {
                QuestTab::InProgress => (Some("inProgress"), "gone"),
                QuestTab::Available => (Some("available"), ""),
                QuestTab::Completed => (None, ""),
            };
            if let Some(from) = from {
                tx.execute(
                    "UPDATE quest_log SET state = ?3, state_seen_at = ?4, on_helper = NULL
                     WHERE character = ?1 AND state = ?2 AND key NOT IN (SELECT value FROM json_each(?5))",
                    params![character, from, to, now, listed],
                )?;
            }
            tx.execute(
                "DELETE FROM quest_log_regions WHERE character = ?1 AND tab = ?2",
                params![character, tab.key()],
            )?;
        }
        for (region, count) in regions {
            let Some(count) = count else { continue };
            tx.execute(
                "INSERT INTO quest_log_regions (character, tab, region, count, seen_at) VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT (character, tab, region) DO UPDATE SET count = excluded.count, seen_at = excluded.seen_at",
                params![character, tab.key(), region, count, now],
            )?;
        }
        tx.execute(
            "INSERT INTO quest_log_looks (character, tab, seen_at, whole_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (character, tab) DO UPDATE SET seen_at = excluded.seen_at,
                 whole_at = COALESCE(excluded.whole_at, quest_log_looks.whole_at)",
            params![character, tab.key(), now, whole.then_some(now)],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// Keeps what the detail pane showed of the quest kept under `key`, adding it if it isn't
    /// kept yet. A completion date makes it done.
    pub fn record_quest_detail(&mut self, character: &str, quest: &SeenQuest, detail: &QuestDetail, now: i64) -> Result<()> {
        let key = quest.key();
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO quest_log (character, key, title, cut, quest_id, region) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (character, key) DO NOTHING",
            params![character, key, quest.title, quest.cut, quest.quest_id, quest.region],
        )?;
        tx.execute(
            "UPDATE quest_log SET level = COALESCE(?3, level), counters = ?4, talk_to = COALESCE(?5, talk_to),
                 rewards = CASE WHEN ?6 = '[]' THEN rewards ELSE ?6 END,
                 reward_exp = COALESCE(?7, reward_exp), reward_mesos = COALESCE(?8, reward_mesos), detail_seen_at = ?9
             WHERE character = ?1 AND key = ?2",
            params![
                character,
                key,
                detail.level,
                serde_json::to_string(&detail.counters).unwrap_or_default(),
                detail.talk_to,
                serde_json::to_string(&detail.rewards).unwrap_or_default(),
                detail.reward_exp,
                detail.reward_mesos,
                now
            ],
        )?;
        if let Some(day) = &detail.completed_on {
            tx.execute(
                "UPDATE quest_log SET done = 1, done_on = ?3, done_seen_at = ?4 WHERE character = ?1 AND key = ?2",
                params![character, key, day, now],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// The character whose Quest window was read last.
    pub fn last_quest_log_character(&self) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT character FROM quest_log_looks ORDER BY seen_at DESC LIMIT 1", [], |row| row.get(0))
            .optional()?)
    }

    /// Everything kept of `character`'s quest log, or of whoever's was read last.
    pub fn quest_log(&self, character: Option<&str>) -> Result<QuestLog> {
        let character = match character {
            Some(name) => Some(name.to_string()),
            None => self.last_quest_log_character()?,
        };
        let Some(name) = character.clone() else { return Ok(QuestLog::default()) };
        let mut stmt = self.conn.prepare(
            "SELECT key, title, cut, quest_id, region, state, state_seen_at, done, done_on, done_seen_at, on_helper,
                    icon, level, counters, talk_to, rewards, reward_exp, reward_mesos, detail_seen_at
             FROM quest_log WHERE character = ?1 ORDER BY title",
        )?;
        let quests = stmt
            .query_map([&name], |row| {
                let counters: Option<String> = row.get(13)?;
                let rewards: Option<String> = row.get(15)?;
                Ok(LoggedQuest {
                    key: row.get(0)?,
                    title: row.get(1)?,
                    cut: row.get(2)?,
                    quest_id: row.get(3)?,
                    region: row.get(4)?,
                    state: LogState::from_key(&row.get::<_, String>(5)?),
                    state_seen_at: row.get(6)?,
                    done: row.get(7)?,
                    done_on: row.get(8)?,
                    done_seen_at: row.get(9)?,
                    on_helper: row.get(10)?,
                    icon: row.get::<_, Option<String>>(11)?.as_deref().and_then(icon_from_key),
                    level: row.get(12)?,
                    counters: counters.and_then(|c| serde_json::from_str(&c).ok()).unwrap_or_default(),
                    talk_to: row.get(14)?,
                    rewards: rewards.and_then(|r| serde_json::from_str(&r).ok()).unwrap_or_default(),
                    reward_exp: row.get(16)?,
                    reward_mesos: row.get(17)?,
                    detail_seen_at: row.get(18)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        let mut stmt = self.conn.prepare("SELECT tab, seen_at, whole_at FROM quest_log_looks WHERE character = ?1")?;
        let looks = stmt
            .query_map([&name], |row| Ok((row.get::<_, String>(0)?, row.get(1)?, row.get(2)?)))?
            .filter_map(|row| {
                let (tab, seen_at, whole_at) = row.ok()?;
                Some(TabLook { tab: QuestTab::from_key(&tab)?, seen_at, whole_at })
            })
            .collect();
        let mut stmt = self
            .conn
            .prepare("SELECT tab, region, count, seen_at FROM quest_log_regions WHERE character = ?1 ORDER BY region")?;
        let regions = stmt
            .query_map([&name], |row| Ok((row.get::<_, String>(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))?
            .filter_map(|row| {
                let (tab, region, count, seen_at) = row.ok()?;
                Some(RegionCount { tab: QuestTab::from_key(&tab)?, region, count, seen_at })
            })
            .collect();
        Ok(QuestLog { character, looks, quests, regions })
    }

    /// Forgets `character`'s quest log, for a player who wants to start it over.
    pub fn forget_quest_log(&mut self, character: &str) -> Result<()> {
        let tx = self.conn.transaction()?;
        for table in ["quest_log", "quest_log_regions", "quest_log_looks"] {
            tx.execute(&format!("DELETE FROM {table} WHERE character = ?1"), [character])?;
        }
        tx.commit()?;
        Ok(())
    }
}

fn icon_key(icon: RowIcon) -> Option<&'static str> {
    match icon {
        RowIcon::Bang => Some("bang"),
        RowIcon::Arrow => Some("arrow"),
        RowIcon::Ring => Some("ring"),
        RowIcon::GreyRing => Some("greyRing"),
        RowIcon::Unknown => None,
    }
}

fn icon_from_key(key: &str) -> Option<RowIcon> {
    match key {
        "bang" => Some(RowIcon::Bang),
        "arrow" => Some(RowIcon::Arrow),
        "ring" => Some(RowIcon::Ring),
        "greyRing" => Some(RowIcon::GreyRing),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(title: &str, id: Option<u32>) -> SeenQuest {
        SeenQuest { title: title.into(), cut: false, quest_id: id, region: Some("Fern Isle".into()), icon: RowIcon::Arrow, on_helper: Some(false) }
    }

    fn state(log: &QuestLog, title: &str) -> LoggedQuest {
        log.quests.iter().find(|q| q.title == title).cloned().unwrap()
    }

    #[test]
    fn a_quest_seen_in_progress_then_missing_from_a_whole_look_is_gone_and_done_stays_done() {
        let mut db = UserData::open_in_memory().unwrap();
        let both = [seen("Pebble Patrol", Some(1)), seen("Owl Feathers", None)];
        db.record_quest_tab("Mossbank", QuestTab::InProgress, &both, &[("Fern Isle".into(), Some(2))], true, 100).unwrap();
        db.record_quest_tab("Mossbank", QuestTab::Completed, &[seen("Moss on the Stones", Some(3))], &[], true, 110).unwrap();

        // A look that isn't whole forgets nothing.
        db.record_quest_tab("Mossbank", QuestTab::InProgress, &both[..1], &[], false, 120).unwrap();
        assert_eq!(state(&db.quest_log(Some("Mossbank")).unwrap(), "Owl Feathers").state, LogState::InProgress);

        db.record_quest_tab("Mossbank", QuestTab::InProgress, &both[..1], &[("Fern Isle".into(), Some(1))], true, 130).unwrap();
        let log = db.quest_log(Some("mossbank")).unwrap();
        let owl = state(&log, "Owl Feathers");
        assert_eq!((owl.state, owl.state_seen_at), (LogState::Gone, Some(130)));
        assert_eq!(state(&log, "Pebble Patrol").state, LogState::InProgress);
        assert!(state(&log, "Moss on the Stones").done);
        assert_eq!(log.regions.len(), 1);
        assert_eq!(log.regions[0].count, 1);

        // A whole look at Completed that lacks it still leaves it done.
        db.record_quest_tab("Mossbank", QuestTab::Completed, &[], &[], true, 140).unwrap();
        assert!(state(&db.quest_log(Some("Mossbank")).unwrap(), "Moss on the Stones").done);
    }

    #[test]
    fn each_character_keeps_its_own_log_and_the_last_looked_at_is_the_default() {
        let mut db = UserData::open_in_memory().unwrap();
        db.record_quest_tab("Mossbank", QuestTab::Available, &[seen("Pebble Patrol", Some(1))], &[], true, 100).unwrap();
        db.record_quest_tab("Fernwick", QuestTab::Completed, &[seen("Pebble Patrol", Some(1))], &[], true, 200).unwrap();
        let moss = db.quest_log(Some("Mossbank")).unwrap();
        assert_eq!(state(&moss, "Pebble Patrol").state, LogState::Available);
        assert!(!state(&moss, "Pebble Patrol").done);
        let last = db.quest_log(None).unwrap();
        assert_eq!(last.character.as_deref(), Some("Fernwick"));
        assert!(state(&last, "Pebble Patrol").done);
        db.record_quest_tab("Mossbank", QuestTab::Available, &[], &[], true, 300).unwrap();
        assert_eq!(state(&db.quest_log(Some("Mossbank")).unwrap(), "Pebble Patrol").state, LogState::None, "no longer offered");
    }

    #[test]
    fn the_detail_pane_adds_counters_and_a_completion_date() {
        let mut db = UserData::open_in_memory().unwrap();
        let quest = seen("Pebble Patrol", Some(1));
        db.record_quest_tab("Mossbank", QuestTab::InProgress, std::slice::from_ref(&quest), &[], true, 100).unwrap();
        let detail = QuestDetail {
            title: "Pebble Patrol".into(),
            level: Some(10),
            counters: vec![DetailCounter { name: "Pebble".into(), have: 30, need: 30 }],
            ..QuestDetail::default()
        };
        db.record_quest_detail("Mossbank", &quest, &detail, 110).unwrap();
        let kept = state(&db.quest_log(Some("Mossbank")).unwrap(), "Pebble Patrol");
        assert_eq!((kept.level, kept.counters.len(), kept.detail_seen_at), (Some(10), 1, Some(110)));
        let done = QuestDetail { completed_on: Some("2026-10-25".into()), ..detail };
        db.record_quest_detail("Mossbank", &quest, &done, 120).unwrap();
        let kept = state(&db.quest_log(Some("Mossbank")).unwrap(), "Pebble Patrol");
        assert!(kept.done);
        assert_eq!(kept.done_on.as_deref(), Some("2026-10-25"));
    }

    #[test]
    fn a_full_title_read_once_is_kept_over_a_cut_one() {
        let mut db = UserData::open_in_memory().unwrap();
        db.record_quest_tab("Mossbank", QuestTab::InProgress, &[seen("Asking After Brin, the Lamplighter", Some(2))], &[], false, 100).unwrap();
        let cut = SeenQuest { title: "Asking After Brin, the La".into(), cut: true, ..seen("", Some(2)) };
        db.record_quest_tab("Mossbank", QuestTab::InProgress, &[cut], &[], false, 110).unwrap();
        let log = db.quest_log(Some("Mossbank")).unwrap();
        assert_eq!(log.quests.len(), 1);
        assert_eq!((log.quests[0].title.as_str(), log.quests[0].cut), ("Asking After Brin, the Lamplighter", false));
    }
}
