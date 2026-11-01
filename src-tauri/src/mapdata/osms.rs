//! The parts of OSMS Data Explorer's v1 JSON that the map card and build planner use.
//!
//! Only the fields listed here are read. Every `*_url` image field is left out on purpose, so
//! none can reach the cache or the screen. Coordinates are read only to tell which third of its
//! map a portal stands in, and are dropped as the file is parsed: only that coarse place is
//! kept, and nothing is ever drawn at a map's real positions.

use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize};

/// The files Leafline needs, in the order they are fetched.
pub const FILES: [&str; 6] = ["maps.json", "monsters.json", "npcs.json", "quests.json", "skills.json", "items.json"];

/// The only `schema_version` this reader understands. OSMS ships breaking changes as `v2/`.
pub const SCHEMA_VERSION: u32 = 1;

/// A portal destination that means "no destination".
pub const NO_MAP: u32 = 999_999_999;

/// `meta.json`: what the dataset was built from, and a checksum for each file.
#[derive(Debug, Clone, Deserialize)]
pub struct Meta {
    pub schema_version: u32,
    pub client_build: String,
    pub data_updated_at: String,
    pub files: HashMap<String, FileMeta>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FileMeta {
    pub sha256: String,
    pub records: u32,
    pub updated_at: String,
}

/// Less of a spread than this across a map, in the game's pixels, says nothing about where
/// things stand in it.
const MIN_SPREAD: i32 = 150;

/// The third of its map a portal stands in, across.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Left,
    Centre,
    Right,
}

impl Side {
    /// In order across the map, so a third's number is its place here.
    pub const ALL: [Self; 3] = [Self::Left, Self::Centre, Self::Right];
}

/// The third of its map a portal stands in, down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Height {
    High,
    Middle,
    Low,
}

impl Height {
    /// In order down the map, so a third's number is its place here.
    pub const ALL: [Self; 3] = [Self::High, Self::Middle, Self::Low];
}

/// The file's fields, coordinates included, until [`MapRecord`] takes what it keeps.
#[derive(Deserialize)]
struct RawMap {
    id: u32,
    name: String,
    street_name: Option<String>,
    region: Option<String>,
    #[serde(default)]
    is_town: bool,
    return_map_id: Option<u32>,
    map_mark: Option<String>,
    #[serde(default)]
    exits: Vec<u32>,
    #[serde(default)]
    portals: Vec<RawPortal>,
    #[serde(default)]
    mobs: Vec<Point>,
    #[serde(default)]
    npcs: Vec<Point>,
}

#[derive(Deserialize)]
struct RawPortal {
    name: String,
    dest_map_id: Option<u32>,
    dest_portal: Option<String>,
    x: Option<i32>,
    y: Option<i32>,
}

#[derive(Deserialize)]
struct Point {
    x: Option<i32>,
    y: Option<i32>,
}

/// The lowest and highest of `values`, if they spread far enough to mean anything.
fn spread(values: impl Iterator<Item = i32>) -> Option<(i32, i32)> {
    let values: Vec<i32> = values.collect();
    let (low, high) = (*values.iter().min()?, *values.iter().max()?);
    (high - low >= MIN_SPREAD).then_some((low, high))
}

/// Which third of `range` `value` falls in: 0, 1 or 2.
fn third(value: i32, (low, high): (i32, i32)) -> usize {
    (i64::from(value - low) * 3 / i64::from(high - low + 1)) as usize
}

