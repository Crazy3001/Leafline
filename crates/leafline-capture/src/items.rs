//! Item tooltips on the website, which has no Windows OCR. Each tooltip's name is read from its
//! own line found by the picture (`tooltip::name_lines`), by ocrs, and settled against the item
//! data: a name that doesn't settle on one item gives no card. An equipment tooltip's lines under
//! the separator (`tooltip::stat_lines`) are read after, each in up to three ways (see [`Pass`]),
//! and a stat stands only when two reads agree and it fits the item: `(base +bonus)` adding up
//! with the data's base, or with no split, the data's value. Any other stat is left to the data,
//! which the card shows as the item's default. ocrs is slow in a browser, so a line is read again
//! only when it matters: one that isn't a stat is read once, and a third read only settles two
//! that disagree.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::LazyLock;

use regex::Regex;

use image::Rgba;
use leafline_core::build::catalog::Catalog;
use leafline_core::mapdata::loot::name_tooltip;
use leafline_core::mapdata::osms::{ItemKind, ItemRecord};
use leafline_core::readers::text::best_match;
use leafline_core::readers::tooltip::{self, ItemTooltip, NameLines, Stat, names, parse_below};
use leafline_core::vision::{Frame, Rect};

use crate::ocr::{self, InkLine, Recogniser};

/// Readings kept before the oldest are forgotten.
const KEEP: usize = 256;
/// Room around a line's ink, in game pixels.
const PAD: f32 = 3.0;

/// One tooltip's name as read, before it is settled.
#[derive(Debug, Clone)]
pub struct ReadName {
    pub drawn: NameLines,
    /// Each line's text.
    pub lines: Vec<String>,
}

/// What one line under the separator says, as one read of it took it.
#[derive(Debug, Clone, PartialEq)]
enum LineRead {
    /// A stat, under the card's label.
    Stat(Stat),
    /// `Remaining Enhancements: N`.
    Left(u32),
    /// Anything else: `Type:`, `Attack Speed:`, or what couldn't be made out.
    Other,
}

/// How a line's picture is handed to ocrs.
#[derive(Clone, Copy, Hash)]
enum Pass {
    /// The name's ink (amber or white) as black on white.
    Name { amber: bool },
    /// A stat line's ink as black on white.
    Keyed,
    /// A stat line as drawn.
    Drawn,
    /// A stat line keyed and enlarged twice, without smoothing.
    Big,
}

#[derive(Default)]
pub struct TooltipReader {
    read: HashMap<u64, String>,
}

impl TooltipReader {
    /// The names of the tooltips in `frame`, as read.
    pub fn read_names(&mut self, recogniser: &Recogniser, frame: &Frame) -> Vec<ReadName> {
        self.read_found(recogniser, frame, tooltip::name_lines(frame))
    }

    /// The names `found` in `frame`, as read.
    pub fn read_found(&mut self, recogniser: &Recogniser, frame: &Frame, found: Vec<NameLines>) -> Vec<ReadName> {
        let wanted: Vec<(Rect, f32, Pass)> = found
            .iter()
            .flat_map(|drawn| drawn.lines.iter().map(|line| (*line, drawn.unit, Pass::Name { amber: drawn.amber })))
            .collect();
        let mut texts = self.read_many(recogniser, frame, &wanted).into_iter();
        found
            .into_iter()
            .map(|drawn| {
                let lines = texts.by_ref().take(drawn.lines.len()).collect();
                ReadName { drawn, lines }
            })
            .collect()
    }

    /// The tooltips in `frame` whose names settle on an item in `catalog`, with their stats, left
    /// to right (as the program orders them: a hovered item's tooltip, then the worn one it is
    /// compared with).
    pub fn read(&mut self, recogniser: &Recogniser, catalog: &Catalog, frame: &Frame) -> Vec<ItemTooltip> {
        let found = tooltip::name_lines(frame);
        let mut named = self.named(recogniser, catalog, frame, found);
        self.fill_stats(recogniser, catalog, frame, &mut named);
        named.into_iter().map(|(item, _)| item).collect()
    }

