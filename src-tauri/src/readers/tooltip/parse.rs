//! Turns the text lines of an item tooltip into a structured item.
//!
//! Layout (top to bottom): name with an optional upgrade count `(+N)`, the icon beside six
//! `REQ` lines, the job flags, a separator, `Type:`, stat lines such as
//! `Weapon Def.: +26 (24 +2)`, other `Label: value` lines, and `Remaining Enhancements: N`.

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::readers::text::{best_match, digitize, similarity, starts_like, trim_noise};
use crate::vision::{Line, Rect};

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemTooltip {
    pub name: String,
    /// Successful upgrades, from the `(+N)` suffix on the name.
    pub upgrades: Option<u32>,
    /// The item has been upgraded, going by its amber name, even if the count couldn't be read.
    pub upgraded: bool,
    pub item_type: Option<String>,
    pub requirements: Requirements,
    pub jobs: Vec<JobFlag>,
    pub stats: Vec<Stat>,
    /// Non-numeric `Label: value` lines (for example attack speed or category).
    pub properties: Vec<Property>,
    pub remaining_enhancements: Option<u32>,
    /// Where the tooltip panel was found, in frame pixels.
    pub panel: Rect,
    /// An item with no stats: only the name and `item_type`, which holds any tags, are read.
    pub plain: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Requirements {
    pub level: Option<u32>,
    pub str: Option<u32>,
    pub dex: Option<u32>,
    pub int: Option<u32>,
    pub luk: Option<u32>,
    pub fam: Option<u32>,
}

impl Requirements {
    /// Keeps these values and fills gaps from `other`.
    pub fn or(self, other: Requirements) -> Requirements {
        Requirements {
            level: self.level.or(other.level),
            str: self.str.or(other.str),
            dex: self.dex.or(other.dex),
            int: self.int.or(other.int),
            luk: self.luk.or(other.luk),
            fam: self.fam.or(other.fam),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JobFlag {
    pub job: &'static str,
    /// Whether the flag is highlighted, meaning the job can use the item.
    pub eligible: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Stat {
    pub label: String,
    pub value: i32,
    /// From `(base +bonus)` when the tooltip shows it.
    pub base: Option<i32>,
    pub bonus: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Property {
    pub label: String,
    pub value: String,
}

pub const JOBS: [&str; 5] = ["BEGINNER", "WARRIOR", "MAGICIAN", "BOWMAN", "THIEF"];

/// Stat labels as old GMS printed them. Classic may differ; OCR'd labels are snapped to the
/// closest entry only when they are clearly similar, otherwise kept as read.
const STAT_LABELS: &[&str] = &[
    "STR",
    "DEX",
    "INT",
    "LUK",
    "MaxHP",
    "MaxMP",
    "Weapon Attack",
    "Magic Attack",
    "Weapon Att.",
    "Magic Att.",
    "Weapon Def.",
    "Magic Def.",
    "Accuracy",
    "Avoidability",
    "Speed",
    "Jump",
    "Hands",
];

const REQ_KEYS: [&str; 6] = ["LEV", "STR", "DEX", "INT", "LUK", "FAM"];

/// `(+1)`, also with its opening bracket read as `4` or `1`.
static UPGRADES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:\(|\s[41lI])\s*\+\s*([0-9OoIlSB]{1,2})\s*\)?\s*$").unwrap());
/// `(+1)` anywhere in a line, for the times OCR reads a wrapped name out of order.
static ANY_UPGRADES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\(\s*\+\s*(\d{1,2})").unwrap());
/// The `(M)` of `White Undershirt (M)`, with OCR's usual brackets, and any `(+N)` after it.
static SIZE_TAG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*[({\[]\s*([MFU])\s*[)}\]|lI1]?(\s*\(\s*\+.*)?$").unwrap());
/// `REQ LEV : 25`. Keying often loses the grey `REQ`, so it is optional.
static REQUIREMENT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:(\S{2,4})\s*)?([A-Za-z]{2,4})\W*([0-9OoIlSB]{1,3})?\s*$").unwrap());
static STAT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(.+?)\s*[:;.•·]+\s*([+-])\s*([0-9OoIlSB]{1,4})(?:\s*[\(\[]\s*([0-9OoIlSB]{1,4})\s*([+-])\s*([0-9OoIlSB]{1,4}))?").unwrap()
});
static PROPERTY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^([A-Za-z][A-Za-z .'-]+?)\s*:\s*(.+)$").unwrap());
/// OCR often splits a capital H into two strokes: `Maxi-IP`.
static SPLIT_H: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[Il1|i]-[Il1|i]").unwrap());
static TRAILING_NUMBER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"([0-9OoIlSB]{1,3})\s*$").unwrap());

impl ItemTooltip {
    /// Combines two readings of the same tooltip, keeping this one's values and filling gaps
    /// from `other`.
    pub fn merge(mut self, other: ItemTooltip) -> ItemTooltip {
        if other.name.len() > self.name.len() {
            self.name = other.name;
        }
        self.upgrades = self.upgrades.or(other.upgrades);
        self.item_type = self.item_type.or(other.item_type);
        self.remaining_enhancements = self.remaining_enhancements.or(other.remaining_enhancements);

        self.requirements = self.requirements.or(other.requirements);

        for stat in other.stats {
            if !self.stats.iter().any(|s| s.label == stat.label) {
                self.stats.push(stat);
            }
        }
        for property in other.properties {
            if !self.properties.iter().any(|p| p.label == property.label) {
                self.properties.push(property);
            }
        }
        self
    }
}

/// Parses a tooltip from its lines, ordered top to bottom. `separator_y` is the rule under the
/// job flags: the name and `REQ` block sit above it, and type, stats and other lines below.
/// Keeping the two apart stops misread `REQ` rows from passing as properties, and stat lines
/// such as `DEX: +2` from passing as requirements.
pub fn parse(lines: &[Line], separator_y: f32) -> ItemTooltip {
    let mut item = ItemTooltip::default();
    let mut first_req_y = None;
    let mut unclaimed = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let text = trim_noise(&line.text);
        if text.is_empty() {
            continue;
        }
        if line.rect.center_y() < separator_y {
            match parse_requirement(text, lines, i) {
                Some((key, value)) => {
                    first_req_y.get_or_insert(line.rect.y);
                    set_requirement(&mut item.requirements, key, value);
                }
                None => unclaimed.push(line),
            }
        } else if is_type_line(text) {
            let value = text.split_once(':').map(|(_, v)| v.trim().to_string()).filter(|v| !v.is_empty());
            item.item_type = item.item_type.or(value);
        } else if starts_like(text, "Remaining") {
            item.remaining_enhancements = TRAILING_NUMBER.captures(text).and_then(|c| digitize(&c[1]).parse().ok());
        } else if let Some(stat) = parse_stat(text) {
            item.stats.push(stat);
        } else if let Some(caps) = PROPERTY.captures(text).filter(|c| is_label(&c[1])) {
            item.properties.push(Property { label: caps[1].trim().to_string(), value: caps[2].trim().to_string() });
        }
    }

    // The name is the tallest remaining line above the requirement block.
    let name_line = unclaimed
        .into_iter()
        .filter(|l| first_req_y.is_none_or(|y| l.rect.bottom() <= y + 1.0))
        .max_by(|a, b| a.rect.h.total_cmp(&b.rect.h));
    if let Some(line) = name_line {
        (item.name, item.upgrades) = split_name(trim_noise(&line.text));
    }
    item
}

/// The count in a `(+N)` anywhere in `text`.
pub fn upgrades_in(text: &str) -> Option<u32> {
    ANY_UPGRADES.captures(text)?[1].parse().ok()
}

/// Splits `Green Robin Hat (+1)` into the name and the upgrade count, and mends the small tag
/// clothes end with, which OCR can read as `{MI`. Names start with a capital, whatever OCR says.
pub fn split_name(text: &str) -> (String, Option<u32>) {
    let text = SIZE_TAG.replace(text, " ($1)$2");
    let (name, upgrades) = match UPGRADES.captures(&text) {
        Some(caps) => (text[..caps.get(0).unwrap().start()].trim(), digitize(&caps[1]).parse().ok()),
        None => (text.trim(), None),
    };
    let mut letters = name.chars();
    let name = letters.next().map_or(String::new(), |first| first.to_uppercase().chain(letters).collect());
    (name, upgrades)
}

/// Matches `REQ LEV : 25`. When OCR split the value onto its own line, the nearest line to
/// the right on the same row supplies it.
fn parse_requirement(text: &str, lines: &[Line], index: usize) -> Option<(&'static str, Option<u32>)> {
    let caps = REQUIREMENT.captures(text)?;
    if caps.get(1).is_some_and(|prefix| similarity(prefix.as_str(), "REQ") < 0.6) {
        return None;
    }
    let key = best_match(&caps[2], &REQ_KEYS, 0.6)?;
    let value = caps.get(3).map(|m| m.as_str().to_string()).or_else(|| {
        let line = &lines[index];
        lines
            .iter()
            .filter(|other| {
                other.rect.x > line.rect.right() && other.rect.vertical_overlap(&line.rect) > 0.5 * line.rect.h
            })
            .min_by(|a, b| a.rect.x.total_cmp(&b.rect.x))
            .and_then(|other| TRAILING_NUMBER.captures(trim_noise(&other.text)).map(|c| c[1].to_string()))
    });
    Some((key, value.and_then(|v| digitize(&v).parse().ok())))
}

fn set_requirement(req: &mut Requirements, key: &str, value: Option<u32>) {
    let slot = match key {
        "LEV" => &mut req.level,
        "STR" => &mut req.str,
        "DEX" => &mut req.dex,
        "INT" => &mut req.int,
        "LUK" => &mut req.luk,
        _ => &mut req.fam,
    };
    // A row read twice, once without its value, keeps the value.
    if value.is_some() {
        *slot = value;
    }
}

/// `Type: Hat`, also behind a stray mark OCR made of the bullet (`II Type: Crossbow`).
fn is_type_line(text: &str) -> bool {
    let mut words = text.split_whitespace();
    match words.next() {
        Some(first) if first.len() <= 2 && !starts_like(first, "Type") => {
            words.next().is_some_and(|w| starts_like(w, "Type"))
        }
        Some(_) => starts_like(text, "Type"),
        None => false,
    }
}

/// A property label is words, not the stray marks OCR makes of scenery.
fn is_label(label: &str) -> bool {
    label.chars().filter(char::is_ascii_alphabetic).count() >= 3
}

fn parse_stat(text: &str) -> Option<Stat> {
    let caps = STAT.captures(text)?;
    let raw_label = SPLIT_H.replace_all(caps[1].trim(), "H");
    let label = best_match(&raw_label, STAT_LABELS, 0.6).map_or_else(|| raw_label.to_string(), str::to_string);
    let sign = if &caps[2] == "-" { -1 } else { 1 };
    let value = sign * digitize(&caps[3]).parse::<i32>().ok()?;
    let base = caps.get(4).and_then(|m| digitize(m.as_str()).parse().ok());
    let bonus = caps
        .get(6)
        .and_then(|m| digitize(m.as_str()).parse::<i32>().ok())
        .map(|b| if &caps[5] == "-" { -b } else { b });
    Some(Stat { label, value, base, bonus })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vision::{Word, group_lines};

    /// Builds lines from `(text, x, y, height)` the way the reader would after OCR.
    fn lines(rows: &[(&str, f32, f32, f32)]) -> Vec<Line> {
        let words: Vec<Word> = rows
            .iter()
            .flat_map(|&(text, x, y, h)| {
                let mut cursor = x;
                text.split(' ')
                    .map(|w| {
                        let rect = Rect::new(cursor, y, w.len() as f32 * h * 0.6, h);
                        cursor = rect.right() + h * 0.4;
                        Word { text: w.to_string(), rect }
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        group_lines(&words)
    }

    #[test]
    fn brown_bamboo_hat() {
        let item = parse(
            &lines(&[
                ("Brown Bamboo Hat", 47.0, 326.0, 18.0),
                ("REQ LEV : 25", 134.0, 350.0, 10.0),
                ("REQ STR : 0", 134.0, 364.0, 10.0),
                ("REQ DEX : 10", 134.0, 377.0, 10.0),
                ("REQ INT : 0", 134.0, 391.0, 10.0),
                ("REQ LUK : 0", 134.0, 405.0, 10.0),
                ("REQ FAM : 0", 134.0, 418.0, 10.0),
                ("BEGINNER WARRIOR MAGICIAN BOWMAN THIEF", 42.0, 440.0, 9.0),
                ("Type: Hat", 45.0, 472.0, 12.0),
                ("· MaxHP: +10", 45.0, 490.0, 12.0),
                ("· Weapon Def.: +26 (24 +2)", 45.0, 508.0, 12.0),
                ("· Accuracy: +1", 45.0, 526.0, 12.0),
                ("Remaining Enhancements: 7", 45.0, 544.0, 12.0),
            ]),
            460.0,
        );
        assert_eq!(item.name, "Brown Bamboo Hat");
        assert_eq!(item.upgrades, None);
        assert_eq!(item.item_type.as_deref(), Some("Hat"));
        assert_eq!(
            item.requirements,
            Requirements { level: Some(25), str: Some(0), dex: Some(10), int: Some(0), luk: Some(0), fam: Some(0) }
        );
        assert_eq!(item.stats.len(), 3);
        assert_eq!(item.stats[1], Stat { label: "Weapon Def.".into(), value: 26, base: Some(24), bonus: Some(2) });
        assert_eq!(item.remaining_enhancements, Some(7));
    }

    #[test]
    fn a_size_tag_comes_back_whole() {
        let split = |text| split_name(text);
        assert_eq!(split("Blue Jean Shorts {MI"), ("Blue Jean Shorts (M)".to_string(), None));
        assert_eq!(split("White Undershirt (M)"), ("White Undershirt (M)".to_string(), None));
        assert_eq!(split("Blue Jean Shorts [F) (+2)"), ("Blue Jean Shorts (F)".to_string(), Some(2)));
        assert_eq!(split("Green Robin Hat (+1)"), ("Green Robin Hat".to_string(), Some(1)));
        assert_eq!(split("Sera's Mirror"), ("Sera's Mirror".to_string(), None));
        assert_eq!(split("wooden Club"), ("Wooden Club".to_string(), None));
        assert_eq!(split("Green Robin Hat 4+1"), ("Green Robin Hat".to_string(), Some(1)));
        assert_eq!(split("Green Robin Hat 1+1"), ("Green Robin Hat".to_string(), Some(1)));
        assert_eq!(split("Brown Hard Leather Top (M) (+1)"), ("Brown Hard Leather Top (M)".to_string(), Some(1)));
    }

    #[test]
    fn an_upgrade_count_is_found_out_of_order() {
        assert_eq!(upgrades_in("Brown Hard Leather (+1) Top"), Some(1));
        assert_eq!(upgrades_in("Brown Hard Leather Top"), None);
    }

    #[test]
    fn upgraded_name_and_noisy_ocr() {
        let item = parse(
            &lines(&[
                ("Green Robin Hat (+1)", 318.0, 326.0, 18.0),
                ("PEQ LEV • 20", 405.0, 349.0, 10.0),
                ("REQ DEX :", 405.0, 377.0, 10.0),
                ("3O", 470.0, 377.0, 10.0),
                ("Type: Hat", 316.0, 472.0, 12.0),
                ("OEX: +2", 317.0, 490.0, 12.0),
                ("Accuracy: +2 (0 +2)", 316.0, 526.0, 12.0),
                ("• «Remaning Enhancements: 6", 300.0, 544.0, 12.0),
            ]),
            460.0,
        );
        assert_eq!(item.name, "Green Robin Hat");
        assert_eq!(item.upgrades, Some(1));
        assert_eq!(item.requirements.level, Some(20));
        assert_eq!(item.requirements.dex, Some(30));
        assert_eq!(item.stats[0], Stat { label: "DEX".into(), value: 2, base: None, bonus: None });
        assert_eq!(item.stats[1].bonus, Some(2));
        assert_eq!(item.remaining_enhancements, Some(6));
    }

    #[test]
    fn properties_are_kept() {
        let item = parse(&lines(&[("Some Sword", 0.0, 0.0, 18.0), ("Attack Speed: Fast", 0.0, 100.0, 12.0)]), 50.0);
        assert_eq!(item.properties, vec![Property { label: "Attack Speed".into(), value: "Fast".into() }]);
    }

    /// Readings like those of a tooltip drawn over an inventory window: garbled `REQ` rows,
    /// a value-only row and a capital H split in two.
    #[test]
    fn junk_rows_and_split_letters() {
        let item = parse(
            &lines(&[
                ("Fernleaf Cap", 47.0, 326.0, 18.0),
                ("LEW • 25", 160.0, 350.0, 10.0),
                ("r-:EEJ. ETA", 134.0, 364.0, 10.0),
                ("OEX", 160.0, 377.0, 10.0),
                ("10", 200.0, 377.0, 10.0),
                ("r-:EEJ. INT", 134.0, 391.0, 10.0),
                ("II Type: Hat", 45.0, 472.0, 12.0),
                ("Maxi-IP: +10", 45.0, 490.0, 12.0),
                ("Remaining Enhancements: 7", 45.0, 544.0, 12.0),
            ]),
            460.0,
        );
        assert_eq!(item.name, "Fernleaf Cap");
        assert_eq!(item.item_type.as_deref(), Some("Hat"));
        assert_eq!(item.requirements.level, Some(25));
        assert_eq!(item.requirements.dex, Some(10));
        assert!(item.properties.is_empty(), "{:?}", item.properties);
        assert_eq!(item.stats, vec![Stat { label: "MaxHP".into(), value: 10, base: None, bonus: None }]);
    }

    #[test]
    fn stat_lines_below_the_rule_are_not_requirements() {
        let item = parse(&lines(&[("DEX: +2", 45.0, 490.0, 12.0)]), 460.0);
        assert_eq!(item.requirements.dex, None);
        assert_eq!(item.stats[0].label, "DEX");
    }
}
