//! The skills and items in the downloaded data, for the build planner and the item search. Jobs
//! are worked out from the skills, so nothing about them is written into Leafline.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use serde::{Deserialize, Serialize};

use super::fit::ARMOUR;
use super::plan::Stat;
use super::rules::Branch;
use crate::mapdata::drops::item_key;
use crate::mapdata::osms::{ItemKind, ItemRecord, RecipeRecord, SkillRecord};

/// The most gear a search returns.
const SEARCH_LIMIT: usize = 40;
/// Items in one page of search results.
const ITEM_PAGE: usize = 40;
/// How closely a typed word must match a word of a name when nothing matches outright.
const MIN_WORD_SCORE: f64 = 0.7;
/// How closely a name read from a tooltip must match an item in the data.
const MIN_NAME_SCORE: f64 = 0.85;
/// `(M)`, `(F)` and `(+1)` after an item's name on its tooltip, with the brackets OCR makes of them.
static TAGS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s*[({\[]\s*(?:[MFU]|\+\s*\d{1,2})\s*(?:[)}\]|lI1]|$)").unwrap());
/// How closely the first words of a read must match an item when the whole read matches none:
/// text seen through the tooltip (a store's list behind it) can run on after the name.
const MIN_PREFIX_SCORE: f64 = 0.92;
/// The slot all weapons share. Their kinds are in `weapon_type`.
const WEAPON: &str = "Weapon";
/// The data prices staff outfits and novelty items at a meso or less.
const NOVELTY_PRICE: i64 = 1;

/// What the item search asks for. Everything but `text` narrows it down.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ItemQuery {
    pub text: String,
    pub kind: Option<ItemKind>,
    /// A slot or a weapon type. Scrolls count as what they go on.
    pub slot: Option<String>,
    /// Only equipment this class can use.
    pub class: Option<String>,
    pub level_min: Option<u32>,
    pub level_max: Option<u32>,
    /// `name`, or a stat's label to put the highest first. By level otherwise.
    pub sort: Option<String>,
    /// Only equipment the character's build can wear at its level.
    pub mine: bool,
    /// Only scrolls that raise this, as `scroll_stat` names it.
    pub stat: Option<String>,
    /// Only scrolls with this chance, as `scroll_chance` names it.
    pub chance: Option<String>,
    pub offset: usize,
}

impl ItemQuery {
    fn admits(&self, item: &ItemRecord) -> bool {
        let equipment = item.kind == ItemKind::Equip;
        self.kind.is_none_or(|kind| item.kind == kind)
            && self.slot.as_deref().is_none_or(|slot| fits_slot(item, slot))
            && self.stat.as_deref().is_none_or(|stat| scroll_stat(item) == Some(stat))
            && self.chance.as_deref().is_none_or(|chance| scroll_chance(item) == Some(chance))
            && self.class.as_deref().is_none_or(|class| {
                equipment && (item.classes.is_empty() || item.classes.iter().any(|c| c.eq_ignore_ascii_case(class)))
            })
            && self.level_min.is_none_or(|level| equipment && item.req_level >= level)
            && self.level_max.is_none_or(|level| equipment && item.req_level <= level)
    }
}

/// Staff outfits, novelty tubes and the like, which players can't get or wouldn't want. The data
/// has no other mark for them.
pub fn novelty(item: &ItemRecord) -> bool {
    item.kind == ItemKind::Equip && item.price.is_some_and(|price| price <= NOVELTY_PRICE)
}

/// Items that are the same in everything but their ID, keyed by the first one's ID. Items that
/// share only a name stay apart: the two Red Relaxers are different chairs, and male and female
/// versions of gear differ by gender.
fn copies_in(items: &[ItemRecord]) -> HashMap<u32, Vec<u32>> {
    let mut by_name: HashMap<(&str, &str), Vec<&ItemRecord>> = HashMap::new();
    for item in items {
        by_name.entry((item.name.as_str(), item.kind.name())).or_default().push(item);
    }
    let same = |a: &ItemRecord, b: &ItemRecord| ItemRecord { id: 0, ..a.clone() } == ItemRecord { id: 0, ..b.clone() };
    let mut copies = HashMap::new();
    for group in by_name.values().filter(|g| g.len() > 1) {
        let mut sets: Vec<Vec<&ItemRecord>> = Vec::new();
        for item in group {
            match sets.iter_mut().find(|set| same(set[0], item)) {
                Some(set) => set.push(item),
                None => sets.push(vec![item]),
            }
        }
        for set in sets.into_iter().filter(|s| s.len() > 1) {
            let mut ids: Vec<u32> = set.iter().map(|i| i.id).collect();
            ids.sort();
            copies.insert(ids[0], ids);
        }
    }
    copies
}

/// Whether an item goes in `slot`, or for a scroll, goes on what does.
fn fits_slot(item: &ItemRecord, slot: &str) -> bool {
    match &item.scroll {
        Some(scroll) => {
            let weapon = scroll.fits != "Other" && !ARMOUR.contains(&scroll.fits.as_str());
            scroll.fits == slot || (slot == WEAPON && weapon) || (slot == PET_EQUIP && pet_scroll(item))
        }
        None => item.slot == slot || item.weapon_type.as_deref() == Some(slot) || item_group(item) == Some(slot),
    }
}

/// Where scrolls for pet equipment are listed. The data files them under `Other`.
const PET_EQUIP: &str = "Pet equip";

fn pet_scroll(item: &ItemRecord) -> bool {
    item.scroll.as_ref().is_some_and(|s| s.fits == "Other") && item.name.starts_with("Pet Equip ")
}