    /// The tooltips `found` whose names settle on an item in `catalog`, left to right, each with
    /// where its name is drawn; their stats aren't read yet (see [`TooltipReader::fill_stats`]).
    pub fn named(
        &mut self,
        recogniser: &Recogniser,
        catalog: &Catalog,
        frame: &Frame,
        found: Vec<NameLines>,
    ) -> Vec<(ItemTooltip, NameLines)> {
        let mut items: Vec<(ItemTooltip, NameLines)> = self
            .read_found(recogniser, frame, found)
            .into_iter()
            .filter_map(|name| Some((settle(catalog, &name)?, name.drawn)))
            .collect();
        items.sort_by(|a, b| a.0.panel.x.total_cmp(&b.0.panel.x));
        items
    }

    /// Reads the stats and the upgrade slots left of each equipment tooltip in `items` (each with
    /// where its name is drawn in `frame`): only what two reads agree on and what fits the item.
    /// Every tooltip's lines are read together, a pass at a time.
    pub fn fill_stats(&mut self, recogniser: &Recogniser, catalog: &Catalog, frame: &Frame, items: &mut [(ItemTooltip, NameLines)]) {
        // Each equipment tooltip's record, and its lines.
        let mut lines: Vec<(usize, Rect, f32)> = Vec::new();
        let mut records: Vec<Option<&ItemRecord>> = Vec::new();
        for (i, (item, drawn)) in items.iter().enumerate() {
            let record = (!item.plain)
                .then(|| catalog.items().iter().find(|r| r.name == item.name && r.kind == ItemKind::Equip))
                .flatten();
            if record.is_some() {
                lines.extend(tooltip::stat_lines(frame, drawn).into_iter().map(|line| (i, line, drawn.unit)));
            }
            records.push(record);
        }
        let reads = agree_all(lines.len(), |pass, which| {
            let wanted: Vec<(Rect, f32, Pass)> = which.iter().map(|&k| (lines[k].1, lines[k].2, pass)).collect();
            self.read_many(recogniser, frame, &wanted)
        });
        for (i, (item, _)) in items.iter_mut().enumerate() {
            let Some(record) = records[i] else { continue };
            let own: Vec<LineRead> = lines.iter().zip(&reads).filter(|((k, _, _), _)| *k == i).map(|(_, read)| read.clone()).collect();
            (item.stats, item.remaining_enhancements) = fitting(record, &own);
        }
    }

    /// The text of each of `lines` (a line, its tooltip's scale, and how to hand it over), read
    /// together in one picture: ocrs is far quicker at many lines at once than one at a time.
    fn read_many(&mut self, recogniser: &Recogniser, frame: &Frame, lines: &[(Rect, f32, Pass)]) -> Vec<String> {
        let prepared: Vec<Option<(u64, Frame, u32)>> = lines.iter().map(|&(line, unit, pass)| prepare(frame, line, unit, pass)).collect();
        let fresh: Vec<usize> =
            (0..lines.len()).filter(|&i| prepared[i].as_ref().is_some_and(|(key, _, _)| !self.read.contains_key(key))).collect();
        if !fresh.is_empty() {
            // One picture, the lines stacked with room between them, on white.
            const GAP: u32 = 12;
            let width = fresh.iter().filter_map(|&i| prepared[i].as_ref()).map(|(_, p, _)| p.width()).max().unwrap_or(1);
            let height: u32 = fresh.iter().filter_map(|&i| prepared[i].as_ref()).map(|(_, p, _)| p.height() + GAP).sum();
            let mut stack = Frame::from_pixel(width, height.max(1), Rgba([255, 255, 255, 255]));
            let mut bounds = Vec::new();
            let mut y = 0;
            for &i in &fresh {
                let Some((_, picture, pad)) = &prepared[i] else { continue };
                image::imageops::replace(&mut stack, picture, 0, i64::from(y));
                let (w, h) = picture.dimensions();
                bounds.push(InkLine { top: y + pad.min(&h), bottom: y + h.saturating_sub(*pad), left: *pad.min(&w), right: w.saturating_sub(*pad) });
                y += h + GAP;
            }
            let texts = recogniser.read_lines(&stack, &bounds);
            if self.read.len() + texts.len() >= KEEP {
                self.read.clear();
            }
            for (&i, text) in fresh.iter().zip(texts) {
                if let Some((key, _, _)) = &prepared[i] {
                    self.read.insert(*key, text);
                }
            }
        }
        prepared.iter().map(|p| p.as_ref().and_then(|(key, _, _)| self.read.get(key).cloned()).unwrap_or_default()).collect()
    }
}

