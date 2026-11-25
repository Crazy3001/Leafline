//! The minimap title on the website, which has no Windows OCR: its words by ocrs (a Rust OCR
//! engine whose recognition model the page downloads once), its trailing roman numeral by
//! counting the thin bars at the end of the line, and the result settled against the known maps
//! the way the program settles its own readings. Only a known map counts, so a misread reads as
//! no map rather than a wrong one.
//!
//! Measured on 1,043 saved frames with a title (the program's and a browser's): 1,041 settle on
//! the same map as the program's reading, none on another; the two misses are `Line 1 <Area 1>`.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use image::Rgba;
use leafline_core::mapdata::atlas::Atlas;
use leafline_core::readers::map::{self, MapReading};
use leafline_core::vision::{Frame, preprocess};

use crate::ocr::{self, Recogniser};

/// The share of a line's height below which a full-height glyph is a numeral's bar (an I).
const BAR_WIDTH: f32 = 0.35;

#[derive(Default)]
pub struct TitleReader {
    /// The last title's picture (as ink) and what it read as, so an unchanged title isn't read
    /// again.
    last: Option<(u64, Option<RawTitle>)>,
}

/// A title as read, before it is settled against the known maps.
#[derive(Debug, Clone, PartialEq)]
pub struct RawTitle {
    pub street: Option<String>,
    pub name: String,
    pub collapsed: bool,
}

impl TitleReader {
    /// The title in `frame`, as read; `None` without a minimap or when nothing could be read.
    pub fn read(&mut self, recogniser: &Recogniser, frame: &Frame) -> Option<RawTitle> {
        let (area, collapsed) = map::title_area(frame)?;
        let crop = ocr::crop(frame, area)?;
        let is_ink = |p: &Rgba<u8>| {
            let l = preprocess::luminance(p);
            if collapsed { l <= 90 } else { l >= 200 }
        };
        let key = {
            let mut hasher = DefaultHasher::new();
            (crop.width(), crop.height(), collapsed).hash(&mut hasher);
            for p in crop.pixels() {
                is_ink(p).hash(&mut hasher);
            }
            hasher.finish()
        };
        if let Some((last, title)) = &self.last
            && *last == key
        {
            return title.clone();
        }
        let title = recognise(recogniser, frame, &crop, collapsed, is_ink);
        self.last = Some((key, title.clone()));
        title
    }
}

fn recognise(
    recogniser: &Recogniser,
    frame: &Frame,
    crop: &Frame,
    collapsed: bool,
    is_ink: impl Fn(&Rgba<u8>) -> bool,
) -> Option<RawTitle> {
    let lines = ocr::ink_lines(crop, is_ink);
    let texts: Vec<String> =
        recogniser.read_lines(crop, &lines).into_iter().filter(|t| !t.is_empty()).collect();
    let (street, name) = if collapsed {
        let text = texts.join(" ");
        match text.split_once(':') {
            Some((street, name)) => (Some(street.trim().to_string()), name.trim().to_string()),
            None => (None, text.trim().to_string()),
        }
    } else {
        match texts.as_slice() {
            [] => return None,
            [name] => (None, name.clone()),
            [street, rest @ ..] => (Some(street.clone()), rest.join(" ")),
        }
    };
    let name = with_numeral(&name, frame);
    (name.len() >= 2).then_some(RawTitle { street, name, collapsed })
}

/// `name` with its trailing roman numeral as counted from the picture: after the title's last
/// word gap, a run of thin, full-height bars is that many I's. OCR reads them poorly (`Rocky Road
/// III` as `Rocky Roarl 1`), and they are what tells a map from its neighbours.
fn with_numeral(name: &str, frame: &Frame) -> String {
    let base = name.trim_end_matches(|c: char| "Il1|!?[]{}".contains(c) || c.is_whitespace());
    let base = base.strip_suffix(" I").unwrap_or(base).trim_end().to_string();
    let Some(title) = map::title_glyphs(frame) else { return base };
    let Some(line) = title.lines.last() else { return base };
    let height = line.iter().map(|(_, m)| m.height()).max().unwrap_or(0) as f32;
    let start = line.iter().rposition(|(gap, _)| *gap as f32 >= 0.25 * height).unwrap_or(0);
    let word = &line[start..];
    let bars = !word.is_empty()
        && word.len() <= 4
        && word.iter().all(|(_, m)| {
            let inked = (0..m.height()).filter(|&y| (0..m.width()).any(|x| m.get(x, y))).count();
            (m.width() as f32) <= BAR_WIDTH * height && inked as f32 >= 0.6 * m.height() as f32
        });
    if bars { format!("{base} {}", "I".repeat(word.len())) } else { base }
}

/// A title settled against the known maps, as the program settles its own: the map's own name
/// and street, whether it is a town, and what each of its monsters gives.
pub struct Settled {
    pub reading: MapReading,
    pub town: bool,
    pub kill_exp: Vec<u32>,
}

pub fn settle(atlas: &Atlas, title: &RawTitle, rect: leafline_core::vision::Rect) -> Option<Settled> {
    let known = atlas.settled(&title.name, title.street.as_deref())?;
    let town = atlas.is_town(known.id);
    let kill_exp = atlas.card(known.id).map(|c| c.mobs.iter().map(|m| m.exp).collect()).unwrap_or_default();
    Some(Settled {
        reading: MapReading { street: known.street, name: known.name, collapsed: title.collapsed, rect, twin: None },
        town,
        kill_exp,
    })
}
