//! The Quest Helper panel: each tracked quest's title and its kill/collect counters.
//!
//! The panel is translucent with black text, so it reads well over dark maps and poorly over
//! bright, busy ones. It is read in three ways (enlarged as is, enlarged with only dark text
//! kept, and the native-size overview), and each counter takes the reading most passes agree
//! on. That matters because OCR often loses the slash in `0 / 1`, which leaves `0 11`.

use std::sync::LazyLock;

use image::imageops;
use regex::Regex;
use serde::Serialize;

use super::text::{digitize, similarity, starts_like, trim_noise};
use crate::error::Result;
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
}

/// Panel size in header text heights, measured on Classic's Quest Helper.
const WIDTH_IN_LINES: f32 = 21.0;
const HEIGHT_IN_LINES: f32 = 26.0;
/// Distance between lines, in header text heights.
const PITCH: f32 = 1.6;
/// Lines further apart than this (in pitches) are no longer part of the panel.
const MAX_GAP: f32 = 2.6;
/// Below this luminance a pixel counts as the panel's black text.
const DARK_TEXT: u8 = 50;

static PAGE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\(\s*(\d)\s*.?\s*(\d)\s*\)").unwrap());
/// A counter followed by its target: `0 / 30 Evil Eye Tail`, however OCR mangled the counter.
static OBJECTIVE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^((?:[0-9OoIlSB$!|\\/r ]|Cl)*[0-9OoIlSB])\s+([A-Za-z].*)$").unwrap());

pub fn read(frame: &Frame, overview: &[Line], ocr: &Ocr) -> Result<Option<QuestReading>> {
    let Some(header) = overview.iter().find(|l| is_header(&l.text)) else { return Ok(None) };
    let h = header.rect.h;
    let panel = Rect::from_edges(
        header.rect.x - 0.8 * h,
        header.rect.bottom(),
        header.rect.x + WIDTH_IN_LINES * h,
        header.rect.bottom() + HEIGHT_IN_LINES * h,
    );
    const SCALE: f32 = 2.0;
    let enlarge = |img: &Frame| {
        let (w, h) = ((img.width() as f32 * SCALE) as u32, (img.height() as f32 * SCALE) as u32);
        preprocess::pad(&imageops::resize(img, w, h, imageops::FilterType::CatmullRom), 24)
    };
    let plain = group_lines(&ocr.words_in(frame, panel, SCALE, enlarge)?);
    let dark = group_lines(&ocr.words_in(frame, panel, SCALE, |crop| {
        enlarge(&preprocess::keep(crop, |p| preprocess::luminance(p) <= DARK_TEXT))
    })?);
    let native: Vec<Line> = overview.iter().filter(|l| panel_contains(&panel, l)).cloned().collect();

    let page = PAGE.captures(&header.text).and_then(|c| Some((c[1].parse().ok()?, c[2].parse().ok()?)));
    let mut quests = parse(&[plain, dark, native], header.rect.bottom(), PITCH * h);
    if let Some((shown, _)) = page {
        quests.truncate(shown as usize);
    }
    Ok(Some(QuestReading { quests, page, panel }))
}

fn panel_contains(panel: &Rect, line: &Line) -> bool {
    line.rect.x >= panel.x
        && line.rect.right() <= panel.right()
        && line.rect.y >= panel.y
        && line.rect.bottom() <= panel.bottom()
}

fn is_header(text: &str) -> bool {
    let mut words = text.split_whitespace();
    let first = words.next().unwrap_or_default();
    let second = words.next().unwrap_or_default();
    starts_like(first, "Quest") && similarity(second, "Helper") >= 0.5
}

/// Builds the quest list from several OCR passes over the panel, best pass first. Lines are
/// matched across passes by height on screen; reading stops at the first big gap, where the
/// panel ends.
pub fn parse(passes: &[Vec<Line>], top: f32, pitch: f32) -> Vec<Quest> {
    let mut rows: Vec<f32> = Vec::new();
    for line in passes.iter().flatten() {
        let y = line.rect.center_y();
        if !rows.iter().any(|r| (r - y).abs() < 0.45 * pitch) {
            rows.push(y);
        }
    }
    rows.sort_by(f32::total_cmp);

    let mut quests: Vec<Quest> = Vec::new();
    let mut previous = top;
    for row in rows {
        if row - previous > MAX_GAP * pitch {
            break;
        }
        previous = row;
        // Each pass's text on this row, words joined left to right.
        let texts: Vec<String> = passes
            .iter()
            .map(|lines| {
                let mut on_row: Vec<&Line> =
                    lines.iter().filter(|l| (l.rect.center_y() - row).abs() < 0.45 * pitch).collect();
                on_row.sort_by(|a, b| a.rect.x.total_cmp(&b.rect.x));
                on_row.iter().map(|l| l.text.as_str()).collect::<Vec<_>>().join(" ")
            })
            .collect();

        match objective(&texts) {
            Some(objective) => {
                if quests.is_empty() {
                    quests.push(Quest::default());
                }
                quests.last_mut().unwrap().objectives.push(objective);
            }
            None => {
                let titles: Vec<String> = texts.iter().map(|t| clean_title(t)).filter(|t| t.len() >= 4).collect();
                if let Some(title) = consensus(&titles) {
                    quests.push(Quest { title: title.clone(), objectives: Vec::new() });
                }
            }
        }
    }
    quests.retain(|q| !q.title.is_empty() || !q.objectives.is_empty());
    quests
}