/// `line` of `frame` as `pass` hands it to ocrs: its key (for remembering what it read), the
/// picture, and the room around the text in it.
fn prepare(frame: &Frame, line: Rect, unit: f32, pass: Pass) -> Option<(u64, Frame, u32)> {
    let pad = PAD * unit;
    let area = Rect::from_edges(line.x - pad, line.y - pad, line.right() + pad, line.bottom() + pad);
    let crop = ocr::crop(frame, area)?;
    let ink = |px: &Rgba<u8>| match pass {
        Pass::Name { amber: true } => names::is_amber(px),
        Pass::Name { amber: false } => names::is_white(px),
        Pass::Keyed | Pass::Drawn | Pass::Big => names::is_stat_text(px),
    };
    let keyed = Frame::from_fn(crop.width(), crop.height(), |x, y| {
        if ink(crop.get_pixel(x, y)) { Rgba([0, 0, 0, 255]) } else { Rgba([255, 255, 255, 255]) }
    });
    let key = {
        let mut hasher = DefaultHasher::new();
        pass.hash(&mut hasher);
        keyed.dimensions().hash(&mut hasher);
        keyed.as_raw().hash(&mut hasher);
        hasher.finish()
    };
    Some(match pass {
        Pass::Drawn => (key, crop, pad as u32),
        Pass::Big => {
            let big = image::imageops::resize(&keyed, keyed.width() * 2, keyed.height() * 2, image::imageops::FilterType::Nearest);
            (key, big, (2.0 * pad) as u32)
        }
        _ => (key, keyed, pad as u32),
    })
}

/// The item `name` is, as a tooltip with only its name (in the data's spelling), its upgrades and
/// where it is: the card fills the rest from the data. A wrapped name is tried whole first.
pub fn settle(catalog: &Catalog, name: &ReadName) -> Option<ItemTooltip> {
    let whole = name.lines.join(" ");
    let first = name.lines.first().cloned().unwrap_or_default();
    let tries = if name.lines.len() > 1 { vec![whole, first] } else { vec![first] };
    tries.into_iter().find_map(|text| {
        let (rest, plus) = upgrades_tail(&text);
        let item = name_tooltip(catalog, &rest).or_else(|| {
            // Clothes end with `(M)` or `(F)`, which ocrs reads as `1M`, `IF1`, `Mp` or `1111`:
            // a last word like that is dropped, if the item is one worn by that gender.
            let (before, tag) = rest.trim_end().rsplit_once(' ')?;
            let gender = size_tag(tag)?;
            let item = name_tooltip(catalog, before)?;
            let fits = catalog.items().iter().any(|i| {
                i.name == item.name && i.gender.as_deref().is_some_and(|g| gender.is_none_or(|want| g.starts_with(want)))
            });
            fits.then_some(item)
        })?;
        // `(+1)` comes back `1+11`: the count is whichever reading the item's slots allow, and
        // only an amber name has one.
        let upgrades = plus.filter(|_| name.drawn.amber).and_then(|digits| {
            let slots = item.upgrade_slots?;
            let mut fits = digit_readings(&digits).into_iter().filter(|&n| n >= 1 && n <= slots);
            let n = fits.next()?;
            fits.next().is_none().then_some(n)
        });
        let unit = name.drawn.unit;
        let b = name.drawn.bullet;
        let panel = Rect::new(b.x - 4.0 * unit, b.y - 4.0 * unit, 240.0 * unit, 40.0 * unit);
        Some(ItemTooltip {
            name: item.name.clone(),
            upgrades,
            upgraded: name.drawn.amber,
            plain: item.kind != ItemKind::Equip,
            panel,
            ..ItemTooltip::default()
        })
    })
}

