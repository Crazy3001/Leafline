//! The status bar at the bottom of the screen: level, job, name, HP, MP and EXP.
//!
//! The HUD is found by its coloured gauges (red HP, blue MP, yellow EXP) rather than by fixed
//! coordinates, so it works at any resolution. Everything else is placed relative to the gauge
//! slots, in units of the slot height, using proportions measured on Classic's default HUD.
//!
//! The numbers use a tiny pixel font that general OCR misreads, so they are read with glyph
//! templates the user teaches once by typing what their HUD shows ([`calibrate`]). Job and name
//! use a plainer font and go through OCR.

use std::sync::LazyLock;

use image::Rgba;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::text::{best_match, digitize, parse_grouped};
use crate::error::{Error, Result};
use crate::vision::glyphs::{GlyphSet, MIN_SIMILARITY, Mask, ink_columns, ink_rows};
use crate::vision::{Frame, Ocr, Rect, color, group_lines, preprocess};

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudReading {
    pub level: Option<u32>,
    pub job: Option<String>,
    pub name: Option<String>,
    pub hp: Option<Gauge>,
    pub mp: Option<Gauge>,
    pub exp: Option<Exp>,
    /// The numbers could not all be read because the HUD font has not been learned at this
    /// size, or shows a character not seen yet. Calibrating fixes it.
    pub needs_calibration: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Gauge {
    pub current: u32,
    pub max: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Exp {
    pub points: u64,
    pub percent: f32,
}

/// Where each part of the HUD sits in the frame.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudLayout {
    /// Full gauge slots (filled and empty parts).
    pub hp: Rect,
    pub mp: Rect,
    pub exp: Rect,
    /// The bracketed values printed above the gauges.
    pub labels: Rect,
    /// The level digits.
    pub level: Option<Rect>,
    /// Job and character name, stacked.
    pub identity: Rect,
}

/// The HUD's two pixel fonts, learned from the user's screen.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HudFont {
    /// Gauge values and EXP: digits, `/`, separators and `%`.
    pub values: GlyphSet,
    /// The bold level digits.
    pub level: GlyphSet,
    /// Character names calibrated so far, which OCR sometimes misreads.
    #[serde(default)]
    pub names: Vec<KnownName>,
}

/// A character name and how the HUD draws it. Names are matched whole, because the letters of
/// the HUD's name font touch and cannot be split reliably.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnownName {
    pub name: String,
    pub mask: Mask,
}

/// What the user reads off their HUD during calibration.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HudValues {
    pub level: u32,
    pub hp: (u32, u32),
    pub mp: (u32, u32),
    pub exp_points: u64,
    /// As shown, for example `25.42`; a trailing `%` is optional.
    pub exp_percent: String,
    pub name: Option<String>,
}

/// Which fields calibration could match to the screen.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Calibration {
    pub learned: Vec<&'static str>,
    pub failed: Vec<&'static str>,
}

pub fn read(frame: &Frame, ocr: &Ocr, font: &HudFont) -> Result<Option<(HudLayout, HudReading)>> {
    let Some(layout) = locate(frame) else { return Ok(None) };
    let glyphs = Glyphs::extract(frame, &layout);
    let mut reading = HudReading::default();

    reading.hp = font.values.read(&glyphs.hp).and_then(|t| parse_gauge(&t));
    reading.mp = font.values.read(&glyphs.mp).and_then(|t| parse_gauge(&t));
    reading.exp = exp_points(&font.values, &glyphs.before_percent)
        .zip(font.values.read(&glyphs.percent))
        .and_then(|(points, percent)| parse_exp(&points, &percent));
    reading.level = font.level.read(&glyphs.level).and_then(|t| parse_level(&t));
    reading.needs_calibration = reading.hp.is_none()
        || reading.mp.is_none()
        || reading.exp.is_none()
        || (!glyphs.level.is_empty() && reading.level.is_none());

    // Job and name use a plainer font that OCR handles once enlarged and keyed.
    const IDENTITY_SCALE: f32 = 3.0;
    let words = ocr.words_in(frame, layout.identity, IDENTITY_SCALE, |crop| {
        preprocess::pad(&preprocess::keep(&preprocess::upscale(crop, IDENTITY_SCALE), is_light_text), 24)
    })?;
    let lines = group_lines(&words);
    reading.job = lines.first().map(|l| best_match(&l.text, &JOBS, 0.6).map_or_else(|| l.text.clone(), String::from));
    // A calibrated name is read exactly by its glyphs; any other name falls back to OCR.
    let known = glyphs
        .name
        .as_ref()
        .and_then(|mask| font.names.iter().find(|n| n.mask.similarity(mask) >= MIN_SIMILARITY).map(|n| n.name.clone()));
    reading.name = known.or_else(|| lines.get(1).map(|l| l.text.replace(' ', "")));

    Ok(Some((layout, reading)))
}