/// What scrolls raise, in the order offered. Names put it just before `Scroll`, after what the
/// scroll goes on (`Gloves Magic Attack Scroll`), so longer names have to be tried first: the data's
/// own `stat` lumps several of these together as `Other` and `Attack`.
const SCROLL_STATS: [&str; 17] = [
    "Attack", "Magic Attack", "STR", "DEX", "INT", "LUK", "HP", "MP", "Accuracy", "Evasion", "Speed", "Jump", "DEF",
    "Weapon Def.", "Magic Def.", "Crit. Rate", "Crit. Damage",
];

/// What a scroll raises, read from its name.
pub fn scroll_stat(item: &ItemRecord) -> Option<&'static str> {
    item.scroll.as_ref()?;
    let stem = item.name.split(" Scroll").next()?;
    let mut stats = SCROLL_STATS;
    stats.sort_by_key(|s| std::cmp::Reverse(s.len()));
    stats.into_iter().find(|stat| stem.ends_with(&format!(" {stat}")))
}

/// A scroll's chance as the chips name it: its success rate, or `Chaos` for chaos scrolls, which
/// change stats at random rather than raise one.
const SCROLL_CHANCES: [&str; 4] = ["100%", "60%", "10%", "Chaos"];

pub fn scroll_chance(item: &ItemRecord) -> Option<&'static str> {
    let scroll = item.scroll.as_ref()?;
    if scroll.tier.as_deref() == Some("Chaos") {
        return Some("Chaos");
    }
    let rate = format!("{}%", scroll.success_rate?);
    SCROLL_CHANCES.into_iter().find(|c| *c == rate)
}

/// How well `words` match an item, best first, or `None` if they don't. An empty search matches
/// everything equally.
fn text_rank(item: &ItemRecord, words: &[String]) -> Option<u8> {
    if words.is_empty() {
        return Some(0);
    }
    let name = item_key(&item.name);
    if name.starts_with(&words.join(" ")) {
        return Some(0);
    }
    // By the start of a word: `hat` shouldn't find `Chat`, or every description with `that` in it.
    let name: Vec<&str> = name.split_whitespace().collect();
    let in_words = |list: &[&str]| words.iter().all(|word| list.iter().any(|w| w.starts_with(word.as_str())));
    if in_words(&name) {
        return Some(1);
    }
    let about = [Some(item.slot.as_str()), item.weapon_type.as_deref(), item.description.as_deref()];
    let about = about.iter().flatten().map(|text| item_key(text)).collect::<Vec<_>>().join(" ");
    let about: Vec<&str> = about.split_whitespace().chain(name).collect();
    in_words(&about).then_some(2)
}

/// Whether each typed word is close to a word of the name, for a search with a typo in it.
fn nearly_named(item: &ItemRecord, words: &[String]) -> bool {
    let name = item_key(&item.name);
    let close = |word: &String| name.split_whitespace().any(|w| strsim::normalized_damerau_levenshtein(w, word) >= MIN_WORD_SCORE);
    words.iter().all(close)
}

fn stat_of(item: &ItemRecord, label: &str) -> Option<i64> {
    item.stats.iter().find(|s| s.label == label).map(|s| s.value)
}

/// One page of an item search.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemPage {
    pub items: Vec<ItemRecord>,
    /// How many matched in all.
    pub total: usize,
    /// Nothing matched outright, so these are the closest names.
    pub closest: bool,
}

/// The choices the item search offers, from what the data holds.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemFilters {
    pub slots: Vec<String>,
    pub weapons: Vec<String>,
    /// What scrolls go on, as slots, with every weapon type under `Weapon`.
    pub scroll_slots: Vec<String>,
    /// What scrolls for each of `scroll_slots` raise.
    pub scroll_stats: HashMap<String, Vec<String>>,
    pub scroll_chances: Vec<String>,
    pub use_groups: Vec<String>,
    pub setup_groups: Vec<String>,
    pub etc_groups: Vec<String>,
}

/// An item that fits none of its kind's groups.
const OTHER: &str = "Other";
/// The groups offered under each kind, in the order shown. See `item_group`.
const USE_GROUPS: [&str; 11] = [
    "Potions",
    "Buffs",
    "Food",
    "Return scrolls",
    "Bow arrows",
    "Crossbow arrows",
    "Throwing stars",
    "Summoning sacks",
    "Crafting",
    "Coupons & boxes",
    OTHER,
];
const SETUP_GROUPS: [&str; 5] = ["Chairs", "Decor", "Crafting", "Event", OTHER];
const ETC_GROUPS: [&str; 7] = ["Monster loot", "Ores & gems", "Crafting", "Quest items", "Party quest", "Minigames", OTHER];

/// The group a Use, Set-up or Etc item is listed under, or `None` for equipment and scrolls,
/// which go by slot. The data files every Use item under `Consume` and every Etc item under
/// `Etc`, but the client numbers items in blocks by what they are (2000xxx potions, 2030xxx
/// return scrolls, 4010xxx ores and so on), so the groups come from where an item's number falls.
/// The blocks were read off the downloaded data on 2026-11-06; an item outside them is `Other`.
pub fn item_group(item: &ItemRecord) -> Option<&'static str> {
    let block = item.id / 1000;
    Some(match item.kind {
        ItemKind::Equip | ItemKind::Scroll => return None,
        // Crafting kits sit among the buff potions.
        ItemKind::Use => match block {
            2002 if item.name.ends_with(" Kit") => "Crafting",
            2000 | 2001 => "Potions",
            2002 | 2023 => "Buffs",
            2010..=2022 => "Food",
            2030 => "Return scrolls",
            2060 => "Bow arrows",
            2061 => "Crossbow arrows",
            2070 => "Throwing stars",
            2100 => "Summoning sacks",
            2430..=2459 | 2540..=2549 | 2890..=2899 => "Coupons & boxes",
            _ => OTHER,
        },
        ItemKind::Setup => match block {
            3010..=3019 => "Chairs",
            3000..=3009 | 3020..=3029 | 3040..=3049 => "Decor",
            3030..=3039 => "Crafting",
            3990..=3999 => "Event",
            _ => OTHER,
        },
        ItemKind::Etc if item.quest_item => "Quest items",
        ItemKind::Etc => match block {
            4000 => "Monster loot",
            4010 | 4020 => "Ores & gems",
            4003 | 4130 => "Crafting",
            4001 => "Party quest",
            4080 => "Minigames",
            _ => OTHER,
        },
    })
}

