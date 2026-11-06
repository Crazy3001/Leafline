//! Frame → readings. The same pipeline runs on live captures and on saved screenshots.

use std::time::Instant;

use serde::Serialize;

use crate::error::Result;
use crate::readers::character::CharacterReading;
use crate::readers::hud::{HudLayout, HudReading};
use crate::readers::map::MapReading;
use crate::readers::mesos::MesoReading;
use crate::readers::quest::QuestReading;
use crate::readers::tooltip::ItemTooltip;
use crate::readers::ui::{self, CANONICAL_SCALE, Canonical};
use crate::readers::{character, hud, map, mesos, party, quest, shop, tooltip};
use crate::vision::{Frame, Line, Ocr, Rect, Word, group_lines};

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
    pub width: u32,
    pub height: u32,
    pub hud: Option<HudReading>,
    pub hud_layout: Option<HudLayout>,
    pub tooltips: Vec<ItemTooltip>,
    pub quest: Option<QuestReading>,
    pub mesos: Option<MesoReading>,
    /// Meso boxes on screen, read or not. More than one means a storage or trade window is
    /// open, where meso changes are not loot.
    pub meso_boxes: usize,
    /// An NPC shop, storage or trade window is open, going by its buttons.
    pub shop_open: bool,
    /// How many are in the party, the player included; 1 with no party list in view.
    pub party_size: u32,
    pub map: Option<MapReading>,
    /// Every line from the whole-frame OCR pass, for the workbench's debug view.
    pub overview: Vec<Line>,
    pub timings: Vec<Timing>,
    /// Parts that hit an error this time. The rest were still read.
    pub failed: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Timing {
    pub stage: &'static str,
    pub ms: f32,
}

impl Analysis {
    /// Just these items, for an item check with nothing else read yet.
    pub fn of_items(width: u32, height: u32, tooltips: Vec<ItemTooltip>) -> Self {
        Self { width, height, tooltips, party_size: 1, ..Self::default() }
    }
}

/// The result of reading one part, or `None` with the part noted: one part failing must not cost
/// the others.
fn kept<T>(failed: &mut Vec<&'static str>, part: &'static str, result: Result<T>) -> Option<T> {
    result.map_err(|err| eprintln!("leafline: {part}: {err}")).inspect_err(|()| failed.push(part)).ok()
}

/// Reads everything in a saved screenshot.
pub fn analyze(frame: &Frame, ocr: &Ocr) -> Result<Analysis> {
    read(frame, ocr, true)
}

/// Reads a live picture. Item tooltips are left out: [`read_items`] reads those once, when the
/// player asks.
pub fn analyze_live(frame: &Frame, ocr: &Ocr) -> Result<Analysis> {
    read(frame, ocr, false)
}

/// Reads only the item tooltips in `frame`.
pub fn read_items(frame: &Frame, ocr: &Ocr) -> Result<Vec<ItemTooltip>> {
    tooltip::read_all(frame, &overview(frame, ocr)?, ocr)
}

/// What the character's windows show, with the name and level from the HUD.
#[derive(Debug, Clone, Default)]
pub struct CharacterScan {
    pub name: Option<String>,
    pub level: Option<u32>,
    pub windows: CharacterReading,
}

/// Reads the stat and skill windows in `frame`.
pub fn read_character(frame: &Frame, ocr: &Ocr) -> Result<CharacterScan> {
    let located = hud::read(frame, ocr)?;
    let (layout, reading) = located.unzip();
    let mut reading = reading.unwrap_or_default();
    if let Some(layout) = &layout {
        hud::settle_name(&mut reading, layout, &overview(frame, ocr)?);
    }
    let windows = character::read(frame, layout.as_ref(), ocr)?;
    Ok(CharacterScan { name: reading.name, level: reading.level, windows })
}