impl From<RawMap> for MapRecord {
    /// Everything on the map marks how far it reaches, so a lone portal still has a place.
    fn from(raw: RawMap) -> Self {
        let places = || {
            let portals = raw.portals.iter().map(|p| (p.x, p.y));
            portals.chain(raw.mobs.iter().chain(&raw.npcs).map(|p| (p.x, p.y)))
        };
        let across = spread(places().filter_map(|(x, _)| x));
        let down = spread(places().filter_map(|(_, y)| y));
        let portals = raw
            .portals
            .iter()
            .map(|p| Portal {
                name: p.name.clone(),
                dest_map_id: p.dest_map_id,
                dest_portal: p.dest_portal.clone(),
                side: p.x.zip(across).map(|(x, range)| Side::ALL[third(x, range)]),
                height: p.y.zip(down).map(|(y, range)| Height::ALL[third(y, range)]),
            })
            .collect();
        Self {
            id: raw.id,
            name: raw.name,
            street_name: raw.street_name,
            region: raw.region,
            is_town: raw.is_town,
            return_map_id: raw.return_map_id,
            map_mark: raw.map_mark,
            exits: raw.exits,
            portals,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(from = "RawMap")]
pub struct MapRecord {
    pub id: u32,
    pub name: String,
    pub street_name: Option<String>,
    pub region: Option<String>,
    pub is_town: bool,
    pub return_map_id: Option<u32>,
    pub map_mark: Option<String>,
    /// Maps reachable from this one.
    pub exits: Vec<u32>,
    pub portals: Vec<Portal>,
}

/// A portal's name, where it leads, and roughly where it stands: which third of the map across
/// and down, when the map's data can tell.
#[derive(Debug, Clone, PartialEq)]
pub struct Portal {
    pub name: String,
    pub dest_map_id: Option<u32>,
    pub dest_portal: Option<String>,
    pub side: Option<Side>,
    pub height: Option<Height>,
}

#[derive(Deserialize)]
struct RawMonster {
    id: u32,
    name: String,
    #[serde(default)]
    level: u32,
    #[serde(default)]
    hp: u32,
    mp: Option<u32>,
    #[serde(default)]
    exp: u32,
    #[serde(default)]
    is_boss: bool,
    #[serde(default)]
    undead: bool,
    #[serde(default)]
    aggro: bool,
    #[serde(default)]
    invincible: bool,
    hp_recovery: Option<u32>,
    mp_recovery: Option<u32>,
    #[serde(default)]
    stats: HashMap<String, serde_json::Value>,
    #[serde(default)]
    elements: HashMap<String, String>,
    #[serde(default)]
    attacks: Vec<RawAttack>,
    #[serde(default)]
    skills: Vec<RawMobSkill>,
    #[serde(default)]
    revives_into: Vec<Split>,
    #[serde(default)]
    spawns: Vec<Spawn>,
}

#[derive(Deserialize)]
struct RawAttack {
    #[serde(default)]
    magic: bool,
    element: Option<String>,
}

#[derive(Deserialize)]
struct RawMobSkill {
    kind: Option<String>,
}

/// In the order a card lists them.
const ELEMENT_EFFECTS: [&str; 3] = ["Weak", "Strong", "Immune"];

impl From<RawMonster> for MonsterRecord {
    fn from(raw: RawMonster) -> Self {
        let stat = |key: &str| raw.stats.get(key).and_then(number_in);
        let mut elements: Vec<ElementEffect> = raw
            .elements
            .iter()
            .map(|(element, effect)| ElementEffect { element: element.clone(), effect: effect.clone() })
            .collect();
        elements.sort_by_key(|e| (ELEMENT_EFFECTS.iter().position(|known| *known == e.effect), e.element.clone()));
        let mut attack_elements: Vec<String> = raw.attacks.iter().filter_map(|a| a.element.clone()).collect();
        attack_elements.sort();
        attack_elements.dedup();
        let mut casts: Vec<String> = raw.skills.iter().filter_map(|s| s.kind.clone()).collect();
        casts.sort();
        casts.dedup();
        let details = MonsterDetails {
            mp: raw.mp,
            attack: stat("PADamage"),
            magic: stat("MADamage"),
            weapon_def: stat("PDDamage"),
            magic_def: stat("MDDamage"),
            accuracy: stat("acc"),
            evasion: stat("eva"),
            speed: stat("speed"),
            knockback: stat("pushed"),
            hp_recovery: raw.hp_recovery,
            mp_recovery: raw.mp_recovery,
            aggressive: raw.aggro,
            invincible: raw.invincible,
            elements,
            magic_attacks: raw.attacks.iter().any(|a| a.magic),
            attack_elements,
            casts,
            splits_into: raw.revives_into,
        };
        Self {
            id: raw.id,
            name: raw.name,
            level: raw.level,
            hp: raw.hp,
            exp: raw.exp,
            is_boss: raw.is_boss,
            undead: raw.undead,
            spawns: raw.spawns,
            details,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(from = "RawMonster")]
pub struct MonsterRecord {
    pub id: u32,
    pub name: String,
    pub level: u32,
    pub hp: u32,
    pub exp: u32,
    pub is_boss: bool,
    pub undead: bool,
    pub spawns: Vec<Spawn>,
    pub details: MonsterDetails,
}

/// What the file says of a monster beyond the few numbers maps and routes use. Icon and animation
/// URLs are left out, like every other image. A stat the file leaves out stays `None`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonsterDetails {
    pub mp: Option<u32>,
    pub attack: Option<i64>,
    pub magic: Option<i64>,
    pub weapon_def: Option<i64>,
    pub magic_def: Option<i64>,
    pub accuracy: Option<i64>,
    pub evasion: Option<i64>,
    /// Added to its movement speed: below zero is slower.
    pub speed: Option<i64>,
    /// The damage one hit must deal to knock it back.
    pub knockback: Option<i64>,
    pub hp_recovery: Option<u32>,
    pub mp_recovery: Option<u32>,
    /// Comes at players on sight.
    pub aggressive: bool,
    pub invincible: bool,
    /// Weak ones first, then strong, then immune.
    pub elements: Vec<ElementEffect>,
    pub magic_attacks: bool,
    /// The elements its attacks carry.
    pub attack_elements: Vec<String>,
    /// What its skills do: `self_buff` or `debuff`. The file doesn't name the skills.
    pub casts: Vec<String>,
    /// What it turns into when defeated.
    pub splits_into: Vec<Split>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementEffect {
    pub element: String,
    /// `Weak`, `Strong` or `Immune`.
    pub effect: String,
}

/// `count` of a monster.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Split {
    pub mob_id: u32,
    pub count: u32,
}

/// How many of a monster a map spawns.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Spawn {
    pub map_id: u32,
    #[serde(default)]
    pub count: u32,
    /// The respawn time the file lists, in seconds, when it is one plain number. The game's
    /// server decides the real timing, so this is only a rough guide.
    #[serde(default, rename = "mob_time", deserialize_with = "plain_seconds")]
    pub respawn: Option<u32>,
}

/// A time written as `3600.00`. Some are ranges, which say nothing a card could use.
fn plain_seconds<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<u32>, D::Error> {
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    let seconds = value.and_then(|v| v.as_f64().or_else(|| v.as_str()?.trim().parse().ok()));
    Ok(seconds.filter(|s| *s > 0.0).map(|s| s as u32))
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct NpcRecord {
    pub id: u32,
    pub name: String,
    #[serde(default)]
    pub maps: Vec<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(from = "RawQuest")]
pub struct QuestRecord {
    pub id: u32,
    pub name: String,
    /// The NPC who gives the quest.
    pub npc_id: Option<u32>,
    pub level_min: Option<u32>,
    /// The quest this one leads to.
    pub next_quest_id: Option<u32>,
    /// The monsters to hunt.
    pub requirements: Vec<MobRequirement>,
    /// Items the quest can give, fixed, chosen or at random.
    pub reward_items: Vec<RewardItem>,
    pub region: Option<String>,
    /// The story so far, a line for each stage.
    pub description: Option<String>,
    /// The name of the chain of quests this one is part of.
    pub chain: Option<String>,
    /// `daily`, `weekly` or `repeatable`.
    pub repeat: Option<String>,
    /// Items to bring.
    pub items: Vec<ItemNeed>,
    /// Quests to finish first.
    pub follows: Vec<u32>,
    pub reward_exp: u32,
    pub reward_mesos: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RewardItem {
    pub item_id: u32,
    pub count: u32,
    /// Always given. Otherwise it is one of a choice, or down to chance.
    pub sure: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemNeed {
    pub item_id: u32,
    pub count: u32,
}

#[derive(Deserialize)]
struct RawQuest {
    id: u32,
    name: String,
    npc_id: Option<u32>,
    level_min: Option<u32>,
    next_quest_id: Option<u32>,
    region: Option<String>,
    description: Option<String>,
    chain: Option<String>,
    repeat: Option<String>,
    #[serde(default)]
    requirements: Vec<serde_json::Value>,
    #[serde(default)]
    rewards: Vec<serde_json::Value>,
    #[serde(default)]
    reward_choices: Vec<serde_json::Value>,
    #[serde(default)]
    reward_random: Vec<serde_json::Value>,
}

impl From<RawQuest> for QuestRecord {
    fn from(raw: RawQuest) -> Self {
        let id_of = |value: &serde_json::Value| value.get("id")?.as_u64().map(|id| id as u32);
        let count_of = |value: &serde_json::Value| value.get("count").and_then(|c| c.as_u64()).map_or(1, |c| c as u32);
        let kind = |value: &serde_json::Value, kind: &str| value.get("type").and_then(|t| t.as_str()) == Some(kind);
        let of_kind = |list: &'_ [serde_json::Value], wanted: &'static str| {
            list.iter().filter(|v| kind(v, wanted)).cloned().collect::<Vec<_>>()
        };
        let amount = |wanted: &'static str| {
            of_kind(&raw.rewards, wanted).iter().filter_map(|r| r.get("value")?.as_u64()).sum::<u64>() as u32
        };
        let requirements: Vec<MobRequirement> = of_kind(&raw.requirements, "mob")
            .iter()
            .filter_map(|r| Some(MobRequirement { mob_id: id_of(r)?, count: count_of(r) }))
            .collect();
        let items: Vec<ItemNeed> = of_kind(&raw.requirements, "item")
            .iter()
            .filter_map(|r| Some(ItemNeed { item_id: id_of(r)?, count: count_of(r) }))
            .collect();
        let follows: Vec<u32> = of_kind(&raw.requirements, "quest").iter().filter_map(id_of).collect();
        let reward = |sure: bool| {
            move |value: &serde_json::Value| Some(RewardItem { item_id: id_of(value)?, count: count_of(value), sure })
        };
        let fixed = raw.rewards.iter().filter(|r| kind(r, "item")).filter_map(|r| {
            reward(r.get("guaranteed").and_then(|g| g.as_bool()).unwrap_or(true))(r)
        });
        // Choices and random rewards list their items in groups, some of them for one class.
        let grouped = raw
            .reward_choices
            .iter()
            .chain(&raw.reward_random)
            .filter_map(|r| r.get("groups")?.as_array())
            .flatten()
            .filter_map(|group| group.get("items")?.as_array())
            .flatten()
            .filter_map(reward(false));
        let mut reward_items: Vec<RewardItem> = Vec::new();
        for item in fixed.chain(grouped) {
            if !reward_items.iter().any(|r| r.item_id == item.item_id) {
                reward_items.push(item);
            }
        }
        Self {
            id: raw.id,
            name: raw.name,
            npc_id: raw.npc_id,
            level_min: raw.level_min,
            next_quest_id: raw.next_quest_id,
            requirements,
            reward_exp: amount("exp"),
            reward_mesos: amount("money"),
            reward_items,
            region: raw.region,
            description: raw.description,
            chain: raw.chain,
            repeat: raw.repeat,
            items,
            follows,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MobRequirement {
    pub mob_id: u32,
    pub count: u32,
}

/// The tier of the crafting disciplines, which share the file but use no skill points.
const CRAFTING: &str = "Crafting";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct SkillRecord {
    pub id: u32,
    pub name: String,
    pub job_id: u32,
    #[serde(default)]
    pub class_name: String,
    pub job_tier: String,
    #[serde(default)]
    pub max_level: u32,
    #[serde(default)]
    pub passive: bool,
    pub description: Option<String>,
    #[serde(default)]
    pub required_skills: Vec<SkillNeed>,
    /// What the skill does at each level, from level 1.
    #[serde(default)]
    pub level_descriptions: Vec<String>,
}

/// A skill that must reach `level` first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct SkillNeed {
    pub skill_id: u32,
    pub level: u32,
}

/// The file's equipment stats and the name each goes by on screen, in the order shown.
const ITEM_STATS: [(&str, &str); 16] = [
    ("incPAD", "Attack"),
    ("incMAD", "Magic"),
    ("incPDD", "Weapon Def."),
    ("incMDD", "Magic Def."),
    ("incSTR", "STR"),
    ("incDEX", "DEX"),
    ("incINT", "INT"),
    ("incLUK", "LUK"),
    ("incMHP", "MaxHP"),
    ("incMMP", "MaxMP"),
    ("incACC", "Accuracy"),
    ("incEVA", "Evasion"),
    ("incCRT", "Crit. Rate"),
    ("incCRD", "Crit. Damage"),
    ("incSpeed", "Speed"),
    ("incJump", "Jump"),
];

#[derive(Deserialize)]
struct RawItem {
    id: u32,
    name: String,
    category: String,
    sub_category: Option<String>,
    gender: Option<String>,
    description: Option<String>,
    price: Option<serde_json::Value>,
    #[serde(default)]
    req_classes: Vec<String>,
    #[serde(default)]
    stats: HashMap<String, serde_json::Value>,
    weapon: Option<RawWeapon>,
    scroll: Option<RawScroll>,
}

#[derive(Deserialize)]
struct RawWeapon {
    #[serde(rename = "type")]
    kind: Option<String>,
}

#[derive(Deserialize)]
struct RawScroll {
    slot: Option<String>,
    tier: Option<String>,
    success_rate: Option<u32>,
}

/// A number that may be written as text, as a few values are.
fn number_in(value: &serde_json::Value) -> Option<i64> {
    value.as_i64().or_else(|| value.as_str()?.trim().parse().ok())
}

/// What a scroll goes on, in the words items use for their slots and weapon types. Scrolls for
/// hats, tops and bottoms are filed under `Other`, so those come from the name.
fn scroll_fits(slot: &str, name: &str) -> String {
    const BY_NAME: [(&str, &str); 3] = [("Hat", "Hat"), ("Topwear", "Top"), ("Bottomwear", "Bottom")];
    match slot {
        "Other" => BY_NAME.iter().find(|(word, _)| name.starts_with(word)).map_or(slot, |(_, fits)| fits).to_string(),
        "1H Blunt" | "2H Blunt" => format!("{slot} Weapon"),
        _ => slot.to_string(),
    }
}

impl From<RawItem> for ItemRecord {
    fn from(raw: RawItem) -> Self {
        let number = |key: &str| number_in(raw.stats.get(key)?);
        let need = |key: &str| number(key).map_or(0, |v| v.max(0) as u32);
        let flag = |key: &str| number(key).is_some_and(|v| v != 0) || raw.stats.get(key).and_then(|v| v.as_bool()) == Some(true);
        let stats = ITEM_STATS
            .iter()
            .filter_map(|(key, label)| Some(ItemStat { label: (*label).to_string(), value: number(key)? }))
            .collect();
        let slot = raw.sub_category.clone().unwrap_or_default();
        let scroll = raw.scroll.as_ref().map(|s| ScrollInfo {
            fits: scroll_fits(s.slot.as_deref().unwrap_or_default(), &raw.name),
            tier: s.tier.clone(),
            success_rate: s.success_rate,
        });
        Self {
            id: raw.id,
            kind: ItemKind::of(&raw.category, &slot),
            slot,
            gender: raw.gender.clone(),
            req_level: need("reqLevel"),
            req_str: need("reqSTR"),
            req_dex: need("reqDEX"),
            req_int: need("reqINT"),
            req_luk: need("reqLUK"),
            req_fame: need("reqPOP"),
            weapon_type: raw.weapon.as_ref().and_then(|w| w.kind.clone()),
            attack_speed: number("attackSpeed").map(|v| v.max(0) as u32),
            upgrade_slots: number("tuc").map(|v| v.max(0) as u32),
            quest_item: flag("quest") || flag("pquest"),
            untradable: flag("tradeBlock"),
            stats,
            price: raw.price.as_ref().and_then(number_in),
            description: raw.description.map(|d| d.trim().to_string()).filter(|d| !d.is_empty()),
            scroll,
            classes: raw.req_classes,
            name: raw.name,
        }
    }
}

/// The groups the game's inventory sorts items into, with scrolls kept apart from other Use
/// items because players look for them separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Equip,
    Use,
    Scroll,
    Setup,
    Etc,
}

impl ItemKind {
    fn of(category: &str, slot: &str) -> Self {
        match (category, slot) {
            ("Equipment", _) => Self::Equip,
            ("Consumable", "Scroll") => Self::Scroll,
            ("Consumable", _) => Self::Use,
            ("Setup", _) => Self::Setup,
            _ => Self::Etc,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Equip => "equip",
            Self::Use => "use",
            Self::Scroll => "scroll",
            Self::Setup => "setup",
            Self::Etc => "etc",
        }
    }

    pub fn named(name: &str) -> Self {
        match name {
            "equip" => Self::Equip,
            "use" => Self::Use,
            "scroll" => Self::Scroll,
            "setup" => Self::Setup,
            _ => Self::Etc,
        }
    }
}

/// An item of any kind. Icon and sprite URLs in the file are left out, like every other image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(from = "RawItem", rename_all(serialize = "camelCase"))]
pub struct ItemRecord {
    pub id: u32,
    pub name: String,
    pub kind: ItemKind,
    pub slot: String,
    pub gender: Option<String>,
    /// Who can use it, as the tooltip's job flags name them. Empty means anyone.
    pub classes: Vec<String>,
    pub weapon_type: Option<String>,
    pub req_level: u32,
    pub req_str: u32,
    pub req_dex: u32,
    pub req_int: u32,
    pub req_luk: u32,
    pub req_fame: u32,
    /// The data's own number for a weapon's speed. Lower is faster.
    pub attack_speed: Option<u32>,
    /// Upgrade slots on a clean copy.
    pub upgrade_slots: Option<u32>,
    pub quest_item: bool,
    pub untradable: bool,
    pub stats: Vec<ItemStat>,
    /// The price the data lists, in mesos. What it means in Classic isn't confirmed.
    pub price: Option<i64>,
    pub description: Option<String>,
    pub scroll: Option<ScrollInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemStat {
    pub label: String,
    pub value: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollInfo {
    /// The slot or weapon type it goes on, or `Other`.
    pub fits: String,
    pub tier: Option<String>,
    pub success_rate: Option<u32>,
}

/// Everything the map card and build planner are built from.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Dataset {
    pub maps: Vec<MapRecord>,
    pub monsters: Vec<MonsterRecord>,
    pub npcs: Vec<NpcRecord>,
    pub quests: Vec<QuestRecord>,
    pub skills: Vec<SkillRecord>,
    pub items: Vec<ItemRecord>,
}

/// One downloaded file, parsed.
#[derive(Debug, Clone, PartialEq)]
pub enum Records {
    Maps(Vec<MapRecord>),
    Monsters(Vec<MonsterRecord>),
    Npcs(Vec<NpcRecord>),
    Quests(Vec<QuestRecord>),
    Skills(Vec<SkillRecord>),
    Items(Vec<ItemRecord>),
}

impl Records {
    /// Parses the file called `name`, one of [`FILES`].
    pub fn parse(name: &str, body: &[u8]) -> serde_json::Result<Self> {
        Ok(match name {
            "maps.json" => Self::Maps(serde_json::from_slice(body)?),
            "monsters.json" => Self::Monsters(serde_json::from_slice(body)?),
            "npcs.json" => Self::Npcs(serde_json::from_slice(body)?),
            "quests.json" => Self::Quests(serde_json::from_slice(body)?),
            "skills.json" => {
                let mut skills: Vec<SkillRecord> = serde_json::from_slice(body)?;
                skills.retain(|s| s.job_tier != CRAFTING);
                Self::Skills(skills)
            }
            "items.json" => Self::Items(serde_json::from_slice(body)?),
            other => unreachable!("not a map data file: {other}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_only_what_the_card_uses() {
        // Made-up records in OSMS's shape, image URLs and coordinates included.
        let json = br#"[{"id": 7, "name": "Fernway Path", "street_name": "Mossy Road", "is_town": false,
            "exits": [8], "portals": [{"name": "east00", "type": 2, "x": 10, "y": 20, "dest_map_id": 8,
            "dest_portal": "west00"}, {"name": "sp", "type": 0, "x": 1, "y": 2}],
            "mobs": [{"mob_id": 1, "x": 5, "y": 6}], "minimap_url": "https://example.invalid/m.png"}]"#;
        let Records::Maps(maps) = Records::parse("maps.json", json).unwrap() else { panic!() };
        assert_eq!(maps[0].portals.len(), 2);
        assert_eq!(maps[0].portals[0].dest_map_id, Some(8));
        assert_eq!(maps[0].portals[1].dest_map_id, None);
        assert_eq!(maps[0].street_name.as_deref(), Some("Mossy Road"));
    }

    fn portals_of(json: &[u8]) -> Vec<Portal> {
        let Records::Maps(maps) = Records::parse("maps.json", json).unwrap() else { panic!() };
        maps.into_iter().next().unwrap().portals
    }

    #[test]
    fn places_portals_in_coarse_thirds() {
        // Made-up map, 900 wide and 600 tall by what stands on it.
        let portals = portals_of(
            br#"[{"id": 1, "name": "Fernway", "portals": [
            {"name": "a", "x": 10, "y": 300, "dest_map_id": 2},
            {"name": "b", "x": 450, "y": 10, "dest_map_id": 3},
            {"name": "c", "x": 890, "y": 590, "dest_map_id": 4},
            {"name": "d", "x": 890, "y": 300, "dest_map_id": 5}],
            "mobs": [{"mob_id": 1, "x": 0, "y": 0}], "npcs": [{"npc_id": 1, "x": 900, "y": 600}]}]"#,
        );
        let places: Vec<_> = portals.iter().map(|p| (p.side, p.height)).collect();
        assert_eq!(
            places,
            [
                (Some(Side::Left), Some(Height::Middle)),
                (Some(Side::Centre), Some(Height::High)),
                (Some(Side::Right), Some(Height::Low)),
                (Some(Side::Right), Some(Height::Middle)),
            ]
        );
    }

    #[test]
    fn the_far_edges_of_a_map_are_in_its_outer_thirds() {
        let portals = portals_of(
            br#"[{"id": 1, "name": "Fernway", "portals": [
            {"name": "a", "x": 0, "y": 0}, {"name": "b", "x": 300, "y": 300}]}]"#,
        );
        assert_eq!((portals[0].side, portals[0].height), (Some(Side::Left), Some(Height::High)));
        assert_eq!((portals[1].side, portals[1].height), (Some(Side::Right), Some(Height::Low)));
    }

    #[test]
    fn a_map_with_too_little_on_it_places_nothing() {
        // A lone portal, and a flat map: nothing to measure against, across or down.
        let lone = portals_of(br#"[{"id": 1, "name": "Fernway", "portals": [{"name": "a", "x": 5, "y": 5}]}]"#);
        assert_eq!((lone[0].side, lone[0].height), (None, None));
        let flat = portals_of(
            br#"[{"id": 1, "name": "Fernway", "portals": [
            {"name": "a", "x": 0, "y": 50}, {"name": "b", "x": 900, "y": 60}]}]"#,
        );
        assert_eq!((flat[0].side, flat[0].height), (Some(Side::Left), None));
        let unplaced = portals_of(br#"[{"id": 1, "name": "Fernway", "portals": [{"name": "a"}]}]"#);
        assert_eq!((unplaced[0].side, unplaced[0].height), (None, None));
    }