/// Learns the HUD font from a frame whose values the user has typed in. Fields that cannot be
/// matched to the screen (a typo, or something covering the HUD) are reported, not learned.
pub fn calibrate(frame: &Frame, font: &mut HudFont, values: &HudValues) -> Result<Calibration> {
    let layout = locate(frame).ok_or_else(|| Error::Invalid("no HUD found in this image".into()))?;
    let glyphs = Glyphs::extract(frame, &layout);
    let mut report = Calibration::default();
    let mut record =
        |field, learned: bool| if learned { report.learned.push(field) } else { report.failed.push(field) };

    record("HP", font.values.learn(&glyphs.hp, &format!("{}/{}", values.hp.0, values.hp.1)));
    record("MP", font.values.learn(&glyphs.mp, &format!("{}/{}", values.mp.0, values.mp.1)));
    let points = group_thousands(values.exp_points);
    let points_glyphs =
        glyphs.before_percent.len().checked_sub(points.len()).map(|start| &glyphs.before_percent[start..]);
    record("EXP", points_glyphs.is_some_and(|g| font.values.learn(g, &points)));
    let percent = format!("{}%", values.exp_percent.trim().trim_end_matches('%'));
    record("EXP %", font.values.learn(&glyphs.percent, &percent));
    record("Level", font.level.learn(&glyphs.level, &values.level.to_string()));
    if let Some(name) = values.name.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
        let mask = glyphs.name.clone();
        record("Name", mask.is_some());
        if let Some(mask) = mask {
            font.names.retain(|n| n.name != name);
            font.names.push(KnownName { name: name.to_string(), mask });
        }
    }
    Ok(report)
}

/// Reads the EXP points, the glyphs between the `EXP.` label and the percentage.
fn exp_points(font: &GlyphSet, glyphs: &[Mask]) -> Option<String> {
    let text: String = glyphs.iter().map(|g| font.recognize(g)).collect::<Option<_>>()?;
    (!text.is_empty()).then_some(text)
}

