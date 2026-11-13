//! The Character Stat and Skill Inventory windows, so the build planner can start from the
//! character as it really is. The stat window's detail pane gives the attack range the training
//! estimates count monsters' defence against. Live pictures are checked for them with [`glance`], which reads no
//! text; the hotkey and a change that has settled read them in full with [`read`].
//!
//! Windows can be dragged anywhere, so each is found by appearance: the stat window by its four
//! green stat labels, the skill window by the pale strips of its rows. Everything else in them
//! sits a fixed distance from those. Numbers are read by the shape rules in [`digits`], and skill
//! names go through OCR, which reads that font well.

use image::Rgba;
use serde::Serialize;

use super::digits;
use super::hud::HudLayout;
use super::ui::{self, CANONICAL_SCALE, Canonical};
use crate::error::Result;
use crate::vision::glyphs::{Mask, ink_columns, ink_rows};
use crate::vision::{Frame, Ocr, Rect, color, group_lines, preprocess};

// All distances are in pixels at the size the digits are read at.

/// Stat labels stand this far apart.
const STAT_PITCH: f32 = 36.0;
const STAT_LABELS: usize = 4;
/// From the top of the STR label up to the top of the LEVEL label.
const LEVEL_ABOVE_STR: f32 = 192.0;
/// From a label's left edge to where its value can start, and how far values run.
const VALUE_LEFT: f32 = 96.0;
const VALUE_WIDTH: f32 = 170.0;
/// The detail pane's labels stand at the same pitch; its values start this far from a label's left
/// edge, and run this far.
const DETAIL_VALUE_LEFT: f32 = 122.0;
const DETAIL_VALUE_WIDTH: f32 = 200.0;
/// The ABILITY POINT box, from the LUK label.
const AP_BOX: (f32, f32, f32, f32) = (158.0, 60.0, 52.0, 26.0);
/// A skill's level strip stands this far below its name strip, and rows this far apart.
const LEVEL_BELOW_NAME: f32 = 36.0;
const SKILL_PITCH: f32 = 80.0;
/// How far a level can run along its strip, short of the buttons.
const SKILL_LEVEL_WIDTH: f32 = 150.0;
/// The SKILL POINT box, from the first skill's name strip.
const SP_BOX: (f32, f32, f32, f32) = (80.0, 502.0, 54.0, 26.0);
/// How far two things meant to line up can be off.
const SLACK: f32 = 3.0;
/// Skill names are enlarged and given a margin, which OCR needs on a strip this thin.
const NAME_SCALE: f32 = 2.0;
const NAME_PAD: u32 = 24;
/// Digits of one number stand this far apart at most, the widest being after a 1, whose cell
/// has blank space to its right. A space is wider.
const NUMBER_GAP: u32 = 8;
/// A label or strip is at least this wide once its text is left out.
const MIN_LABEL_RUN: u32 = 30;
const MIN_STRIP_RUN: u32 = 120;

/// Every this many pixels across and down, a picture is checked for the windows' colours. The
/// labels and strips are wider and taller than this, so an open window can't be missed.
const LOOK_STEP: usize = 8;

/// One read of what the windows show. A window that isn't open, or doesn't read cleanly, is `None`.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterReading {
    pub stats: Option<StatWindow>,
    pub skills: Option<SkillWindow>,
    /// The stat window's detail pane, when it is open.
    pub detail: Option<DetailWindow>,
}

/// What the detail pane shows that the plans use.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailWindow {
    /// The Attack line, `107 - 289`.
    pub attack_min: u32,
    pub attack_max: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatWindow {
    pub level: u32,
    /// Each stat's own points: the number inside the brackets of `30 (28+2)` when gear adds to it.
    pub str: u32,
    pub dex: u32,
    pub int: u32,
    pub luk: u32,
    /// What gear adds to each, the `+2` of `30 (28+2)`. Zero when it adds nothing or didn't read.
    pub bonus: Bonus,
    /// Unspent AP.
    pub ability_points: u32,
}

/// What worn gear adds to each stat.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bonus {
    pub str: u32,
    pub dex: u32,
    pub int: u32,
    pub luk: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillWindow {
    /// The rows of the tab in view, as far as it is scrolled.
    pub rows: Vec<SkillRow>,
    /// Unspent SP.
    pub skill_points: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkillRow {
    pub name: String,
    pub level: u32,
}

/// The numbers in the open windows and a measure of each skill's name, enough to tell that
/// something changed without any OCR.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Glance {
    level: Option<u32>,
    stats: Option<StatWindow>,
    skills: Vec<(Option<u32>, usize)>,
    skill_points: Option<u32>,
    detail: Option<DetailWindow>,
}

