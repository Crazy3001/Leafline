//! The Quest Helper panel: each tracked quest's title and its kill/collect counters.
//!
//! The panel is translucent with black text, so it reads well over dark maps and poorly over
//! bright, busy ones. Text is read by OCR in several passes (the panel with its contrast
//! stretched so scenery fades to white, as is, and the native-size overview), and each row
//! takes the reading most passes agree on.
//!
//! Counters are the numbers that matter most and the ones OCR reads worst: it often loses the
//! slash in `0 / 1`, which leaves `0 11`. They are drawn in a pixel font in pure black, so
//! each counter's digits are also cut out by shape. Their count rules out impossible readings,
//! and digits read cleanly elsewhere in the panel teach the shapes of the ones OCR missed.

use std::sync::LazyLock;

use image::{Rgba, imageops};
use regex::Regex;
use serde::Serialize;

use super::digits;
use super::text::{digitize, similarity, squash, starts_like, trim_noise};
use crate::error::Result;
use crate::vision::glyphs::{GlyphSet, Mask, ink_columns, ink_rows};
use crate::vision::{Frame, Line, Ocr, Rect, group_lines, preprocess};

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestReading {
    pub quests: Vec<Quest>,
    /// From the `(1/5)` in the panel header: quests shown and the most that can be tracked.
    pub page: Option<(u32, u32)>,
    pub panel: Rect,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Quest {
    pub title: String,
    pub objectives: Vec<Objective>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Objective {
    pub target: String,
    pub current: u32,
    pub required: u32,
    /// A line the quest data says is there that no reading has given a count for yet, shown
    /// without one rather than as 0.
    #[serde(skip_serializing_if = "is_false")]
    pub unread: bool,
}

fn is_false(value: &bool) -> bool {
    !value
}

/// Panel size in header text heights, measured on Classic's Quest Helper.
const WIDTH_IN_LINES: f32 = 21.0;
const HEIGHT_IN_LINES: f32 = 26.0;
/// Distance between lines, in header text heights.
const PITCH: f32 = 1.6;
/// Lines further apart than this (in pitches) are no longer part of the panel.
const MAX_GAP: f32 = 2.6;
/// Brightness (of a pixel's strongest channel) that the contrast-stretched passes turn
/// white. The text's black and its soft edges stay dark; scenery seen through the panel fades.
const STRETCH: [u8; 2] = [130, 100];

static PAGE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\(\s*(\d)\s*.?\s*(\d)\s*\)").unwrap());
/// A counter followed by its target: `0 / 30 Evil Eye Tail`, however OCR mangled the counter.
static OBJECTIVE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^((?:[0-9OoIlSB$!|\\/r ]|C[lI])*(?:[0-9OoIlSB]|C[lI]))\s+([A-Za-z].*)$").unwrap());

/// The Quest Helper's header line and the panel under it, if the overview found the header.
pub fn panel(overview: &[Line]) -> Option<(&Line, Rect)> {
    let header = overview.iter().find(|l| is_header(&l.text))?;
    let h = header.rect.h;
    let panel = Rect::from_edges(
        header.rect.x - 0.8 * h,
        header.rect.bottom(),
        header.rect.x + WIDTH_IN_LINES * h,
        header.rect.bottom() + HEIGHT_IN_LINES * h,
    );
    Some((header, panel))
}

pub fn read(frame: &Frame, overview: &[Line], ocr: &Ocr) -> Result<Option<QuestReading>> {
    read_with_digits(frame, frame, overview, ocr)
}

/// [`read`], cutting the counters' digits out of `digits`, the same view drawn to keep thin
/// strokes (see `Canonical::inked`).
pub fn read_with_digits(frame: &Frame, digits: &Frame, overview: &[Line], ocr: &Ocr) -> Result<Option<QuestReading>> {
    let Some((header, panel)) = panel(overview) else { return Ok(None) };
    let h = header.rect.h;
    const SCALE: f32 = 2.0;
    let enlarge = |img: &Frame, filter| {
        let (w, h) = ((img.width() as f32 * SCALE) as u32, (img.height() as f32 * SCALE) as u32);
        preprocess::pad(&imageops::resize(img, w, h, filter), 24)
    };
    // Only the black text, so this pass sees the same picture whatever is behind the panel.
    let keyed = ocr.words_in(frame, panel, SCALE, |crop| enlarge(&black_text(crop), imageops::FilterType::Triangle))?;
    let mut passes = vec![group_lines(&keyed)];
    for limit in STRETCH {
        let words =
            ocr.words_in(frame, panel, SCALE, |crop| enlarge(&stretch(crop, limit), imageops::FilterType::Triangle))?;
        passes.push(group_lines(&words));
    }
    passes
        .push(group_lines(&ocr.words_in(frame, panel, SCALE, |crop| enlarge(crop, imageops::FilterType::CatmullRom))?));
    passes.push(overview.iter().filter(|l| panel_contains(&panel, l)).cloned().collect());

    let page = PAGE.captures(&header.text).and_then(|c| Some((c[1].parse().ok()?, c[2].parse().ok()?)));
    let mut rows = parse_rows(&passes, header.rect.bottom(), PITCH * h);
    // Scenery and name tags under the panel can look like text to OCR, but only the panel's
    // own rows are drawn in pure black.
    rows.retain(|row| has_text(frame, panel, row.y(), h) && !on_white(frame, panel, row.y(), h));
    // OCR can lose a counter altogether, leaving `O Snail` looking like a title, but a row that
    // starts `digits / digits` as drawn is an objective, whatever is behind the panel.
    // The digits cut from the view that keeps thin strokes, where they read as a counter: there
    // an 8 keeps the bar the plain view can lose, and a 0 its top and bottom. Elsewhere the
    // plain cut-out, as OCR's readings are checked against it.
    let glyphs: Vec<Option<CounterGlyphs>> = rows
        .iter()
        .map(|row| {
            let plain = CounterGlyphs::read(frame, panel, row.y(), h)?;
            let inked = CounterGlyphs::read(digits, panel, row.y(), h).filter(|inked| inked.shaped().is_some());
            Some(inked.unwrap_or(plain))
        })
        .collect();
    for (row, glyphs) in rows.iter_mut().zip(&glyphs) {
        match (&mut *row, glyphs) {
            (Row::Title { y, title }, Some(_)) => {
                *row = Row::Objective { y: *y, target: past_counter_debris(&after_counter(title)), counters: Vec::new() };
            }
            (Row::Objective { target, .. }, Some(_)) => *target = past_counter_debris(target),
            _ => {}
        }
    }
    let mut quests = assemble(&rows, &settle_counters(&rows, &glyphs));
    if let Some((shown, _)) = page {
        quests.truncate(shown as usize);
    }
    Ok(Some(QuestReading { quests, page, panel }))
}

/// The panel's text is pure black, a shade the scenery seen through the panel never reaches.
fn is_text(px: &Rgba<u8>) -> bool {
    px.0[..3].iter().all(|&c| c <= 8)
}

/// Text and its edges. Screenshots scaled even slightly blend each stroke's edge with the
/// panel, leaving strokes only partly pure black.
fn is_ink(px: &Rgba<u8>) -> bool {
    px.0[..3].iter().all(|&c| c < 64)
}

/// Whether the row centred at `y` has pure black text about a line tall.
fn has_text(frame: &Frame, panel: Rect, y: f32, text_height: f32) -> bool {
    let band = Rect::from_edges(panel.x, y - 0.6 * text_height, panel.right(), y + 0.6 * text_height);
    ink_rows(frame, band, is_text).is_some_and(|(top, bottom)| (bottom - top) as f32 >= 0.5 * text_height)
}

/// Whether the row centred at `y` stands partly on plain white where its text starts: a speech
/// bubble or name tag below the panel, as the search area runs past its bottom. The panel is
/// see-through and tinted, so even over white clouds its ground is never plain white.
fn on_white(frame: &Frame, panel: Rect, y: f32, text_height: f32) -> bool {
    let left = Rect::from_edges(panel.x, y - 0.6 * text_height, panel.x + 0.4 * panel.w, y + 0.6 * text_height);
    let (x0, y0, w, h) = left.to_pixels(frame.width(), frame.height());
    let white = |px: &Rgba<u8>| {
        let [r, g, b, _] = px.0;
        r.min(g).min(b) >= 235 && r.max(g).max(b) - r.min(g).min(b) <= 6
    };
    let pixels = (y0..y0 + h).flat_map(|py| (x0..x0 + w).map(move |px| (px, py)));
    let plain = pixels.clone().filter(|&(px, py)| white(frame.get_pixel(px, py))).count();
    let all = pixels.count();
    // Measured: at most 5% on the panel, over a pale grey wall; 20% and up on a speech bubble.
    all > 0 && plain * 100 > all * 12
}

/// The panel's text as black on white, with everything else gone.
fn black_text(img: &Frame) -> Frame {
    Frame::from_fn(img.width(), img.height(), |x, y| {
        if is_ink(img.get_pixel(x, y)) { Rgba([0, 0, 0, 255]) } else { Rgba([255, 255, 255, 255]) }
    })
}

/// Grey image in which anything at least `limit` bright (by its strongest channel) is white
/// and darker shades are spread over the full range.
fn stretch(img: &Frame, limit: u8) -> Frame {
    Frame::from_fn(img.width(), img.height(), |x, y| {
        let [r, g, b, _] = img.get_pixel(x, y).0;
        let v = (u32::from(r.max(g).max(b)) * 255 / u32::from(limit)).min(255) as u8;
        Rgba([v, v, v, 255])
    })
}

fn panel_contains(panel: &Rect, line: &Line) -> bool {
    line.rect.x >= panel.x
        && line.rect.right() <= panel.right()
        && line.rect.y >= panel.y
        && line.rect.bottom() <= panel.bottom()
}

/// `Quest Helper`, also when OCR runs the two words together (`QuasVHelpar`).
fn is_header(text: &str) -> bool {
    let mut words = text.split_whitespace();
    let first = words.next().unwrap_or_default();
    let second = words.next().unwrap_or_default();
    if starts_like(first, "Quest") && similarity(second, "Helper") >= 0.5 {
        return true;
    }
    let joined: String = squash(text).chars().take(11).collect();
    joined.len() == 11 && similarity(&joined, "questhelper") >= 0.65
}

/// One row of the panel as OCR read it.
#[derive(Debug, Clone, PartialEq)]
enum Row {
    Title {
        y: f32,
        title: String,
    },
    /// `y` is the row's centre on screen; `counters` are the plausible `(current, required)`
    /// readings, the one most passes agree on first.
    Objective {
        y: f32,
        target: String,
        counters: Vec<(u32, u32)>,
    },
}

impl Row {
    fn y(&self) -> f32 {
        match self {
            Row::Title { y, .. } | Row::Objective { y, .. } => *y,
        }
    }
}

/// Builds the quest list from several OCR passes over the panel, best pass first, taking each
/// counter as OCR read it.
#[cfg(test)]
fn parse(passes: &[Vec<Line>], top: f32, pitch: f32) -> Vec<Quest> {
    let rows = parse_rows(passes, top, pitch);
    let counters: Vec<Option<(u32, u32)>> = rows
        .iter()
        .map(|row| match row {
            Row::Objective { counters, .. } => counters.first().copied(),
            Row::Title { .. } => None,
        })
        .collect();
    assemble(&rows, &counters)
}

/// Reads the panel's rows from several OCR passes, best pass first. Lines are matched across
/// passes by height on screen; reading stops at the first big gap, where the panel ends.
fn parse_rows(passes: &[Vec<Line>], top: f32, pitch: f32) -> Vec<Row> {
    let mut centres: Vec<f32> = Vec::new();
    for line in passes.iter().flatten() {
        let y = line.rect.center_y();
        if !centres.iter().any(|r| (r - y).abs() < 0.45 * pitch) {
            centres.push(y);
        }
    }
    centres.sort_by(f32::total_cmp);

    let mut rows = Vec::new();
    let mut previous = top;
    for centre in centres {
        if centre - previous > MAX_GAP * pitch {
            break;
        }
        previous = centre;
        let on_row = |lines: &[Line]| -> Vec<Line> {
            let mut on_row: Vec<Line> =
                lines.iter().filter(|l| (l.rect.center_y() - centre).abs() < 0.45 * pitch).cloned().collect();
            on_row.sort_by(|a, b| a.rect.x.total_cmp(&b.rect.x));
            on_row
        };
        // Each pass's text on this row, words joined left to right.
        let texts: Vec<String> = passes
            .iter()
            .map(|lines| on_row(lines).iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join(" "))
            .collect();
        // The middle of what the passes saw, which one pass's stray mark cannot pull aside.
        let mut middles: Vec<f32> = passes.iter().flat_map(|lines| on_row(lines)).map(|l| l.rect.center_y()).collect();
        middles.sort_by(f32::total_cmp);
        let y = middles[middles.len() / 2];

        if let Some((target, counters)) = objective(&texts) {
            rows.push(Row::Objective { y, target, counters });
        } else {
            let titles: Vec<String> = texts.iter().map(|t| clean_title(t)).filter(|t| t.len() >= 3).collect();
            if let Some(title) = consensus(&titles) {
                rows.push(Row::Title { y, title: title.clone() });
            }
        }
    }
    rows
}

/// Quests from rows, with each objective's settled counter. An objective whose counter could
/// not be read is left out rather than shown with made-up numbers.
fn assemble(rows: &[Row], counters: &[Option<(u32, u32)>]) -> Vec<Quest> {
    let mut quests: Vec<Quest> = Vec::new();
    for (row, counter) in rows.iter().zip(counters) {
        match row {
            Row::Title { title, .. } => quests.push(Quest { title: title.clone(), objectives: Vec::new() }),
            Row::Objective { target, .. } => {
                let Some((current, required)) = *counter else { continue };
                if quests.is_empty() {
                    quests.push(Quest::default());
                }
                let objective = Objective { target: target.clone(), current, required, unread: false };
                quests.last_mut().unwrap().objectives.push(objective);
            }
        }
    }
    quests.retain(|q| !q.title.is_empty() || !q.objectives.is_empty());
    quests
}

/// The reading closest to all the others, which outvotes one pass's garbling. Ties go to
/// the earliest, unless a reading about as close carries on where it stops: OCR often drops
/// a line's last letter.
fn consensus(readings: &[String]) -> Option<&String> {
    let support = |r: &String| readings.iter().map(|other| similarity(r, other)).sum::<f64>();
    let mut best: Option<(&String, f64)> = None;
    for reading in readings {
        let score = support(reading);
        if best.is_none_or(|(_, most)| score > most + 1e-9) {
            best = Some((reading, score));
        }
    }
    let (best, most) = best?;
    let longer = readings.iter().find(|r| {
        r.len() > best.len()
            && r.len() <= best.len() + 2
            && r.strip_prefix(best.as_str()).is_some_and(|rest| rest.chars().all(char::is_alphabetic))
            && support(r) >= most - 0.1
    });
    Some(longer.unwrap_or(best))
}

/// The objective on a row, if any pass reads one: the target the passes agree on most, and
/// every counter reading, the one most passes agree on first (the earliest pass breaking ties).
fn objective(texts: &[String]) -> Option<(String, Vec<(u32, u32)>)> {
    let parsed: Vec<(Vec<(u32, u32)>, String)> = texts
        .iter()
        .filter_map(|t| OBJECTIVE.captures(trim_noise(t)))
        .filter_map(|caps| {
            let counts = counter_readings(&caps[1]);
            // `B ringing a Mirror` is a title with its first letter split off, not a counter.
            let counter = !counts.is_empty() || caps[1].contains(|c: char| c.is_ascii_digit());
            counter.then(|| (counts, clean_target(&caps[2])))
        })
        .collect();
    let targets: Vec<String> = parsed.iter().map(|(_, target)| target.clone()).collect();
    let target = consensus(&targets)?.clone();
    let votes = |c: &(u32, u32)| parsed.iter().filter(|(counts, _)| counts.contains(c)).count();
    let mut counters: Vec<(u32, u32)> = Vec::new();
    for count in parsed.iter().flat_map(|(counts, _)| counts) {
        if !counters.contains(count) {
            counters.push(*count);
        }
    }
    // A stable sort keeps the earliest pass first among equal votes.
    counters.sort_by_key(|c| std::cmp::Reverse(votes(c)));
    Some((target, counters))
}

/// Every plausible `(current, required)` a counter could be, most likely first. OCR reads
/// the slash as `!`, `$`, `|`, `r` or `1`, or drops it, and a `0` as `Cl` or `CI`.
fn counter_readings(counter: &str) -> Vec<(u32, u32)> {
    let counter = counter.replace("Cl", "0").replace("CI", "0");
    const SLASHES: [char; 6] = ['/', '!', '|', '\\', '$', 'r'];
    // Items to use up count down to a target of 0: `1 / 0 Roger's Apple`, then `0 / 0`. Only a
    // clear slash vouches for that, since a lost digit can look the same.
    let slashed = counter.contains(SLASHES);
    let mut readings = Vec::new();
    let mut push = |current: &str, required: &str| {
        let (current, required) = (digitize(current).replace(' ', ""), digitize(required).replace(' ', ""));
        // A count never has a leading zero unless it is zero.
        let tidy = |s: &str| !s.is_empty() && (s == "0" || !s.starts_with('0'));
        if !tidy(&current) || !tidy(&required) {
            return;
        }
        if let (Ok(c), Ok(r)) = (current.parse::<u32>(), required.parse::<u32>())
            && ((r > 0 && c <= r) || (r == 0 && slashed))
            && !readings.contains(&(c, r))
        {
            readings.push((c, r));
        }
    };

    if let Some((current, required)) = counter.split_once(SLASHES) {
        push(current, required);
        return readings;
    }
    let tokens: Vec<&str> = counter.split_whitespace().collect();
    match tokens.as_slice() {
        [current, slash, required] if digitize(slash) == "1" => {
            push(current, required);
            push(current, &format!("{slash}{required}"));
        }
        [current, required] => {
            push(current, required);
            if let Some(rest) = digitize(required).strip_prefix('1') {
                push(current, rest);
            }
        }
        _ => {
            // Fused: try each 1-like character as the lost slash.
            let fused: String = tokens.concat();
            for (i, c) in fused.char_indices() {
                if i > 0 && i + 1 < fused.len() && "1lI|".contains(c) {
                    push(&fused[..i], &fused[i + 1..]);
                }
            }
        }
    }
    readings
}

/// How many word gaps a digit can stand after a 1 and still belong to the same number.
const ONE_GAP: u32 = 3;

/// A counter's digits as drawn, cut out by shape: those before the slash and those after.
struct CounterGlyphs {
    current: Vec<Mask>,
    required: Vec<Mask>,
}

impl CounterGlyphs {
    /// Cuts out the counter at the start of the row centred at `y`. `text_height` is the
    /// header's text height. `None` when the row does not start `digits / digits`.
    fn read(frame: &Frame, panel: Rect, y: f32, text_height: f32) -> Option<Self> {
        let band = Rect::from_edges(panel.x, y - 0.6 * text_height, panel.right(), y + 0.6 * text_height);
        let column = |(left, right): (u32, u32)| Rect::from_edges(left as f32, band.y, right as f32, band.bottom());
        let runs: Vec<(u32, u32)> = ink_columns(frame, band, is_ink)
            .into_iter()
            .filter(|&run| ink_rows(frame, column(run), is_text).is_some())
            .collect();
        let rows = ink_rows(frame, column(*runs.first()?), is_ink)?;
        // Letters sit about one pixel of the font apart; a space is wider.
        let word_gap = (0.22 * (rows.1 - rows.0) as f32).max(2.0) as u32;
        let mut words: Vec<Vec<(u32, u32)>> = Vec::new();
        for run in runs {
            match words.last_mut() {
                Some(word) if run.0 - word.last().unwrap().1 <= word_gap => word.push(run),
                _ => words.push(vec![run]),
            }
        }
        let mask = |cols: &(u32, u32)| Mask::new(frame, *cols, rows, is_ink);
        // A 1 is narrow in a wider cell, so the digit after it stands further off than letters do.
        let height = rows.1 - rows.0;
        let narrow = |run: (u32, u32)| (run.1 - run.0) * 2 <= height;
        // Before the slash too: `10 / 10` would otherwise cut into `1` and `0`. The slash is
        // narrow as well, so a word that is the slash joins nothing.
        if words.len() > 1 && words[0].len() == 1 && narrow(words[0][0]) && !is_slash(&mask(&words[0][0])) {
            let gap = words[1][0].0 - words[0][0].1;
            if gap <= ONE_GAP * word_gap && !is_slash(&mask(&words[1][0])) {
                let next = words.remove(1);
                words[0].extend(next);
            }
        }
        let [current, slash, required, after @ ..] = words.as_slice() else { return None };
        if slash.len() != 1 || !is_slash(&mask(&slash[0])) {
            return None;
        }
        let mut required = required.clone();
        for word in after {
            let last = required[required.len() - 1];
            if !narrow(last) || word[0].0 - last.1 > ONE_GAP * word_gap {
                break;
            }
            required.extend(word);
        }
        // At Default Ratio the target's first letter can stand as close to the last digit as
        // digits stand to each other (`10Slime`), so the count ends at the first shape that is
        // no digit.
        let mut required: Vec<Mask> = required.iter().map(mask).collect();
        let digits = required.iter().take_while(|m| digits::quest_number(std::slice::from_ref(*m)).is_some()).count();
        if digits > 0 {
            required.truncate(digits);
        }
        Some(Self { current: current.iter().map(mask).collect(), required })
    }

    fn fits(&self, (current, required): (u32, u32)) -> bool {
        current.to_string().len() == self.current.len() && required.to_string().len() == self.required.len()
    }

    /// The counter read from the digits' shapes, which the scenery behind them can't change.
    fn shaped(&self) -> Option<(u32, u32)> {
        let number = |masks: &[Mask]| digits::quest_number(masks)?.parse().ok();
        Some((number(&self.current)?, number(&self.required)?))
    }
}

/// A target with OCR's leftovers of the counter drawn before it taken off: in `g '10 Slime` the
/// `9 /` became letters, so it reads as part of the name. Words up to the last of the first three
/// holding a digit go; a name has none there.
fn past_counter_debris(target: &str) -> String {
    let words: Vec<&str> = target.split_whitespace().collect();
    match words.iter().take(3).rposition(|w| w.contains(|c: char| c.is_ascii_digit())) {
        Some(last) if last + 1 < words.len() => words[last + 1..].join(" "),
        _ => target.to_string(),
    }
}

/// What a row's text names once what is left of its counter is taken off the front.
fn after_counter(text: &str) -> String {
    let debris = |word: &str| word.chars().all(|c| c.is_ascii_digit() || "OoIlSB/!|$".contains(c));
    match text.split_once(' ') {
        Some((first, rest)) if debris(first) => clean_target(rest),
        _ => clean_target(text),
    }
}

/// A stroke rising from bottom left to top right: ink only in the right half of its top row
/// and only in the left half of its bottom row.
fn is_slash(mask: &Mask) -> bool {
    let (w, h) = (mask.width(), mask.height());
    let inked = |y: u32, xs: std::ops::Range<u32>| xs.into_iter().any(|x| mask.get(x, y));
    let top = (0..h).find(|&y| inked(y, 0..w));
    let bottom = (0..h).rev().find(|&y| inked(y, 0..w));
    match (top, bottom) {
        (Some(top), Some(bottom)) if bottom > top && w >= 2 => {
            !inked(top, 0..w / 2)
                && inked(top, w / 2..w)
                && inked(bottom, 0..w.div_ceil(2))
                && !inked(bottom, w.div_ceil(2)..w)
        }
        _ => false,
    }
}

/// Each objective's counter: the most agreed OCR reading with as many digits as are drawn.
/// Readings that fit teach a digit shape set, which then reads the counters no OCR reading
/// fits. Without glyphs (the row could not be cut out) the most agreed reading stands.
fn settle_counters(rows: &[Row], glyphs: &[Option<CounterGlyphs>]) -> Vec<Option<(u32, u32)>> {
    let mut settled: Vec<Option<(u32, u32)>> = rows
        .iter()
        .zip(glyphs)
        .map(|(row, glyphs)| {
            let Row::Objective { counters, .. } = row else { return None };
            match glyphs {
                Some(glyphs) => glyphs.shaped().or_else(|| counters.iter().copied().find(|&c| glyphs.fits(c))),
                None => counters.first().copied(),
            }
        })
        .collect();

    let mut digits = GlyphSet::default();
    for (glyphs, counter) in glyphs.iter().zip(&settled) {
        if let (Some(glyphs), Some((current, required))) = (glyphs, counter) {
            digits.learn(&glyphs.current, &current.to_string());
            digits.learn(&glyphs.required, &required.to_string());
        }
    }
    for ((row, glyphs), counter) in rows.iter().zip(glyphs).zip(&mut settled) {
        let (Row::Objective { counters, .. }, Some(glyphs), None) = (row, glyphs, &counter) else { continue };
        let read = |masks: &[Mask]| digits.read(masks).filter(|t| !t.is_empty())?.parse::<u32>().ok();
        *counter = match (read(&glyphs.current), read(&glyphs.required)) {
            (Some(current), Some(required)) if required > 0 && current <= required => Some((current, required)),
            _ => counters.first().copied(),
        };
    }
    settled
}

/// How OCR reads the close button at the end of a title row.
const CLOSE_BUTTON: [&str; 12] = ["x", "X", "o", "O", "0", "@", "C", "El", "13", "CI", "Cl", "IZI"];

/// Drops a slash OCR misread as a letter and left at the start of the target (`r Maple Leaf`).
fn clean_target(text: &str) -> String {
    let text = text.trim();
    match text.split_once(' ') {
        Some((first, rest)) if ["r", "l", "I", "!", "$", "|", "/"].contains(&first) => rest.trim().to_string(),
        _ => text.to_string(),
    }
}

/// Removes stray marks and the close button OCR picks up at the end of a title, keeping the
/// brackets and ellipsis titles really have, and fixes a `]` read as `l`.
fn clean_title(text: &str) -> String {
    let mut title = text.trim_matches(|c: char| !c.is_alphanumeric() && !"[].'".contains(c)).to_string();
    while let Some((rest, last)) = title.rsplit_once(' ')
        && CLOSE_BUTTON.contains(&last)
    {
        title = rest.trim_end().to_string();
    }
    if title.starts_with('[')
        && !title.contains(']')
        && let Some(end) = title.find("l ")
    {
        title.replace_range(end..end + 1, "]");
    } else if title.starts_with('[')
        && !title.contains(']')
        && let Some(end) = title
            .char_indices()
            .position(|(i, c)| c == 'l' && title[i + 1..].chars().next().is_some_and(char::is_uppercase))
    {
        // `[EventlThe`: the bracket and the space after it both lost.
        title.replace_range(end..end + 1, "] ");
    }
    title
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vision::Word;

    fn line(text: &str, y: f32) -> Line {
        let rect = Rect::new(0.0, y, 100.0, 20.0);
        Line { text: text.into(), rect, words: vec![Word { text: text.into(), rect }] }
    }

    fn objective(target: &str, current: u32, required: u32) -> Objective {
        Objective { target: target.into(), current, required, unread: false }
    }

    #[test]
    fn several_quests_across_passes() {
        let plain = vec![
            line("Cutthroat Manny's Request x", 50.0),
            line("O r 30 Evil Eye Tail", 82.0),
            line("[Eventl Halloween Candies", 146.0),
            line("55 1 00 Halloween Candies", 178.0),
            line("A rainbow snail shell that makes... O", 242.0),
            line("Cl 11 Rainbow-colored Snail Shell", 274.0),
            line("Some scenery far below", 600.0),
        ];
        let native = vec![line("O $1 Rainbow-colorad Snail Shall", 275.0)];
        let quests = parse(&[plain, native], 30.0, 32.0);
        assert_eq!(
            quests,
            [
                Quest {
                    title: "Cutthroat Manny's Request".into(),
                    objectives: vec![objective("Evil Eye Tail", 0, 30)]
                },
                Quest {
                    title: "[Event] Halloween Candies".into(),
                    objectives: vec![objective("Halloween Candies", 55, 100)]
                },
                Quest {
                    title: "A rainbow snail shell that makes...".into(),
                    objectives: vec![objective("Rainbow-colored Snail Shell", 0, 1)],
                },
            ]
        );
    }

    #[test]
    fn a_title_starting_like_a_digit_stays_a_title() {
        let plain = vec![line("Bringing a Mirror to Heena", 50.0), line("1 $1 Sera's Mirror", 82.0)];
        let split = vec![line("B ringing a Mirror to Heena", 50.0)];
        assert_eq!(
            parse(&[plain, split], 30.0, 32.0),
            [Quest { title: "Bringing a Mirror to Heena".into(), objectives: vec![objective("Sera's Mirror", 1, 1)] }]
        );
        // A counter OCR read past its target is still a counter, for the shapes to settle.
        assert_eq!(parse(&[vec![line("31 / 30 Evil Eye", 50.0)]], 30.0, 32.0), Vec::<Quest>::new());
    }

    #[test]
    fn counter_leftovers_come_off_a_target() {
        assert_eq!(past_counter_debris("g '10 Slime"), "Slime");
        assert_eq!(past_counter_debris("A rainbow snail shell"), "A rainbow snail shell");
        assert_eq!(past_counter_debris("Slime 81413171"), "Slime 81413171", "digits after the name are noise for later");
        assert_eq!(past_counter_debris("8 10"), "8 10", "nothing left to name");
    }

    #[test]
    fn counters() {
        assert_eq!(counter_readings("0 / 30"), [(0, 30)]);
        assert_eq!(counter_readings("O ! 30"), [(0, 30)]);
        assert_eq!(counter_readings("55 1 00"), [(55, 100)]);
        assert_eq!(counter_readings("0 11"), [(0, 11), (0, 1)]);
        assert_eq!(counter_readings("O 30"), [(0, 30)]);
        assert_eq!(counter_readings("O r 30"), [(0, 30)]);
        assert_eq!(counter_readings("Cl $1"), [(0, 1)]);
        assert_eq!(counter_readings("55 1 CICI"), [(55, 100)]);
        assert_eq!(counter_readings("0130"), [(0, 30)]);
        assert_eq!(counter_readings("12150"), [(12, 50)]);
        assert!(counter_readings("31 / 30").is_empty());
    }

    #[test]
    fn a_counter_can_count_down_to_zero() {
        assert_eq!(counter_readings("1 / 0"), [(1, 0)]);
        assert_eq!(counter_readings("0 / 0"), [(0, 0)]);
        assert_eq!(counter_readings("O ! O"), [(0, 0)]);
        // Without a slash, `1 0` may be a counter that lost one.
        assert!(!counter_readings("1 0").contains(&(1, 0)));

        let rows = [line("Roger's Apple", 50.0), line("O / O Roger's Apple", 82.0)];
        assert_eq!(
            parse(&[rows.to_vec()], 30.0, 32.0),
            [Quest { title: "Roger's Apple".into(), objectives: vec![objective("Roger's Apple", 0, 0)] }]
        );
    }

    #[test]
    fn what_is_left_of_a_counter_comes_off_the_target() {
        assert_eq!(after_counter("O Snail"), "Snail");
        assert_eq!(after_counter("0 Blue Snail"), "Blue Snail");
        assert_eq!(after_counter("Snail"), "Snail");
        assert_eq!(after_counter("Old Snail"), "Old Snail");
    }

    #[test]
    fn titles_and_targets_lose_ocr_debris() {
        assert_eq!(clean_title("A rainbow snail shell that makes... C"), "A rainbow snail shell that makes...");
        assert_eq!(clean_title("[Eventl The Journey Begins"), "[Event] The Journey Begins");
        assert_eq!(clean_title("[EventlThe Journey Begins"), "[Event] The Journey Begins");
        assert_eq!(clean_target("r Rainbow-colored Snail Shell"), "Rainbow-colored Snail Shell");
        assert_eq!(clean_target("Evil Eye Tail"), "Evil Eye Tail");
        let readings = ["Halloween Candie".to_string(), "Halloween Candies".to_string()];
        assert_eq!(consensus(&readings).unwrap(), "Halloween Candies");
        let readings = ["Halloween Candies".to_string(), "Halloween Candies 1".to_string()];
        assert_eq!(consensus(&readings).unwrap(), "Halloween Candies");
    }

    #[test]
    fn header_tolerates_ocr_noise() {
        assert!(is_header("Ouest Helpor (1/5)"));
        assert!(is_header("QuasVHelpar (5E)"));
        assert!(!is_header("Questionable things"));
    }

    const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);
    const PANEL: Rgba<u8> = Rgba([200, 200, 190, 255]);

    /// Made-up 3×5 digits, a slash and a letter, drawn at 2× like the game's UI.
    fn glyph(c: char) -> [&'static str; 5] {
        match c {
            '0' => ["###", "#.#", "#.#", "#.#", "###"],
            '1' => [".#.", "##.", ".#.", ".#.", "###"],
            '3' => ["###", "..#", "###", "..#", "###"],
            '/' => ["..#", "..#", ".#.", "#..", "#.."],
            _ => ["#.#", "#.#", "###", "#.#", "#.#"],
        }
    }

    /// A panel strip with `text` drawn in it: one blank font pixel between letters, three for
    /// a space.
    fn strip(text: &str) -> Frame {
        let mut columns: Vec<[bool; 5]> = vec![[false; 5]; 2];
        for c in text.chars() {
            if c == ' ' {
                columns.extend([[false; 5]; 2]);
                continue;
            }
            let rows = glyph(c);
            for x in 0..3 {
                columns.push(std::array::from_fn(|y| rows[y].as_bytes()[x] == b'#'));
            }
            columns.push([false; 5]);
        }
        Frame::from_fn(2 * columns.len() as u32 + 20, 20, |x, y| {
            let (col, row) = (x as usize / 2, (y as usize).wrapping_sub(5) / 2);
            if row < 5 && col < columns.len() && columns[col][row] { BLACK } else { PANEL }
        })
    }

    fn glyphs_of(frame: &Frame) -> Option<CounterGlyphs> {
        let panel = Rect::new(0.0, 0.0, frame.width() as f32, frame.height() as f32);
        CounterGlyphs::read(frame, panel, 10.0, 10.0)
    }

    #[test]
    fn cuts_counters_by_shape() {
        let counter = glyphs_of(&strip("0 / 30 HH")).unwrap();
        assert_eq!((counter.current.len(), counter.required.len()), (1, 2));
        assert!(counter.fits((0, 30)));
        assert!(!counter.fits((0, 3)));
        assert!(glyphs_of(&strip("HHH HH")).is_none());
    }

    #[test]
    fn digit_count_and_learned_shapes_settle_counters() {
        let objective = |y: f32, counters: Vec<(u32, u32)>| Row::Objective { y, target: "Mob".into(), counters };
        let rows = vec![
            Row::Title { y: 0.0, title: "A quest".into() },
            objective(1.0, vec![(10, 30)]),
            // OCR lost the slash: `0 11` could be 0/11 or 0/1, and only one digit is drawn.
            objective(2.0, vec![(0, 11), (0, 1)]),
            // OCR read nothing that fits; the shapes learned above read it.
            objective(3.0, vec![(1, 300)]),
        ];
        let glyphs =
            vec![None, glyphs_of(&strip("10 / 30 HH")), glyphs_of(&strip("0 / 1 HH")), glyphs_of(&strip("3 / 13 HH"))];
        assert_eq!(settle_counters(&rows, &glyphs), [None, Some((10, 30)), Some((0, 1)), Some((3, 13))]);
    }
}