/// `24331` as the HUD prints it, `24,331`.
fn group_thousands(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The value glyphs around the gauges, split into fields.
struct Glyphs {
    hp: Vec<Mask>,
    mp: Vec<Mask>,
    /// The EXP points, without the bold `EXP.` label.
    before_percent: Vec<Mask>,
    percent: Vec<Mask>,
    level: Vec<Mask>,
    /// The character name, the lower of the two identity lines, as one mask.
    name: Option<Mask>,
}

impl Glyphs {
    fn extract(frame: &Frame, layout: &HudLayout) -> Self {
        let unit = layout.hp.h;
        let field =
            |left: f32, right: f32| Rect::from_edges(left - 0.2 * unit, layout.labels.y, right, layout.labels.bottom());
        let hp = field(layout.hp.x, layout.mp.x - 0.2 * unit);
        let mp = field(layout.mp.x, layout.exp.x - 0.2 * unit);
        let exp = field(layout.exp.x, layout.exp.right() + 0.3 * unit);

        // The values sit between green brackets, which also mark the digit rows. The labels in
        // front (HP, MP, EXP.) are bold and taller, so ink above the brackets marks a label.
        let (level, name) = (level_glyphs(frame, layout), name_glyphs(frame, layout));
        let Some(rows) = ink_rows(frame, layout.labels, is_bracket) else {
            return Self { hp: vec![], mp: vec![], before_percent: vec![], percent: vec![], level, name };
        };
        let label_band = Rect::from_edges(layout.labels.x, layout.labels.y, layout.labels.right(), rows.0 as f32);
        let mask = |cols: (u32, u32)| Mask::new(frame, cols, rows, is_light_text);
        let is_label = |cols: (u32, u32)| {
            let column = Rect::from_edges(cols.0 as f32, label_band.y, cols.1 as f32, label_band.bottom());
            ink_rows(frame, column, is_light_text).is_some()
        };
        let bracketed = |region: Rect| -> (Vec<Mask>, Vec<Mask>) {
            let brackets = ink_columns(frame, region, is_bracket);
            let (Some(open), Some(close)) = (brackets.first(), brackets.last()) else { return (vec![], vec![]) };
            let text = ink_columns(frame, region, is_light_text);
            let inside = text.iter().filter(|c| c.0 >= open.1 && c.1 <= close.0).map(|&c| mask(c)).collect();
            let before = text.iter().filter(|c| c.1 <= open.0 && !is_label(**c)).map(|&c| mask(c)).collect();
            (before, inside)
        };

        let (_, hp) = bracketed(hp);
        let (_, mp) = bracketed(mp);
        let (before_percent, percent) = bracketed(exp);
        Self { hp, mp, before_percent, percent, level, name }
    }
}

/// The level digits, white on orange tiles.
fn level_glyphs(frame: &Frame, layout: &HudLayout) -> Vec<Mask> {
    layout.level.map_or_else(Vec::new, |area| glyphs_in(frame, area, is_level_digit))
}

fn name_glyphs(frame: &Frame, layout: &HudLayout) -> Option<Mask> {
    let area = Rect::from_edges(layout.identity.x, layout.hp.y, layout.identity.right(), layout.identity.bottom());
    let rows = ink_rows(frame, area, is_light_text)?;
    let columns = ink_columns(frame, area, is_light_text);
    Some(Mask::new(frame, (columns.first()?.0, columns.last()?.1), rows, is_light_text))
}

/// One mask per run of inked columns, all spanning the inked rows of `area`.
fn glyphs_in(frame: &Frame, area: Rect, is_ink: fn(&Rgba<u8>) -> bool) -> Vec<Mask> {
    let Some(rows) = ink_rows(frame, area, is_ink) else { return vec![] };
    ink_columns(frame, area, is_ink).into_iter().map(|cols| Mask::new(frame, cols, rows, is_ink)).collect()
}

pub fn locate(frame: &Frame) -> Option<HudLayout> {
    let (width, height) = (frame.width() as f32, frame.height() as f32);
    let search = Rect::new(0.0, height * 0.65, width, height * 0.35);
    let min_run = (width * 0.02).max(8.0) as u32;
    let bars = |matches: fn(&Rgba<u8>) -> bool| -> Vec<Rect> {
        color::find_blocks(frame, search, min_run, matches)
            .into_iter()
            .filter(|b| b.h >= 3.0 && b.w >= 3.0 * b.h)
            .collect()
    };
    let reds = bars(is_hp_red);
    let blues = bars(is_mp_blue);
    let yellows = bars(is_exp_yellow);

    // An HP bar with an MP bar directly to its right, ideally followed by an EXP bar. Fills
    // start at the left of their slots, so the distance between their starts is the slot pitch
    // whatever the gauges hold; a slot is about 15 times as wide as its fill is tall. Other red
    // and blue UI (buttons, party HP) rarely forms that sequence; prefer the thickest.
    let (hp_bar, mp_bar, exp_bar) = reds
        .iter()
        .flat_map(|r| blues.iter().map(move |b| (*r, *b)))
        .filter(|(r, b)| beside(r, b) && b.x - r.x <= 20.0 * r.h)
        .map(|(r, b)| {
            let pitch = b.x - r.x;
            let exp = yellows
                .iter()
                .filter(|y| beside(&b, y) && y.x > b.x && y.x - b.x <= 1.3 * pitch)
                .min_by(|a, c| a.x.total_cmp(&c.x));
            (r, b, exp.copied())
        })
        .max_by(|a, b| score(a).total_cmp(&score(b)))?;

    // The gauges sit in white-framed slots; the slot height is the unit for everything else.
    let (top, bottom) = slot_edges(frame, &hp_bar);
    let unit = bottom - top;
    let gap = (0.1 * unit).max(1.0);
    let pitch = mp_bar.x - hp_bar.x;

    let hp = Rect::from_edges(hp_bar.x, top, mp_bar.x - gap, bottom);
    // Without an EXP fill to mark it, the next slot is assumed one pitch along.
    let exp_x = exp_bar.map_or(mp_bar.x + pitch, |e| e.x);
    let mp = Rect::from_edges(mp_bar.x, top, exp_x - gap, bottom);
    let exp = match exp_bar {
        Some(bar) => Rect::from_edges(bar.x, top, extend_gauge_right(frame, bar), bottom),
        None => Rect::from_edges(exp_x, top, exp_x + pitch, bottom),
    };

    let labels = Rect::from_edges(hp.x - 0.1 * unit, top - 0.85 * unit, exp.right(), top - 0.05 * unit);
    let level_area = Rect::from_edges(hp.x - 12.5 * unit, top - 0.6 * unit, hp.x - 8.6 * unit, top + 0.6 * unit);
    let level = color::find_blocks(frame, level_area, 2, is_level_tile)
        .into_iter()
        .filter(|b| b.h >= 0.4 * unit)
        .reduce(|a, b| a.union(&b));
    let identity = Rect::from_edges(hp.x - 8.8 * unit, top - 0.85 * unit, hp.x - 0.7 * unit, top + 0.75 * unit);

    Some(HudLayout { hp, mp, exp, labels, level, identity })
}

/// Whether `right` starts at or after the end of `left` on the same row.
fn beside(left: &Rect, right: &Rect) -> bool {
    left.vertical_overlap(right) >= 0.5 * left.h.min(right.h) && right.x > left.right() - 2.0
}

fn score((hp, mp, exp): &(Rect, Rect, Option<Rect>)) -> f32 {
    let thickness = hp.h.min(mp.h);
    if exp.is_some() { thickness * 2.0 } else { thickness }
}

/// The top and bottom of the white frame around a gauge, found by walking out from its fill.
/// Falls back to the fill itself when no frame is visible.
fn slot_edges(frame: &Frame, bar: &Rect) -> (f32, f32) {
    let x = (bar.x + bar.w / 2.0) as u32;
    let center = bar.center_y() as i64;
    let reach = (2.0 * bar.h) as i64;
    let is_frame = |y: i64| y >= 0 && y < i64::from(frame.height()) && is_slot_frame(frame.get_pixel(x, y as u32));
    // Walk out to the frame, then through it to its outer edge.
    let edge = |step: i64| {
        let inner = (1..reach).map(|d| center + step * d).find(|&y| is_frame(y))?;
        Some((0..).map(|d| inner + step * d).take_while(|&y| is_frame(y)).last().unwrap_or(inner))
    };
    match (edge(-1), edge(1)) {
        (Some(top), Some(bottom)) => (top as f32, (bottom + 1) as f32),
        _ => (bar.y, bar.bottom()),
    }
}

/// Follows the unfilled (light) part of a gauge to find where its slot ends.
fn extend_gauge_right(frame: &Frame, bar: Rect) -> f32 {
    let y = bar.center_y() as u32;
    let mut x = bar.right() as u32;
    while x < frame.width() && preprocess::luminance(frame.get_pixel(x, y)) >= 150 {
        x += 1;
    }
    x as f32
}

fn is_hp_red(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0;
    r >= 170 && g <= 80 && b <= 80
}

fn is_mp_blue(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0;
    b >= 170 && r <= 90 && b > g
}

fn is_exp_yellow(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0;
    r >= 180 && g >= 180 && b <= 90
}

fn is_slot_frame(px: &Rgba<u8>) -> bool {
    preprocess::luminance(px) >= 225 && preprocess::saturation(px) <= 40
}

/// The orange tiles behind the level digits.
fn is_level_tile(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0.map(i16::from);
    r >= 190 && g >= 80 && b <= 110 && r >= g + 50
}

fn is_level_digit(px: &Rgba<u8>) -> bool {
    preprocess::luminance(px) >= 200 && preprocess::saturation(px) <= 60
}

/// The light grey of HUD text.
fn is_light_text(px: &Rgba<u8>) -> bool {
    preprocess::luminance(px) >= 150 && preprocess::saturation(px) <= 60
}

/// The green brackets around gauge values.
fn is_bracket(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0.map(i16::from);
    g >= 130 && g >= b + 70 && r < g
}

/// Job names as the HUD shows them, for correcting OCR. Taken from the original game; Classic
/// may name some differently, in which case the raw OCR text is kept.
const JOBS: [&str; 39] = [
    "Beginner",
    "Swordman",
    "Fighter",
    "Page",
    "Spearman",
    "Crusader",
    "White Knight",
    "Dragon Knight",
    "Hero",
    "Paladin",
    "Dark Knight",
    "Magician",
    "Wizard (Fire, Poison)",
    "Wizard (Ice, Lightning)",
    "Cleric",
    "Mage (Fire, Poison)",
    "Mage (Ice, Lightning)",
    "Priest",
    "Arch Mage (Fire, Poison)",
    "Arch Mage (Ice, Lightning)",
    "Bishop",
    "Archer",
    "Hunter",
    "Crossbowman",
    "Ranger",
    "Sniper",
    "Bowmaster",
    "Marksman",
    "Rogue",
    "Assassin",
    "Bandit",
    "Hermit",
    "Chief Bandit",
    "Night Lord",
    "Shadower",
    "Pirate",
    "Brawler",
    "Gunslinger",
    "GM",
];

static GAUGE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d{1,7})/(\d{1,7})$").unwrap());
static PERCENT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\d{1,3}[.,]\d{1,2})%$").unwrap());
static LEVEL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d{1,3}$").unwrap());