fn read(frame: &Frame, ocr: &Ocr, items: bool) -> Result<Analysis> {
    let mut failed = Vec::new();
    let mut timings = Vec::new();
    let mut timed = |stage: &'static str, started: Instant| {
        timings.push(Timing { stage, ms: started.elapsed().as_secs_f32() * 1000.0 });
    };

    let started = Instant::now();
    let (hud_layout, mut hud): (Option<HudLayout>, Option<HudReading>) =
        kept(&mut failed, "HUD", hud::read(frame, ocr)).flatten().unzip();
    timed("HUD", started);

    let started = Instant::now();
    let overview = kept(&mut failed, "text", overview(frame, ocr)).unwrap_or_default();
    timed("Overview OCR", started);
    if let (Some(hud), Some(layout)) = (hud.as_mut(), hud_layout.as_ref()) {
        hud::settle_name(hud, layout, &overview);
    }

    let mut tooltips = Vec::new();
    if items {
        let started = Instant::now();
        tooltips = kept(&mut failed, "item tooltip", tooltip::read_all(frame, &overview, ocr)).unwrap_or_default();
        timed("Tooltips", started);
    }

    let started = Instant::now();
    let scale = redrawn_scale(frame, hud_layout.as_ref());
    let quest = kept(&mut failed, "Quest Helper", read_quests(frame, scale, &overview, ocr)).flatten();
    let (mesos, meso_boxes) = kept(&mut failed, "mesos", read_mesos(frame, scale, ocr)).unwrap_or_default();
    timed("Quest & mesos", started);

    let started = Instant::now();
    let shop_open = shop::is_open(frame);
    let party_size = party::size(frame, hud_layout.as_ref());
    timed("Shop & party", started);

    let started = Instant::now();
    let map = kept(&mut failed, "map name", map::read(frame, ocr)).flatten();
    timed("Map", started);

    Ok(Analysis {
        width: frame.width(),
        height: frame.height(),
        hud,
        hud_layout,
        tooltips,
        quest,
        mesos,
        meso_boxes,
        shop_open,
        party_size,
        map,
        overview,
        timings,
        failed,
    })
}

/// The frame redrawn at the size the readers know, when the HUD shows the game drawing its UI at
/// another size (UI Size Default Ratio draws it at window height / 768, 1.875x at 1440p). The
/// Quest Helper's and meso box's pixel fonts only read cleanly at that size.
fn redrawn_scale(frame: &Frame, hud: Option<&HudLayout>) -> Option<f32> {
    let scale = ui::hud_scale(frame, hud?);
    ((scale - CANONICAL_SCALE).abs() > 0.005).then_some(scale)
}

/// Lines found in the window that lie within `area`, moved onto `view`.
fn lines_in_view(lines: &[Line], area: Rect, view: &Canonical) -> Vec<Line> {
    let inside = |rect: &Rect| {
        rect.x >= area.x && rect.y >= area.y && rect.right() <= area.right() && rect.bottom() <= area.bottom()
    };
    lines
        .iter()
        .filter(|line| inside(&line.rect))
        .map(|line| Line {
            text: line.text.clone(),
            rect: view.to_view(line.rect),
            words: line.words.iter().map(|w| Word { text: w.text.clone(), rect: view.to_view(w.rect) }).collect(),
        })
        .collect()
}

fn read_quests(frame: &Frame, scale: Option<f32>, overview: &[Line], ocr: &Ocr) -> Result<Option<QuestReading>> {
    let (Some(scale), Some((header, panel))) = (scale, quest::panel(overview)) else {
        return quest::read(frame, overview, ocr);
    };
    let smoothed = ui::smoothed(frame, panel).unwrap_or(false);
    // The header and the panel under it, with room for the panel's edges.
    let margin = header.rect.h;
    let area =
        Rect::from_edges(panel.x - margin, header.rect.y - margin, panel.right() + margin, panel.bottom() + margin);
    let near_black = if smoothed { ui::NEAR_BLACK_SMOOTHED } else { ui::NEAR_BLACK };
    let view = Canonical::sharp(frame, scale, area, near_black);
    // The counters' digits again, keeping one-pixel strokes that the middle of a design pixel
    // can miss (the bar across an 8). Used only where it agrees on how many digits there are.
    let digits = Canonical::inked(frame, scale, area, near_black);
    Ok(match quest::read_with_digits(&view.frame, &digits.frame, &lines_in_view(overview, area, &view), ocr)? {
        Some(reading) => Some(QuestReading { panel: view.to_window(reading.panel), ..reading }),
        None => quest::read(frame, overview, ocr)?,
    })
}

/// At UI Size Default Ratio the digits are read as drawn, with the ink thresholds for thin
/// smoothed strokes. Redrawn on the design grid they smear across two columns wherever the
/// inventory sits off that grid, so the redraw is only tried when that fails with the box shown.
fn read_mesos(frame: &Frame, scale: Option<f32>, ocr: &Ocr) -> Result<(Option<MesoReading>, usize)> {
    let Some(scale) = scale else { return mesos::read(frame, ocr) };
    let (reading, boxes) = mesos::read_with(frame, ocr, mesos::Ink::SMOOTHED)?;
    if reading.is_some() || boxes == 0 {
        return Ok((reading, boxes));
    }
    let view = Canonical::new(frame, scale);
    if let (Some(reading), _) = mesos::read_with(&view.frame, ocr, mesos::Ink::SMOOTHED)? {
        return Ok((Some(MesoReading { rect: view.to_window(reading.rect), ..reading }), boxes));
    }
    mesos::read(frame, ocr)
}

/// A quick OCR pass over the whole frame at native size. It only needs to find anchors such
/// as panel headers; each reader then reads its own region more carefully.
fn overview(frame: &Frame, ocr: &Ocr) -> Result<Vec<Line>> {
    Ok(group_lines(&ocr.words(frame)?))
}