    #[test]
    fn reads_skills_but_not_crafting_or_icons() {
        // Made-up skills in OSMS's shape. Crafting records carry no levels at all.
        let json = br#"[
            {"id": 7, "name": "Pebble Toss", "job_id": 400, "class_name": "Rogue", "job_tier": "1st Job",
             "max_level": 2, "passive": false, "description": "Throws a pebble.", "icon_url": "https://example.invalid/i.png",
             "required_skills": [{"skill_id": 6, "level": 3}], "level_descriptions": ["Damage 1", "Damage 2"]},
            {"id": 92000000, "name": "Knitting", "job_id": 9200, "job_tier": "Crafting", "max_level": 10}]"#;
        let Records::Skills(skills) = Records::parse("skills.json", json).unwrap() else { panic!() };
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].required_skills, vec![SkillNeed { skill_id: 6, level: 3 }]);
        assert_eq!(skills[0].level_descriptions.len(), 2);
    }

    #[test]
    fn reads_every_kind_of_item_and_numbers_written_as_text() {
        let json = br#"[
            {"id": 1, "name": "Twig Wand", "category": "Equipment", "sub_category": "Weapon", "price": "5",
             "req_classes": ["Magician"], "weapon": {"type": "Wand", "swing_reach_px": 88},
             "stats": {"reqLevel": 8, "reqINT": 20, "reqPOP": 2, "incMAD": 12, "incPDD": "3", "tuc": 7, "attackSpeed": 6},
             "icon_url": "https://example.invalid/w.png", "sprite_urls": ["https://example.invalid/s.png"]},
            {"id": 2, "name": "Plain Cap", "category": "Equipment", "sub_category": "Hat", "gender": "male"},
            {"id": 3, "name": "Red Berry", "category": "Consumable", "sub_category": "Consume",
             "description": " Restores 5 HP.\n", "stats": {"tradeBlock": 1}},
            {"id": 4, "name": "Bent Twig", "category": "Etc", "sub_category": "Etc", "stats": {"quest": 1}},
            {"id": 5, "name": "Moss Stool", "category": "Setup", "sub_category": "Install"}]"#;
        let Records::Items(items) = Records::parse("items.json", json).unwrap() else { panic!() };
        let kinds: Vec<_> = items.iter().map(|i| i.kind).collect();
        assert_eq!(kinds, [ItemKind::Equip, ItemKind::Equip, ItemKind::Use, ItemKind::Etc, ItemKind::Setup]);
        let wand = &items[0];
        assert_eq!((wand.req_level, wand.req_int, wand.req_str, wand.req_fame), (8, 20, 0, 2));
        assert_eq!(wand.weapon_type.as_deref(), Some("Wand"));
        assert_eq!(wand.classes, ["Magician"]);
        assert_eq!((wand.upgrade_slots, wand.attack_speed, wand.price), (Some(7), Some(6), Some(5)));
        let stats: Vec<_> = wand.stats.iter().map(|s| (s.label.as_str(), s.value)).collect();
        assert_eq!(stats, [("Magic", 12), ("Weapon Def.", 3)]);
        assert!(items[1].classes.is_empty() && items[1].req_level == 0);
        assert_eq!((items[2].description.as_deref(), items[2].untradable), (Some("Restores 5 HP."), true));
        assert!(items[3].quest_item);
    }

    #[test]
    fn scrolls_say_what_they_go_on_in_the_words_items_use() {
        let json = br#"[
            {"id": 1, "name": "Hat Moss Scroll: Lesser", "category": "Consumable", "sub_category": "Scroll",
             "scroll": {"slot": "Other", "stat": "Moss", "tier": "Lesser", "success_rate": 100}},
            {"id": 2, "name": "Bottomwear Moss Scroll: Lesser", "category": "Consumable", "sub_category": "Scroll",
             "scroll": {"slot": "Other", "tier": "Lesser"}},
            {"id": 3, "name": "One-Handed Blunt Moss Scroll", "category": "Consumable", "sub_category": "Scroll",
             "scroll": {"slot": "1H Blunt"}},
            {"id": 4, "name": "Cape Moss Scroll", "category": "Consumable", "sub_category": "Scroll",
             "scroll": {"slot": "Cape"}}]"#;
        let Records::Items(items) = Records::parse("items.json", json).unwrap() else { panic!() };
        assert!(items.iter().all(|i| i.kind == ItemKind::Scroll));
        let fits: Vec<_> = items.iter().map(|i| i.scroll.as_ref().unwrap().fits.as_str()).collect();
        assert_eq!(fits, ["Hat", "Bottom", "1H Blunt Weapon", "Cape"]);
        assert_eq!(items[0].scroll.as_ref().unwrap().success_rate, Some(100));
    }

    #[test]
    fn reads_what_a_monster_can_do_but_not_its_images() {
        // A made-up monster in OSMS's shape.
        let json = br#"[
            {"id": 7, "name": "Moss Golem", "level": 30, "hp": 900, "mp": 40, "exp": 50, "is_boss": true, "aggro": true,
             "hp_recovery": 12, "stats": {"PADamage": 80, "PDDamage": 0, "acc": 60, "speed": -20, "pushed": 75},
             "elements": {"Ice": "Immune", "Holy": "Strong", "Fire": "Weak", "Earth": "Weak"},
             "attacks": [{"index": 1, "ratio": 125}, {"index": 2, "magic": true, "element": "Ice"}],
             "skills": [{"id": 1, "kind": "self_buff"}, {"id": 2, "kind": "debuff"}, {"id": 3, "kind": "debuff"}],
             "revives_into": [{"mob_id": 8, "count": 3}],
             "spawns": [{"map_id": 1, "count": 1, "mob_time": "3600.00"}, {"map_id": 2, "count": 2, "mob_time": "180.00-1800.00 [720.00]"},
                        {"map_id": 3, "count": 4, "mob_time": null}, {"map_id": 4, "count": 1}],
             "icon_url": "https://example.invalid/i.png", "animation_urls": {"move": "https://example.invalid/m.webp"}},
            {"id": 8, "name": "Mossling"}]"#;
        let Records::Monsters(monsters) = Records::parse("monsters.json", json).unwrap() else { panic!() };
        let golem = &monsters[0];
        let details = &golem.details;
        assert_eq!((details.mp, details.attack, details.weapon_def, details.magic), (Some(40), Some(80), Some(0), None));
        assert_eq!((details.speed, details.knockback, details.hp_recovery), (Some(-20), Some(75), Some(12)));
        assert!(details.aggressive && details.magic_attacks && !details.invincible);
        let elements: Vec<_> = details.elements.iter().map(|e| (e.element.as_str(), e.effect.as_str())).collect();
        assert_eq!(elements, [("Earth", "Weak"), ("Fire", "Weak"), ("Holy", "Strong"), ("Ice", "Immune")]);
        assert_eq!(details.attack_elements, ["Ice"]);
        assert_eq!(details.casts, ["debuff", "self_buff"]);
        assert_eq!(details.splits_into, [Split { mob_id: 8, count: 3 }]);
        let times: Vec<_> = golem.spawns.iter().map(|s| s.respawn).collect();
        assert_eq!(times, [Some(3600), None, None, None]);
        assert_eq!(monsters[1].details, MonsterDetails::default());

        let stored = serde_json::to_string(details).unwrap();
        assert!(!stored.contains("http"));
        assert_eq!(&serde_json::from_str::<MonsterDetails>(&stored).unwrap(), details, "the cache gets back what it stored");
    }

    #[test]
    fn keeps_only_mob_requirements() {
        let json = br#"[{"id": 1, "name": "Pebble Patrol", "npc_id": 3, "requirements": [
            {"type": "mob", "id": 11, "count": 20}, {"type": "item", "id": 400, "count": 5},
            {"type": "quest", "id": 9, "state": 2}]}]"#;
        let Records::Quests(quests) = Records::parse("quests.json", json).unwrap() else { panic!() };
        assert_eq!(quests[0].requirements, vec![MobRequirement { mob_id: 11, count: 20 }]);
    }

    #[test]
    fn gathers_every_item_a_quest_can_give() {
        let json = br#"[{"id": 1, "name": "Pebble Patrol",
            "rewards": [{"type": "exp", "value": 2}, {"type": "item", "guaranteed": true, "id": 40, "count": 3},
                {"type": "item", "guaranteed": false, "id": 41, "count": 1, "prop": 1}],
            "reward_choices": [{"type": "choose_one", "groups": [{"classes": ["Thief"], "items": [{"id": 42, "count": 1}]},
                {"classes": ["Warrior"], "items": [{"id": 43, "count": 1}]}]}],
            "reward_random": [{"type": "weighted_random", "groups": [{"items": [{"id": 40, "count": 1, "weight": 1},
                {"id": 44, "count": 1, "weight": 1}]}]}]}]"#;
        let Records::Quests(quests) = Records::parse("quests.json", json).unwrap() else { panic!() };
        let rewards = &quests[0].reward_items;
        assert_eq!(rewards.iter().map(|r| r.item_id).collect::<Vec<_>>(), [40, 41, 42, 43, 44]);
        assert_eq!((rewards[0].count, rewards[0].sure), (3, true));
        assert!(!rewards[1].sure && !rewards[2].sure, "by chance, and one to choose");
    }

    #[test]
    fn reads_what_a_quest_asks_and_pays() {
        let json = br#"[{"id": 1, "name": "Pebble Patrol", "region": "Fern Isle", "chain": "Pebbles", "repeat": "daily",
            "description": "Go.\nBack.", "requirements": [{"type": "item", "id": 400, "count": 5},
                {"type": "quest", "id": 9, "state": 2}, {"type": "citizenship", "value": 1}],
            "rewards": [{"type": "exp", "value": 30}, {"type": "money", "value": 200}]}]"#;
        let Records::Quests(quests) = Records::parse("quests.json", json).unwrap() else { panic!() };
        let quest = &quests[0];
        assert_eq!(quest.items, [ItemNeed { item_id: 400, count: 5 }]);
        assert_eq!(quest.follows, [9]);
        assert_eq!((quest.reward_exp, quest.reward_mesos), (30, 200));
        assert_eq!((quest.region.as_deref(), quest.repeat.as_deref()), (Some("Fern Isle"), Some("daily")));
    }
}