/// Parses `1149/1149`.
pub fn parse_gauge(text: &str) -> Option<Gauge> {
    let caps = GAUGE.captures(text)?;
    let (current, max): (u32, u32) = (caps[1].parse().ok()?, caps[2].parse().ok()?);
    (max > 0 && current <= max).then_some(Gauge { current, max })
}

/// Parses the points (`24,331`) and percentage (`25.42%`) shown next to the EXP gauge.
pub fn parse_exp(points: &str, percent: &str) -> Option<Exp> {
    let points = parse_grouped(points)?;
    let caps = PERCENT.captures(percent)?;
    let percent: f32 = caps[1].replace(',', ".").parse().ok()?;
    (percent <= 100.0).then_some(Exp { points, percent })
}

pub fn parse_level(text: &str) -> Option<u32> {
    LEVEL.is_match(text).then(|| digitize(text).parse().ok()).flatten().filter(|lv| (1..=250).contains(lv))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gauges() {
        assert_eq!(parse_gauge("1149/1149"), Some(Gauge { current: 1149, max: 1149 }));
        assert_eq!(parse_gauge("1112/1149"), Some(Gauge { current: 1112, max: 1149 }));
        assert_eq!(parse_gauge("1150/1149"), None, "current never exceeds max");
        assert_eq!(parse_gauge("0/0"), None);
        assert_eq!(parse_gauge("11491149"), None);
    }

    #[test]
    fn exp() {
        assert_eq!(parse_exp("24,331", "25.42%"), Some(Exp { points: 24331, percent: 25.42 }));
        assert_eq!(parse_exp("24.331", "25,42%"), Some(Exp { points: 24331, percent: 25.42 }));
        assert_eq!(parse_exp("24,331", "25.42"), None);
        assert_eq!(parse_exp("1", "100.01%"), None);
    }

    #[test]
    fn level() {
        assert_eq!(parse_level("30"), Some(30));
        assert_eq!(parse_level("0"), None);
        assert_eq!(parse_level("3/0"), None);
    }

    #[test]
    fn thousands_are_grouped_like_the_hud() {
        assert_eq!(group_thousands(24331), "24,331");
        assert_eq!(group_thousands(331), "331");
        assert_eq!(group_thousands(1234567), "1,234,567");
    }
}