/// A job a player can choose.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobChoice {
    pub id: u32,
    pub name: String,
    /// 1 for a first job, 2 for a second.
    pub tier: u32,
    /// The first job a second job comes from.
    pub parent: Option<u32>,
    /// The stat a new build starts by putting AP into. A place to start, not advice.
    pub primary: Option<Stat>,
}

/// 0 for Beginner, 1 for a first job such as 400, 2 for a second such as 420.
pub fn tier_of(job: u32) -> u32 {
    match job {
        0 => 0,
        j if j.is_multiple_of(100) => 1,
        _ => 2,
    }
}

/// The first job and, if one is chosen, the second job of `job`.
pub fn path_of(job: u32) -> Vec<u32> {
    match tier_of(job) {
        0 => Vec::new(),
        1 => vec![job],
        _ => vec![job / 100 * 100, job],
    }
}

#[derive(Debug, Default)]
pub struct Catalog {
    skills: Vec<SkillRecord>,
    items: Vec<ItemRecord>,
    skill_index: HashMap<u32, usize>,
    item_index: HashMap<u32, usize>,
    /// For an item the data holds more than once, every ID it goes by, first one first. See
    /// `copies_of`.
    copies: HashMap<u32, Vec<u32>>,
    /// Crafting recipes, in the data's order. See `super::crafting`.
    recipes: Vec<RecipeRecord>,
}

impl Catalog {
    pub fn new(skills: Vec<SkillRecord>, items: Vec<ItemRecord>) -> Self {
        let skill_index = skills.iter().enumerate().map(|(i, s)| (s.id, i)).collect();
        let item_index = items.iter().enumerate().map(|(i, s)| (s.id, i)).collect();
        let copies = copies_in(&items);
        Self { skills, items, skill_index, item_index, copies, recipes: Vec::new() }
    }

    /// The same catalog with the crafting recipes that came with the data.
    pub fn with_recipes(mut self, recipes: Vec<RecipeRecord>) -> Self {
        self.recipes = recipes;
        self
    }

    pub fn recipe_list(&self) -> &[RecipeRecord] {
        &self.recipes
    }

    /// Every ID the item goes by: the data repeats some items under new numbers, such as a
    /// potion that only differs by an expiry flag Leafline doesn't keep, or four of the same
    /// quest letter. Just `id` for most items.
    pub fn copies_of(&self, id: u32) -> Vec<u32> {
        let first = self.copies.iter().find(|(_, ids)| ids.contains(&id)).map(|(first, _)| *first);
        first.and_then(|f| self.copies.get(&f)).cloned().unwrap_or_else(|| vec![id])
    }

    /// Whether the item is a later copy of one listed already, which lists leave out.
    fn repeated(&self, item: &ItemRecord) -> bool {
        self.copies.values().any(|ids| ids[1..].contains(&item.id))
    }

    pub fn skill(&self, id: u32) -> Option<&SkillRecord> {
        self.skills.get(*self.skill_index.get(&id)?)
    }

    pub fn item(&self, id: u32) -> Option<&ItemRecord> {
        self.items.get(*self.item_index.get(&id)?)
    }

    pub fn skills(&self) -> &[SkillRecord] {
        &self.skills
    }

    /// The skill called `name` in one of `jobs`. Several jobs have a skill of the same name.
    pub fn skill_named(&self, name: &str, jobs: &[u32]) -> Option<&SkillRecord> {
        self.skills.iter().find(|s| s.name == name && jobs.contains(&s.job_id))
    }

    /// The piece of equipment called `name`.
    pub fn item_named(&self, name: &str) -> Option<&ItemRecord> {
        self.equipment().find(|i| i.name == name)
    }

    pub fn equipment(&self) -> impl Iterator<Item = &ItemRecord> {
        self.items.iter().filter(|i| i.kind == ItemKind::Equip)
    }

    /// The item a name read from the game most likely means. Several items can share a name, so
    /// `level` settles it when it is known.
    pub fn item_like(&self, name: &str, level: Option<u32>) -> Option<&ItemRecord> {
        // The tooltip adds `(F)` or `(M)` after the name of gendered equipment, which the data keeps in
        // a field of its own, and an upgrade count `(+N)`: neither is part of the name.
        let bare = TAGS.replace_all(name, "");
        let name = bare.trim();
        if let Some(item) = self.item_close_to(name, level, MIN_NAME_SCORE) {
            return Some(item);
        }
        // The longest run of first words that names an item, as in `Iron Ore ushroom Cap A
        // tough iron ore`, where a store's list and the description ran on after the name.
        let words: Vec<&str> = name.split_whitespace().collect();
        if let Some(item) = (1..words.len()).rev().find_map(|n| self.item_close_to(&words[..n].join(" "), level, MIN_PREFIX_SCORE)) {
            return Some(item);
        }
        self.item_inside(name, level).or_else(|| self.item_ending_with(name, level))
    }

    /// The one item at this level whose name ends with the words read, for a name whose first word
    /// the screen hid (`Split Pants` for `Black Split Pants`). Needs the level and two words, and
    /// says nothing when more than one item fits.
    fn item_ending_with(&self, text: &str, level: Option<u32>) -> Option<&ItemRecord> {
        let level = level?;
        let key = item_key(text);
        let words: Vec<&str> = key.split_whitespace().collect();
        if words.len() < 2 {
            return None;
        }
        let mut found = self.items.iter().filter(|item| {
            let name = item_key(&item.name);
            let held: Vec<&str> = name.split_whitespace().collect();
            item.req_level == level && held.len() > words.len() && held.ends_with(&words)
        });
        let first = found.next()?;
        found.next().is_none().then_some(first)
    }