/// Decides when a change in the windows is worth a full read: once it has shown the same in two
/// pictures, which skips a window being dragged, and only the once.
#[derive(Debug, Default)]
pub struct Watch {
    last: Option<Glance>,
    read: Option<Glance>,
}

impl Watch {
    /// Takes in this picture's glance, `None` with no window open, and says whether to read.
    pub fn settled(&mut self, glance: Option<Glance>) -> bool {
        let settled = glance.is_some() && glance == self.last && glance != self.read;
        if settled {
            self.read.clone_from(&glance);
        }
        self.last = glance;
        settled
    }
}

/// What the open windows show at a glance, or `None` with neither open. `level` is the HUD's,
/// since a level gained is a change too.
pub fn glance(frame: &Frame, hud: &HudLayout, level: Option<u32>) -> Option<Glance> {
    if !any_window(frame) {
        return None;
    }
    let scale = ui::hud_scale(frame, hud);
    let canonical = ((scale - CANONICAL_SCALE).abs() > 0.005).then(|| Canonical::new(frame, scale).frame);
    let view = canonical.as_ref().unwrap_or(frame);
    let stats = stat_labels(view).and_then(|labels| read_stats(view, &labels));
    let strips = level_strips(view);
    let skills: Vec<(Option<u32>, usize)> = strips
        .iter()
        .map(|strip| (number_in(view, level_area(strip)), ink_in(view, name_area(strip))))
        .collect();
    let skill_points = strips.first().and_then(|first| number_in(view, sp_area(first)));
    let detail = read_detail(view);
    (stats.is_some() || !skills.is_empty() || detail.is_some()).then_some(Glance { level, stats, skills, skill_points, detail })
}

/// Whether anything in `frame` has the colours of the windows.
fn any_window(frame: &Frame) -> bool {
    (0..frame.height()).step_by(LOOK_STEP).any(|y| {
        (0..frame.width()).step_by(LOOK_STEP).any(|x| {
            let px = frame.get_pixel(x, y);
            is_stat_label(px) || is_skill_strip(px) || is_detail_label(px)
        })
    })
}

fn ink_in(view: &Frame, area: Rect) -> usize {
    let (x, y, w, h) = area.to_pixels(view.width(), view.height());
    (y..y + h).flat_map(|row| (x..x + w).map(move |col| (col, row))).filter(|&(col, row)| is_ink(view.get_pixel(col, row))).count()
}

fn level_area(strip: &Rect) -> Rect {
    Rect::new(strip.x, strip.y, SKILL_LEVEL_WIDTH, strip.h)
}

fn name_area(strip: &Rect) -> Rect {
    Rect::new(strip.x, strip.y - LEVEL_BELOW_NAME, strip.w, strip.h)
}

fn sp_area(first: &Rect) -> Rect {
    let (x, y, w, h) = SP_BOX;
    Rect::new(first.x + x, first.y - LEVEL_BELOW_NAME + y, w, h)
}

/// Reads whichever of the two windows are open.
pub fn read(frame: &Frame, hud: Option<&HudLayout>, ocr: &Ocr) -> Result<CharacterReading> {
    if let Some(reading) = read_view(frame, ocr)? {
        return Ok(reading);
    }
    // At another UI size, put the picture back on the grid the distances are measured on.
    for scale in ui::scales(frame, hud) {
        if (scale - CANONICAL_SCALE).abs() > 0.005
            && let Some(reading) = read_view(&Canonical::new(frame, scale).frame, ocr)?
        {
            return Ok(reading);
        }
    }
    Ok(CharacterReading::default())
}

fn read_view(view: &Frame, ocr: &Ocr) -> Result<Option<CharacterReading>> {
    let stats = stat_labels(view).and_then(|labels| read_stats(view, &labels));
    let skills = read_skills(view, ocr)?;
    let detail = read_detail(view);
    Ok((stats.is_some() || skills.is_some() || detail.is_some()).then_some(CharacterReading { stats, skills, detail }))
}

