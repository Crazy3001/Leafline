//! Frame → readings. The same pipeline runs on live captures and on saved screenshots.

use std::time::Instant;

use serde::Serialize;

use crate::error::Result;
use crate::readers::hud::{HudFont, HudLayout, HudReading};
use crate::readers::map::MapReading;
use crate::readers::mesos::MesoReading;
use crate::readers::quest::QuestReading;
use crate::readers::tooltip::ItemTooltip;
use crate::readers::{hud, map, mesos, quest, tooltip};
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
    pub map: Option<MapReading>,
    /// Every line from the whole-frame OCR pass, for the workbench's debug view.
    pub overview: Vec<Line>,
    pub timings: Vec<Timing>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Timing {
    pub stage: &'static str,
    pub ms: f32,
}

pub fn analyze(frame: &Frame, ocr: &Ocr, hud_font: &HudFont) -> Result<Analysis> {
    let mut timings = Vec::new();
    let mut timed = |stage: &'static str, started: Instant| {
        timings.push(Timing { stage, ms: started.elapsed().as_secs_f32() * 1000.0 });
    };

    let started = Instant::now();
    let (hud_layout, hud) = hud::read(frame, ocr, hud_font)?.unzip();
    timed("HUD", started);

    let started = Instant::now();
    let overview = overview(frame, ocr)?;
    timed("Overview OCR", started);

    let started = Instant::now();
    let tooltips = tooltip::read_all(frame, &overview, ocr)?;
    timed("Tooltips", started);

    let started = Instant::now();
    let quest = quest::read(frame, &overview, ocr)?;
    let (mesos, meso_boxes) = mesos::read(frame, ocr)?;
    timed("Quest & mesos", started);

    let started = Instant::now();
    let map = map::read(frame, ocr)?;
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
        map,
        overview,
        timings,
    })
}

/// A quick OCR pass over the whole frame at native size. It only needs to find anchors such
/// as panel headers; each reader then reads its own region more carefully.
fn overview(frame: &Frame, ocr: &Ocr) -> Result<Vec<Line>> {
    Ok(group_lines(&ocr.words(frame)?))
}