    /// The longest item name that stands whole inside `text`, as words in a row: text seen through
    /// the tooltip can come before or inside a name as well as after it, as in `ossbow Brown Hard
    /// Leather Pants`. A name of one word is too short to trust anywhere but at the start, which
    /// the other passes cover.
    fn item_inside(&self, text: &str, level: Option<u32>) -> Option<&ItemRecord> {
        let key = item_key(text);
        let words: Vec<&str> = key.split_whitespace().collect();
        self.items
            .iter()
            .filter_map(|item| {
                let name = item_key(&item.name);
                let held: Vec<&str> = name.split_whitespace().collect();
                let inside = held.len() >= 2 && held.len() <= words.len() && words.windows(held.len()).any(|run| run == held.as_slice());
                inside.then_some((name.len(), item))
            })
            .max_by(|a, b| a.0.cmp(&b.0).then((Some(a.1.req_level) == level).cmp(&(Some(b.1.req_level) == level))))
            .map(|(_, item)| item)
    }

    fn item_close_to(&self, name: &str, level: Option<u32>, min_score: f64) -> Option<&ItemRecord> {
        let key = item_key(name);
        if key.is_empty() {
            return None;
        }
        self.items
            .iter()
            .map(|item| (strsim::normalized_levenshtein(&item_key(&item.name), &key), item))
            .filter(|(score, _)| *score >= min_score)
            .max_by(|a, b| a.0.total_cmp(&b.0).then((Some(a.1.req_level) == level).cmp(&(Some(b.1.req_level) == level))))
            .map(|(_, item)| item)
    }

    /// The weapons a job trains in, going by its Mastery skills, as written in `weapon_type`.
    /// Empty for a path with none, such as a first job or a Magician.
    pub fn weapons_of(&self, job: u32) -> Vec<String> {
        let path = path_of(job);
        self.mastered(|skill| path.contains(&skill.job_id))
    }

    /// The weapons a job or any job it can lead to trains in. A first job has no Mastery skill of
    /// its own yet, so this says what it is working towards.
    pub fn weapons_of_branch(&self, job: u32) -> Vec<String> {
        if job == 0 {
            return Vec::new();
        }
        self.mastered(|skill| skill.job_id / 100 == job / 100)
    }

    /// The weapon types named by the Mastery skills that `counts` lets through.
    fn mastered(&self, counts: impl Fn(&SkillRecord) -> bool) -> Vec<String> {
        let words: Vec<&str> = self.skills.iter().filter(|s| counts(s)).filter_map(|s| s.name.strip_suffix(" Mastery")).collect();
        self.weapon_types().into_iter().filter(|t| words.iter().any(|w| t.ends_with(w))).collect()
    }

    fn weapon_types(&self) -> Vec<String> {
        let mut types: Vec<String> = self.items.iter().filter_map(|i| i.weapon_type.clone()).collect();
        types.sort();
        types.dedup();
        types
    }

    pub fn item_filters(&self) -> ItemFilters {
        let has = |slot: &&str| self.equipment().any(|i| i.slot == *slot);
        let slots = std::iter::once(WEAPON).chain(ARMOUR).filter(has).map(str::to_string).collect();
        let scrolls: Vec<&ItemRecord> = self.items.iter().filter(|i| i.kind == ItemKind::Scroll).collect();
        let scroll_slots: Vec<String> = std::iter::once(WEAPON)
            .chain(ARMOUR)
            .chain([PET_EQUIP])
            .filter(|slot| scrolls.iter().any(|i| fits_slot(i, slot)))
            .map(str::to_string)
            .collect();
        let raised = |slot: &str| -> Vec<String> {
            let held: Vec<&str> = scrolls.iter().filter(|i| fits_slot(i, slot)).filter_map(|i| scroll_stat(i)).collect();
            SCROLL_STATS.iter().filter(|s| held.contains(s)).map(|s| s.to_string()).collect()
        };
        let scroll_stats = scroll_slots.iter().map(|slot| (slot.clone(), raised(slot))).collect();
        let chances: Vec<&str> = scrolls.iter().filter_map(|i| scroll_chance(i)).collect();
        let scroll_chances = SCROLL_CHANCES.iter().filter(|c| chances.contains(c)).map(|c| c.to_string()).collect();
        let groups = |kind: ItemKind, order: &[&str]| -> Vec<String> {
            let held: Vec<&str> = self.items.iter().filter(|i| i.kind == kind).filter_map(item_group).collect();
            order.iter().filter(|g| held.contains(g)).map(|g| g.to_string()).collect()
        };
        ItemFilters {
            slots,
            weapons: self.weapon_types(),
            scroll_slots,
            scroll_stats,
            scroll_chances,
            use_groups: groups(ItemKind::Use, &USE_GROUPS),
            setup_groups: groups(ItemKind::Setup, &SETUP_GROUPS),
            etc_groups: groups(ItemKind::Etc, &ETC_GROUPS),
        }
    }