fn whole(frame: &Frame) -> Rect {
    Rect::new(0.0, 0.0, frame.width() as f32, frame.height() as f32)
}

fn near(px: &Rgba<u8>, colour: [u8; 3], tolerance: u8) -> bool {
    px.0.iter().zip(colour).all(|(a, b)| a.abs_diff(b) <= tolerance)
}

fn is_stat_label(px: &Rgba<u8>) -> bool {
    near(px, [184, 199, 77], 6)
}

fn is_detail_label(px: &Rgba<u8>) -> bool {
    near(px, [90, 136, 182], 6)
}

fn is_skill_strip(px: &Rgba<u8>) -> bool {
    near(px, [205, 217, 227], 4)
}

/// The windows' black text.
fn is_ink(px: &Rgba<u8>) -> bool {
    px.0[..3].iter().all(|&c| c < 120)
}

/// The STR, DEX, INT and LUK labels, top to bottom: four of a size, stacked at the right pitch.
fn stat_labels(view: &Frame) -> Option<[Rect; STAT_LABELS]> {
    stacked_labels(view, is_stat_label, false)
}

/// Four labels of a colour, of a size, stacked at the right pitch: the first is the top one.
/// With `broken`, the labels under the first may show only a sliver, since text across them
/// splits their fill (the detail pane's do); the first must still be whole.
fn stacked_labels(view: &Frame, is_label: fn(&Rgba<u8>) -> bool, broken: bool) -> Option<[Rect; STAT_LABELS]> {
    let mut blocks = color::find_blocks(view, whole(view), MIN_LABEL_RUN, is_label);
    blocks.retain(|b| b.h <= 36.0 && (broken || b.h >= 20.0));
    blocks.sort_by(|a, b| a.y.total_cmp(&b.y));
    blocks.iter().filter(|b| b.h >= 20.0).find_map(|first| {
        let at = |i: usize| {
            let y = first.y + i as f32 * STAT_PITCH;
            blocks.iter().find(|b| (b.x - first.x).abs() <= SLACK && (b.y - y).abs() <= SLACK).copied()
        };
        Some([at(0)?, at(1)?, at(2)?, at(3)?])
    })
}

/// The Attack line of the detail pane: its blue labels stand four or more in a stack, ATTACK on top.
fn read_detail(view: &Frame) -> Option<DetailWindow> {
    let attack = stacked_labels(view, is_detail_label, true)?[0];
    let (attack_min, attack_max) = range_in(
        view,
        Rect::new(attack.x + DETAIL_VALUE_LEFT, attack.y - 2.0, DETAIL_VALUE_WIDTH, attack.h + 4.0),
    )?;
    (attack_min <= attack_max).then_some(DetailWindow { attack_min, attack_max })
}

fn read_stats(view: &Frame, labels: &[Rect; STAT_LABELS]) -> Option<StatWindow> {
    let row = |label: &Rect, top: f32| Rect::new(label.x + VALUE_LEFT, top - 2.0, VALUE_WIDTH, label.h + 4.0);
    let value = |label: &Rect| stat_in(view, row(label, label.y));
    let [str_label, dex_label, int_label, luk_label] = labels;
    let (x, y, w, h) = AP_BOX;
    let (str, str_gear) = value(str_label)?;
    let (dex, dex_gear) = value(dex_label)?;
    let (int, int_gear) = value(int_label)?;
    let (luk, luk_gear) = value(luk_label)?;
    Some(StatWindow {
        level: number_in(view, row(str_label, str_label.y - LEVEL_ABOVE_STR))?,
        str,
        dex,
        int,
        luk,
        bonus: Bonus { str: str_gear, dex: dex_gear, int: int_gear, luk: luk_gear },
        ability_points: number_in(view, Rect::new(luk_label.x + x, luk_label.y + y, w, h))?,
    })
}

/// How many of `columns` make up the first number: it ends at the first gap wider than the space
/// between digits.
fn number_end(columns: &[(u32, u32)]) -> usize {
    columns.windows(2).position(|pair| pair[1].0 - pair[0].1 > NUMBER_GAP).map_or(columns.len(), |i| i + 1)
}

