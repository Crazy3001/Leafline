//! Frame → readings. The same pipeline runs on live captures and on saved screenshots.

use std::time::Instant;

use serde::Serialize;

use crate::error::Result;
use crate::readers::hud::{HudFont, HudLayout, HudReading};
use crate::readers::map::MapReading;
use crate::readers::mesos::MesoReading;
use crate::readers::quest::QuestReading;
use crate::readers::tooltip::ItemTooltip;
use crate::readers::{hud, map, mesos, party, quest, shop, tooltip};
use crate::vision::{Frame, Line, Ocr, group_lines};

#[derive(Debug, Clone, Serialize)]
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

/// The result of reading one part, or `None` with the part noted: one part failing must not cost
/// the others.
fn kept<T>(failed: &mut Vec<&'static str>, part: &'static str, result: Result<T>) -> Option<T> {
    result.map_err(|err| eprintln!("leafline: {part}: {err}")).inspect_err(|()| failed.push(part)).ok()
}

pub fn analyze(frame: &Frame, ocr: &Ocr, hud_font: &HudFont) -> Result<Analysis> {
    let mut failed = Vec::new();
    let mut timings = Vec::new();
    let mut timed = |stage: &'static str, started: Instant| {
        timings.push(Timing { stage, ms: started.elapsed().as_secs_f32() * 1000.0 });
    };

    let started = Instant::now();
    let (hud_layout, hud) = kept(&mut failed, "HUD", hud::read(frame, ocr, hud_font)).flatten().unzip();
    timed("HUD", started);

    let started = Instant::now();
    let overview = kept(&mut failed, "text", overview(frame, ocr)).unwrap_or_default();
    timed("Overview OCR", started);

    let started = Instant::now();
    let tooltips = kept(&mut failed, "item tooltip", tooltip::read_all(frame, &overview, ocr)).unwrap_or_default();
    timed("Tooltips", started);

    let started = Instant::now();
    let quest = kept(&mut failed, "Quest Helper", quest::read(frame, &overview, ocr)).flatten();
    let (mesos, meso_boxes) = kept(&mut failed, "mesos", mesos::read(frame, ocr)).unwrap_or_default();
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

/// A quick OCR pass over the whole frame at native size. It only needs to find anchors such
/// as panel headers; each reader then reads its own region more carefully.
fn overview(frame: &Frame, ocr: &Ocr) -> Result<Vec<Line>> {
    Ok(group_lines(&ocr.words(frame)?))
}
