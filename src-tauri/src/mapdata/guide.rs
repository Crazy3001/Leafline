//! Where to go for the quests the Quest Helper shows: which monster each objective needs, the
//! nearest maps with a fair number of it, and how many portals away each map is.
//!
//! An objective names either a monster to defeat or an item to collect. It is taken for a
//! monster when it reads like one in the data; otherwise it is an item, and the monsters that
//! drop it come from players' logs on mapleclassic.wiki, looked up separately.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::atlas::{Atlas, MapRef, QuestRef, Shortcut, fold, similarity};
use super::drops::{ItemSources, item_key};
use super::osms::{MonsterRecord, QuestRecord};
use super::training::{Measured, MeasuredRate};
use super::travel::Travel;
use crate::readers::quest::{Objective, Quest, QuestReading};

/// How closely an objective must match a monster's name to be taken for that monster.
const MIN_MONSTER_SCORE: f64 = 0.85;
/// Looser when the quest the title matched asks for that monster.
const MIN_QUEST_MONSTER_SCORE: f64 = 0.7;
/// How closely a dropper's name on the wiki must match a monster in the map data.
pub(super) const MIN_DROPPER_SCORE: f64 = 0.9;
const MIN_QUEST_SCORE: f64 = 0.8;
/// Below this a title is another quest, not a bad reading of one already known.
const MIN_HELD_SCORE: f64 = 0.6;
/// How closely a line read must match a known line of the same quest to update it. Above
/// `Blue Snail` against `Red Snail` (0.6), which are two lines, not one misread.
const MIN_LINE_SCORE: f64 = 0.7;
/// A title this close to a quest in the data, short of matching it, is a garbled reading.
const MIN_ROUGH_SCORE: f64 = 0.5;
/// Words of a title shorter than this say too little to look a quest up by.
const MIN_TITLE_WORD: usize = 4;
/// A quest's one-line hint names the best few; an item's own page lists every one.
const MAPS_PER_MONSTER: usize = 4;
const DROPPERS_PER_ITEM: usize = 4;

/// How many readings in a row a known quest can go unread before it counts as gone, or the
/// panel as closed. A few, so a bad moment shows nothing, but closing it soon shows.
const HELD_READINGS: u32 = 4;

/// A quest matched in this live run and what it last counted.
struct Known {
    name: String,
    /// The quest in the data, once the title or counts matched one. Its lines then come from
    /// the data, and readings only give the counts.
    id: Option<u32>,
    objectives: Vec<Objective>,
    unread: u32,
    /// Seen in two readings in a row. Until then it isn't shown: one garbled reading can
    /// match a quest the panel doesn't hold.
    confirmed: bool,
    /// A count read lower than shown, by line: kills never go down, so a lower count waits for
    /// a second reading that agrees.
    falling: Vec<(String, u32)>,
}

/// The quests matched so far in a live run. OCR can read one title differently from one moment
/// to the next as the scenery moves behind the panel, so once a quest is known it stays: a
/// reading too rough to match keeps the quest it most resembles, one with no counter keeps the
/// last counter, and one that loses the quest altogether shows it a little longer.
#[derive(Default)]
pub struct QuestTitles {
    known: Vec<Known>,
    last: Option<QuestReading>,
    missed: u32,
    /// Titles no quest in the data matched, with how many readings in a row have read them.
    unmatched: Vec<(String, u32)>,
}

/// Readings in a row a title no quest matches must read the same before it shows. An event
/// quest the data lacks reads the same each time; a garbled line rarely does.
const UNMATCHED_READINGS: u32 = 3;

impl QuestTitles {
    /// Steadies one live reading of the panel. `None` when the panel wasn't found this time.
    pub fn hold(&mut self, atlas: &Atlas, reading: &mut Option<QuestReading>) {
        let Some(read) = reading else {
            self.missed += 1;
            if self.missed <= HELD_READINGS {
                reading.clone_from(&self.last);
            } else {
                *self = Self::default();
            }
            return;
        };
        self.missed = 0;
        if read.page.is_some_and(|(shown, _)| shown == 0) {
            *self = Self::default();
            return;
        }
        self.settle(atlas, &mut read.quests);
        self.keep_known(read);
        self.last = Some(read.clone());
    }

    /// Takes out rows that are no quest, and puts back known quests that went unread.
    fn keep_known(&mut self, read: &mut QuestReading) {
        let is_known = |known: &[Known], title: &str| known.iter().any(|k| k.name == title);
        // A row with no counter that is no quest found is a stray line, such as `o Slime`.
        read.quests.retain(|q| !q.objectives.is_empty() || is_known(&self.known, &q.title));
        let unconfirmed = |known: &[Known], title: &str| known.iter().any(|k| k.name == title && !k.confirmed);
        read.quests.retain(|q| !unconfirmed(&self.known, &q.title));
        // A title no quest matched: a counter line taken for a title (`Of8Pjg`, `15.1`) holds
        // a digit and is no quest; any other shows once it has read the same a few times.
        let unmatched: Vec<String> = read
            .quests
            .iter()
            .filter(|q| !q.title.is_empty() && !is_known(&self.known, &q.title))
            .map(|q| q.title.clone())
            .collect();
        let before = |title: &String| self.unmatched.iter().find(|(t, _)| t == title).map_or(0, |(_, n)| *n);
        let streaks: Vec<(String, u32)> = unmatched.iter().map(|t| (t.clone(), before(t) + 1)).collect();
        self.unmatched = streaks;
        let steady = |title: &String| self.unmatched.iter().any(|(t, n)| t == title && *n >= UNMATCHED_READINGS);
        read.quests.retain(|q| {
            !unmatched.contains(&q.title) || (!q.title.contains(|c: char| c.is_ascii_digit()) && steady(&q.title))
        });
        let mut seen: Vec<String> = Vec::new();
        read.quests.retain(|q| {
            let new = q.title.is_empty() || !seen.contains(&q.title);
            seen.push(q.title.clone());
            new
        });
        for known in &mut self.known {
            known.unread = if seen.contains(&known.name) { 0 } else { known.unread + 1 };
        }
        self.known.retain(|k| k.unread <= HELD_READINGS);
        let room = read.page.map_or(usize::MAX, |(shown, _)| shown as usize);
        for known in &self.known {
            if known.confirmed && known.unread > 0 && read.quests.len() < room {
                read.quests.push(Quest { title: known.name.clone(), objectives: known.objectives.clone() });
            }
        }
        let place = |q: &Quest| self.known.iter().position(|k| k.name == q.title).unwrap_or(usize::MAX);
        read.quests.sort_by_key(place);
    }