/// A stat's own points, and what gear adds. With gear on, the window shows the total and then the
/// split, as in `30 (28+2)`: the plan counts the points put in, and works gear out itself, so the
/// 28 is the stat and the 2 is only shown. A stat with no bonus is a plain number. The bonus is
/// best effort: unreadable, it is nothing, and the stat still reads.
fn stat_in(view: &Frame, area: Rect) -> Option<(u32, u32)> {
    let columns = ink_columns(view, area, is_ink);
    let end = number_end(&columns);
    // After the total comes the bracket, then the points put in, a `+`, what gear adds and the
    // closing bracket. Heights are taken from the first digit, since the brackets run taller.
    let Some(after) = columns.get(end + 1..).filter(|rest| !rest.is_empty()) else { return Some((number_in(view, area)?, 0)) };
    let first = Rect::new(after[0].0 as f32, area.y, (after[0].1 - after[0].0) as f32, area.h);
    let rows = ink_rows(view, first, is_ink)?;
    let masks: Vec<Mask> = after.iter().map(|&cols| Mask::new(view, cols, rows, is_ink)).collect();
    let own = digits::window_prefix(&masks)?;
    let gear = masks
        .get(own.len() + 1..masks.len().saturating_sub(1))
        .and_then(digits::window_number)
        .and_then(|digits| digits.parse().ok())
        .unwrap_or(0);
    Some((own.parse().ok()?, gear))
}

/// The first number in `area`. Anything after a gap is left alone, so a bonus shown beside a
/// value can't be read into it, but one digit that can't be read leaves it all unread.
fn number_in(view: &Frame, area: Rect) -> Option<u32> {
    let columns = ink_columns(view, area, is_ink);
    let end = number_end(&columns);
    number_of(view, area, &columns[..end])
}

/// A range such as `107 - 289`: two numbers with a dash between. The widest gap in the line is the
/// one after the dash; the dash itself is a low mark that is left out of the first number.
fn range_in(view: &Frame, area: Rect) -> Option<(u32, u32)> {
    let columns = ink_columns(view, area, is_ink);
    let split = (1..columns.len()).max_by_key(|&i| columns[i].0 - columns[i - 1].1)?;
    let (first, second) = columns.split_at(split);
    let height = |c: &(u32, u32)| {
        ink_rows(view, Rect::new(c.0 as f32, area.y, (c.1 - c.0) as f32, area.h), is_ink).map_or(0, |(top, bottom)| bottom - top)
    };
    let tallest = first.iter().map(height).max()?;
    let first = match first.split_last() {
        Some((dash, digits)) if !digits.is_empty() && height(dash) * 2 < tallest => digits,
        _ => first,
    };
    Some((number_of(view, area, first)?, number_of(view, area, second)?))
}

/// The number the ink columns `digits` make up.
fn number_of(view: &Frame, area: Rect, digits: &[(u32, u32)]) -> Option<u32> {
    // The digits' height comes from the digits alone: a bonus bracket beside them, as in
    // `30 (28+2)`, runs taller and would make every digit read the wrong size.
    let (left, right) = (digits.first()?.0, digits.last()?.1);
    let rows = ink_rows(view, Rect::new(left as f32, area.y, (right - left) as f32, area.h), is_ink)?;
    let masks: Vec<Mask> = digits.iter().map(|&cols| Mask::new(view, cols, rows, is_ink)).collect();
    digits::window_number(&masks)?.parse().ok()
}

/// The level strips of the skill rows, top to bottom. These are looked for rather than the name
/// strips because a long name leaves too little of its strip showing. A strip counts only with
/// the row's name strip above it, since sky can be this colour too.
fn level_strips(view: &Frame) -> Vec<Rect> {
    let mut strips = color::find_blocks(view, whole(view), MIN_STRIP_RUN, is_skill_strip);
    strips.retain(|s| s.h >= 24.0 && s.h <= 40.0);
    strips.sort_by(|a, b| a.y.total_cmp(&b.y));
    let named = |s: &Rect| {
        let y = s.y - LEVEL_BELOW_NAME / 2.0;
        y >= 0.0 && is_skill_strip(view.get_pixel(s.x as u32 + 2, y as u32))
    };
    let Some(first) = strips.iter().find(|s| named(s)).copied() else { return Vec::new() };
    let on_grid = |s: &Rect| {
        let rows = (s.y - first.y) / SKILL_PITCH;
        (s.x - first.x).abs() <= SLACK && (rows - rows.round()).abs() * SKILL_PITCH <= SLACK
    };
    strips.iter().filter(|s| on_grid(s) && named(s)).copied().collect()
}