/// What each of `count` lines under the separator says, by reads that agree: all keyed first;
/// as drawn, those whose first read is worth having; enlarged, only those two reads disagree on.
/// `read` reads the lines it is given (by index) in one go.
fn agree_all(count: usize, mut read: impl FnMut(Pass, &[usize]) -> Vec<String>) -> Vec<LineRead> {
    let all: Vec<usize> = (0..count).collect();
    let first: Vec<LineRead> = read(Pass::Keyed, &all).iter().map(|t| one(t)).collect();
    let mut agreed = vec![LineRead::Other; count];
    let worth: Vec<usize> = all.into_iter().filter(|&i| first[i] != LineRead::Other).collect();
    let second: Vec<LineRead> = read(Pass::Drawn, &worth).iter().map(|t| one(t)).collect();
    let mut doubt: Vec<(usize, LineRead)> = Vec::new();
    for (&i, second) in worth.iter().zip(second) {
        if second == first[i] {
            agreed[i] = second;
        } else {
            doubt.push((i, second));
        }
    }
    let which: Vec<usize> = doubt.iter().map(|(i, _)| *i).collect();
    let third: Vec<LineRead> = if which.is_empty() { Vec::new() } else { read(Pass::Big, &which).iter().map(|t| one(t)).collect() };
    for ((i, second), third) in doubt.into_iter().zip(third) {
        if third != LineRead::Other && (third == first[i] || third == second) {
            agreed[i] = third;
        }
    }
    agreed
}

/// [`agree_all`] for one line, `read` reading it each way.
#[cfg(test)]
fn agreed(mut read: impl FnMut(Pass) -> String) -> LineRead {
    agree_all(1, |pass, which| which.iter().map(|_| read(pass)).collect()).remove(0)
}

/// One read of a line.
fn one(text: &str) -> LineRead {
    let read = parse_below(&[text.to_string()]);
    if let Some(stat) = read.stats.into_iter().next() {
        return card_label(&stat.label).map_or(LineRead::Other, |label| LineRead::Stat(Stat { label: label.to_string(), ..stat }));
    }
    read.remaining_enhancements.map_or(LineRead::Other, LineRead::Left)
}

/// The stats and upgrade slots left among `lines` that fit `item`: a `(base +bonus)` that adds up
/// from the data's base, or with no split, the data's own value; no more slots than it has.
fn fitting(item: &ItemRecord, lines: &[LineRead]) -> (Vec<Stat>, Option<u32>) {
    let mut stats: Vec<Stat> = Vec::new();
    let mut left = None;
    for line in lines {
        match line {
            LineRead::Stat(stat) => {
                let own = data_value(item, &stat.label);
                let fits = match (stat.base, stat.bonus) {
                    // The tooltip shows the item's own value and what was added to it, or taken.
                    (Some(base), Some(bonus)) => base + bonus == stat.value && base == own,
                    (None, None) => stat.value == own && own != 0,
                    _ => false,
                };
                if fits && !stats.iter().any(|s| s.label == stat.label) {
                    stats.push(stat.clone());
                }
            }
            LineRead::Left(n) => left = left.or(Some(*n)),
            LineRead::Other => {}
        }
    }
    (stats, left.filter(|&n| item.upgrade_slots.is_some_and(|slots| n <= slots)))
}

/// Stat labels as tooltips print them, and the data's label for each.
const LABELS: [(&str, &str); 22] = [
    ("Weapon Attack", "Attack"),
    ("Weapon Att.", "Attack"),
    ("Magic Attack", "Magic"),
    ("Magic Att.", "Magic"),
    ("Weapon Def.", "Weapon Def."),
    ("Magic Def.", "Magic Def."),
    ("STR", "STR"),
    ("DEX", "DEX"),
    ("INT", "INT"),
    ("LUK", "LUK"),
    ("MaxHP", "MaxHP"),
    ("MaxMP", "MaxMP"),
    ("Accuracy", "Accuracy"),
    ("Avoidability", "Evasion"),
    ("Evasion", "Evasion"),
    ("Critical Rate", "Crit. Rate"),
    ("Critical Damage", "Crit. Damage"),
    ("Crit. Rate", "Crit. Rate"),
    ("Crit. Damage", "Crit. Damage"),
    ("Speed", "Speed"),
    ("Jump", "Jump"),
    ("Hands", "Hands"),
];