    /// Gives each quest the data's name and lines, from its title and what it counts, or the
    /// known quest it most resembles. A quest the data has shows the data's lines, with the
    /// counts read; one it lacks shows the lines read, steadied.
    pub fn settle(&mut self, atlas: &Atlas, quests: &mut [Quest]) {
        for quest in quests {
            quest.objectives.retain(|o| !is_noise(&o.target));
            let mut id = atlas.quest_for(&quest.title, &quest.objectives);
            if let Some(found) = id {
                quest.title = atlas.quests[&found].name.clone();
            } else if let Some(i) = self.closest(&quest.title, &quest.objectives) {
                quest.title = self.known[i].name.clone();
                id = self.known[i].id;
            } else {
                // A title near a quest in the data, but not near enough, is a garbled reading of
                // it: too rough to show until it matches. One like no quest there (an event quest
                // the data lacks, say) is shown as read.
                if atlas.quest_resemblance(&quest.title) >= MIN_ROUGH_SCORE {
                    quest.title.clear();
                }
                continue;
            }
            let lines = id.and_then(|id| atlas.quest_lines(id));
            let position = self.known.iter().position(|k| k.name == quest.title);
            let known = match position {
                Some(i) => {
                    self.known[i].confirmed = true;
                    &mut self.known[i]
                }
                None => {
                    let objectives = lines.clone().unwrap_or_default();
                    self.known.push(Known {
                        name: quest.title.clone(),
                        id,
                        objectives,
                        unread: 0,
                        confirmed: false,
                        falling: Vec::new(),
                    });
                    self.known.last_mut().expect("just pushed")
                }
            };
            if let Some(lines) = lines {
                let item_name = |item: u32| atlas.item_names.get(&item).cloned();
                let items = id.map(|id| atlas.quest_items(id, item_name)).unwrap_or_default();
                if known.id.is_none() || known.objectives.len() != lines.len() {
                    // Matched in the data only now: start from its lines, keeping counts read.
                    let before = std::mem::replace(&mut known.objectives, lines);
                    fill_counts(&mut known.objectives, &before, &mut Vec::new(), &items);
                }
                known.id = id;
                fill_counts(&mut known.objectives, &quest.objectives, &mut known.falling, &items);
            } else {
                if let Some(found) = id {
                    atlas.fit_counts(found, &mut quest.objectives);
                }
                merge_lines(&mut known.objectives, &quest.objectives, &mut known.falling);
            }
            quest.objectives.clone_from(&known.objectives);
        }
    }

    /// The known quest counting the same things, or else with the most similar name. Every line
    /// read resembling one of a known quest's lines counts as the same things: with scenery
    /// behind the panel a title can read as `Of8Pig` while `Orange Muehroom` under it still
    /// names Olaf's Second Training. Between two quests whose lines both fit, the closer name wins.
    fn closest(&self, title: &str, objectives: &[Objective]) -> Option<usize> {
        let query = fold(title);
        self.known
            .iter()
            .enumerate()
            .map(|(i, known)| {
                let name = similarity(&query, &fold(&known.name));
                let same = !objectives.is_empty() && pair_lines(&known.objectives, objectives).len() == objectives.len();
                (if same { 1.0 + name / 100.0 } else { name }, i)
            })
            .filter(|(score, _)| *score >= MIN_HELD_SCORE)
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, i)| i)
    }
}

/// An objective read through scenery behind the see-through panel, such as `Slime 81413171`
/// where a counter ran into the line below. No quest target has a run of three digits.
fn is_noise(target: &str) -> bool {
    let mut run = 0;
    for c in target.chars() {
        run = if c.is_ascii_digit() { run + 1 } else { 0 };
        if run >= 3 {
            return true;
        }
    }
    target.chars().filter(|c| c.is_alphabetic()).count() < 3
}

/// The known line each line read resembles closest, in read order: `(read, known)` pairs.
/// Each known line is taken at most once.
fn pair_lines(known: &[Objective], read: &[Objective]) -> Vec<(usize, usize)> {
    let mut taken = vec![false; known.len()];
    let mut pairs = Vec::new();
    for (r, line) in read.iter().enumerate() {
        let target = fold(&line.target);
        let best = known
            .iter()
            .enumerate()
            .filter(|(k, _)| !taken[*k])
            .map(|(k, old)| (similarity(&target, &fold(&old.target)), old.required == line.required, k))
            .filter(|(score, _, _)| *score >= MIN_LINE_SCORE)
            .max_by(|a, b| (a.0 + f64::from(u8::from(a.1)) * 0.1).total_cmp(&(b.0 + f64::from(u8::from(b.1)) * 0.1)));
        if let Some((_, _, k)) = best {
            taken[k] = true;
            pairs.push((r, k));
        }
    }
    pairs
}

/// Takes the counts read into a quest's lines from the data, whose names and targets stay. A
/// line not read keeps its count. A kill count above its line's target is a misread, since the
/// game stops counting there, but an item line counts what the inventory holds, so `9 / 5
/// Leather` is right: `items` names those lines. Once a reading shows every line, the lines
/// take its order, the Quest Helper's.
fn fill_counts(lines: &mut [Objective], read: &[Objective], falling: &mut Vec<(String, u32)>, items: &[String]) {
    let pairs = pair_lines(lines, read);
    for &(r, k) in &pairs {
        if read[r].current <= lines[k].required || items.contains(&lines[k].target) {
            take_count(&mut lines[k], read[r].current, falling);
            lines[k].unread = false;
        }
    }
    if pairs.len() == lines.len() {
        let mut order = vec![0; lines.len()];
        for &(r, k) in &pairs {
            order[k] = r;
        }
        let mut keyed: Vec<(usize, Objective)> = order.into_iter().zip(lines.iter().cloned()).collect();
        keyed.sort_by_key(|(r, _)| *r);
        for (slot, (_, line)) in lines.iter_mut().zip(keyed) {
            *slot = line;
        }
    }
}

/// Takes a count read into a line. A higher one shows at once; a lower one (a misread, as kills
/// never go down, or an item used up) only when the next reading of the line agrees.
fn take_count(line: &mut Objective, read: u32, falling: &mut Vec<(String, u32)>) {
    let waiting = falling.iter().position(|(target, _)| *target == line.target);
    let agreed = waiting.is_some_and(|i| falling[i].1 == read);
    if let Some(i) = waiting {
        falling.remove(i);
    }
    if read >= line.current || agreed {
        line.current = read;
    } else {
        falling.push((line.target.clone(), read));
    }
}

/// Steadies the lines of a quest the data lacks: a line read updates the line it resembles, a
/// line not read keeps its last count, and a new line is added.
fn merge_lines(known: &mut Vec<Objective>, read: &[Objective], falling: &mut Vec<(String, u32)>) {
    let pairs = pair_lines(known, read);
    for (r, line) in read.iter().enumerate() {
        match pairs.iter().find(|(pr, _)| *pr == r) {
            Some(&(_, k)) => {
                if line.current <= line.required {
                    take_count(&mut known[k], line.current, falling);
                }
                if similarity(&fold(&line.target), &fold(&known[k].target)) >= MIN_QUEST_SCORE {
                    known[k].target.clone_from(&line.target);
                    known[k].required = line.required;
                }
            }
            None => known.push(line.clone()),
        }
    }
}

