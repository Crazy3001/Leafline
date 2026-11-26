//! Every quest in the data, with where it stands for one character: done, in progress, ready to
//! hand in, one they can take, or one they can't take yet and why. What the game's Quest window
//! showed comes first; the data fills in the rest. A quest that needs another done first shows
//! that one done once it is in progress or done itself, and a series the game lists under its own
//! name stands for the step the player is on. The data has levels and earlier quests but not every
//! condition (jobs, events), so "can take" from the data alone is a guess, and the Available tab,
//! once looked through, says which of those the game doesn't offer.
//!
//! Quests the data chains together are put in questlines: a series' steps, with a quest that leads
//! into it alone. Two series the data names apart stay apart, as the game's Quest window lists them.

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::Serialize;

use super::atlas::{Atlas, QuestRef, fold};
use super::osms::QuestRecord;
use crate::readers::questlog::{DetailCounter, QuestTab, RowIcon};
use crate::userdata::questlog::{LogState, LoggedQuest, QuestLog};

/// Where a quest stands for the character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Stand {
    /// In progress with its counters full, or shown with the ring that stands by a full one.
    Ready,
    InProgress,
    /// On the Available tab, or as far as the data says, one they can start.
    CanTake,
    /// Their level or an earlier quest stands in the way, or the game doesn't offer it.
    NotYet,
    Done,
    /// Was in progress, and a full look at the In Progress tab no longer had it.
    Gone,
}

/// Why a quest can't be taken yet.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotYet {
    /// The level it asks for, when the character is below it.
    pub level: Option<u32>,
    /// Quests to finish first that aren't done.
    pub after: Vec<QuestRef>,
    /// When the Available tab was last looked through without it, though the data says it could
    /// be taken.
    pub not_offered_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WholeQuest {
    /// The quest in the data; none for a quest the game listed that the data lacks.
    pub quest_id: Option<u32>,
    pub title: String,
    pub stand: Stand,
    /// Done because a quest that needs it is in progress or done, not seen on the Completed tab.
    pub inferred: bool,
    /// The game listed it under its series' name, and this is the step the player is likely on.
    pub series: Option<String>,
    pub not_yet: Option<NotYet>,
    pub level_min: Option<u32>,
    pub giver: Option<String>,
    pub repeat: Option<String>,
    /// When the Quest window last showed where it stands.
    pub seen_at: Option<i64>,
    /// The questline it is a step of, when the data chains it with others.
    pub line: Option<LineStep>,
    /// The counts the Quest window's detail pane last showed, while it is in progress.
    pub counters: Vec<DetailCounter>,
}

/// A quest's place in its questline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LineStep {
    /// The line's first quest, which tells it apart from every other line.
    pub head: u32,
    /// The series' name in the data, or the first quest's for a line the data doesn't name.
    pub name: String,
    /// Which step this is, from 1; 0 for a row the game listed by the line's name that couldn't be
    /// put on a step.
    pub step: usize,
    /// How many steps the line has.
    pub of: usize,
}

/// Joins quests into lines, keeping two the data names as different series apart.
struct Lines {
    parent: HashMap<u32, u32>,
    /// A line's series key (its folded name and region), by its root.
    series: HashMap<u32, String>,
}

impl Lines {
    fn root(&mut self, mut id: u32) -> u32 {
        while let Some(&up) = self.parent.get(&id).filter(|&&up| up != id) {
            let top = self.parent.get(&up).copied().unwrap_or(up);
            self.parent.insert(id, top);
            id = up;
        }
        id
    }

