//! Item tooltips (hover and comparison).
//!
//! Each tooltip is found by its light separator rule with tooltip-like text nearby. The rule
//! is a fixed 224 game pixels wide, which gives the scale for everything else, and the panel
//! edges come from its light blue frame.
//!
//! Tooltips are translucent, so the scenery behind them confuses OCR. The text is read twice,
//! as is and with the text colour-keyed out of the background, and the passes are merged. The
//! `REQ` block uses a pixel font: its six rows are read by position, by the rules that read the
//! HUD's numbers, and failing that by OCR, with digits it is unsure of matched against ones it
//! read cleanly elsewhere in the frame.
//!
//! Items with no stats have no separator, so [`plain`] finds those.

mod parse;
mod plain;

use std::sync::LazyLock;

use image::{Rgba, imageops};
use regex::Regex;
use serde::Serialize;

pub use parse::{ItemTooltip, JOBS, JobFlag, Property, Requirements, Stat};

use super::digits::number;
use super::text::{best_match, starts_like, trim_noise};
use crate::error::Result;
use crate::vision::glyphs::{GlyphSet, MIN_SIMILARITY, Mask, ink_columns};
use crate::vision::{Frame, Line, Ocr, Rect, Word, color, group_lines, preprocess};

/// A tooltip panel found in the frame.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Panel {
    pub rect: Rect,
    /// The rule between the job flags and `Type:`.
    pub separator: Rect,
    /// Screen pixels per game pixel.
    pub unit: f32,
}

/// The separator's width in game pixels; the offsets below are in game pixels too.
const SEPARATOR_WIDTH: f32 = 224.0;
/// Distance between text lines.
const LINE_PITCH: f32 = 12.0;
/// The first `REQ` row's top, above the separator, and the height of its digits.
const REQ_TOP: f32 = 96.0;
const REQ_DIGIT_HEIGHT: f32 = 7.0;
/// Where `REQ` values start, right of the separator's left end.
const REQ_VALUE_LEFT: f32 = 138.0;
/// OCR reads best with text around this many pixels tall.
const TARGET_TEXT_HEIGHT: f32 = 30.0;
/// How much amber above the `REQ` block makes a name upgraded, in square game pixels. A name has
/// hundreds; one that isn't upgraded has none.
const MIN_AMBER_AREA: f32 = 25.0;
/// Amber text is keyed before enlarging, without smoothing, which keeps it crisp.
const AMBER_SCALE: u32 = 3;
/// How many lines above the `REQ` block a name can reach, wrapped.
const NAME_LINES: f32 = 3.25;