/// One quest as the Quest Helper showed it.
#[derive(Debug, Clone, Deserialize)]
pub struct QuestQuery {
    pub title: String,
    /// What each objective names, as read.
    pub objectives: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestGuide {
    pub title: String,
    /// The quest in the data whose name the title matched, if any.
    pub quest: Option<QuestInfo>,
    pub objectives: Vec<ObjectiveGuide>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestInfo {
    pub quest: QuestRef,
    pub giver: Option<String>,
    /// Where the quest giver stands.
    pub giver_maps: Vec<MapRef>,
    /// Who takes the quest in, when something shows it. Not always the giver.
    pub turn_in: Option<String>,
    /// Where they stand.
    pub turn_in_maps: Vec<MapRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetKind {
    Monster,
    Item,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ObjectiveGuide {
    /// The objective as read.
    pub target: String,
    pub kind: TargetKind,
    /// For an item: the wiki's name for it, once looked up and found.
    pub item: Option<String>,
    /// For an item: whether the wiki has been asked who drops it.
    pub looked_up: bool,
    /// Monsters to hunt: the one named, or the item's droppers, highest drop rate first.
    pub hunts: Vec<Hunt>,
}

/// Where to get an item.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemOrigin {
    pub drops: ObjectiveGuide,
    /// Quests that can give it, the lowest level first.
    pub quests: Vec<QuestInfo>,
    /// NPC shops that sell it, the cheapest first.
    pub shops: Vec<super::shops::ShopOffer>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Hunt {
    pub monster: MonsterRef,
    /// For an item: how often players saw this monster drop it.
    pub drop: Option<DropRate>,
    /// The nearest maps with a fair number of this monster first (see `nearest_first`).
    pub maps: Vec<HuntingGround>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonsterRef {
    pub name: String,
    /// False for a dropper the wiki names that the map data has no monster for.
    pub known: bool,
    pub level: u32,
    pub hp: u32,
    pub exp: u32,
    pub boss: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DropRate {
    /// Percent, with its confidence interval.
    pub rate: Option<f64>,
    pub rate_low: Option<f64>,
    pub rate_high: Option<f64>,
    pub sample_kills: Option<u32>,
    pub game_version: Option<String>,
    /// Drops only while a quest wants it.
    pub quest: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HuntingGround {
    pub map: MapRef,
    /// How many of the monster the map spawns.
    pub count: u32,
    /// How many monsters of any kind the map spawns.
    pub map_total: u32,
    /// Portals from the current map, when it is known and a route exists.
    pub hops: Option<u32>,
    /// Fewer steps with the travel the player turned on, when that is shorter.
    pub travel: Option<Shortcut>,
    /// The player's EXP rate measured here, as kills of this monster.
    pub pace: Option<Pace>,
}

/// A rough kill rate for one monster from the EXP per hour the player measured on its map. It
/// assumes the player defeats the map's monsters in proportion to how many of each spawn, and
/// earns the map data's base EXP for them, so EXP events and parties throw it off.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Pace {
    pub exp_per_hour: f64,
    pub active_seconds: f64,
    pub kills_per_hour: f64,
}

/// Sorts the versions of one monster: the one that spawns most, which players meet, comes first.
pub(super) fn main_first(monster: &MonsterRecord) -> (std::cmp::Reverse<u32>, u32) {
    (std::cmp::Reverse(monster.spawns.iter().map(|s| s.count).sum()), monster.id)
}

/// A map with fewer than this share of the richest map's spawns is listed after the others,
/// however close: a quest's few kills go faster on a fuller map a few portals further.
const THIN_SHARE: u32 = 4;

/// Orders hunting grounds for someone about to walk there: maps they can reach first, then the
/// fewest steps away (counting travel they turned on), and most spawns among equals. Maps with
/// only a few of the monster go after the rest. Without a known position no map has a route,
/// and count alone decides.
pub(super) fn nearest_first(maps: &mut [HuntingGround]) {
    let richest = maps.iter().map(|m| m.count).max().unwrap_or(0);
    let thin = |m: &HuntingGround| m.count * THIN_SHARE < richest;
    let steps = |m: &HuntingGround| m.travel.as_ref().map(|t| t.hops).or(m.hops);
    maps.sort_by(|a, b| {
        steps(a)
            .is_none()
            .cmp(&steps(b).is_none())
            .then(thin(a).cmp(&thin(b)))
            .then(steps(a).unwrap_or(u32::MAX).cmp(&steps(b).unwrap_or(u32::MAX)))
            .then(b.count.cmp(&a.count))
            .then_with(|| a.map.name.cmp(&b.map.name))
    });
}

/// What the guide knows about the player, beyond the map they are on.
#[derive(Debug, Default)]
pub(super) struct Context {
    pub(super) hops: HashMap<u32, u32>,
    pub(super) shortcuts: HashMap<u32, Shortcut>,
    measured: HashMap<u32, Measured>,
}

impl Atlas {
    /// What the guide can tell from the map the player is on, the rates they measured and the
    /// rides and scrolls they turned on.
    pub(super) fn context(&self, from: Option<u32>, measured: &[MeasuredRate], travel: &Travel) -> Context {
        let hops = from.map(|id| self.hops_from(id)).unwrap_or_default();
        let shortcuts = from.map(|id| self.shortcuts_from(id, travel, &hops)).unwrap_or_default();
        Context { hops, shortcuts, measured: self.measured_by_map(measured) }
    }

    /// A guide for each quest. `from` is the map the player is on, for portal counts; `items`
    /// holds the drop table's answers for item objectives; `measured` the EXP rates the
    /// player recorded, matched to maps by name; `travel` the rides and scrolls turned on.
    pub fn quest_guide(
        &self,
        quests: &[QuestQuery],
        from: Option<u32>,
        items: &[ItemSources],
        measured: &[MeasuredRate],
        travel: &Travel,
    ) -> Vec<QuestGuide> {
        let context = self.context(from, measured, travel);
        quests
            .iter()
            .map(|query| {
                let quest = self.find_quest(&query.title);
                let wanted: Vec<u32> = quest
                    .and_then(|id| self.quests.get(&id))
                    .map(|q| q.requirements.iter().map(|r| r.mob_id).collect())
                    .unwrap_or_default();
                let objectives =
                    query.objectives.iter().map(|target| self.objective(target, &wanted, items, &context)).collect();
                QuestGuide { title: query.title.clone(), quest: quest.and_then(|id| self.quest_info(id)), objectives }
            })
            .collect()
    }

    fn objective(&self, target: &str, wanted: &[u32], items: &[ItemSources], context: &Context) -> ObjectiveGuide {
        let named = self
            .monsters_named(target, Some(wanted))
            .filter(|(score, _)| *score >= MIN_QUEST_MONSTER_SCORE)
            .or_else(|| self.monsters_named(target, None).filter(|(score, _)| *score >= MIN_MONSTER_SCORE));
        if let Some((_, ids)) = named {
            return ObjectiveGuide {
                target: target.to_string(),
                kind: TargetKind::Monster,
                item: None,
                looked_up: false,
                hunts: vec![self.hunt(&ids, None, context, false)],
            };
        }

        let key = item_key(target);
        self.item_objective(target, items.iter().find(|s| s.key == key), context, false)
    }

    /// An item and the monsters players saw drop it. `sources` is `None` until it is looked up.
    /// `every` lists all of them and all their maps, for the item's own page.
    fn item_objective(&self, target: &str, sources: Option<&ItemSources>, context: &Context, every: bool) -> ObjectiveGuide {
        let hunts = sources
            .into_iter()
            .flat_map(|s| &s.droppers)
            .take(if every { usize::MAX } else { DROPPERS_PER_ITEM })
            .map(|dropper| {
                let drop = DropRate {
                    rate: dropper.rate,
                    rate_low: dropper.rate_low,
                    rate_high: dropper.rate_high,
                    sample_kills: dropper.sample_kills,
                    game_version: dropper.game_version.clone(),
                    quest: dropper.quest,
                };
                match self.monsters_named(&dropper.monster, None).filter(|(score, _)| *score >= MIN_DROPPER_SCORE) {
                    Some((_, ids)) => self.hunt(&ids, Some(drop), context, every),
                    None => Hunt {
                        monster: MonsterRef {
                            name: dropper.monster.clone(),
                            known: false,
                            level: 0,
                            hp: 0,
                            exp: 0,
                            boss: false,
                        },
                        drop: Some(drop),
                        maps: Vec::new(),
                    },
                }
            })
            .collect();
        ObjectiveGuide {
            target: target.to_string(),
            kind: TargetKind::Item,
            item: sources.and_then(|s| s.item.clone()),
            looked_up: sources.is_some(),
            hunts,
        }
    }

    /// Where to get an item by its data ids (all of an item's copies): who drops it, as a quest
    /// item's objective would show, and the quests that can give it. `from` is the map the player
    /// is on, for portal counts.
    pub fn item_origin(
        &self,
        items: &[u32],
        name: &str,
        sources: Option<&ItemSources>,
        from: Option<u32>,
        travel: &Travel,
    ) -> ItemOrigin {
        let context = self.context(from, &[], travel);
        let mut quests: Vec<u32> = items.iter().filter_map(|id| self.quests_rewarding.get(id)).flatten().copied().collect();
        quests.sort();
        quests.dedup();
        ItemOrigin {
            shops: self.sold_by(items, name, |map| context.hops.get(&map).copied()),
            drops: self.item_objective(name, sources, &context, true),
            quests: quests.iter().filter_map(|id| self.quest_info(*id)).collect(),
        }
    }

    /// The monsters (by ID) whose name best matches `name`, and how well. Several IDs can share
    /// a name; all of them are returned so their spawns add up. `among` limits the search.
    pub(super) fn monsters_named(&self, name: &str, among: Option<&[u32]>) -> Option<(f64, Vec<u32>)> {
        let query = fold(name);
        if query.is_empty() {
            return None;
        }
        let candidates: Vec<&MonsterRecord> = match among {
            Some(ids) => ids.iter().filter_map(|id| self.monsters.get(id)).collect(),
            None => self.monsters.values().collect(),
        };
        let (score, best) = candidates
            .iter()
            .map(|m| {
                let key = fold(&m.name);
                (similarity(&query, &key), key)
            })
            .max_by(|a, b| a.0.total_cmp(&b.0).then_with(|| b.1.cmp(&a.1)))?;
        let mut ids: Vec<u32> = self.monsters.values().filter(|m| fold(&m.name) == best).map(|m| m.id).collect();
        ids.sort_unstable();
        Some((score, ids))
    }

    /// Where to hunt the monster with these IDs: the best few maps, or `every` one.
    fn hunt(&self, ids: &[u32], drop: Option<DropRate>, context: &Context, every: bool) -> Hunt {
        let records: Vec<&MonsterRecord> = ids.iter().filter_map(|id| self.monsters.get(id)).collect();
        // Of IDs sharing a name, the one that spawns most is the one players meet.
        let main = records.iter().copied().min_by_key(|m| main_first(m)).expect("hunt needs a known monster");
        let mut maps = self.hunting_grounds(&records, context);
        if !every {
            maps.truncate(MAPS_PER_MONSTER);
        }
        Hunt {
            monster: MonsterRef {
                name: main.name.clone(),
                known: true,
                level: main.level,
                hp: main.hp,
                exp: main.exp,
                boss: main.is_boss,
            },
            drop,
            maps,
        }
    }

    /// Every map these monsters spawn on, the best first.
    pub(super) fn hunting_grounds(&self, records: &[&MonsterRecord], context: &Context) -> Vec<HuntingGround> {
        let mut counts: HashMap<u32, u32> = HashMap::new();
        for spawn in records.iter().flat_map(|m| &m.spawns).filter(|s| s.count > 0) {
            *counts.entry(spawn.map_id).or_default() += spawn.count;
        }
        let mut maps: Vec<HuntingGround> = counts
            .into_iter()
            .filter(|(id, _)| self.maps.contains_key(id))
            .map(|(id, count)| HuntingGround {
                map: self.map_ref(id),
                count,
                map_total: self.spawns.get(&id).into_iter().flatten().map(|(_, count)| count).sum(),
                hops: context.hops.get(&id).copied(),
                travel: context.shortcuts.get(&id).cloned(),
                pace: self.pace(id, count, context),
            })
            .collect();
        nearest_first(&mut maps);
        maps
    }

    /// Kills per hour of a monster that spawns `count` times on map `id`, from the EXP rate the
    /// player measured there: their EXP per hour is so many clears of the map, and each clear
    /// has `count` of the monster.
    fn pace(&self, id: u32, count: u32, context: &Context) -> Option<Pace> {
        let measured = context.measured.get(&id)?;
        let per_clear = self.card(id)?.exp_per_clear;
        (per_clear > 0 && measured.exp_per_hour > 0.0).then(|| Pace {
            exp_per_hour: measured.exp_per_hour,
            active_seconds: measured.active_seconds,
            kills_per_hour: measured.exp_per_hour * f64::from(count) / per_clear as f64,
        })
    }

    /// The quest a reading of the panel means: by its title, or when that lost words, by a name
    /// holding what is left of it and by what it counts, if only one quest fits.
    fn quest_for(&self, title: &str, objectives: &[Objective]) -> Option<u32> {
        if let Some(id) = self.find_quest(title) {
            return Some(id);
        }
        let query = fold(title);
        let words: Vec<&str> = query.split_whitespace().filter(|w| w.len() >= MIN_TITLE_WORD).collect();
        // The words looked up must say most of the title: one word of a longer line, such as
        // `place` in a speech bubble's `place for you`, names nothing.
        let letters = |s: &str| s.chars().filter(|c| !c.is_whitespace()).count();
        let telling = words.iter().map(|w| letters(w)).sum::<usize>() * 10 >= letters(&query) * 6;
        let named = |q: &&QuestRecord| {
            let name = fold(&q.name);
            telling && !words.is_empty() && words.iter().all(|w| name.contains(w))
        };
        let counts = |q: &&QuestRecord| {
            !objectives.is_empty() && objectives.iter().all(|o| self.count_of(q, &o.target) == Some(o.required))
        };
        let only = |found: Vec<u32>| (found.len() == 1).then(|| found[0]);
        let quests = || self.quests.values();
        only(quests().filter(named).filter(counts).map(|q| q.id).collect())
            .or_else(|| only(quests().filter(counts).map(|q| q.id).collect()))
            .or_else(|| only(quests().filter(named).map(|q| q.id).collect()))
    }

    /// The monster called `target` that a quest asks for, and how many.
    fn wanted(&self, quest: &QuestRecord, target: &str) -> Option<(&str, u32)> {
        let target = fold(target);
        quest.requirements.iter().find_map(|need| {
            let monster = self.monsters.get(&need.mob_id).filter(|m| fold(&m.name) == target)?;
            Some((monster.name.as_str(), need.count))
        })
    }

    fn count_of(&self, quest: &QuestRecord, target: &str) -> Option<u32> {
        self.wanted(quest, target).map(|(_, count)| count)
    }

    /// The lines the Quest Helper shows for a quest, from the data: the monsters to defeat, then
    /// the items to bring, each with its target and a count of 0. `None` when the data names
    /// none, or misses a name.
    fn quest_lines(&self, id: u32) -> Option<Vec<Objective>> {
        let quest = self.quests.get(&id)?;
        let line = |target: &String, required| Objective { target: target.clone(), current: 0, required, unread: true };
        let mobs = quest.requirements.iter().map(|r| Some(line(&self.monsters.get(&r.mob_id)?.name, r.count)));
        let items = quest.items.iter().map(|i| Some(line(self.item_names.get(&i.item_id)?, i.count)));
        let lines: Option<Vec<Objective>> = mobs.chain(items).collect();
        lines.filter(|l| !l.is_empty())
    }

    /// Corrects what a quest counts to what the data says it asks for, spelled the data's way.
    fn fit_counts(&self, id: u32, objectives: &mut [Objective]) {
        let Some(quest) = self.quests.get(&id) else { return };
        for objective in objectives {
            if let Some((name, count)) = self.wanted(quest, &objective.target) {
                objective.target = name.to_string();
                objective.required = count;
            }
        }
    }

    /// How closely the title resembles the most similar quest name in the data, 0 to 1. A
    /// title of a few letters resembles nothing.
    fn quest_resemblance(&self, title: &str) -> f64 {
        let query = fold(title);
        if query.len() < 4 {
            return 1.0;
        }
        self.quests.values().map(|q| similarity(&query, &fold(&q.name))).fold(0.0, f64::max)
    }

    /// The quest whose name the Quest Helper's title matches. Long titles are cut short with
    /// `...` there, so those are compared with the start of each name.
    fn find_quest(&self, title: &str) -> Option<u32> {
        let trimmed = title.trim_end();
        // OCR can read the ellipsis as `..` or `. . .` as well as `...`.
        let dots = trimmed.chars().rev().take_while(|c| matches!(c, '.' | '…' | ' ' | '·')).filter(|c| *c != ' ').count();
        let truncated = dots >= 2 || trimmed.ends_with('…');
        let query = fold(trimmed);
        if query.len() < 4 {
            return None;
        }
        self.quests
            .values()
            .map(|q| {
                let name = fold(&q.name);
                let start: String = name.chars().take(query.chars().count()).collect();
                // A long reading that stands for the start of a name counts as that name even with no dots,
                // as when OCR lost them.
                let score = if truncated {
                    similarity(&query, &start)
                } else if query.len() >= 14 {
                    similarity(&query, &name).max(similarity(&query, &start))
                } else {
                    similarity(&query, &name)
                };
                (score, q.id)
            })
            .filter(|(score, _)| *score >= MIN_QUEST_SCORE)
            .max_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
            .map(|(_, id)| id)
    }

    pub(super) fn quest_info(&self, id: u32) -> Option<QuestInfo> {
        let quest = self.quests.get(&id)?;
        let name = |npc: Option<u32>| npc.and_then(|n| self.npc_names.get(&n).cloned());
        let maps = |npc: Option<u32>| npc.map(|n| self.npc_maps(n)).unwrap_or_default();
        let turn_in = self.turn_in(quest);
        Some(QuestInfo {
            quest: self.quest_ref(id)?,
            giver: name(quest.npc_id),
            giver_maps: maps(quest.npc_id),
            turn_in: name(turn_in),
            turn_in_maps: maps(turn_in),
        })
    }

    /// Where an NPC stands.
    pub(super) fn npc_maps(&self, npc: u32) -> Vec<MapRef> {
        let mut maps: Vec<MapRef> = self
            .npcs_on
            .iter()
            .filter(|(_, npcs)| npcs.contains(&npc))
            .map(|(map, _)| self.map_ref(*map))
            .filter(|m| m.known)
            .collect();
        maps.sort_by_key(|m| m.id);
        maps
    }

    /// Who a quest is handed in to. OSMS's data has no field for it, so this is the game
    /// client's own answer from Leafline's quest file where it has one. Without that file it goes
    /// by what shows it: the next quest of a chain starts with them, or the title ends
    /// `to Heena`. Failing that it stays unknown: the giver is not a safe guess, being someone
    /// else on more than half of all quests.
    pub(super) fn turn_in(&self, quest: &QuestRecord) -> Option<u32> {
        let next = quest.next_quest_id.and_then(|id| self.quests.get(&id)).and_then(|q| q.npc_id);
        self.turn_ins.get(&quest.id).copied().or(next).or_else(|| {
            let (_, name) = quest.name.rsplit_once(" to ")?;
            let name = name.trim_end_matches(['.', '!', '?']);
            let mut named: Vec<u32> =
                self.npc_names.iter().filter(|(_, n)| n.as_str() == name).map(|(id, _)| *id).collect();
            named.sort_unstable();
            named.into_iter().find(|&id| !self.npc_maps(id).is_empty())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mapdata::atlas::tests::{island_world, world};
    use crate::mapdata::drops::Dropper;
    use crate::mapdata::osms::ItemNeed;
    use crate::readers::quest::Objective;

    fn query(title: &str, objectives: &[&str]) -> QuestQuery {
        QuestQuery { title: title.into(), objectives: objectives.iter().map(|o| o.to_string()).collect() }
    }

    fn wing_sources() -> ItemSources {
        let dropper = |monster: &str, rate: f64| Dropper {
            monster: monster.into(),
            rate: Some(rate),
            rate_low: None,
            rate_high: None,
            sample_kills: Some(200),
            game_version: None,
            quest: true,
        };
        ItemSources {
            key: "rat whisker".into(),
            item: Some("Rat Whisker".into()),
            fetched_at: 0,
            droppers: vec![dropper("Burrow Rat", 12.0), dropper("Sewer King", 50.0)],
        }
    }

    #[test]
    fn monster_objectives_list_the_best_maps_and_portals_away() {
        let atlas = world();
        // From the inn: out to Mossbank, east to Meadow I, east again to Meadow II.
        let guide = atlas.quest_guide(&[query("Shell Sortng", &["Pebble Snai1"])], Some(5), &[], &[], &Travel::none());
        let quest = guide[0].quest.as_ref().unwrap();
        assert_eq!(quest.quest.name, "Shell Sorting");
        assert_eq!(quest.giver.as_deref(), Some("Warden Ilse"));
        assert_eq!(quest.giver_maps.iter().map(|m| m.id).collect::<Vec<_>>(), [1]);

        let objective = &guide[0].objectives[0];
        assert_eq!(objective.kind, TargetKind::Monster);
        let hunt = &objective.hunts[0];
        assert_eq!((hunt.monster.name.as_str(), hunt.monster.level), ("Pebble Snail", 2));
        let maps: Vec<(&str, u32, Option<u32>)> =
            hunt.maps.iter().map(|g| (g.map.name.as_str(), g.count, g.hops)).collect();
        assert_eq!(maps, [("Mossbank Meadow I", 6, Some(2)), ("Mossbank Meadow II", 2, Some(3))]);
    }

    fn ground(name: &str, count: u32, hops: Option<u32>) -> HuntingGround {
        HuntingGround {
            map: MapRef { id: 0, name: name.into(), street: None, known: true },
            count,
            map_total: count,
            hops,
            travel: None,
            pace: None,
        }
    }

    #[test]
    fn hunting_grounds_list_the_nearest_fair_map_first() {
        // Made-up maps: a rich one far off, fair ones near, a thin one next door, one with no route.
        let mut maps = vec![
            ground("Far Grove", 28, Some(19)),
            ground("Lost Isle", 30, None),
            ground("Next Door", 2, Some(1)),
            ground("Shore II", 10, Some(4)),
            ground("Shore I", 11, Some(2)),
            ground("Shore Twin", 12, Some(2)),
        ];
        nearest_first(&mut maps);
        let names: Vec<&str> = maps.iter().map(|m| m.map.name.as_str()).collect();
        assert_eq!(names, ["Shore Twin", "Shore I", "Shore II", "Far Grove", "Next Door", "Lost Isle"]);

        // With no position known, the richest comes first.
        let mut unplaced = vec![ground("Shore I", 11, None), ground("Far Grove", 28, None)];
        nearest_first(&mut unplaced);
        assert_eq!(unplaced[0].map.name, "Far Grove");
    }

    #[test]
    fn a_garbled_title_keeps_the_quest_read_before_it() {
        let atlas = world();
        let mut titles = QuestTitles::default();
        let mut settle = |title: &str, target: &str| {
            let objectives = vec![Objective { target: target.into(), current: 0, required: 8, unread: false }];
            let mut quests = [Quest { title: title.into(), objectives }];
            titles.settle(&atlas, &mut quests);
            quests[0].title.clone()
        };
        const GARBLED: &str = "Sheli Sor 101ng";
        assert_eq!(settle(GARBLED, "Fern Moth"), "", "nothing to hold on to yet, and too rough to name");
        assert_eq!(settle(GARBLED, "Pebble Snail"), "Shell Sorting", "only this quest counts these");
        assert_eq!(settle("Shell Sortng", "Pebble Snail"), "Shell Sorting");
        assert_eq!(settle(GARBLED, "Pebble Snail"), "Shell Sorting");
        assert_eq!(settle(GARBLED, "Pebb1e Snail"), "Shell Sorting", "resembles it, counter or not");
        assert_eq!(settle("", "Pebble Snail"), "Shell Sorting", "the title lost, the same counter");
        assert_eq!(settle("Another Errand", "Moss Slime"), "Another Errand");
        assert_eq!(settle("", "Moss Slime"), "");
    }

    fn reading(quests: Vec<Quest>, page: Option<(u32, u32)>) -> Option<QuestReading> {
        Some(QuestReading { quests, page, ..QuestReading::default() })
    }

    fn quest(title: &str, counted: &[(&str, u32, u32)]) -> Quest {
        let objectives = counted
            .iter()
            .map(|(target, current, required)| Objective {
                target: (*target).into(),
                current: *current,
                required: *required,
                unread: false,
            })
            .collect();
        Quest { title: title.into(), objectives }
    }

    fn held(titles: &mut QuestTitles, atlas: &Atlas, mut read: Option<QuestReading>) -> Option<QuestReading> {
        titles.hold(atlas, &mut read);
        read
    }

    #[test]
    fn a_title_that_lost_a_word_finds_its_quest() {
        let atlas = world();
        assert_eq!(atlas.quest_for("Sorting", &[]), Some(70), "only one name holds it");
        let wanted = [Objective { target: "Moss Slime".into(), current: 0, required: 5, unread: false }];
        assert_eq!(atlas.quest_for("Xq", &wanted), Some(71), "only one quest counts that");
        assert_eq!(atlas.quest_for("Xq", &[]), None);
        assert_eq!(atlas.quest_for("is just the sorting for you", &[]), None, "one word of a longer line");
    }

    #[test]
    fn a_locked_quest_keeps_its_counter_through_a_bad_reading() {
        let (atlas, mut titles) = (world(), QuestTitles::default());
        held(&mut titles, &atlas, reading(vec![quest("Slime Samples", &[("Moss Slime", 2, 5)])], None));
        let bad = held(&mut titles, &atlas, reading(vec![quest("Slime Samples", &[])], None)).unwrap();
        assert_eq!(bad.quests, [quest("Slime Samples", &[("Moss Slime", 2, 5)])]);
    }

    #[test]
    fn scenery_behind_the_panel_cannot_drop_or_garble_a_known_line() {
        // Seen live on Teo's Weird Hobby with a palm leaf behind the Quest Helper: one reading
        // lost the second line, the next ran a counter into the line below.
        let (atlas, mut titles) = (world(), QuestTitles::default());
        let full = quest("Shell Sorting", &[("Pebble Snail", 1, 8), ("Burrow Rat", 0, 2)]);
        held(&mut titles, &atlas, reading(vec![full.clone()], None));
        let lost = held(&mut titles, &atlas, reading(vec![quest("Shell Sorting", &[("Pebble Snail", 1, 8)])], None));
        assert_eq!(lost.unwrap().quests, std::slice::from_ref(&full), "the line not read keeps its last count");
        let garbled = quest("Shell Sorting", &[("Pebble Snail 81413171", 0, 1), ("Burrow Rat", 1, 2)]);
        let read = held(&mut titles, &atlas, reading(vec![garbled], None)).unwrap();
        assert_eq!(read.quests, [quest("Shell Sorting", &[("Pebble Snail", 1, 8), ("Burrow Rat", 1, 2)])]);
        let misread = quest("Shell Sorting", &[("Pebb1e Snai1", 2, 8), ("Stray Line", 0, 4)]);
        let read = held(&mut titles, &atlas, reading(vec![misread], None)).unwrap();
        assert_eq!(
            read.quests,
            [quest("Shell Sorting", &[("Pebble Snail", 2, 8), ("Burrow Rat", 1, 2)])],
            "a misread line updates its count; the data says there are only two lines"
        );
    }

    #[test]
    fn a_quest_held_by_resemblance_takes_the_datas_lines_and_targets() {
        // Seen live after a restart: rough readings were held before the data matched, and kept
        // targets of 0 and 50 and two junk lines.
        let (mut atlas, mut titles) = (world(), QuestTitles::default());
        atlas.item_names.insert(95, "Snail Shell".into());
        atlas.quests.get_mut(&70).unwrap().items.push(ItemNeed { item_id: 95, count: 1 });
        let rough = quest("Sheli Sor 101ng", &[("Pebble Snail", 1, 0), ("Stl.tmr", 1, 3), ("Tillie", 2, 0)]);
        held(&mut titles, &atlas, reading(vec![rough], None));
        let garbled = quest("Shell Sortinq", &[("Snail Shel", 0, 1), ("Pebble Snail", 2, 50), ("Burrow Rot", 1, 2)]);
        held(&mut titles, &atlas, reading(vec![garbled.clone()], None));
        let read = held(&mut titles, &atlas, reading(vec![garbled], None)).unwrap();
        let lines = [("Snail Shell", 0, 1), ("Pebble Snail", 2, 8), ("Burrow Rat", 1, 2)];
        assert_eq!(read.quests, [quest("Shell Sorting", &lines)], "the data's lines, in the order read");
        let junk = quest("Shell Sorting", &[("Tillie", 9, 9), ("Pebble Snail", 9, 8)]);
        let read = held(&mut titles, &atlas, reading(vec![junk], None)).unwrap();
        assert_eq!(read.quests, [quest("Shell Sorting", &lines)], "no junk line, no kill count over its target");
    }

    #[test]
    fn an_item_line_can_count_past_its_target() {
        // Seen live: the Quest Helper showed `9 / 5 Leather`, and Leafline showed 0/5.
        let (mut atlas, mut titles) = (world(), QuestTitles::default());
        atlas.item_names.insert(95, "Snail Shell".into());
        atlas.quests.get_mut(&70).unwrap().items.push(ItemNeed { item_id: 95, count: 5 });
        let full = quest("Shell Sorting", &[("Pebble Snail", 8, 8), ("Burrow Rat", 2, 2), ("Snail Shell", 9, 5)]);
        held(&mut titles, &atlas, reading(vec![full.clone()], None));
        let read = held(&mut titles, &atlas, reading(vec![full.clone()], None)).unwrap();
        assert_eq!(read.quests, [full], "nine shells held for five wanted");
        let over = quest("Shell Sorting", &[("Pebble Snail", 9, 8), ("Burrow Rat", 2, 2), ("Snail Shell", 9, 5)]);
        let read = held(&mut titles, &atlas, reading(vec![over], None)).unwrap();
        assert_eq!(read.quests[0].objectives[0].current, 8, "kills still stop at the target");
    }

    #[test]
    fn a_title_lost_behind_scenery_is_found_by_its_lines() {
        // Seen live: a stone platform behind the panel turned "0 / 8 Pig" into the title Of8Pig.
        let (atlas, mut titles) = (world(), QuestTitles::default());
        let both = vec![
            quest("Shell Sorting", &[("Pebble Snail", 1, 8), ("Burrow Rat", 0, 2)]),
            quest("Slime Samples", &[("Moss Slime", 2, 5)]),
        ];
        held(&mut titles, &atlas, reading(both.clone(), None));
        held(&mut titles, &atlas, reading(both.clone(), None));
        let rough = vec![quest("Of1Peb", &[("Burrow Rot", 1, 2)]), quest("Slime Samples", &[("Moss Slime", 2, 5)])];
        let read = held(&mut titles, &atlas, reading(rough, None)).unwrap();
        let expected = [
            quest("Shell Sorting", &[("Pebble Snail", 1, 8), ("Burrow Rat", 1, 2)]),
            quest("Slime Samples", &[("Moss Slime", 2, 5)]),
        ];
        assert_eq!(read.quests, expected);
    }

    #[test]
    fn a_title_no_quest_matches_shows_only_once_it_reads_the_same_a_few_times() {
        let (atlas, mut titles) = (world(), QuestTitles::default());
        let event = || reading(vec![quest("Another Errand", &[("Moss Slime", 0, 3)])], None);
        let shown = |read: Option<QuestReading>| read.unwrap().quests.len();
        assert_eq!(shown(held(&mut titles, &atlas, event())), 0);
        assert_eq!(shown(held(&mut titles, &atlas, event())), 0);
        assert_eq!(shown(held(&mut titles, &atlas, event())), 1, "an event quest the data lacks");
        let line = || reading(vec![quest("Of8Pjg", &[("Cit 3rge", 0, 8)])], None);
        for _ in 0..5 {
            assert_eq!(shown(held(&mut titles, &atlas, line())), 0, "a counter line taken for a title");
        }
    }

    #[test]
    fn a_line_missed_on_the_first_reading_is_added_later() {
        let (atlas, mut titles) = (world(), QuestTitles::default());
        held(&mut titles, &atlas, reading(vec![quest("Shell Sorting", &[("Pebble Snail", 1, 8)])], None));
        let full = quest("Shell Sorting", &[("Pebble Snail", 1, 8), ("Burrow Rat", 0, 2)]);
        assert_eq!(held(&mut titles, &atlas, reading(vec![full.clone()], None)).unwrap().quests, [full]);
    }

    #[test]
    fn a_quest_shows_once_two_readings_in_a_row_find_it() {
        // Seen live: one reading during a turn-in matched a quest the panel didn't hold.
        let (atlas, mut titles) = (world(), QuestTitles::default());
        let samples = quest("Slime Samples", &[("Moss Slime", 2, 5)]);
        held(&mut titles, &atlas, reading(vec![samples.clone()], None));
        held(&mut titles, &atlas, reading(vec![samples.clone()], None));
        let stray = quest("Shell Sortng", &[("Pebble Snail", 0, 8)]);
        let read = held(&mut titles, &atlas, reading(vec![samples.clone(), stray], None)).unwrap();
        assert_eq!(read.quests, std::slice::from_ref(&samples), "one reading is not enough");
        assert_eq!(held(&mut titles, &atlas, reading(vec![samples.clone()], None)).unwrap().quests, [samples]);
    }

    #[test]
    fn a_count_going_down_waits_for_a_second_reading() {
        // Seen live: 10 / 10 read once as 1 / 10.
        let (atlas, mut titles) = (world(), QuestTitles::default());
        let at = |n| reading(vec![quest("Slime Samples", &[("Moss Slime", n, 5)])], None);
        let count = |read: Option<QuestReading>| read.unwrap().quests[0].objectives[0].current;
        held(&mut titles, &atlas, at(4));
        assert_eq!(count(held(&mut titles, &atlas, at(5))), 5, "up shows at once");
        assert_eq!(count(held(&mut titles, &atlas, at(1))), 5, "one low reading is a misread");
        assert_eq!(count(held(&mut titles, &atlas, at(5))), 5);
        assert_eq!(count(held(&mut titles, &atlas, at(0))), 5);
        assert_eq!(count(held(&mut titles, &atlas, at(0))), 0, "two in a row: an item used up");
    }

    #[test]
    fn the_data_corrects_what_a_quest_counts() {
        let (atlas, mut titles) = (world(), QuestTitles::default());
        let samples = || reading(vec![quest("Slime Samples", &[("Moss Slime", 2, 6)])], None);
        held(&mut titles, &atlas, samples());
        let read = held(&mut titles, &atlas, samples()).unwrap();
        assert_eq!(read.quests[0].objectives[0].required, 5);
    }

    #[test]
    fn a_row_that_is_no_quest_goes_and_a_lost_quest_comes_back_in_order() {
        let (atlas, mut titles) = (world(), QuestTitles::default());
        let both = vec![
            quest("Shell Sorting", &[("Pebble Snail", 1, 8), ("Burrow Rat", 0, 2)]),
            quest("Slime Samples", &[("Moss Slime", 2, 5)]),
        ];
        held(&mut titles, &atlas, reading(both.clone(), None));
        held(&mut titles, &atlas, reading(both.clone(), None));
        let rough = vec![quest("O Slime", &[]), quest("Slime Samples", &[("Moss Slime", 2, 5)])];
        assert_eq!(held(&mut titles, &atlas, reading(rough, None)).unwrap().quests, both);
    }

    #[test]
    fn a_quest_unread_for_long_is_gone() {
        let (atlas, mut titles) = (world(), QuestTitles::default());
        held(&mut titles, &atlas, reading(vec![quest("Slime Samples", &[("Moss Slime", 2, 5)])], None));
        held(&mut titles, &atlas, reading(vec![quest("Slime Samples", &[("Moss Slime", 2, 5)])], None));
        for _ in 0..HELD_READINGS {
            assert_eq!(held(&mut titles, &atlas, reading(vec![], None)).unwrap().quests.len(), 1);
        }
        assert!(held(&mut titles, &atlas, reading(vec![], None)).unwrap().quests.is_empty());
    }

    #[test]
    fn the_panel_going_missing_shows_for_a_moment_then_clears() {
        let (atlas, mut titles) = (world(), QuestTitles::default());
        held(&mut titles, &atlas, reading(vec![quest("Slime Samples", &[("Moss Slime", 2, 5)])], None));
        for _ in 0..HELD_READINGS {
            assert!(held(&mut titles, &atlas, None).is_some());
        }
        assert!(held(&mut titles, &atlas, None).is_none());
    }

    #[test]
    fn the_panels_own_count_decides_how_many_show() {
        let (atlas, mut titles) = (world(), QuestTitles::default());
        let both = vec![quest("Shell Sorting", &[("Pebble Snail", 1, 8)]), quest("Slime Samples", &[("Moss Slime", 2, 5)])];
        held(&mut titles, &atlas, reading(both, Some((2, 5))));
        let one = held(&mut titles, &atlas, reading(vec![quest("Slime Samples", &[("Moss Slime", 2, 5)])], Some((1, 5))));
        assert_eq!(one.unwrap().quests.len(), 1);
        assert!(held(&mut titles, &atlas, reading(vec![], Some((0, 5)))).unwrap().quests.is_empty());
    }

    #[test]
    fn a_title_garbled_live_is_too_rough_to_match_but_close_enough_to_hold() {
        let score = similarity(&fold("Blinging a Mil 101 to Heena"), &fold("Bringing a Mirror to Heena"));
        assert!((MIN_HELD_SCORE..MIN_QUEST_SCORE).contains(&score), "scored {score}");
    }

    #[test]
    fn a_quest_is_turned_in_only_where_something_shows_it() {
        let mut atlas = world();
        let mut add = |id, name: &str, npc_id, next_quest_id| {
            let name = name.into();
            let quest =
                QuestRecord { id, name, npc_id: Some(npc_id), next_quest_id, ..QuestRecord::default() };
            atlas.quests.insert(id, quest);
        };
        add(80, "Borrowing a Lamp", 50, Some(81));
        add(81, "Bringing a Lamp to Warden Ilse", 51, None);
        add(82, "Lamp Oil", 51, None);
        add(83, "A Trip to Town", 51, None);

        let chained = atlas.quest_info(80).unwrap();
        assert_eq!(chained.giver.as_deref(), Some("Warden Ilse"));
        assert_eq!(chained.turn_in.as_deref(), Some("Innkeeper Bo"));
        assert_eq!(chained.turn_in_maps.iter().map(|m| m.id).collect::<Vec<_>>(), [5]);

        let titled = atlas.quest_info(81).unwrap();
        assert_eq!(titled.giver.as_deref(), Some("Innkeeper Bo"));
        assert_eq!(titled.turn_in.as_deref(), Some("Warden Ilse"));
        assert_eq!(titled.turn_in_maps.iter().map(|m| m.id).collect::<Vec<_>>(), [1]);

        assert_eq!(atlas.quest_info(82).unwrap().turn_in, None);
        assert_eq!(atlas.quest_info(83).unwrap().turn_in, None, "`to` followed by no NPC");
    }

    #[test]
    fn the_quest_file_says_who_takes_a_quest_in_before_anything_is_inferred() {
        use crate::mapdata::atlas::tests::map;
        use crate::mapdata::osms::{Dataset, NpcRecord, QuestRecord};
        use crate::mapdata::turnins::TurnIn;
        let npc = |id: u32, name: &str, maps: Vec<u32>| NpcRecord { id, name: name.into(), maps, icon_url: None };
        let quest = |id: u32, name: &str, giver: u32, next: Option<u32>| QuestRecord {
            id,
            name: name.into(),
            npc_id: Some(giver),
            next_quest_id: next,
            ..QuestRecord::default()
        };
        let turn_in = |quest: u32, npc: u32| TurnIn { quest, npc };
        let atlas = Atlas::new(Dataset {
            maps: vec![map(1, "Mossbank", "Fern Road", vec![]), map(2, "Split Road", "Fern Road", vec![]), map(9, "Instance", "Fern Road", vec![])],
            npcs: vec![
                npc(1, "Sam", vec![1]),
                npc(2, "Mai", vec![2]),
                npc(3, "Tutor", vec![9]),
                npc(4, "Tutor", vec![2]),
                npc(5, "Biggs", vec![1]),
            ],
            quests: vec![
                quest(10, "Sam's Suggestion", 1, None),
                quest(11, "Into a Chain", 1, Some(12)),
                quest(12, "Tutor's Test", 4, None),
                quest(13, "Off the Wiki", 5, None),
                quest(14, "Unmatched", 1, Some(12)),
            ],
            turn_ins: vec![
                turn_in(10, 2),
                turn_in(11, 5),
                turn_in(12, 4),
                turn_in(14, 77),
                turn_in(99, 2),
            ],
            ..Dataset::default()
        });
        let who = |id: u32| atlas.quest_info(id).unwrap().turn_in;
        assert_eq!(who(10).as_deref(), Some("Mai"), "the client data has nothing to say on this one");
        assert_eq!(atlas.quest_info(10).unwrap().turn_in_maps[0].name, "Split Road");
        assert_eq!(who(11).as_deref(), Some("Biggs"), "the file outranks the next quest's giver");
        assert_eq!(atlas.quest_info(12).unwrap().turn_in_maps[0].name, "Split Road", "the id picks one of two Tutors");
        assert_eq!(who(13), None, "nothing to go on, so nothing is guessed");
        assert_eq!(who(14).as_deref(), Some("Tutor"), "an NPC the data lacks falls back on the chain");
    }

    #[test]
    fn maps_the_player_can_reach_come_before_richer_ones() {
        let atlas = island_world();
        let hunting = |from| {
            let guide =
                atlas.quest_guide(&[query("Shell Sorting", &["Pebble Snail"])], from, &[], &[], &Travel::none());
            guide[0].objectives[0].hunts[0].maps.iter().map(|g| g.map.name.clone()).collect::<Vec<_>>()
        };
        assert_eq!(hunting(Some(21)), ["Cove Shore", "Harbor Field"]);
        assert_eq!(hunting(None), ["Harbor Field", "Cove Shore"]);
    }

    #[test]
    fn a_measured_exp_rate_becomes_a_kill_pace() {
        let atlas = world();
        let measured = [MeasuredRate {
            label: "Fern Road : Mossbank Meadow I".into(),
            exp_per_hour: 600.0,
            active_seconds: 900.0,
            ..MeasuredRate::default()
        }];
        let guide =
            atlas.quest_guide(&[query("Shell Sorting", &["Pebble Snail"])], Some(1), &[], &measured, &Travel::none());
        let grounds = &guide[0].objectives[0].hunts[0].maps;
        // A clear of Meadow I is worth 60 base EXP and has 6 Pebble Snails: 10 clears an hour.
        let pace = grounds[0].pace.as_ref().unwrap();
        assert_eq!((pace.exp_per_hour, pace.active_seconds), (600.0, 900.0));
        assert!((pace.kills_per_hour - 60.0).abs() < 1e-9);
        assert!(grounds[1].pace.is_none(), "nothing was measured on Meadow II");
    }

    #[test]
    fn items_need_a_lookup_then_list_their_droppers() {
        let atlas = world();
        let quests = [query("[Event] Whisker Hunt", &["Rat Whisker"])];

        let before = &atlas.quest_guide(&quests, None, &[], &[], &Travel::none())[0];
        assert!(before.quest.is_none());
        let objective = &before.objectives[0];
        assert_eq!((objective.kind, objective.looked_up, objective.hunts.len()), (TargetKind::Item, false, 0));

        let after = &atlas.quest_guide(&quests, Some(1), &[wing_sources()], &[], &Travel::none())[0].objectives[0];
        assert!(after.looked_up);
        assert_eq!(after.item.as_deref(), Some("Rat Whisker"));
        let rat = &after.hunts[0];
        assert_eq!(rat.monster.name, "Burrow Rat");
        assert_eq!(rat.drop.as_ref().unwrap().rate, Some(12.0));
        assert_eq!((rat.maps[0].map.name.as_str(), rat.maps[0].hops), ("Burrow Den", Some(2)));
        // The wiki names a monster the map data doesn't have: still listed, without maps.
        assert!(!after.hunts[1].monster.known && after.hunts[1].maps.is_empty());
    }

    #[test]
    fn an_item_named_after_its_monster_is_not_the_monster() {
        let atlas = world();
        let guide = atlas.quest_guide(
            &[query("Untracked", &["Moss Slime Jelly", "Moss Slime"])],
            None,
            &[],
            &[],
            &Travel::none(),
        );
        let kinds: Vec<TargetKind> = guide[0].objectives.iter().map(|o| o.kind).collect();
        assert_eq!(kinds, [TargetKind::Item, TargetKind::Monster]);
    }

    #[test]
    fn truncated_titles_match_the_start_of_a_name() {
        let atlas = world();
        let guide = atlas.quest_guide(&[query("Slime Sam...", &[])], None, &[], &[], &Travel::none());
        assert_eq!(guide[0].quest.as_ref().map(|q| q.quest.id), Some(71));
        assert!(atlas.quest_guide(&[query("Slime", &[])], None, &[], &[], &Travel::none())[0].quest.is_none());
        for read in ["Slime Sam..", "Slime Sam. . .", "Slime Sam…"] {
            let guide = atlas.quest_guide(&[query(read, &[])], None, &[], &[], &Travel::none());
            assert_eq!(guide[0].quest.as_ref().map(|q| q.quest.id), Some(71), "{read}");
        }
    }
}