/// The label the card goes by for a read one: the data's, except attack and magic, which the
/// card names as the tooltip does. `None` for a label that is no stat.
fn card_label(read: &str) -> Option<&'static str> {
    let printed: Vec<&str> = LABELS.iter().map(|(p, _)| *p).collect();
    // Short labels (STR, LUK) must be exact: one letter off is another stat.
    let found = if read.len() <= 4 { printed.iter().copied().find(|p| *p == read)? } else { best_match(read, &printed, 0.8)? };
    let data = LABELS.iter().find(|(p, _)| *p == found).map(|(_, d)| *d)?;
    Some(match data {
        "Attack" => "Weapon Attack",
        "Magic" => "Magic Attack",
        other => other,
    })
}

/// The data's value for a stat by the card's label, or 0 when the item has none.
fn data_value(item: &ItemRecord, label: &str) -> i32 {
    let data = match label {
        "Weapon Attack" => "Attack",
        "Magic Attack" => "Magic",
        other => other,
    };
    item.stats.iter().find(|s| s.label == data).map_or(0, |s| s.value as i32)
}

/// `(+N)` at the end of a read name, as ocrs reads its brackets (`(`, `1`, `I`, `l`, `|`, `[`):
/// the name before it and the digits.
fn upgrades_tail(text: &str) -> (String, Option<String>) {
    match UPGRADES.captures(text) {
        Some(caps) => (text[..caps.get(0).unwrap().start()].trim().to_string(), Some(caps[1].to_string())),
        None => (text.trim().to_string(), None),
    }
}