static DIGITS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(\d{1,3})\s*$").unwrap());
/// Words, an optional `(M)`-style tag and an optional `(+N)`, with nothing OCR made up.
static CLEAN_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z][A-Za-z' :-]*(?: \([A-Z]\))?(?: \(\+\d{1,2}\)?)?$").unwrap());

pub fn read_all(frame: &Frame, overview: &[Line], ocr: &Ocr) -> Result<Vec<ItemTooltip>> {
    let panels = locate(frame, overview);
    let requirements = panels.iter().map(|p| RequirementRows::read(frame, *p, ocr)).collect::<Result<Vec<_>>>()?;
    // Digits OCR read cleanly teach the ones it missed, such as lone zeros.
    let mut digits = GlyphSet::default();
    for row in requirements.iter().flat_map(|r| &r.rows) {
        if let Some(text) = &row.ocr {
            digits.learn(&row.glyphs, text);
        }
    }
    let (mut items, mut kept) = (Vec::new(), Vec::new());
    for (panel, requirements) in panels.into_iter().zip(requirements) {
        let mut item = read(frame, panel, ocr)?;
        // The Equip window's lines and slot labels can pass for a tooltip, but nothing there
        // reads as a type, a stat or enhancement slots.
        if item.item_type.is_some() || item.remaining_enhancements.is_some() || !item.stats.is_empty() {
            item.requirements = requirements.resolve(&digits).or(item.requirements);
            items.push(item);
            kept.push(panel);
        }
    }
    items.extend(plain::read_all(frame, overview, &kept, ocr)?);
    Ok(items)
}

/// Finds tooltip panels using the separator rules in `frame` and the text in `overview`
/// (a quick OCR pass over the whole frame).
pub fn locate(frame: &Frame, overview: &[Line]) -> Vec<Panel> {
    let width = frame.width() as f32;
    let whole = Rect::new(0.0, 0.0, width, frame.height() as f32);
    let pieces =
        color::find_blocks(frame, whole, (width * 0.03) as u32, is_rule).into_iter().filter(|r| r.h <= 3.0).collect();

    let mut panels: Vec<Panel> = Vec::new();
    for separator in join_collinear(pieces, width * 0.003).into_iter().filter(|r| r.w >= width * 0.08) {
        let unit = separator.w / SEPARATOR_WIDTH;
        let guess = LINE_PITCH * unit;
        // Job flags sit just above the separator and `Type:` just below it.
        let anchors: Vec<&Line> = overview
            .iter()
            .filter(|line| {
                line.rect.x >= separator.x - 2.0 * guess
                    && line.rect.x <= separator.right()
                    && (line.rect.center_y() - separator.y).abs() <= 2.5 * guess
                    && looks_like_tooltip_text(&line.text)
            })
            .collect();
        if anchors.is_empty() {
            continue;
        }
        // The panel's top is found from its frame, since long names wrap and make it taller. Its
        // bottom fades into the scenery, so a generous one is used; reading stops at the last line.
        let top = frame_top(frame, separator, unit).unwrap_or(separator.y - 150.0 * unit);
        let bottom = separator.y + 13.0 * LINE_PITCH * unit;
        let rect = Rect::from_edges(separator.x - 7.0 * unit, top, separator.right() + 7.0 * unit, bottom);
        panels.push(Panel { rect, separator, unit });
    }
    panels
}

pub fn read(frame: &Frame, panel: Panel, ocr: &Ocr) -> Result<ItemTooltip> {
    let scale = (TARGET_TEXT_HEIGHT / (LINE_PITCH * panel.unit)).clamp(1.5, 4.0);
    let plain = ocr.words_in(frame, panel.rect, scale, |crop| preprocess::upscale(crop, scale))?;
    let keyed = ocr.words_in(frame, panel.rect, scale, |crop| {
        preprocess::pad(&preprocess::keep(&preprocess::upscale(crop, scale), is_tooltip_text), 24)
    })?;

    let mut item = parse_pass(&plain, panel).merge(parse_pass(&keyed, panel));
    // An upgraded item's name is amber, which says so whatever OCR makes of its `(+1)`, and
    // keying that colour alone leaves the scenery behind the name out.
    let top = req_row_top(panel, 0) - NAME_LINES * LINE_PITCH * panel.unit;
    let above = Rect::from_edges(panel.rect.x, top.max(panel.rect.y), panel.rect.right(), req_row_top(panel, 0));
    let upgraded = amber_pixels(frame, above) >= MIN_AMBER_AREA * panel.unit * panel.unit;
    let amber = if upgraded { amber_name(frame, above, ocr)? } else { None };
    // Prefer whichever pass read the name cleanly; keying helps over busy scenery but can
    // garble the `(+1)` suffix.
    let names = [amber, name_text(&plain, panel), name_text(&keyed, panel)].map(|name| name.map(|n| tidy_name(&n)));
    if let Some(name) = names
        .iter()
        .flatten()
        .find(|n| CLEAN_NAME.is_match(n))
        .or(names[0].as_ref())
        .or(names[2].as_ref())
        .or(names[1].as_ref())
    {
        (item.name, item.upgrades) = parse::split_name(name);
    }
    // A wrapped name can lose its `(+N)` or read it out of order in the pass that won.
    if upgraded && item.upgrades.is_none() {
        item.upgrades = names.iter().flatten().find_map(|name| parse::upgrades_in(name));
    }
    item.upgraded = upgraded || item.upgrades.is_some_and(|n| n > 0);
    item.panel = panel.rect;
    item.jobs = read_jobs(frame, panel);
    Ok(item)
}

/// Rounds the brackets OCR reads as `{` or `[`, as in `Green Robin Hat {+1}`.
fn tidy_name(name: &str) -> String {
    name.replace(['{', '['], "(").replace(['}', ']'], ")")
}

/// An amber name, each line read alone: OCR loses or reorders the lines of a wrapped name read
/// together.
fn amber_name(frame: &Frame, area: Rect, ocr: &Ocr) -> Result<Option<String>> {
    let (x, y, w, h) = area.to_pixels(frame.width(), frame.height());
    let ink = |row: u32| (x..x + w).filter(|&col| is_amber(frame.get_pixel(col, row))).count();
    let mut lines: Vec<(u32, u32)> = Vec::new();
    for row in (y..y + h).filter(|&row| ink(row) > 0) {
        match lines.last_mut() {
            Some(line) if row - line.1 <= 2 => line.1 = row + 1,
            _ => lines.push((row, row + 1)),
        }
    }
    let mut name = Vec::new();
    for (top, bottom) in lines {
        // Cropped close: OCR finds nothing in a short line left in a wide, empty picture.
        let inked: Vec<u32> =
            (x..x + w).filter(|&col| (top..bottom).any(|row| is_amber(frame.get_pixel(col, row)))).collect();
        let (Some(left), Some(right)) = (inked.first(), inked.last()) else { continue };
        let line = Rect::from_edges(*left as f32 - 6.0, top as f32 - 4.0, *right as f32 + 7.0, bottom as f32 + 4.0);
        let words = ocr.words_in(frame, line, AMBER_SCALE as f32, |crop| {
            // Looser than the check for amber: text over a dark patch of scenery comes out dimmer.
            let keyed = preprocess::keep(crop, |px| {
                let [r, g, b, _] = px.0;
                r >= 150 && g >= 90 && b <= 70 && r >= g + 40
            });
            let (w, h) = (keyed.width() * AMBER_SCALE, keyed.height() * AMBER_SCALE);
            preprocess::pad(&imageops::resize(&keyed, w, h, imageops::FilterType::Nearest), 24)
        })?;
        let text: Vec<&str> = words.iter().map(|word| word.text.as_str()).collect();
        name.push(text.join(" "));
    }
    let name = name.join(" ");
    Ok((!name.trim().is_empty()).then(|| trim_noise(&name).to_string()))
}

fn amber_pixels(frame: &Frame, area: Rect) -> f32 {
    let (x, y, w, h) = area.to_pixels(frame.width(), frame.height());
    (y..y + h).map(|row| (x..x + w).filter(|&col| is_amber(frame.get_pixel(col, row))).count()).sum::<usize>() as f32
}

/// The item name: it sits just above the REQ block and may wrap onto two lines, so it is the
/// run of lines that follow each other closely upwards from there.
fn name_text(words: &[Word], panel: Panel) -> Option<String> {
    let mut lines = group_lines(words);
    lines.retain(|l| l.rect.center_y() < req_row_top(panel, 0) && !trim_noise(&l.text).is_empty());
    lines.sort_by(|a, b| b.rect.y.total_cmp(&a.rect.y));
    let mut edge = req_row_top(panel, 0) + panel.unit;
    let mut name: Vec<&str> = Vec::new();
    for line in &lines {
        let limit = if name.is_empty() { 3.0 } else { 0.8 } * LINE_PITCH * panel.unit;
        if edge - line.rect.bottom() > limit {
            break;
        }
        name.insert(0, trim_noise(&line.text));
        edge = line.rect.y;
    }
    (!name.is_empty()).then(|| name.join(" "))
}

/// Top of `REQ` row `index` (0 = LEV ... 5 = FAM).
fn req_row_top(panel: Panel, index: usize) -> f32 {
    panel.separator.y - (REQ_TOP - LINE_PITCH * index as f32) * panel.unit
}

/// The six `REQ` rows of one tooltip.
struct RequirementRows {
    rows: Vec<RequirementRow>,
}

/// One `REQ` value: its glyphs, and OCR's reading when OCR saw only digits.
struct RequirementRow {
    glyphs: Vec<Mask>,
    ocr: Option<String>,
}

impl RequirementRows {
    fn read(frame: &Frame, panel: Panel, ocr: &Ocr) -> Result<Self> {
        let u = panel.unit;
        let sep = panel.separator;
        let block = Rect::from_edges(
            sep.x + 90.0 * u,
            req_row_top(panel, 0) - 2.0 * u,
            sep.right(),
            req_row_top(panel, 5) + 9.0 * u,
        );
        // The pixel font reads best keyed and enlarged without smoothing.
        let words = ocr.words_in(frame, block, 2.0, |crop| {
            let keyed = preprocess::keep(crop, is_req_text);
            let (w, h) = (keyed.width() * 2, keyed.height() * 2);
            preprocess::pad(&imageops::resize(&keyed, w, h, imageops::FilterType::Nearest), 30)
        })?;
        let lines = group_lines(&words);

        let rows = (0..6)
            .map(|i| {
                let top = req_row_top(panel, i);
                let value = Rect::from_edges(sep.x + REQ_VALUE_LEFT * u, top, sep.right(), top + REQ_DIGIT_HEIGHT * u);
                let (_, y, _, h) = value.to_pixels(frame.width(), frame.height());
                let glyphs = ink_columns(frame, value, is_req_text)
                    .into_iter()
                    .map(|cols| Mask::new(frame, cols, (y, y + h), is_req_text))
                    .collect();
                // Only digits OCR actually printed count; letters it might have meant are not trusted.
                let middle = top + 0.5 * REQ_DIGIT_HEIGHT * u;
                let ocr = lines
                    .iter()
                    .find(|l| (l.rect.center_y() - middle).abs() < 0.5 * LINE_PITCH * u)
                    .and_then(|l| DIGITS.captures(&l.text).map(|c| c[1].to_string()));
                RequirementRow { glyphs, ocr }
            })
            .collect();
        Ok(Self { rows })
    }

    /// Whether every row shows one lone digit, the same one, which can only be 0: an item with
    /// no requirements. OCR reads nothing for a lone digit, so nothing else would settle these.
    fn all_zero(&self) -> bool {
        let Some(first) = self.rows.first().and_then(|row| row.glyphs.first()) else { return false };
        self.rows.iter().all(|row| {
            row.ocr.as_deref().is_none_or(|text| text == "0")
                && row.glyphs.len() == 1
                && first.similarity(&row.glyphs[0]) >= MIN_SIMILARITY
        })
    }

    /// Each row's value: OCR's when it agrees with the glyph count, otherwise the glyphs
    /// matched against `digits`.
    fn resolve(&self, digits: &GlyphSet) -> Requirements {
        if self.all_zero() {
            let zero = Some(0);
            return Requirements { level: zero, str: zero, dex: zero, int: zero, luk: zero, fam: zero };
        }
        let values: Vec<Option<u32>> = self
            .rows
            .iter()
            .map(|row| {
                // Only a UI at the size the rules read needs nothing else.
                let shape = number(&row.glyphs).filter(|t| !t.is_empty());
                let ocr = row.ocr.as_ref().filter(|t| t.chars().count() == row.glyphs.len());
                shape
                    .or_else(|| ocr.cloned())
                    .or_else(|| digits.read(&row.glyphs).filter(|t| !t.is_empty()))?
                    .parse()
                    .ok()
            })
            .collect();
        Requirements {
            level: values[0],
            str: values[1],
            dex: values[2],
            int: values[3],
            luk: values[4],
            fam: values[5],
        }
    }
}

/// The top of the light blue frame on the panel's left side, a few game pixels out from the
/// separator. The frame is translucent, so it is told apart by being lighter than the panel
/// just inside it, and short breaks (something drawn over it) are bridged.
fn frame_top(frame: &Frame, separator: Rect, unit: f32) -> Option<f32> {
    let columns = (separator.x - 7.0 * unit).max(0.0) as u32..=(separator.x - 3.0 * unit) as u32;
    let inset = (3.0 * unit) as u32;
    let on_frame = |y: u32| {
        columns.clone().any(|x| {
            let (edge, inside) = (frame.get_pixel(x, y), frame.get_pixel((x + inset).min(frame.width() - 1), y));
            let lighter = u16::from(preprocess::luminance(edge)) >= u16::from(preprocess::luminance(inside)) + 30;
            is_panel_frame(edge) && lighter
        })
    };
    let max_gap = (4.0 * unit) as u32;
    let (mut top, mut gap) = (None, 0);
    for y in (0..separator.y as u32).rev() {
        if on_frame(y) {
            (top, gap) = (Some(y), 0);
        } else {
            gap += 1;
            if gap > max_gap {
                break;
            }
        }
    }
    top.map(|y| y as f32)
}

fn parse_pass(words: &[Word], panel: Panel) -> ItemTooltip {
    let lines = group_lines(words);
    // Anything below the enhancements line is scenery.
    let end = lines.iter().position(|l| starts_like(&l.text, "Remaining")).map_or(lines.len(), |i| i + 1);
    parse::parse(&lines[..end], panel.separator.y)
}

/// The five job flags always appear in the same order, highlighted orange when the job can
/// use the item and grey otherwise. They are split at the four widest gaps in the row and
/// judged by colour, without OCR.
fn read_jobs(frame: &Frame, panel: Panel) -> Vec<JobFlag> {
    let u = panel.unit;
    let sep = panel.separator;
    let row = preprocess::crop(frame, Rect::from_edges(sep.x, sep.y - 18.0 * u, sep.right(), sep.y - 2.0 * u));

    // Per column: (text pixels, orange pixels).
    let columns: Vec<(u32, u32)> = (0..row.width())
        .map(|x| {
            (0..row.height()).fold((0, 0), |(text, orange), y| {
                let px = row.get_pixel(x, y);
                let is_orange = is_flag_orange(px);
                (text + u32::from(is_orange || is_flag_grey(px)), orange + u32::from(is_orange))
            })
        })
        .collect();

    let mut runs: Vec<(usize, usize)> = Vec::new();
    for (x, &(text, _)) in columns.iter().enumerate() {
        match runs.last_mut() {
            Some(run) if text > 0 && run.1 == x => run.1 = x + 1,
            _ if text > 0 => runs.push((x, x + 1)),
            _ => {}
        }
    }
    if runs.len() < JOBS.len() {
        return Vec::new();
    }

    // The widest gaps separate the flags; narrower ones are between letters.
    let mut gaps: Vec<(usize, usize)> = runs.windows(2).enumerate().map(|(i, w)| (w[1].0 - w[0].1, i)).collect();
    gaps.sort_by_key(|&(gap, _)| std::cmp::Reverse(gap));
    let mut splits: Vec<usize> = gaps[..JOBS.len() - 1].iter().map(|&(_, i)| i + 1).collect();
    splits.sort_unstable();

    let bounds = std::iter::once(0).chain(splits).chain(std::iter::once(runs.len()));
    let bounds: Vec<usize> = bounds.collect();
    JOBS.iter()
        .zip(bounds.windows(2))
        .map(|(&job, b)| {
            let (text, orange) =
                columns[runs[b[0]].0..runs[b[1] - 1].1].iter().fold((0, 0), |acc, c| (acc.0 + c.0, acc.1 + c.1));
            JobFlag { job, eligible: orange * 2 >= text.max(1) }
        })
        .collect()
}

/// Joins pieces of a line broken up by things drawn over it.
fn join_collinear(mut pieces: Vec<Rect>, max_gap: f32) -> Vec<Rect> {
    pieces.sort_by(|a, b| a.x.total_cmp(&b.x));
    let mut joined: Vec<Rect> = Vec::new();
    for piece in pieces {
        match joined.iter_mut().find(|r| r.vertical_overlap(&piece) > 0.0 && piece.x - r.right() <= max_gap) {
            Some(rule) => *rule = rule.union(&piece),
            None => joined.push(piece),
        }
    }
    joined
}

fn looks_like_tooltip_text(text: &str) -> bool {
    ["Type", "REQ", "Remaining"].iter().any(|w| starts_like(text, w))
        || text.split_whitespace().any(|w| best_match(w, &JOBS, 0.75).is_some())
}

/// The separator is a thin, very light line with a slight blue tint.
fn is_rule(px: &Rgba<u8>) -> bool {
    preprocess::luminance(px) >= 180 && preprocess::saturation(px) <= 80
}

/// Tooltip text is white or light grey; names of upgraded items are amber.
fn is_tooltip_text(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0;
    let light_grey = preprocess::luminance(px) >= 150 && preprocess::saturation(px) <= 70;
    let amber = r >= 200 && g >= 140 && b <= 120;
    light_grey || amber
}

/// The amber of an upgraded item's name, solid (255, 170, 0). The scenery behind a tooltip is
/// dimmed by the panel, and this close to that shade it doesn't pass for it.
fn is_amber(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0;
    r >= 235 && (150..=190).contains(&g) && b <= 40
}

/// The jobs that can use the item: bright orange.
fn is_flag_orange(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0.map(i16::from);
    r >= 200 && (60..=170).contains(&g) && r >= b + 150
}

/// Other jobs: a dull, warm grey. The blue panel showing between flags is excluded.
fn is_flag_grey(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0.map(i16::from);
    (85..=200).contains(&preprocess::luminance(px)) && preprocess::saturation(px) <= 50 && r >= b && g + 5 >= b
}

/// `REQ` text: white when required, grey when not.
fn is_req_text(px: &Rgba<u8>) -> bool {
    preprocess::luminance(px) >= 100 && preprocess::saturation(px) <= 30
}

/// The panel's light blue frame, paler where it crosses bright scenery.
fn is_panel_frame(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0.map(i16::from);
    b >= 150 && b >= r + 10 && b + 10 >= g
}
