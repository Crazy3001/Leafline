//! The status bar at the bottom of the screen: level, job, name, HP, MP and EXP.
//!
//! The HUD is found by its coloured gauges (red HP, blue MP, yellow EXP) rather than by fixed
//! coordinates, so it works at any resolution. Everything else is placed relative to the gauge
//! slots, in units of the slot height, using proportions measured on Classic's default HUD.
//!
//! The numbers use a tiny pixel font that general OCR misreads, so they are read by rules about
//! their shape ([`digits`]). Job and name use a plainer font and go through OCR.

use std::sync::LazyLock;

use image::Rgba;
use regex::Regex;
use serde::Serialize;

use super::digits;
use super::text::{best_match, digitize, parse_grouped};
use super::ui::{self, CANONICAL_SCALE, Canonical};
use crate::error::Result;
use crate::vision::glyphs::{Mask, ink_columns, ink_rows};
use crate::vision::{Frame, Line, Ocr, Rect, color, group_lines, preprocess};

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudReading {
    pub level: Option<u32>,
    pub job: Option<String>,
    pub name: Option<String>,
    pub hp: Option<Gauge>,
    pub mp: Option<Gauge>,
    pub exp: Option<Exp>,
    /// The EXP percentage, when the HUD shows it with no points beside it, as it does at high
    /// levels.
    pub exp_percent: Option<f32>,
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

/// The HUD as glyphs are read from it: the frame itself when the UI is at the size the digits are
/// read at, otherwise put back on the design grid at that size.
fn canonical(frame: &Frame, layout: HudLayout) -> Option<(Glyphs, HudLayout)> {
    let scale = ui::hud_scale(frame, &layout);
    if (scale - CANONICAL_SCALE).abs() <= 0.005 {
        return Some((Glyphs::extract(frame, &layout), layout));
    }
    let view = Canonical::new(frame, scale);
    let found = locate(&view.frame)?;
    Some((Glyphs::extract(&view.frame, &found), view.layout_to_window(&found)))
}

pub fn read(frame: &Frame, ocr: &Ocr) -> Result<Option<(HudLayout, HudReading)>> {
    let Some(located) = locate(frame) else { return Ok(None) };
    let Some((glyphs, layout)) = canonical(frame, located) else { return Ok(None) };
    let mut reading = HudReading::default();

    reading.hp = digits::values(&glyphs.hp).and_then(|t| parse_gauge(&t));
    reading.mp = digits::values(&glyphs.mp).and_then(|t| parse_gauge(&t));
    let points = digits::values(&glyphs.before_percent).filter(|t| !t.is_empty());
    reading.exp =
        points.zip(digits::values(&glyphs.percent)).and_then(|(points, percent)| parse_exp(&points, &percent));
    if glyphs.before_percent.is_empty() {
        reading.exp_percent = digits::values(&glyphs.percent).and_then(|t| parse_percent(&t));
    }
    reading.level = digits::number(&glyphs.level).and_then(|t| parse_level(&t));

    // Job and name use a plainer font that OCR handles once enlarged and keyed.
    const IDENTITY_SCALE: f32 = 3.0;
    let words = ocr.words_in(frame, layout.identity, IDENTITY_SCALE, |crop| {
        preprocess::pad(&preprocess::keep(&preprocess::upscale(crop, IDENTITY_SCALE), is_light_text), 24)
    })?;
    let lines = group_lines(&words);
    reading.job = lines.first().map(|l| best_match(&l.text, &JOBS, 0.6).map_or_else(|| l.text.clone(), String::from));
    reading.name = lines.get(1).map(|l| l.text.replace(' ', ""));

    Ok(Some((layout, reading)))
}

/// Takes the character's name from the pass over the whole frame, which gets it right where a
/// crop of it alone stumbles on letters that touch (`PebuieThief`). That pass may miss the line
/// or read only the job above it, and then the crop's reading stands.
pub fn settle_name(reading: &mut HudReading, layout: &HudLayout, overview: &[Line]) {
    let area = layout.identity;
    let inside = |line: &&Line| {
        let (x, y) = (line.rect.x, line.rect.center_y());
        x >= area.x && x <= area.right() && y >= area.y && y <= area.bottom()
    };
    let lowest = overview.iter().filter(inside).max_by(|a, b| a.rect.y.total_cmp(&b.rect.y));
    if let Some(line) = lowest.filter(|line| best_match(&line.text, &JOBS, 0.75).is_none()) {
        reading.name = Some(line.text.replace(' ', ""));
    }
}

/// The value glyphs around the gauges, split into fields.
struct Glyphs {
    hp: Vec<Mask>,
    mp: Vec<Mask>,
    /// The EXP points, without the bold `EXP.` label.
    before_percent: Vec<Mask>,
    percent: Vec<Mask>,
    level: Vec<Mask>,
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
        let level = level_glyphs(frame, layout);
        let Some(rows) = ink_rows(frame, layout.labels, is_bracket) else {
            return Self { hp: vec![], mp: vec![], before_percent: vec![], percent: vec![], level };
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
        Self { hp, mp, before_percent, percent, level }
    }
}

/// The level digits, white on orange tiles.
fn level_glyphs(frame: &Frame, layout: &HudLayout) -> Vec<Mask> {
    layout.level.map_or_else(Vec::new, |area| glyphs_in(frame, area, is_level_digit))
}

/// One mask per run of inked columns, all spanning the inked rows of `area`.
fn glyphs_in(frame: &Frame, area: Rect, is_ink: fn(&Rgba<u8>) -> bool) -> Vec<Mask> {
    let Some(rows) = ink_rows(frame, area, is_ink) else { return vec![] };
    ink_columns(frame, area, is_ink).into_iter().map(|cols| Mask::new(frame, cols, rows, is_ink)).collect()
}

pub fn locate(frame: &Frame) -> Option<HudLayout> {
    let (width, height) = (frame.width() as f32, frame.height() as f32);
    let search = Rect::new(0.0, height * 0.65, width, height * 0.35);
    // Low on HP or MP a gauge's fill is short, so short ones count.
    let min_run = (width * 0.002).max(3.0) as u32;
    let bars = |matches: fn(&Rgba<u8>) -> bool| -> Vec<Rect> {
        color::find_blocks(frame, search, min_run, matches)
            .into_iter()
            .filter(|b| b.h >= 3.0 && b.w >= 0.5 * b.h)
            .collect()
    };
    let reds = bars(is_hp_red);
    let blues = bars(is_mp_blue);
    let yellows = bars(is_exp_yellow);

    // An HP bar with an MP bar directly to its right, ideally followed by an EXP bar. Fills
    // start at the left of their slots, so the distance between their starts is the slot pitch
    // whatever the gauges hold; a slot is about 15 times as wide as its fill is tall. Other red
    // and blue UI (buttons, party HP) rarely forms that sequence; prefer the thickest.
    let gauges = |fits: &dyn Fn(&Rect, &Rect) -> bool| {
        reds.iter()
            .flat_map(|r| blues.iter().map(move |b| (*r, *b)))
            .filter(|(r, b)| beside(r, b) && b.x - r.x <= 20.0 * r.h && fits(r, b))
            .map(|(r, b)| {
                let pitch = b.x - r.x;
                let exp = yellows
                    .iter()
                    .filter(|y| beside(&b, y) && y.x > b.x && y.x - b.x <= 1.3 * pitch)
                    .min_by(|a, c| a.x.total_cmp(&c.x));
                (r, b, exp.copied())
            })
            .collect::<Vec<_>>()
    };
    // Short fills are fine when the gauges stand exactly where and as far apart as the UI puts
    // them. The red and blue menu buttons can too, but they are to the right of the gauges. At a
    // size not known, only long fills are trusted.
    let long = |bar: &Rect| bar.w >= 3.0 * bar.h && bar.w >= width * 0.02;
    let leftmost = gauges(&|r, b| ui::fits_gauges(frame, b.x - r.x, r.bottom()))
        .into_iter()
        .min_by(|a, b| a.0.x.total_cmp(&b.0.x));
    let thickest = || gauges(&|r, b| long(r) && long(b)).into_iter().max_by(|a, b| score(a).total_cmp(&score(b)));
    let (hp_bar, mp_bar, exp_bar) = leftmost.or_else(thickest)?;

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

/// The gauge's white frame. A HUD drawn at an uneven scale softens it and tints it with the fill.
fn is_slot_frame(px: &Rgba<u8>) -> bool {
    preprocess::luminance(px) >= 170 && preprocess::saturation(px) <= 80
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

/// Whether `job` is a job the game has, not a misread.
pub fn is_known_job(job: &str) -> bool {
    JOBS.contains(&job)
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
    Some(Exp { points, percent: parse_percent(percent)? })
}

/// Parses a percentage such as `25.42%`.
pub fn parse_percent(percent: &str) -> Option<f32> {
    let caps = PERCENT.captures(percent)?;
    let percent: f32 = caps[1].replace(',', ".").parse().ok()?;
    (percent <= 100.0).then_some(percent)
}

pub fn parse_level(text: &str) -> Option<u32> {
    LEVEL.is_match(text).then(|| digitize(text).parse().ok()).flatten().filter(|lv| (1..=250).contains(lv))
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: [u8; 3] = [220, 40, 40];
    const BLUE: [u8; 3] = [40, 60, 220];
    const YELLOW: [u8; 3] = [220, 220, 40];

    /// A made-up 1440p picture with these (x, y, width, colour) bars on it, each 14 tall.
    fn picture(bars: &[(u32, u32, u32, [u8; 3])]) -> Frame {
        Frame::from_fn(2560, 1440, |x, y| {
            let bar = bars.iter().find(|b| (b.0..b.0 + b.2).contains(&x) && (b.1..b.1 + 14).contains(&y));
            let [r, g, b] = bar.map_or([10, 10, 10], |b| b.3);
            Rgba([r, g, b, 255])
        })
    }

    #[test]
    fn the_name_comes_from_the_whole_frame_when_it_read_one() {
        let area = Rect::new(500.0, 1380.0, 300.0, 60.0);
        let slot = Rect::new(0.0, 0.0, 10.0, 10.0);
        let layout = HudLayout { hp: slot, mp: slot, exp: slot, labels: slot, level: None, identity: area };
        let line = |text: &str, x: f32, y: f32| {
            let rect = Rect::new(x, y, 120.0, 20.0);
            Line { text: text.into(), rect, words: vec![] }
        };
        let settled = |overview: &[Line]| {
            let mut reading = HudReading { name: Some("PebuieThief".into()), ..HudReading::default() };
            settle_name(&mut reading, &layout, overview);
            reading.name.unwrap()
        };
        assert_eq!(settled(&[line("Beginner", 510.0, 1385.0), line("PebbleThief", 510.0, 1410.0)]), "PebbleThief");
        assert_eq!(settled(&[line("PebbleThief", 510.0, 1410.0)]), "PebbleThief");
        assert_eq!(settled(&[line("Beginner", 510.0, 1385.0)]), "PebuieThief", "only the job was read");
        assert_eq!(settled(&[line("Someone", 900.0, 1000.0)]), "PebuieThief", "another player's name tag");
    }

    #[test]
    fn scenery_spaced_like_the_gauges_is_not_the_hud() {
        let frame = picture(&[
            (396, 936, 160, RED),
            (617, 936, 160, BLUE),
            (920, 1412, 150, RED),
            (1136, 1412, 150, BLUE),
            (1366, 1412, 150, YELLOW),
        ]);
        assert_eq!(locate(&frame).map(|hud| hud.hp.x), Some(920.0));
    }

    #[test]
    fn nearly_empty_gauges_are_still_found() {
        let frame = picture(&[(920, 1412, 10, RED), (1136, 1412, 10, BLUE), (1366, 1412, 150, YELLOW)]);
        assert_eq!(locate(&frame).map(|hud| hud.hp.x), Some(920.0));
    }

    #[test]
    fn short_scenery_alone_is_not_the_hud() {
        assert!(locate(&picture(&[(396, 936, 10, RED), (612, 936, 10, BLUE)])).is_none());
    }

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
        assert_eq!(parse_percent("58.47%"), Some(58.47));
    }

    #[test]
    fn level() {
        assert_eq!(parse_level("30"), Some(30));
        assert_eq!(parse_level("0"), None);
        assert_eq!(parse_level("3/0"), None);
    }
}