    /// One page of the items that `query` finds and `allowed` lets through. Novelty gear only
    /// comes up by name, so it doesn't fill the lists.
    pub fn search_items(&self, query: &ItemQuery, allowed: impl Fn(&ItemRecord) -> bool) -> ItemPage {
        let words: Vec<String> = item_key(&query.text).split_whitespace().map(str::to_string).collect();
        let shown = |i: &&ItemRecord| query.admits(i) && allowed(i) && (!words.is_empty() || !novelty(i)) && !self.repeated(i);
        let candidates: Vec<&ItemRecord> = self.items.iter().filter(shown).collect();
        let mut found: Vec<(u8, &ItemRecord)> =
            candidates.iter().filter_map(|i| Some((text_rank(i, &words)?, *i))).collect();
        let closest = found.is_empty() && !words.is_empty();
        if closest {
            found = candidates.iter().filter(|i| nearly_named(i, &words)).map(|i| (0, *i)).collect();
        }
        match query.sort.as_deref() {
            Some("name") => found.sort_by(|a, b| a.1.name.cmp(&b.1.name)),
            Some(label) => {
                found.retain(|(_, i)| stat_of(i, label).is_some());
                found.sort_by_key(|(_, i)| std::cmp::Reverse(stat_of(i, label)));
            }
            None => found.sort_by(|a, b| (a.0, a.1.req_level, &a.1.name).cmp(&(b.0, b.1.req_level, &b.1.name))),
        }
        ItemPage {
            total: found.len(),
            items: found.into_iter().skip(query.offset).take(ITEM_PAGE).map(|(_, i)| i.clone()).collect(),
            closest,
        }
    }

    pub fn job_name(&self, job: u32) -> Option<&str> {
        self.skills.iter().find(|s| s.job_id == job).map(|s| s.class_name.as_str())
    }

    /// Every job with skills, first jobs before the second jobs that come from them.
    pub fn jobs(&self) -> Vec<JobChoice> {
        let mut jobs: Vec<JobChoice> = Vec::new();
        for skill in self.skills.iter().filter(|s| tier_of(s.job_id) > 0) {
            if !jobs.iter().any(|j| j.id == skill.job_id) {
                let tier = tier_of(skill.job_id);
                let parent = (tier == 2).then_some(skill.job_id / 100 * 100);
                let primary = Branch::of_job(skill.job_id).map(Branch::primary);
                jobs.push(JobChoice { id: skill.job_id, name: skill.class_name.clone(), tier, parent, primary });
            }
        }
        jobs.sort_by_key(|j| j.id);
        jobs
    }

    /// Equipment whose name holds `text`, lowest level first. With a branch, only what it can use.
    pub fn search_gear(&self, text: &str, branch: Option<Branch>) -> Vec<ItemRecord> {
        let text = text.trim().to_lowercase();
        let mut found: Vec<&ItemRecord> = self
            .items
            .iter()
            .filter(|i| i.kind == ItemKind::Equip && i.name.to_lowercase().contains(&text))
            .filter(|i| branch.is_none_or(|b| usable_by(i, b)))
            .collect();
        found.sort_by(|a, b| (a.req_level, &a.name).cmp(&(b.req_level, &b.name)));
        found.into_iter().take(SEARCH_LIMIT).cloned().collect()
    }
}