    fn join(&mut self, a: u32, b: u32) {
        let (ra, rb) = (self.root(a), self.root(b));
        if ra == rb {
            return;
        }
        let (sa, sb) = (self.series.get(&ra).cloned(), self.series.get(&rb).cloned());
        if sa.is_some() && sb.is_some() && sa != sb {
            return;
        }
        self.parent.insert(ra, rb);
        if let Some(key) = sb.or(sa) {
            self.series.insert(rb, key);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WholeRegion {
    pub region: String,
    /// The quests here the Quest window showed done, or handed in; not those only taken to be done.
    pub done: usize,
    /// The quests the data and the log have here.
    pub total: usize,
    /// The count the game's Completed tab showed for the region, when read. The game counts a
    /// questline once and a repeated quest each time, so it is a check, not a count of quests.
    pub completed_tab: Option<usize>,
    pub quests: Vec<WholeQuest>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WholeLog {
    pub regions: Vec<WholeRegion>,
    /// Whether the Quest window has been read at all; without it everything is from the data.
    pub seen: bool,
    /// The level "not yet" went by, when it was known.
    pub level: Option<u32>,
}

/// A chain longer than this is cut short: no real one is, so a loop in the data stops here.
const CHAIN_LIMIT: usize = 40;

/// Where the log has a quest, by id, after series rows are put on their steps.
struct Placed<'a> {
    logged: &'a LoggedQuest,
    series: Option<String>,
}

fn ready(q: &LoggedQuest) -> bool {
    if q.counters.is_empty() {
        q.icon == Some(RowIcon::Ring)
    } else {
        q.counters.iter().all(|c| c.have >= c.need)
    }
}

impl Atlas {
    /// Every quest in the data and every one the log has that the data lacks, by region, with
    /// where each stands for a character at `level` whose Quest Helper shows `helper`.
    pub fn whole_quest_log(&self, log: &QuestLog, level: Option<u32>, helper: &[String]) -> WholeLog {
        let mut placed: HashMap<u32, Placed> = HashMap::new();
        let mut extras: Vec<&LoggedQuest> = Vec::new();
        let mut series_rows: Vec<&LoggedQuest> = Vec::new();
        for q in &log.quests {
            match q.quest_id {
                Some(id) if self.quests.contains_key(&id) => {
                    placed.insert(id, Placed { logged: q, series: None });
                }
                Some(_) => extras.push(q),
                None if self.series_quests(&q.title, q.region.as_deref()).len() > 1 => series_rows.push(q),
                None => extras.push(q),
            }
        }

        // What is done or under way, and so what every quest before them must be.
        let started = |placed: &HashMap<u32, Placed>| -> HashSet<u32> {
            let mut known: HashSet<u32> = placed
                .iter()
                .filter(|(_, p)| p.logged.done || p.logged.state == LogState::InProgress)
                .map(|(id, _)| *id)
                .collect();
            let mut queue: Vec<u32> = known.iter().copied().collect();
            while let Some(id) = queue.pop() {
                let before = self.quests.get(&id).map(|q| q.follows.clone()).unwrap_or_default();
                let leads: Vec<u32> = self.quests.values().filter(|q| q.next_quest_id == Some(id)).map(|q| q.id).collect();
                for earlier in before.into_iter().chain(leads) {
                    if known.insert(earlier) {
                        queue.push(earlier);
                    }
                }
            }
            known
        };

        // A series row stands for its step on the Quest Helper, or else the first step not done.
        let helped: HashSet<String> = helper.iter().map(|t| fold(t)).collect();
        for row in series_rows {
            let known = started(&placed);
            let steps = self.series_quests(&row.title, row.region.as_deref());
            let on_helper = steps.iter().find(|id| self.quests.get(id).is_some_and(|q| helped.contains(&fold(&q.name))));
            let step = on_helper.or_else(|| steps.iter().find(|id| !known.contains(id) && !placed.contains_key(id)));
            match step {
                Some(&id) if !placed.contains_key(&id) => {
                    placed.insert(id, Placed { logged: row, series: Some(row.title.clone()) });
                }
                _ => extras.push(row),
            }
        }

        let known = started(&placed);
        let done: HashSet<u32> = self
            .quests
            .keys()
            .copied()
            .filter(|id| match placed.get(id) {
                Some(p) => {
                    p.logged.done || p.logged.state == LogState::Gone || (known.contains(id) && p.logged.state != LogState::InProgress)
                }
                None => known.contains(id),
            })
            .collect();
        let available_whole = log.looks.iter().find(|l| l.tab == QuestTab::Available).and_then(|l| l.whole_at);
        let available_at = |q: &LoggedQuest| q.state == LogState::Available;

        let lines = self.questlines();
        // A row the game listed by a line's name, that no step could stand for, still goes with it.
        let named_lines: HashMap<String, &LineStep> = lines.values().filter(|l| l.step == 1).map(|l| (fold(&l.name), l)).collect();

        let mut regions: BTreeMap<String, Vec<WholeQuest>> = BTreeMap::new();
        for quest in self.quests.values() {
            let entry = placed.get(&quest.id);
            let logged = entry.map(|p| p.logged);
            let (stand, inferred, not_yet) = match logged {
                Some(q) if q.state == LogState::InProgress && entry.is_some_and(|p| p.series.is_none() || !done.contains(&quest.id)) => {
                    (if ready(q) { Stand::Ready } else { Stand::InProgress }, false, None)
                }
                Some(q) if q.done => (Stand::Done, false, None),
                Some(q) if q.state == LogState::Gone => (Stand::Gone, false, None),
                Some(q) if available_at(q) => (Stand::CanTake, false, None),
                _ if done.contains(&quest.id) => (Stand::Done, true, None),
                _ => self.can_take(quest, level, &done, available_whole),
            };
            let row = WholeQuest {
                quest_id: Some(quest.id),
                title: quest.name.clone(),
                stand,
                inferred,
                series: entry.and_then(|p| p.series.clone()),
                not_yet,
                level_min: quest.level_min,
                giver: quest.npc_id.and_then(|npc| self.npc_names.get(&npc).cloned()),
                repeat: quest.repeat.clone(),
                seen_at: logged.and_then(|q| if q.done { q.done_seen_at.or(q.state_seen_at) } else { q.state_seen_at }),
                line: lines.get(&quest.id).cloned(),
                counters: logged.filter(|q| q.state == LogState::InProgress).map(|q| q.counters.clone()).unwrap_or_default(),
            };
            regions.entry(quest.region.clone().unwrap_or_else(|| "Other".into())).or_default().push(row);
        }
        for q in extras {
            let stand = match q.state {
                LogState::InProgress if ready(q) => Stand::Ready,
                LogState::InProgress => Stand::InProgress,
                _ if q.done => Stand::Done,
                LogState::Available => Stand::CanTake,
                LogState::Gone => Stand::Gone,
                LogState::None => continue,
            };
            let title = if q.cut { format!("{}…", q.title) } else { q.title.clone() };
            regions.entry(q.region.clone().unwrap_or_else(|| "Other".into())).or_default().push(WholeQuest {
                quest_id: None,
                title,
                stand,
                inferred: false,
                series: None,
                not_yet: None,
                level_min: q.level,
                giver: None,
                repeat: None,
                seen_at: q.state_seen_at,
                line: named_lines.get(&fold(&q.title)).map(|l| LineStep { step: 0, ..(*l).clone() }),
                counters: if q.state == LogState::InProgress { q.counters.clone() } else { Vec::new() },
            });
        }

        let counted: HashMap<&str, usize> = log
            .regions
            .iter()
            .filter(|r| r.tab == QuestTab::Completed)
            .map(|r| (r.region.as_str(), r.count as usize))
            .collect();
        let regions = regions
            .into_iter()
            .map(|(region, mut quests)| {
                quests.sort_by(|a, b| a.level_min.unwrap_or(0).cmp(&b.level_min.unwrap_or(0)).then_with(|| a.title.cmp(&b.title)));
                let done = quests.iter().filter(|q| matches!(q.stand, Stand::Done | Stand::Gone) && !q.inferred).count();
                let completed_tab = counted.get(region.as_str()).copied();
                WholeRegion { total: quests.len(), done, completed_tab, region, quests }
            })
            .collect();
        WholeLog { regions, seen: !log.looks.is_empty(), level }
    }

    /// Every quest in a line of two or more, with its place there. A series' steps make a line;
    /// so do a quest and the next one it leads to, and a quest with the one quest it alone needs
    /// first, when nothing else needs that one. Lines never join two series the data names apart.
    pub fn questlines(&self) -> HashMap<u32, LineStep> {
        let series_key = |q: &QuestRecord| q.chain.as_deref().map(|c| format!("{}|{}", fold(c), q.region.as_deref().unwrap_or("")));
        let mut lines = Lines { parent: self.quests.keys().map(|&id| (id, id)).collect(), series: HashMap::new() };
        let mut by_series: BTreeMap<String, Vec<u32>> = BTreeMap::new();
        for q in self.quests.values() {
            if let Some(key) = series_key(q) {
                lines.series.insert(q.id, key.clone());
                by_series.entry(key).or_default().push(q.id);
            }
        }
        for ids in by_series.values() {
            for &id in &ids[1..] {
                lines.join(ids[0], id);
            }
        }
        // What each quest leads on to: its next quest, and the quests that need it first.
        let mut leads: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut before: HashMap<u32, Vec<u32>> = HashMap::new();
        for q in self.quests.values() {
            let earlier: Vec<u32> = q.follows.iter().copied().filter(|id| *id != q.id && self.quests.contains_key(id)).collect();
            for &e in &earlier {
                let next = leads.entry(e).or_default();
                if !next.contains(&q.id) {
                    next.push(q.id);
                }
            }
            before.insert(q.id, earlier);
            if let Some(next) = q.next_quest_id.filter(|n| *n != q.id && self.quests.contains_key(n)) {
                let after = leads.entry(q.id).or_default();
                if !after.contains(&next) {
                    after.push(next);
                }
            }
        }
        for q in self.quests.values() {
            if let Some(next) = q.next_quest_id.filter(|n| *n != q.id && self.quests.contains_key(n)) {
                lines.join(q.id, next);
            }
            if let [only] = before[&q.id][..]
                && leads.get(&only).is_some_and(|l| l.len() == 1)
            {
                lines.join(only, q.id);
            }
        }

        let mut members: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for &id in self.quests.keys() {
            let root = lines.root(id);
            members.entry(root).or_default().push(id);
        }
        let mut out = HashMap::new();
        for ids in members.into_values().filter(|ids| ids.len() > 1) {
            let order = self.line_order(&ids, &leads);
            let head = order[0];
            let name = order.iter().find_map(|id| self.quests[id].chain.clone()).unwrap_or_else(|| self.quests[&head].name.clone());
            for (i, &id) in order.iter().enumerate() {
                out.insert(id, LineStep { head, name: name.clone(), step: i + 1, of: order.len() });
            }
        }
        out
    }

    /// A line's quests in the order they're done: each after the ones leading to it, the lower
    /// level first where the data doesn't say. A loop in the data ends the walk; the rest follow.
    fn line_order(&self, ids: &[u32], leads: &HashMap<u32, Vec<u32>>) -> Vec<u32> {
        let inside: HashSet<u32> = ids.iter().copied().collect();
        let mut waiting: HashMap<u32, usize> = ids.iter().map(|&id| (id, 0)).collect();
        for id in ids {
            for next in leads.get(id).into_iter().flatten().filter(|n| inside.contains(n)) {
                *waiting.get_mut(next).unwrap() += 1;
            }
        }
        let rank = |id: &u32| (self.quests[id].level_min.unwrap_or(0), *id);
        let mut order: Vec<u32> = Vec::with_capacity(ids.len());
        let mut free: Vec<u32> = ids.iter().copied().filter(|id| waiting[id] == 0).collect();
        while !free.is_empty() && order.len() < CHAIN_LIMIT {
            free.sort_by_key(|id| std::cmp::Reverse(rank(id)));
            let id = free.pop().unwrap();
            order.push(id);
            for next in leads.get(&id).into_iter().flatten().filter(|n| inside.contains(n)) {
                let left = waiting.get_mut(next).unwrap();
                *left -= 1;
                if *left == 0 {
                    free.push(*next);
                }
            }
        }
        let mut rest: Vec<u32> = ids.iter().copied().filter(|id| !order.contains(id)).collect();
        rest.sort_by_key(rank);
        order.extend(rest);
        order
    }

    /// Whether a quest not in the log can be taken, as far as the data and the Available tab say.
    fn can_take(&self, quest: &QuestRecord, level: Option<u32>, done: &HashSet<u32>, available_whole: Option<i64>) -> (Stand, bool, Option<NotYet>) {
        let low = level.zip(quest.level_min).and_then(|(at, min)| (at < min).then_some(min));
        let after: Vec<QuestRef> = quest.follows.iter().filter(|id| !done.contains(id)).filter_map(|id| self.quest_ref(*id)).collect();
        let not_offered_at = (low.is_none() && after.is_empty()).then_some(available_whole).flatten();
        if low.is_none() && after.is_empty() && not_offered_at.is_none() {
            return (Stand::CanTake, false, None);
        }
        (Stand::NotYet, false, Some(NotYet { level: low, after, not_offered_at }))
    }

    /// The quests of the series named `name` (as the data names it), in the order they're done,
    /// within `region` when the data files that series there.
    fn series_quests(&self, name: &str, region: Option<&str>) -> Vec<u32> {
        let key = fold(name);
        let all: Vec<&QuestRecord> = self.quests.values().filter(|q| q.chain.as_deref().is_some_and(|c| fold(c) == key)).collect();
        let here: Vec<&QuestRecord> = all.iter().copied().filter(|q| region.is_some_and(|r| q.region.as_deref() == Some(r))).collect();
        let members = if here.is_empty() { all } else { here };
        let ids: HashSet<u32> = members.iter().map(|q| q.id).collect();
        // Walk each step to the next, from the ones no other step leads to.
        let mut order: Vec<u32> = Vec::new();
        let mut heads: Vec<&QuestRecord> =
            members.iter().copied().filter(|q| !members.iter().any(|m| m.next_quest_id == Some(q.id) || (q.follows.contains(&m.id) && m.id != q.id))).collect();
        heads.sort_by_key(|q| (q.level_min.unwrap_or(0), q.id));
        for head in heads {
            let mut at = Some(head.id);
            while let Some(id) = at.filter(|id| ids.contains(id) && !order.contains(id) && order.len() < CHAIN_LIMIT) {
                order.push(id);
                at = self.quests.get(&id).and_then(|q| q.next_quest_id).or_else(|| members.iter().find(|m| m.follows.contains(&id)).map(|m| m.id));
            }
        }
        let mut rest: Vec<&QuestRecord> = members.into_iter().filter(|q| !order.contains(&q.id)).collect();
        rest.sort_by_key(|q| (q.level_min.unwrap_or(0), q.id));
        order.extend(rest.into_iter().map(|q| q.id));
        order
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapdata::osms::Dataset;
    use crate::readers::questlog::DetailCounter;
    use crate::userdata::questlog::{RegionCount, TabLook};

    fn quest(id: u32, name: &str, region: &str, level: u32) -> QuestRecord {
        QuestRecord { id, name: name.into(), region: Some(region.into()), level_min: Some(level), ..QuestRecord::default() }
    }

    fn atlas() -> Atlas {
        let mut first = quest(1, "Baskets One", "Meadow", 5);
        first.chain = Some("Baskets For Bea".into());
        first.next_quest_id = Some(2);
        let mut second = quest(2, "Baskets Two", "Meadow", 8);
        second.chain = Some("Baskets For Bea".into());
        second.follows = vec![1];
        let mut third = quest(3, "Baskets Three", "Meadow", 10);
        third.chain = Some("Baskets For Bea".into());
        third.follows = vec![2];
        let lone = quest(4, "Lone Errand", "Meadow", 3);
        let high = quest(5, "Far Errand", "Cliffs", 40);
        let mut after = quest(6, "Errand After", "Cliffs", 1);
        after.follows = vec![4];
        Atlas::new(Dataset { quests: vec![first, second, third, lone, high, after], ..Dataset::default() })
    }

    fn logged(id: Option<u32>, title: &str, region: &str, state: LogState, done: bool) -> LoggedQuest {
        LoggedQuest {
            key: title.into(),
            title: title.into(),
            cut: false,
            quest_id: id,
            region: Some(region.into()),
            state,
            state_seen_at: Some(100),
            done,
            done_on: None,
            done_seen_at: done.then_some(100),
            on_helper: None,
            icon: None,
            level: None,
            counters: Vec::new(),
            talk_to: None,
            rewards: Vec::new(),
            reward_exp: None,
            reward_mesos: None,
            detail_seen_at: None,
        }
    }

    fn stands(whole: &WholeLog) -> Vec<(String, Stand, bool)> {
        let mut all: Vec<(String, Stand, bool)> =
            whole.regions.iter().flat_map(|r| r.quests.iter().map(|q| (q.title.clone(), q.stand, q.inferred))).collect();
        all.sort_by(|a, b| a.0.cmp(&b.0));
        all
    }

    fn of<'a>(whole: &'a WholeLog, title: &str) -> &'a WholeQuest {
        whole.regions.iter().flat_map(|r| &r.quests).find(|q| q.title == title).unwrap()
    }

    #[test]
    fn with_no_log_the_data_says_what_can_be_taken_and_why_not() {
        let whole = atlas().whole_quest_log(&QuestLog::default(), Some(6), &[]);
        assert!(!whole.seen);
        assert_eq!(of(&whole, "Baskets One").stand, Stand::CanTake);
        assert_eq!(of(&whole, "Lone Errand").stand, Stand::CanTake);
        let far = of(&whole, "Far Errand");
        assert_eq!((far.stand, far.not_yet.as_ref().unwrap().level), (Stand::NotYet, Some(40)));
        let two = of(&whole, "Baskets Two");
        assert_eq!(two.not_yet.as_ref().unwrap().level, Some(8), "both its level and the step before");
        assert_eq!(two.not_yet.as_ref().unwrap().after.iter().map(|q| q.id).collect::<Vec<_>>(), [1]);
        let without = atlas().whole_quest_log(&QuestLog::default(), None, &[]);
        assert_eq!(of(&without, "Far Errand").stand, Stand::CanTake, "with no level, only earlier quests stand in the way");
    }

    #[test]
    fn a_series_row_stands_for_its_step_and_the_steps_before_are_done() {
        let log = QuestLog {
            character: Some("Tester".into()),
            looks: vec![TabLook { tab: QuestTab::InProgress, seen_at: 100, whole_at: None }],
            quests: vec![logged(None, "Baskets For Bea", "Meadow", LogState::InProgress, false), logged(Some(4), "Lone Errand", "Meadow", LogState::None, true)],
            regions: vec![RegionCount { tab: QuestTab::Completed, region: "Meadow".into(), count: 7, seen_at: 100 }],
        };
        // With nothing else to go by, it is the first step not done.
        let whole = atlas().whole_quest_log(&log, Some(20), &[]);
        assert_eq!(of(&whole, "Baskets One").stand, Stand::InProgress);
        assert_eq!(of(&whole, "Baskets One").series.as_deref(), Some("Baskets For Bea"));
        assert_eq!(of(&whole, "Baskets Two").stand, Stand::NotYet);
        // The Quest Helper names the step.
        let whole = atlas().whole_quest_log(&log, Some(20), &["Baskets Two".into()]);
        assert_eq!(
            stands(&whole),
            [
                ("Baskets One".into(), Stand::Done, true),
                ("Baskets Three".into(), Stand::NotYet, false),
                ("Baskets Two".into(), Stand::InProgress, false),
                ("Errand After".into(), Stand::CanTake, false),
                ("Far Errand".into(), Stand::NotYet, false),
                ("Lone Errand".into(), Stand::Done, false),
            ]
        );
        let meadow = whole.regions.iter().find(|r| r.region == "Meadow").unwrap();
        assert_eq!((meadow.done, meadow.total), (1, 4), "only what the Quest window showed done; Baskets One is taken to be");
        assert_eq!(meadow.completed_tab, Some(7), "the game's own count, kept as a check");
    }

    #[test]
    fn a_series_and_the_quest_leading_into_it_make_one_line_and_named_series_stay_apart() {
        let mut atlas_quests = vec![quest(10, "Lead In", "Town", 2)];
        let mut a = quest(11, "Bells One", "Town", 3);
        a.chain = Some("Ringing Bells".into());
        a.follows = vec![10];
        let mut b = quest(12, "Bells Two", "Town", 3);
        b.chain = Some("Ringing Bells".into());
        let mut c = quest(13, "Bells Again", "Town", 9);
        c.chain = Some("More Bells".into());
        c.follows = vec![12];
        atlas_quests.extend([a, b, c, quest(14, "Alone", "Town", 1)]);
        let atlas = Atlas::new(Dataset { quests: atlas_quests, ..Dataset::default() });
        let lines = atlas.questlines();
        let step = |id: u32| lines.get(&id).map(|l| (l.head, l.name.as_str(), l.step, l.of));
        assert_eq!(step(10), Some((10, "Ringing Bells", 1, 3)), "the quest leading in starts the line, named for its series");
        assert_eq!(step(11), Some((10, "Ringing Bells", 2, 3)));
        assert_eq!(step(12), Some((10, "Ringing Bells", 3, 3)));
        assert_eq!(step(13), None, "a series of one, named apart, is no line");
        assert_eq!(step(14), None);

        // Real data has a series the game lists by its own name: it lands on its step, in its line.
        let mut first = quest(20, "Maya's First Collection", "Henesys", 30);
        first.chain = Some("Collecting For Maya".into());
        let mut second = quest(21, "Maya's Second Collection", "Henesys", 30);
        second.chain = Some("Collecting For Maya".into());
        let atlas = Atlas::new(Dataset { quests: vec![first, second], ..Dataset::default() });
        let mut row = logged(None, "Collecting For Maya", "Henesys", LogState::InProgress, false);
        row.counters = vec![DetailCounter { name: "Cap".into(), have: 28, need: 20 }];
        let log = QuestLog { character: Some("Tester".into()), looks: Vec::new(), quests: vec![row], regions: Vec::new() };
        let whole = atlas.whole_quest_log(&log, Some(34), &["Maya's First Collection".into()]);
        let on = of(&whole, "Maya's First Collection");
        assert_eq!(on.stand, Stand::Ready, "a count past its target is kept, and full");
        assert_eq!(on.counters[0].have, 28);
        assert_eq!(on.line.as_ref().map(|l| (l.name.as_str(), l.step, l.of)), Some(("Collecting For Maya", 1, 2)));
        assert!(whole.regions.iter().flat_map(|r| &r.quests).all(|q| q.quest_id.is_some()), "nothing left out as not in the data");
    }

    #[test]
    fn full_counters_are_ready_and_a_quest_the_data_lacks_is_kept() {
        let mut errand = logged(Some(4), "Lone Errand", "Meadow", LogState::InProgress, false);
        errand.counters = vec![DetailCounter { name: "Leaves".into(), have: 10, need: 10 }];
        let log = QuestLog {
            character: Some("Tester".into()),
            looks: vec![TabLook { tab: QuestTab::Available, seen_at: 100, whole_at: Some(90) }],
            quests: vec![errand, logged(None, "Festival Lanterns", "Event", LogState::InProgress, false)],
            regions: Vec::new(),
        };
        let whole = atlas().whole_quest_log(&log, Some(6), &[]);
        assert_eq!(of(&whole, "Lone Errand").stand, Stand::Ready);
        assert_eq!(of(&whole, "Festival Lanterns").quest_id, None);
        // The Available tab was looked through without Baskets One, so the game doesn't offer it.
        let one = of(&whole, "Baskets One");
        assert_eq!((one.stand, one.not_yet.as_ref().unwrap().not_offered_at), (Stand::NotYet, Some(90)));
    }
}