static UPGRADES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*[(\[{1Il|]?\s*\+\s*([0-9OoIlSB]{1,2})\s*[)\]}1Il|]?\s*$").unwrap());
/// `(M)` or `(F)` as ocrs reads it; the letter when it was read.
static SIZE_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(?:[(\[{1Il|]*([MF])[A-Za-z:)\]}1Il|]{0,2}|[(\[{1Il|]{3,4})$").unwrap());

/// Whether `word` is a misread `(M)` or `(F)`, and which (`male` or `female`'s first letter) if
/// the letter was read.
fn size_tag(word: &str) -> Option<Option<&'static str>> {
    let caps = SIZE_TAG.captures(word)?;
    Some(caps.get(1).map(|m| if m.as_str() == "M" { "m" } else { "f" }))
}

/// What the digits of `(+N)` could be: as read, and without a last `1` that was the bracket.
fn digit_readings(digits: &str) -> Vec<u32> {
    let fixed: String = digits
        .chars()
        .map(|c| match c {
            'O' | 'o' => '0',
            'I' | 'l' => '1',
            'S' => '5',
            'B' => '8',
            c => c,
        })
        .collect();
    let mut readings: Vec<u32> = fixed.parse().ok().into_iter().collect();
    if fixed.len() == 2 && fixed.ends_with('1') {
        readings.extend(fixed[..1].parse::<u32>().ok());
    }
    readings.dedup();
    readings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_misread_size_tag_is_known() {
        for tag in ["1M", "IF1", "Mp", "1Mp", "IM:", "M:", "FM", "1FH", "1Ft", "1111", "(M)"] {
            assert!(size_tag(tag).is_some(), "{tag}");
        }
        assert_eq!(size_tag("IF1"), Some(Some("f")));
        assert_eq!(size_tag("1111"), Some(None));
        for word in ["Hat", "Kilt", "Ore", "Egg", "Top", "Crow"] {
            assert!(size_tag(word).is_none(), "{word}");
        }
    }

    /// An item as the data file has it, with these OSMS stats (`incPAD`...) and 7 slots.
    fn record(stats: &[(&str, i64)]) -> ItemRecord {
        let mut raw: serde_json::Map<String, serde_json::Value> = stats.iter().map(|&(k, v)| (k.to_string(), v.into())).collect();
        raw.insert("tuc".into(), 7.into());
        serde_json::from_value(serde_json::json!({ "id": 1, "name": "Test", "category": "Equipment", "sub_category": "Weapon", "stats": raw }))
            .expect("an item")
    }

    /// `lines` each read the same every way.
    fn stats_of(item: &ItemRecord, lines: &[&str]) -> (Vec<Stat>, Option<u32>) {
        let reads: Vec<LineRead> = lines.iter().map(|l| agreed(|_| l.to_string())).collect();
        fitting(item, &reads)
    }

    #[test]
    fn a_stat_stands_when_it_fits_the_item() {
        let crow = record(&[("incPAD", 52), ("incDEX", 1)]);
        let (got, left) = stats_of(&crow, &["Type: Crossbow", "DEX: +1", "Weapon Attack: +56 (52 +4)", "Remaining Enhancements: 7"]);
        assert_eq!(got, vec![
            Stat { label: "DEX".into(), value: 1, base: None, bonus: None },
            Stat { label: "Weapon Attack".into(), value: 56, base: Some(52), bonus: Some(4) },
        ]);
        assert_eq!(left, Some(7));
        assert!(stats_of(&crow, &["Weapon Attack: +55 (52 +4)"]).0.is_empty(), "doesn't add up");
        assert!(stats_of(&crow, &["Weapon Attack: +56 (50 +6)"]).0.is_empty(), "not the item's base");
        assert!(stats_of(&crow, &["DEX: +3"]).0.is_empty(), "no split, not the item's value");
        assert_eq!(stats_of(&crow, &["Remaining Enhancements: 9"]).1, None, "more slots than the item has");
    }

    #[test]
    fn a_loss_and_a_scroll_on_a_stat_the_item_lacks_are_kept() {
        let skirt = record(&[("incMDD", 16)]);
        let (got, _) = stats_of(&skirt, &["Magic Def.: +15 (16 -1)", "Accuracy: +2 (0 +2)"]);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].bonus, Some(-1));
    }

    /// A line read as `keyed`, `drawn` and `big`, and how many reads it took.
    fn reads(keyed: &str, drawn: &str, big: &str) -> (LineRead, usize) {
        let mut count = 0;
        let line = agreed(|pass| {
            count += 1;
            match pass {
                Pass::Keyed => keyed,
                Pass::Drawn => drawn,
                _ => big,
            }
            .to_string()
        });
        (line, count)
    }

    #[test]
    fn two_reads_must_agree() {
        let stat = |value, base, bonus| LineRead::Stat(Stat { label: "Weapon Attack".into(), value, base, bonus });
        assert_eq!(reads("Type: Crossbow", "", ""), (LineRead::Other, 1), "not a stat: read once");
        assert_eq!(reads("Weapon Attack: +56 (52 +4)", "Weapon Attack: +56(52+4)", ""), (stat(56, Some(52), Some(4)), 2));
        assert_eq!(reads("Weapon Attack: +56 (52 +4)", "Weapon Attack: +56 (52 19)", "Weapon Attack: +56 (52 +4)").0, stat(56, Some(52), Some(4)));
        assert_eq!(reads("Weapon Attack: +56 (52 +4)", "Weapon Attack: +58 (52 +6)", "Weapon Attack: +57").0, LineRead::Other, "all three differ");
        assert_eq!(reads("Weapon Det.: +56", "Weapon Attack: +56", "").0, LineRead::Other, "Weapon Def. is another stat");
    }

    #[test]
    fn labels_go_by_the_datas_names() {
        assert_eq!(card_label("Weapon Det."), Some("Weapon Def."));
        assert_eq!(card_label("Critical Damage"), Some("Crit. Damage"));
        assert_eq!(card_label("Avoidability"), Some("Evasion"));
        assert_eq!(card_label("Weapon Attack"), Some("Weapon Attack"));
        assert_eq!(card_label("LWK"), None);
        assert_eq!(card_label("Knockback Chance"), None);
    }

    #[test]
    fn upgrades_come_off_the_end() {
        assert_eq!(upgrades_tail("Green Robin Hat 1+11"), ("Green Robin Hat".into(), Some("11".into())));
        assert_eq!(upgrades_tail("Green Robin Hat (+2)"), ("Green Robin Hat".into(), Some("2".into())));
        assert_eq!(upgrades_tail("Eagle Crow"), ("Eagle Crow".into(), None));
        assert_eq!(digit_readings("11"), vec![11, 1]);
        assert_eq!(digit_readings("3"), vec![3]);
    }
}