/// Whether `branch` can use the item, going by the data.
pub fn usable_by(item: &ItemRecord, branch: Branch) -> bool {
    item.classes.is_empty() || item.classes.iter().any(|c| c.eq_ignore_ascii_case(branch.name()))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::mapdata::osms::{ItemStat, ScrollInfo, SkillNeed};

    /// A made-up skill.
    pub fn skill(id: u32, name: &str, job_id: u32, max_level: u32, needs: &[(u32, u32)]) -> SkillRecord {
        SkillRecord {
            id,
            name: name.into(),
            job_id,
            class_name: format!("Job {job_id}"),
            job_tier: String::new(),
            max_level,
            passive: false,
            description: None,
            required_skills: needs.iter().map(|&(skill_id, level)| SkillNeed { skill_id, level }).collect(),
            level_descriptions: Vec::new(),
        }
    }

    /// A made-up piece of equipment.
    pub fn item(id: u32, name: &str, req_level: u32, classes: &[&str]) -> ItemRecord {
        ItemRecord {
            id,
            name: name.into(),
            kind: ItemKind::Equip,
            slot: "Hat".into(),
            gender: None,
            classes: classes.iter().map(|c| c.to_string()).collect(),
            weapon_type: None,
            req_level,
            req_str: 0,
            req_dex: 0,
            req_int: 0,
            req_luk: 0,
            req_fame: 0,
            attack_speed: None,
            knockback: None,
            upgrade_slots: None,
            quest_item: false,
            untradable: false,
            stats: Vec::new(),
            price: None,
            description: None,
            scroll: None,
        }
    }

    /// A made-up item of another kind.
    fn other(id: u32, name: &str, kind: ItemKind, description: &str) -> ItemRecord {
        ItemRecord { kind, slot: String::new(), description: Some(description.into()), ..item(id, name, 0, &[]) }
    }

    fn scroll(id: u32, name: &str, fits: &str) -> ItemRecord {
        let info = ScrollInfo { fits: fits.into(), tier: None, success_rate: Some(60) };
        ItemRecord { slot: "Scroll".into(), scroll: Some(info), ..other(id, name, ItemKind::Scroll, "") }
    }

    fn weapon(id: u32, name: &str, kind: &str) -> ItemRecord {
        ItemRecord { slot: "Weapon".into(), weapon_type: Some(kind.into()), ..item(id, name, 10, &[]) }
    }

    fn found(catalog: &Catalog, query: ItemQuery) -> Vec<String> {
        catalog.search_items(&query, |_| true).items.into_iter().map(|i| i.name).collect()
    }

    fn stocked() -> Catalog {
        Catalog::new(
            Vec::new(),
            vec![
                item(1, "Green Fern Hat", 20, &[]),
                ItemRecord { slot: "Weapon".into(), ..item(2, "Fern Dagger", 10, &["Thief"]) },
                ItemRecord { slot: "Weapon".into(), ..item(3, "Twig Wand", 5, &["Magician"]) },
                other(4, "Red Berry", ItemKind::Use, "Restores 50 HP."),
                other(5, "Chat Note", ItemKind::Etc, "A note that says hello."),
                other(6, "Moss Stool", ItemKind::Setup, "Somewhere to sit."),
                scroll(7, "Hat Moss Scroll", "Hat"),
                scroll(8, "Claw Moss Scroll", "Claw"),
                weapon(9, "Thorn Claw", "Claw"),
            ],
        )
    }

    #[test]
    fn jobs_come_from_the_skills() {
        let catalog = Catalog::new(
            vec![skill(1, "A", 0, 3, &[]), skill(2, "B", 410, 5, &[]), skill(3, "C", 400, 5, &[]), skill(4, "D", 410, 5, &[])],
            Vec::new(),
        );
        let jobs = catalog.jobs();
        assert_eq!(jobs.iter().map(|j| j.id).collect::<Vec<_>>(), [400, 410], "no Beginner, no repeats");
        assert_eq!((jobs[0].tier, jobs[0].parent), (1, None));
        assert_eq!((jobs[1].tier, jobs[1].parent), (2, Some(400)));
    }

    #[test]
    fn a_job_path_starts_at_the_first_job() {
        assert!(path_of(0).is_empty());
        assert_eq!(path_of(400), [400]);
        assert_eq!(path_of(420), [400, 420]);
    }

    #[test]
    fn gear_search_keeps_to_what_the_branch_can_use() {
        let catalog = Catalog::new(
            Vec::new(),
            vec![item(1, "Fern Hat", 20, &[]), item(2, "Fern Dagger", 10, &["Thief"]), item(3, "Fern Wand", 5, &["Magician"])],
        );
        let names = |found: Vec<ItemRecord>| found.into_iter().map(|i| i.name).collect::<Vec<_>>();
        assert_eq!(names(catalog.search_gear("fern", Some(Branch::Thief))), ["Fern Dagger", "Fern Hat"]);
        assert_eq!(names(catalog.search_gear("  WAND ", None)), ["Fern Wand"]);
    }

    #[test]
    fn gear_search_and_names_keep_to_equipment() {
        let catalog = Catalog::new(
            Vec::new(),
            vec![other(1, "Fern Hat", ItemKind::Etc, "A hat-shaped leaf."), item(2, "Fern Hat", 20, &[])],
        );
        assert_eq!(catalog.search_gear("fern", None).len(), 1);
        assert_eq!(catalog.item_named("Fern Hat").unwrap().id, 2);
    }

    #[test]
    fn item_search_puts_names_first_then_what_items_say_about_themselves() {
        let text = |text: &str| ItemQuery { text: text.into(), ..ItemQuery::default() };
        assert_eq!(found(&stocked(), text("fern")), ["Fern Dagger", "Green Fern Hat"], "the name's start comes first");
        assert_eq!(found(&stocked(), text("hat green")), ["Green Fern Hat"], "words in any order");
        assert_eq!(found(&stocked(), text("hp")), ["Red Berry"], "from its description");
        assert_eq!(found(&stocked(), text("hat")), ["Hat Moss Scroll", "Green Fern Hat"], "not `that`");
    }

    #[test]
    fn a_search_with_a_typo_finds_the_closest_names_and_says_so() {
        let query = ItemQuery { text: "thron claw".into(), ..ItemQuery::default() };
        let page = stocked().search_items(&query, |_| true);
        assert!(page.closest);
        assert_eq!(page.items[0].name, "Thorn Claw");
        assert!(!stocked().search_items(&ItemQuery { text: "thorn".into(), ..query }, |_| true).closest);
    }

    #[test]
    fn filters_narrow_the_search() {
        let catalog = stocked();
        let kind = ItemQuery { kind: Some(ItemKind::Setup), ..ItemQuery::default() };
        assert_eq!(found(&catalog, kind), ["Moss Stool"]);
        let class = ItemQuery { class: Some("thief".into()), ..ItemQuery::default() };
        assert_eq!(found(&catalog, class), ["Fern Dagger", "Thorn Claw", "Green Fern Hat"], "no potions, and gear for anyone counts");
        let levels = ItemQuery { level_min: Some(6), level_max: Some(10), ..ItemQuery::default() };
        assert_eq!(found(&catalog, levels), ["Fern Dagger", "Thorn Claw"]);
    }

    #[test]
    fn a_slot_finds_gear_and_the_scrolls_for_it() {
        let catalog = stocked();
        let slot = |slot: &str, kind| ItemQuery { slot: Some(slot.into()), kind, ..ItemQuery::default() };
        assert_eq!(found(&catalog, slot("Claw", None)), ["Claw Moss Scroll", "Thorn Claw"]);
        assert_eq!(found(&catalog, slot("Hat", Some(ItemKind::Scroll))), ["Hat Moss Scroll"]);
        assert_eq!(found(&catalog, slot("Weapon", Some(ItemKind::Scroll))), ["Claw Moss Scroll"], "any weapon's");
    }

    #[test]
    fn results_can_be_sorted_by_a_stat_and_paged() {
        let stat = |id, name, value| ItemRecord {
            stats: vec![ItemStat { label: "LUK".into(), value }],
            ..item(id, name, 1, &[])
        };
        let mut items: Vec<ItemRecord> = (0..50).map(|i| item(100 + i, &format!("Filler {i:02}"), 1, &[])).collect();
        items.extend([stat(1, "Low", 1), stat(2, "High", 9)]);
        let catalog = Catalog::new(Vec::new(), items);
        let by_luk = ItemQuery { sort: Some("LUK".into()), ..ItemQuery::default() };
        assert_eq!(found(&catalog, by_luk), ["High", "Low"], "only items with the stat");
        let page = catalog.search_items(&ItemQuery { sort: Some("name".into()), offset: 40, ..ItemQuery::default() }, |_| true);
        assert_eq!((page.total, page.items.len()), (52, 12));
    }

    #[test]
    fn a_name_read_from_the_game_finds_its_item() {
        let catalog = Catalog::new(
            Vec::new(),
            vec![item(1, "Red Cloth Vest", 10, &[]), item(2, "Red Cloth Vest", 25, &[]), item(3, "Blue Cloth Vest", 10, &[])],
        );
        assert_eq!(catalog.item_like("Red Cloth Vest", Some(25)).unwrap().id, 2);
        assert_eq!(catalog.item_like("Rcd Cloth Vest", Some(10)).unwrap().id, 1, "a misread letter");
        assert!(catalog.item_like("Pebble", None).is_none());
        assert_eq!(catalog.item_like("Rcd Cloth Vest (F) (+1)", Some(10)).unwrap().id, 1, "the tooltip's tags are not part of the name");
    }

    #[test]
    fn a_name_missing_its_first_word_finds_the_one_item_that_ends_so() {
        let catalog = Catalog::new(
            Vec::new(),
            vec![item(1, "Black Split Pants", 20, &[]), item(2, "Brown Split Pants", 30, &[]), item(3, "Grey Split Pants", 30, &[])],
        );
        assert_eq!(catalog.item_like("Split Pants", Some(20)).unwrap().id, 1);
        assert!(catalog.item_like("Split Pants", Some(30)).is_none(), "two items fit, so none is guessed");
        assert!(catalog.item_like("Split Pants", None).is_none(), "needs the level");
    }

    #[test]
    fn text_before_or_inside_a_name_does_not_hide_the_item() {
        let catalog = Catalog::new(
            Vec::new(),
            vec![item(1, "Brown Hard Leather Pants", 30, &[]), item(2, "Hard Leather Pants", 10, &[]), item(3, "Pan Lid", 10, &[])],
        );
        let id = |read: &str| catalog.item_like(read, None).map(|i| i.id);
        assert_eq!(id("Brown Hard Leather Pants ossbow"), Some(1), "after it");
        assert_eq!(id("ossbow Brown Hard Leather Pants"), Some(1), "before it");
        assert_eq!(id("Eagle Brown Hard Leather Pants (+1) ossbow"), Some(1), "around it, and the longest name wins");
        assert_eq!(id("Some Hard Leather Pants"), Some(2), "a shorter item stands where the longer one doesn't");
        assert_eq!(id("The Lid Pan"), None, "words out of order are not a name");
        assert_eq!(id("Eagle Pan"), None, "half a name is not a name");
    }

    #[test]
    fn a_name_with_text_run_on_after_it_finds_its_item() {
        let catalog = Catalog::new(Vec::new(), vec![item(1, "Iron Ore", 0, &[]), item(2, "Orange Mushroom Cap", 0, &[])]);
        assert_eq!(catalog.item_like("Iron Ore ushroom Cap A tough iron ore", None).unwrap().id, 1);
        assert!(catalog.item_like("Iron Fist of the Ore", None).is_none(), "no run of first words names an item");
    }

    #[test]
    fn a_job_trains_in_the_weapons_its_mastery_skills_name() {
        let catalog = Catalog::new(
            vec![skill(1, "Claw Mastery", 410, 20, &[]), skill(2, "Sword Mastery", 110, 20, &[]), skill(3, "Bow Mastery", 310, 20, &[])],
            vec![weapon(1, "A", "Claw"), weapon(2, "B", "Dagger"), weapon(3, "C", "1H Sword"), weapon(4, "D", "2H Sword"), weapon(5, "E", "Crossbow")],
        );
        assert_eq!(catalog.weapons_of(410), ["Claw"]);
        assert_eq!(catalog.weapons_of(110), ["1H Sword", "2H Sword"]);
        assert!(catalog.weapons_of(400).is_empty(), "a first job has no Mastery skill yet");
        assert!(catalog.weapons_of(310).is_empty(), "`Bow` isn't `Crossbow`");
    }

    #[test]
    fn the_filters_come_from_the_data() {
        let filters = stocked().item_filters();
        assert_eq!(filters.slots, ["Weapon", "Hat"]);
        assert_eq!(filters.weapons, ["Claw"]);
        assert_eq!(filters.scroll_slots, ["Weapon", "Hat"]);
        assert_eq!(filters.scroll_chances, ["60%"]);
        assert!(filters.scroll_stats["Hat"].is_empty(), "`Moss` isn't a stat");
        assert_eq!((filters.use_groups, filters.setup_groups, filters.etc_groups), (vec!["Other".to_string()], vec!["Other".into()], vec!["Other".into()]));
    }

    #[test]
    fn scrolls_are_found_by_what_they_raise_and_their_chance() {
        let rated = |id, name: &str, fits: &str, tier: &str, rate| ItemRecord {
            scroll: Some(ScrollInfo { fits: fits.into(), tier: Some(tier.into()), success_rate: Some(rate) }),
            ..scroll(id, name, fits)
        };
        let catalog = Catalog::new(
            Vec::new(),
            vec![
                rated(1, "Gloves Attack Scroll: Intermediate", "Glove", "Intermediate", 60),
                rated(2, "Gloves Magic Attack Scroll: Intermediate", "Glove", "Intermediate", 60),
                rated(3, "Gloves Attack Scroll: Greater", "Glove", "Greater", 10),
                rated(4, "Cape Weapon Def. Scroll: Lesser", "Cape", "Lesser", 100),
                rated(5, "Gloves Attack Scroll: Chaos", "Glove", "Chaos", 10),
                rated(6, "Pet Equip Speed Scroll: Lesser", "Other", "Lesser", 100),
                rated(7, "Claw Attack Scroll: Lesser", "Claw", "Lesser", 100),
            ],
        );
        let name = |id| catalog.item(id).unwrap();
        assert_eq!(scroll_stat(name(2)), Some("Magic Attack"), "the longer name wins");
        assert_eq!(scroll_stat(name(4)), Some("Weapon Def."));
        assert_eq!((scroll_chance(name(3)), scroll_chance(name(5))), (Some("10%"), Some("Chaos")));

        let filters = catalog.item_filters();
        assert_eq!(filters.scroll_slots, ["Weapon", "Glove", "Cape", "Pet equip"]);
        assert_eq!(filters.scroll_stats["Glove"], ["Attack", "Magic Attack"]);
        assert_eq!(filters.scroll_stats["Pet equip"], ["Speed"]);
        assert_eq!(filters.scroll_chances, ["100%", "60%", "10%", "Chaos"]);

        let query = |stat: &str, chance: &str| ItemQuery {
            kind: Some(ItemKind::Scroll),
            slot: Some("Glove".into()),
            stat: Some(stat.into()),
            chance: Some(chance.into()),
            ..ItemQuery::default()
        };
        assert_eq!(found(&catalog, query("Attack", "60%")), ["Gloves Attack Scroll: Intermediate"]);
        let pets = ItemQuery { kind: Some(ItemKind::Scroll), slot: Some("Pet equip".into()), ..ItemQuery::default() };
        assert_eq!(found(&catalog, pets), ["Pet Equip Speed Scroll: Lesser"]);
    }

    #[test]
    fn copies_of_an_item_are_listed_once() {
        let potion = |id| other(id, "Red Potion", ItemKind::Use, "Restores 100 HP.");
        let catalog = Catalog::new(
            Vec::new(),
            vec![
                potion(2000000),
                potion(2000007),
                other(3010002, "The Red Relaxer", ItemKind::Setup, "Restores 30 HP."),
                other(3015007, "The Red Relaxer", ItemKind::Setup, "Restores 35 MP."),
                ItemRecord { gender: Some("male".into()), ..item(1040010, "Red Cloth Vest", 10, &[]) },
                ItemRecord { gender: Some("female".into()), ..item(1041011, "Red Cloth Vest", 10, &[]) },
            ],
        );
        let names = found(&catalog, ItemQuery::default());
        assert_eq!(names.iter().filter(|n| *n == "Red Potion").count(), 1);
        assert_eq!(names.iter().filter(|n| *n == "The Red Relaxer").count(), 2, "different chairs");
        assert_eq!(names.iter().filter(|n| *n == "Red Cloth Vest").count(), 2, "male and female");
        assert_eq!(catalog.copies_of(2000007), [2000000, 2000007], "either copy finds both");
        assert_eq!(catalog.copies_of(3010002), [3010002]);
        assert!(catalog.item(2000007).is_some(), "a copy still opens by its own ID");
    }

    #[test]
    fn use_setup_and_etc_items_are_grouped_by_their_number() {
        let catalog = Catalog::new(
            Vec::new(),
            vec![
                other(2000000, "Red Potion", ItemKind::Use, ""),
                other(2002000, "Dexterity Potion", ItemKind::Use, ""),
                other(2002007, "Smithing Kit", ItemKind::Use, ""),
                other(2020001, "Fried Chicken", ItemKind::Use, ""),
                other(2030001, "Return Scroll to Lith Harbor", ItemKind::Use, ""),
                other(2070000, "Subi Throwing Stars", ItemKind::Use, ""),
                other(2190000, "Lie Detector Test", ItemKind::Use, ""),
                other(3010000, "The Green Relaxer", ItemKind::Setup, ""),
                other(3992000, "Santa Ornament", ItemKind::Setup, ""),
                other(4000000, "Snail Shell", ItemKind::Etc, ""),
                other(4010000, "Bronze Ore", ItemKind::Etc, ""),
                ItemRecord { quest_item: true, ..other(4031013, "Sera's Mirror", ItemKind::Etc, "") },
                ItemRecord { quest_item: true, ..other(4000999, "Quest Shell", ItemKind::Etc, "") },
            ],
        );
        let group = |name: &str| catalog.items.iter().find(|i| i.name == name).and_then(item_group);
        assert_eq!(group("Red Potion"), Some("Potions"));
        assert_eq!(group("Dexterity Potion"), Some("Buffs"));
        assert_eq!(group("Smithing Kit"), Some("Crafting"), "a kit isn't a potion");
        assert_eq!(group("Fried Chicken"), Some("Food"));
        assert_eq!(group("Return Scroll to Lith Harbor"), Some("Return scrolls"));
        assert_eq!(group("Subi Throwing Stars"), Some("Throwing stars"));
        assert_eq!(group("Quest Shell"), Some("Quest items"), "the quest mark comes before the number");

        let filters = catalog.item_filters();
        assert_eq!(
            filters.use_groups,
            ["Potions", "Buffs", "Food", "Return scrolls", "Throwing stars", "Crafting", "Other"],
            "in the set order"
        );
        assert_eq!(filters.setup_groups, ["Chairs", "Event"]);
        assert_eq!(filters.etc_groups, ["Monster loot", "Ores & gems", "Quest items"]);

        let in_group = |kind, slot: &str| ItemQuery { kind: Some(kind), slot: Some(slot.into()), ..ItemQuery::default() };
        assert_eq!(found(&catalog, in_group(ItemKind::Use, "Potions")), ["Red Potion"]);
        assert_eq!(found(&catalog, in_group(ItemKind::Etc, "Quest items")), ["Quest Shell", "Sera's Mirror"]);
    }

    #[test]
    fn every_group_an_item_gets_is_offered() {
        let kinds = [(ItemKind::Use, &USE_GROUPS[..], 2000), (ItemKind::Setup, &SETUP_GROUPS[..], 3000), (ItemKind::Etc, &ETC_GROUPS[..], 4000)];
        for (kind, offered, first) in kinds {
            for block in first..first + 1000 {
                let group = item_group(&other(block * 1000, "X", kind, "")).unwrap();
                assert!(offered.contains(&group), "{group} for block {block}");
            }
        }
    }
}
