//! Item tooltips on the website, which has no Windows OCR: only each tooltip's name is read, from
//! its own line found by the picture (`tooltip::name_lines`), by ocrs, and settled against the
//! item data. A name that doesn't settle on one item gives no card: the card's stats come from
//! the data anyway, so the name is all that has to be right.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::LazyLock;

use regex::Regex;

use image::Rgba;
use leafline_core::build::catalog::Catalog;
use leafline_core::mapdata::loot::name_tooltip;
use leafline_core::mapdata::osms::ItemKind;
use leafline_core::readers::tooltip::{self, ItemTooltip, NameLines, names};
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
        found
            .into_iter()
            .map(|drawn| {
                let lines = drawn.lines.iter().map(|line| self.read_line(recogniser, frame, *line, drawn.unit, drawn.amber)).collect();
                ReadName { drawn, lines }
            })
            .collect()
    }

    /// The tooltips in `frame` whose names settle on an item in `catalog`, left to right (as the
    /// program orders them: a hovered item's tooltip, then the worn one it is compared with).
    pub fn read(&mut self, recogniser: &Recogniser, catalog: &Catalog, frame: &Frame) -> Vec<ItemTooltip> {
        let found = tooltip::name_lines(frame);
        self.settled(recogniser, catalog, frame, found)
    }

    /// [`TooltipReader::read`] for names already found.
    pub fn settled(&mut self, recogniser: &Recogniser, catalog: &Catalog, frame: &Frame, found: Vec<NameLines>) -> Vec<ItemTooltip> {
        let mut items: Vec<ItemTooltip> =
            self.read_found(recogniser, frame, found).iter().filter_map(|name| settle(catalog, name)).collect();
        items.sort_by(|a, b| a.panel.x.total_cmp(&b.panel.x));
        items
    }

    fn read_line(&mut self, recogniser: &Recogniser, frame: &Frame, line: Rect, unit: f32, amber: bool) -> String {
        let pad = PAD * unit;
        let area = Rect::from_edges(line.x - pad, line.y - pad, line.right() + pad, line.bottom() + pad);
        let Some(crop) = ocr::crop(frame, area) else { return String::new() };
        let ink = |px: &Rgba<u8>| if amber { names::is_amber(px) } else { names::is_white(px) };
        let keyed = Frame::from_fn(crop.width(), crop.height(), |x, y| {
            if ink(crop.get_pixel(x, y)) { Rgba([0, 0, 0, 255]) } else { Rgba([255, 255, 255, 255]) }
        });
        let key = {
            let mut hasher = DefaultHasher::new();
            keyed.dimensions().hash(&mut hasher);
            keyed.as_raw().hash(&mut hasher);
            hasher.finish()
        };
        if let Some(text) = self.read.get(&key) {
            return text.clone();
        }
        let (w, h) = keyed.dimensions();
        let p = pad as u32;
        let bounds = InkLine { top: p.min(h), bottom: h.saturating_sub(p), left: p.min(w), right: w.saturating_sub(p) };
        let text = recogniser.read_lines(&keyed, &[bounds]).pop().unwrap_or_default();
        if self.read.len() >= KEEP {
            self.read.clear();
        }
        self.read.insert(key, text.clone());
        text
    }
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

    #[test]
    fn upgrades_come_off_the_end() {
        assert_eq!(upgrades_tail("Green Robin Hat 1+11"), ("Green Robin Hat".into(), Some("11".into())));
        assert_eq!(upgrades_tail("Green Robin Hat (+2)"), ("Green Robin Hat".into(), Some("2".into())));
        assert_eq!(upgrades_tail("Eagle Crow"), ("Eagle Crow".into(), None));
        assert_eq!(digit_readings("11"), vec![11, 1]);
        assert_eq!(digit_readings("3"), vec![3]);
    }
}