/// The reading closest to all the others, which outvotes one pass's garbling. Ties go to
/// the earliest.
fn consensus(readings: &[String]) -> Option<&String> {
    let support = |r: &String| readings.iter().map(|other| similarity(r, other)).sum::<f64>();
    let mut best: Option<(&String, f64)> = None;
    for reading in readings {
        let score = support(reading);
        if best.is_none_or(|(_, most)| score > most + 1e-9) {
            best = Some((reading, score));
        }
    }
    best.map(|(reading, _)| reading)
}

/// The objective on a row, if any pass reads one: the counter most passes agree on (the
/// earliest pass breaking ties), and the target the passes agree on most.
fn objective(texts: &[String]) -> Option<Objective> {
    let parsed: Vec<(Vec<(u32, u32)>, String)> = texts
        .iter()
        .filter_map(|t| OBJECTIVE.captures(trim_noise(t)))
        .map(|caps| (counter_readings(&caps[1]), caps[2].trim().to_string()))
        .collect();
    let targets: Vec<String> = parsed.iter().map(|(_, target)| target.clone()).collect();
    let target = consensus(&targets)?.clone();
    let votes = |c: &(u32, u32)| parsed.iter().filter(|(counts, _)| counts.contains(c)).count();
    let mut best: Option<((u32, u32), usize)> = None;
    for count in parsed.iter().flat_map(|(counts, _)| counts) {
        let n = votes(count);
        if best.is_none_or(|(_, most)| n > most) {
            best = Some((*count, n));
        }
    }
    let ((current, required), _) = best?;
    Some(Objective { target, current, required })
}

/// Every plausible `(current, required)` a counter could be, most likely first. OCR reads
/// the slash as `!`, `$`, `|`, `r` or `1`, or drops it, and a `0` as `Cl`.
fn counter_readings(counter: &str) -> Vec<(u32, u32)> {
    let counter = counter.replace("Cl", "0");
    let mut readings = Vec::new();
    let mut push = |current: &str, required: &str| {
        let (current, required) = (digitize(current).replace(' ', ""), digitize(required).replace(' ', ""));
        // A count never has a leading zero unless it is zero.
        let tidy = |s: &str| !s.is_empty() && (s == "0" || !s.starts_with('0'));
        if !tidy(&current) || !tidy(&required) {
            return;
        }
        if let (Ok(c), Ok(r)) = (current.parse::<u32>(), required.parse::<u32>())
            && r > 0
            && c <= r
            && !readings.contains(&(c, r))
        {
            readings.push((c, r));
        }
    };

    if let Some((current, required)) = counter.split_once(['/', '!', '|', '\\', '$', 'r']) {
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

/// Removes stray marks and the close button OCR picks up at the end of a title, keeping the
/// brackets and ellipsis titles really have, and fixes a `]` read as `l`.
fn clean_title(text: &str) -> String {
    let mut title = text.trim_matches(|c: char| !c.is_alphanumeric() && !"[].'".contains(c)).to_string();
    while let Some((rest, last)) = title.rsplit_once(' ')
        && last.chars().count() == 1
        && "xXoO0".contains(last)
    {
        title = rest.trim_end().to_string();
    }
    if title.starts_with('[')
        && !title.contains(']')
        && let Some(end) = title.find("l ")
    {
        title.replace_range(end..end + 1, "]");
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
        Objective { target: target.into(), current, required }
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
    fn counters() {
        assert_eq!(counter_readings("0 / 30"), [(0, 30)]);
        assert_eq!(counter_readings("O ! 30"), [(0, 30)]);
        assert_eq!(counter_readings("55 1 00"), [(55, 100)]);
        assert_eq!(counter_readings("0 11"), [(0, 11), (0, 1)]);
        assert_eq!(counter_readings("O 30"), [(0, 30)]);
        assert_eq!(counter_readings("O r 30"), [(0, 30)]);
        assert_eq!(counter_readings("Cl $1"), [(0, 1)]);
        assert_eq!(counter_readings("0130"), [(0, 30)]);
        assert_eq!(counter_readings("12150"), [(12, 50)]);
        assert!(counter_readings("31 / 30").is_empty());
    }

    #[test]
    fn header_tolerates_ocr_noise() {
        assert!(is_header("Ouest Helpor (1/5)"));
        assert!(!is_header("Questionable things"));
    }
}