fn read_skills(view: &Frame, ocr: &Ocr) -> Result<Option<SkillWindow>> {
    let strips = level_strips(view);
    let Some(first) = strips.first().copied() else { return Ok(None) };
    let mut rows = Vec::with_capacity(strips.len());
    for strip in &strips {
        let Some(level) = number_in(view, level_area(strip)) else { continue };
        let words = ocr.words_in(view, name_area(strip), NAME_SCALE, |crop| preprocess::pad(&preprocess::upscale(crop, NAME_SCALE), NAME_PAD))?;
        let name = group_lines(&words).into_iter().next().map(|line| line.text);
        if let Some(name) = name {
            rows.push(SkillRow { name, level });
        }
    }
    let skill_points = number_in(view, sp_area(&first));
    Ok((!rows.is_empty()).then_some(SkillWindow { rows, skill_points }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAPER: Rgba<u8> = Rgba([225, 223, 213, 255]);
    const LABEL: Rgba<u8> = Rgba([184, 199, 77, 255]);
    const STRIP: Rgba<u8> = Rgba([205, 217, 227, 255]);

    fn fill(frame: &mut Frame, (x, y, w, h): (u32, u32, u32, u32), colour: Rgba<u8>) {
        for row in y..y + h {
            for col in x..x + w {
                frame.put_pixel(col, row, colour);
            }
        }
    }

    /// Draws made-up digits on the window font's grid, each two pixels to a dot.
    fn draw(frame: &mut Frame, mut x: u32, y: u32, glyphs: &[(&[&str], u32)]) {
        for (rows, gap) in glyphs {
            for (dy, row) in rows.iter().enumerate() {
                for (dx, dot) in row.chars().enumerate() {
                    if dot == '#' {
                        fill(frame, (x + dx as u32 * 2, y + dy as u32 * 2, 2, 2), Rgba([0, 0, 0, 255]));
                    }
                }
            }
            x += rows[0].len() as u32 * 2 + gap;
        }
    }

    const ONE: &[&str] = &["..#", ".##", "#.#", "..#", "..#", "..#", "..#", "..#", "..#"];
    const SEVEN: &[&str] = &["#####", "...#.", "...#.", "..#..", "..#..", "..#..", ".#...", ".#...", ".#..."];

    fn a_glance(luk: u32) -> Option<Glance> {
        let stats = StatWindow { level: 2, str: 4, dex: 5, int: 4, luk, ability_points: 0, ..StatWindow::default() };
        Some(Glance { stats: Some(stats), ..Glance::default() })
    }

    #[test]
    fn a_change_is_read_once_after_showing_twice() {
        let mut watch = Watch::default();
        assert!(!watch.settled(a_glance(12)), "the first sight of it");
        assert!(watch.settled(a_glance(12)));
        assert!(!watch.settled(a_glance(12)), "already read");
        assert!(!watch.settled(a_glance(17)));
        assert!(watch.settled(a_glance(17)));
    }

    #[test]
    fn a_passing_misread_is_never_read() {
        let mut watch = Watch::default();
        watch.settled(a_glance(12));
        watch.settled(a_glance(12));
        assert!(!watch.settled(a_glance(1)));
        assert!(!watch.settled(a_glance(12)), "back to what was read");
    }

    #[test]
    fn reopening_a_window_unchanged_reads_nothing_and_closing_is_no_change() {
        let mut watch = Watch::default();
        watch.settled(a_glance(12));
        watch.settled(a_glance(12));
        assert!(!watch.settled(None));
        assert!(!watch.settled(None));
        assert!(!watch.settled(a_glance(12)));
        assert!(!watch.settled(a_glance(12)));
    }

    #[test]
    fn a_picture_with_neither_window_is_skipped_at_a_look() {
        let mut frame = Frame::from_pixel(600, 400, PAPER);
        assert!(!any_window(&frame));
        fill(&mut frame, (40, 60, 90, 28), LABEL);
        assert!(any_window(&frame));
    }

    #[test]
    fn the_wide_gap_after_a_1_stays_inside_a_number() {
        let mut frame = Frame::from_pixel(120, 40, PAPER);
        draw(&mut frame, 10, 10, &[(ONE, 8), (SEVEN, 0)]);
        assert_eq!(number_in(&frame, Rect::new(4.0, 4.0, 100.0, 30.0)), Some(17));
    }

    #[test]
    fn what_follows_a_space_is_left_out() {
        let mut frame = Frame::from_pixel(120, 40, PAPER);
        draw(&mut frame, 10, 10, &[(SEVEN, 10), (ONE, 0)]);
        assert_eq!(number_in(&frame, Rect::new(4.0, 4.0, 100.0, 30.0)), Some(7));
    }

    // Made-up marks of the right sizes: brackets run a dot above and below the digits.
    const OPEN: &[&str] = &["..#", ".#.", "#..", "#..", "#..", "#..", "#..", "#..", "#..", ".#.", "..#"];
    const CLOSE: &[&str] = &["#..", ".#.", "..#", "..#", "..#", "..#", "..#", "..#", "..#", ".#.", "#.."];
    const PLUS: &[&str] = &[".....", "..#..", "..#..", "#####", "..#..", "..#..", ".....", ".....", "....."];

    /// `7 (17+7)`: a total, then the points put in, then what gear adds, as the stat window shows it.
    fn with_a_bonus() -> Frame {
        let mut frame = Frame::from_pixel(120, 40, PAPER);
        draw(&mut frame, 10, 10, &[(SEVEN, 10)]);
        draw(&mut frame, 30, 8, &[(OPEN, 2)]);
        draw(&mut frame, 38, 10, &[(ONE, 8), (SEVEN, 4), (PLUS, 4), (SEVEN, 4)]);
        draw(&mut frame, 94, 8, &[(CLOSE, 0)]);
        frame
    }

    #[test]
    fn a_bonus_in_brackets_does_not_make_the_total_unreadable_and_the_points_put_in_are_what_counts() {
        let area = Rect::new(4.0, 4.0, 110.0, 30.0);
        // The bracket is taller than the digits, which used to make the total unreadable.
        assert_eq!(number_in(&with_a_bonus(), area), Some(7));
        assert_eq!(stat_in(&with_a_bonus(), area), Some((17, 7)), "the 17 put in, not the total 7, and the 7 gear adds");
    }

    #[test]
    fn a_stat_with_no_gear_is_one_plain_number() {
        let mut frame = Frame::from_pixel(120, 40, PAPER);
        draw(&mut frame, 10, 10, &[(ONE, 8), (SEVEN, 0)]);
        assert_eq!(stat_in(&frame, Rect::new(4.0, 4.0, 110.0, 30.0)), Some((17, 0)));
    }

    #[test]
    fn four_labels_in_a_stack_are_the_stat_window() {
        let mut frame = Frame::from_pixel(600, 400, PAPER);
        for i in 0..4 {
            fill(&mut frame, (40, 60 + i * 36, 90, 28), LABEL);
        }
        let labels = stat_labels(&frame).unwrap();
        assert_eq!((labels[0].y, labels[3].y), (60.0, 168.0));
    }

    #[test]
    fn scenery_of_the_same_green_is_not() {
        let mut frame = Frame::from_pixel(600, 400, PAPER);
        for i in 0..3 {
            fill(&mut frame, (40, 60 + i * 36, 90, 28), LABEL);
        }
        fill(&mut frame, (200, 168, 90, 28), LABEL);
        assert!(stat_labels(&frame).is_none());
    }

    #[test]
    fn skill_rows_are_level_strips_with_a_name_strip_above() {
        let mut frame = Frame::from_pixel(600, 400, PAPER);
        for i in 0..3 {
            fill(&mut frame, (40, 60 + i * 80, 240, 34), STRIP);
            fill(&mut frame, (40, 96 + i * 80, 240, 34), STRIP);
        }
        // Sky of the same colour, with nothing above it.
        fill(&mut frame, (340, 60, 200, 34), STRIP);
        let strips = level_strips(&frame);
        assert_eq!(strips.iter().map(|s| s.y).collect::<Vec<_>>(), [96.0, 176.0, 256.0]);
    }

    #[test]
    fn a_long_name_leaves_its_row_in_place() {
        let mut frame = Frame::from_pixel(600, 400, PAPER);
        fill(&mut frame, (40, 60, 240, 34), STRIP);
        fill(&mut frame, (40, 96, 240, 34), STRIP);
        // Text across nearly all of the name strip.
        fill(&mut frame, (46, 70, 220, 14), Rgba([0, 0, 0, 255]));
        assert_eq!(level_strips(&frame).len(), 1);
    }
}
