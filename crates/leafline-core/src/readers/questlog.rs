//! The game's Quest window: its Available, In Progress and Completed lists, and the detail pane
//! beside them. Read only while the player has it open, from the pictures live reading already
//! takes, the same way as the Character Stat and Skill windows.
//!
//! The window is opaque and flat: dark grey text on white, white text on the region bars and on
//! the picked row, so a keyed copy reads cleanly with OCR whatever is behind it. It is found by
//! its look, not by fixed pixels: the red rule under the tab strip, with the pink active tab right
//! above it. Everything else sits a fixed distance from that rule, measured at the size the
//! readers work at (Ideal Ratio at 1440p, two window pixels to a design pixel). At other UI sizes
//! the window is put back on that grid first, as the stat window is.
//!
//! Only the rows in view can be read. [`Gathered`](crate::questlog::Gathered) puts the views
//! together as the player scrolls.

use std::sync::LazyLock;

use image::Rgba;
use image::imageops::{self, FilterType};
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::text::digitize;
use super::ui::{CANONICAL_SCALE, Canonical, DESIGN_HEIGHT};
use crate::error::Result;
use crate::vision::glyphs::ink_columns;
use crate::vision::{Frame, Line, Ocr, Rect, color, group_lines, preprocess};

// All distances are in pixels at the size the window is read at, from the red rule's left end
// and top edge. Measured on three frames of the window at 2560x1440, Ideal Ratio.

/// The rule's length, and how far off a found one may be once put on the grid.
const RULE_LENGTH: f32 = 478.0;
const RULE_SLACK: f32 = 24.0;
/// The tabs stand side by side this far apart, the first at the rule's left end, in a strip
/// this tall right above the rule.
const TAB_PITCH: f32 = 145.0;
const TAB_STRIP: f32 = 36.0;
const TABS: usize = 3;
/// Where each tab's fill is looked at, from its left end along the rule, and how much of it, in
/// hundredths, must show a tab's pink or grey. The tabs' labels and edges take the rest.
const TAB_SAMPLE: (f32, f32, f32, f32) = (20.0, -30.0, 100.0, 22.0);
const TAB_FILLED: usize = 8;
/// The list's rows: the first starts this far under the rule, each this tall and this far apart,
/// as many as fit above the window's foot.
const FIRST_SLOT: f32 = 20.0;
const SLOT_HEIGHT: f32 = 36.0;
const SLOT_PITCH: f32 = 42.0;
const SLOTS: usize = 14;
/// The region bars and rows run between these, from the rule's left end.
const LIST_LEFT: f32 = 12.0;
const LIST_RIGHT: f32 = 434.0;
/// Where a bar's or row's colour is looked at, clear of icons and text on every tab.
const SLOT_SAMPLE: f32 = 300.0;
/// The Quest Helper checkbox, on In Progress rows.
const CHECKBOX: (f32, f32) = (16.0, 44.0);
/// The row's icon, after the checkbox on In Progress and first on the other tabs.
const ICONS: (f32, f32) = (14.0, 74.0);
/// The scroll bar: the column its thumb is looked for in, and the track's ends under the rule.
const SCROLL_COLUMN: f32 = 458.0;
const TRACK: (f32, f32) = (38.0, 613.0);
/// Of the track's length, at least this many tenths show its grey: the thumb covers the rest.
const TRACK_SHARE: u32 = 4;
/// The track's ends: a thumb this close to one is there.
const TRACK_SLACK: f32 = 4.0;
/// The detail pane's blue box is looked for to the right of the list, its top near the rule.
/// The picture of the NPC splits it, so what is found is its title side.
const DETAIL_SEARCH: (f32, f32, f32, f32) = (470.0, -60.0, 660.0, 220.0);
const DETAIL_TITLE_WIDTH: f32 = 345.0;
/// The pane's text runs from under the box to the window's foot, this wide, short of its scroll
/// bar.
const DETAIL_FOOT: f32 = 641.0;
const DETAIL_WIDTH: f32 = 534.0;
/// In the grey boxes, an item's or monster's picture stands this far into the box, before its
/// name.
const BOX_ICON: f32 = 64.0;
/// Text that reaches past its rows is cut short with dots: three narrow marks on the baseline.
const DOT_WIDTH: u32 = 5;
const DOTS: usize = 3;

/// A ring's tile shows at least this many of its pale grey pixels.
const ICON_TILE: usize = 100;
/// OCR gets the keyed pictures enlarged and with a margin. No one way of enlarging read every
/// line of the three frames right, so the list is read three ways and the readings vote; the
/// detail pane is read the first way.
const PASSES: [(f32, FilterType); 3] =
    [(2.0, FilterType::Triangle), (3.0, FilterType::CatmullRom), (2.0, FilterType::Nearest)];
const OCR_PAD: u32 = 24;
/// The picked row's bar has a pale edge, left out of its text.
const BAR_EDGE: f32 = 4.0;
/// The detail box's blue, as a luminance: its white text stands out above it.
const BLUE_BOX_LUMINANCE: u8 = 121;

/// The window around its rule, in design pixels from the rule's left end and top edge: left,
/// up, right and down, with room to spare.
const WINDOW_AROUND_RULE: (f32, f32, f32, f32) = (-20.0, -60.0, 570.0, 340.0);
/// Every this many pixels across, a picture is checked for the rule's red, which must show at
/// this many in a row: the rule is half its length long at the smallest UI size.
const LOOK_STEP: u32 = 16;
const RULE_SAMPLES: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuestTab {
    Available,
    InProgress,
    Completed,
}

impl QuestTab {
    const ALL: [QuestTab; TABS] = [QuestTab::Available, QuestTab::InProgress, QuestTab::Completed];

    pub fn key(self) -> &'static str {
        match self {
            QuestTab::Available => "available",
            QuestTab::InProgress => "inProgress",
            QuestTab::Completed => "completed",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tab| tab.key() == key)
    }
}

/// One read of the window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestLogReading {
    pub tab: QuestTab,
    /// What the list shows, top to bottom.
    pub items: Vec<LogItem>,
    pub scroll: Scroll,
    /// The detail pane, when it is open and its title reads.
    pub detail: Option<QuestDetail>,
    /// Bars and rows in view whose text didn't read, so aren't in `items`.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub unread: u32,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

/// Where the list stands. A list that fits shows its top and its end at once.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scroll {
    pub at_top: bool,
    pub at_end: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "kind")]
pub enum LogItem {
    /// A region bar, `Henesys (3)`: how many quests the tab has there.
    Region { name: String, count: Option<u32> },
    Quest(LogQuest),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogQuest {
    pub title: String,
    /// Other readings of the title, for matching it to the data when the first doesn't.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub readings: Vec<String>,
    /// The game cut the title short with dots; `title` is what shows, without them.
    pub cut: bool,
    pub icon: RowIcon,
    /// The Quest Helper checkbox, on In Progress rows only.
    pub on_helper: Option<bool>,
    /// The row picked, whose detail the pane shows.
    pub selected: bool,
}

/// The mark before a row's title. What the game means by each is read from three frames, so
/// it is kept as the shape, not a meaning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RowIcon {
    /// The yellow `!` of the Available tab.
    Bang,
    /// A blue arrow (one or two), on In Progress rows.
    Arrow,
    /// A blue ring on a grey tile. Every Completed row has one; on In Progress it stood by a quest
    /// whose counter was full, and one quest showed an arrow at 0 / 30 and the ring at 30 / 30, so
    /// it likely means ready to hand in.
    Ring,
    /// The same tile with a grey ring, on In Progress rows whose checkbox is greyed too. Its
    /// meaning isn't known; one such quest showed the blue ring later.
    GreyRing,
    Unknown,
}

/// The detail pane: the picked quest's title and level or completion date in the blue box, then
/// its counters, whom to talk to, and its rewards.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct QuestDetail {
    pub title: String,
    pub cut: bool,
    /// `Level 12+`.
    pub level: Option<u32>,
    /// `Completed on 10/25/2026`, as `2026-10-25`.
    pub completed_on: Option<String>,
    /// The Quest Summary's lines, `Maple Leaf 30 / 30`.
    pub counters: Vec<DetailCounter>,
    /// `! Talk to NPC Community Board`.
    pub talk_to: Option<String>,
    /// The Reward box: items with their count, and EXP and mesos as numbers.
    pub rewards: Vec<DetailReward>,
    pub reward_exp: Option<u32>,
    pub reward_mesos: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailCounter {
    pub name: String,
    pub have: u32,
    pub need: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetailReward {
    pub name: String,
    pub count: u32,
}

/// What the window shows at a glance, with no OCR: enough to tell that something changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Glance {
    tab: QuestTab,
    slots: Vec<(Slot, usize)>,
    thumb: Option<(u32, u32)>,
    detail: usize,
}

/// Decides when a change in the window is worth a full read: once it has shown the same in two
/// pictures, which skips a list mid-scroll, and only the once.
#[derive(Debug, Default)]
pub struct Watch {
    last: Option<Glance>,
    read: Option<Glance>,
}

impl Watch {
    /// Takes in this picture's glance, `None` with the window closed, and says whether to read.
    pub fn settled(&mut self, glance: Option<Glance>) -> bool {
        let settled = glance.is_some() && glance == self.last && glance != self.read;
        if settled {
            self.read.clone_from(&glance);
        }
        if glance.is_none() {
            // Opened again, the window is a fresh look, read even if it shows the same.
            self.read = None;
        }
        self.last = glance;
        settled
    }
}

/// The window's place in a view: the rule's left end and top edge.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Anchor {
    left: f32,
    top: f32,
    tab: QuestTab,
}

impl Anchor {
    fn at(&self, x: f32, y: f32, w: f32, h: f32) -> Rect {
        Rect::new(self.left + x, self.top + y, w, h)
    }

    fn slot(&self, i: usize) -> Rect {
        self.at(LIST_LEFT, FIRST_SLOT + i as f32 * SLOT_PITCH, LIST_RIGHT - LIST_LEFT, SLOT_HEIGHT)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Slot {
    Empty,
    Region,
    Row,
    Picked,
}

/// What the window shows at a glance, or `None` with it closed. `hud_scale` is the UI's size
/// from the HUD, when it was found.
pub fn glance(frame: &Frame, hud_scale: Option<f32>) -> Option<Glance> {
    let (view, anchor) = locate(frame, hud_scale)?;
    let view = view.as_ref().unwrap_or(frame);
    let slots = (0..SLOTS)
        .map(|i| {
            let slot = slot_kind(view, &anchor, i);
            (slot, ink_in(view, text_area(&anchor, i), slot))
        })
        .collect();
    let detail = detail_box(view, &anchor).map_or(0, |b| {
        count(view, title_side(&b), is_white) + count(view, detail_body(&anchor, &b), is_detail_ink)
    });
    Some(Glance { tab: anchor.tab, slots, thumb: thumb(view, &anchor), detail })
}

/// Reads the window, or `None` with it closed. `hud_scale` is the UI's size from the HUD, when
/// it was found.
pub fn read(frame: &Frame, hud_scale: Option<f32>, ocr: &Ocr) -> Result<Option<QuestLogReading>> {
    let Some((view, anchor)) = locate(frame, hud_scale) else { return Ok(None) };
    read_view(view.as_ref().unwrap_or(frame), &anchor, ocr, true).map(Some)
}

/// [`read`] without the detail pane: the list alone (the website's, whose OCR misreads the
/// pane's numbers).
pub fn read_list_only(frame: &Frame, hud_scale: Option<f32>, ocr: &Ocr) -> Result<Option<QuestLogReading>> {
    let Some((view, anchor)) = locate(frame, hud_scale) else { return Ok(None) };
    read_view(view.as_ref().unwrap_or(frame), &anchor, ocr, false).map(Some)
}

/// The window in `frame`: found as drawn when the UI is at the size the distances are measured
/// at, or else in a copy of the window's part of the picture put back on that grid, which is
/// returned with it. The rule's length says the size, and the sizes the game draws at settle it.
fn locate(frame: &Frame, hud_scale: Option<f32>) -> Option<(Option<Frame>, Anchor)> {
    if !any_rule(frame) {
        return None;
    }
    if let Some(anchor) = find(frame) {
        return Some((None, anchor));
    }
    let whole = Rect::new(0.0, 0.0, frame.width() as f32, frame.height() as f32);
    let design = RULE_LENGTH / CANONICAL_SCALE;
    let mut rules = color::find_blocks(frame, whole, (design * 0.9) as u32, is_rule);
    rules.retain(|r| r.h <= 12.0 && r.w <= design * 2.7);
    // Only sizes the game draws at: the Item Inventory has the same tabs on a shorter rule, which
    // any size at all would stretch to fit.
    let window = (frame.height() as f32 / DESIGN_HEIGHT).max(1.0);
    let sizes: Vec<f32> = hud_scale.into_iter().chain([window, 1.0, 3.0]).collect();
    rules.iter().find_map(|rule| {
        let measured = rule.w / design;
        let scale = sizes.iter().copied().find(|s| (s - measured).abs() <= 0.04 * s)?;
        if (scale - CANONICAL_SCALE).abs() <= 0.005 {
            return None;
        }
        let (left, top, right, bottom) = WINDOW_AROUND_RULE;
        let area = Rect::from_edges(
            rule.x + left * scale,
            rule.y + top * scale,
            rule.x + right * scale,
            rule.y + bottom * scale,
        );
        let view = Canonical::within(frame, scale, area).frame;
        let anchor = find(&view)?;
        Some((Some(view), anchor))
    })
}

fn read_view(view: &Frame, anchor: &Anchor, ocr: &Ocr, with_detail: bool) -> Result<QuestLogReading> {
    let (items, unread) = read_list(view, anchor, ocr)?;
    let scroll = scroll(view, anchor);
    let detail = match detail_box(view, anchor).filter(|_| with_detail) {
        Some(b) => read_detail(view, anchor, &b, ocr)?,
        None => None,
    };
    Ok(QuestLogReading { tab: anchor.tab, items, scroll, detail, unread })
}

fn near(px: &Rgba<u8>, colour: [u8; 3], tolerance: u8) -> bool {
    px.0.iter().zip(colour).all(|(a, b)| a.abs_diff(b) <= tolerance)
}

fn is_rule(px: &Rgba<u8>) -> bool {
    near(px, [204, 0, 0], 16)
}

fn is_pink_tab(px: &Rgba<u8>) -> bool {
    near(px, [238, 102, 136], 12)
}

/// The two colours of the region bars: tan for Event, Citizenship and the jobs, blue-grey for
/// the towns.
fn is_grey_tab(px: &Rgba<u8>) -> bool {
    near(px, [187, 187, 187], 4)
}

fn is_region_bar(px: &Rgba<u8>) -> bool {
    near(px, [176, 148, 109], 10) || near(px, [156, 176, 200], 10)
}

fn is_picked_row(px: &Rgba<u8>) -> bool {
    near(px, [51, 102, 153], 10)
}

fn is_detail_blue(px: &Rgba<u8>) -> bool {
    near(px, [68, 136, 187], 8)
}

fn is_box_grey(px: &Rgba<u8>) -> bool {
    near(px, [225, 223, 213], 6)
}

fn is_track(px: &Rgba<u8>) -> bool {
    near(px, [231, 231, 231], 6)
}

/// The rows' text: a flat dark grey, never part of an icon, whose dark edges are coloured.
fn is_grey_ink(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0;
    r.max(g).max(b) <= 140 && preprocess::saturation(px) <= 16
}

/// Text on the bars and the picked row, and in the detail pane's blue box.
fn is_white(px: &Rgba<u8>) -> bool {
    px.0[..3].iter().all(|&c| c >= 230)
}

/// The detail pane's text: grey, the blue of names and `Talk to NPC`, and red.
fn is_detail_ink(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0;
    is_grey_ink(px) || (b >= 200 && r <= 120 && g <= 120) || (r >= 200 && g <= 60 && b <= 60)
}

fn is_red_check(px: &Rgba<u8>) -> bool {
    near(px, [204, 0, 0], 30)
}

fn is_checkbox_edge(px: &Rgba<u8>) -> bool {
    near(px, [153, 153, 153], 10)
}

fn is_bang(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0;
    r >= 220 && (120..=230).contains(&g) && b <= 20
}

fn is_ring(px: &Rgba<u8>) -> bool {
    near(px, [51, 187, 238], 12)
}

/// The pale grey tile a ring sits on.
fn is_icon_tile(px: &Rgba<u8>) -> bool {
    near(px, [238, 238, 238], 3)
}

fn is_arrow(px: &Rgba<u8>) -> bool {
    let [r, g, b, _] = px.0;
    r <= 40 && b >= 100 && b > g
}

fn count(view: &Frame, area: Rect, is: impl Fn(&Rgba<u8>) -> bool) -> usize {
    let (x, y, w, h) = area.to_pixels(view.width(), view.height());
    (y..y + h).flat_map(|row| (x..x + w).map(move |col| (col, row))).filter(|&(col, row)| is(view.get_pixel(col, row))).count()
}

/// Whether any row of `frame` has a run of the rule's red as long as the rule at the smallest UI
/// size, checked on a coarse grid across, so a red speck in the scenery costs no more.
fn any_rule(frame: &Frame) -> bool {
    (0..frame.height()).any(|y| {
        let mut run = 0;
        (0..frame.width()).step_by(LOOK_STEP as usize).any(|x| {
            run = if is_rule(frame.get_pixel(x, y)) { run + 1 } else { 0 };
            run >= RULE_SAMPLES
        })
    })
}

/// The window in `view`: a red rule of the right length, with a pink tab standing on it.
fn find(view: &Frame) -> Option<Anchor> {
    let whole = Rect::new(0.0, 0.0, view.width() as f32, view.height() as f32);
    let mut rules = color::find_blocks(view, whole, (RULE_LENGTH - RULE_SLACK) as u32, is_rule);
    rules.retain(|r| (r.w - RULE_LENGTH).abs() <= RULE_SLACK && r.h <= 10.0);
    rules.iter().find_map(|rule| {
        let strip = Rect::new(rule.x, rule.y - TAB_STRIP, RULE_LENGTH, TAB_STRIP - 2.0);
        let pink = color::find_blocks(view, strip, 20, is_pink_tab);
        let (sum, n) = pink.iter().fold((0.0, 0.0), |(s, n), b| (s + (b.x + b.w / 2.0) * b.w, n + b.w));
        if n < 40.0 {
            return None;
        }
        let index = (((sum / n) - rule.x) / TAB_PITCH).floor().clamp(0.0, (TABS - 1) as f32) as usize;
        let anchor = Anchor { left: rule.x, top: rule.y, tab: QuestTab::ALL[index] };
        (has_tabs(view, &anchor) && has_track(view, &anchor)).then_some(anchor)
    })
}

fn slot_kind(view: &Frame, anchor: &Anchor, i: usize) -> Slot {
    let slot = anchor.slot(i);
    let (x, y) = (anchor.left + SLOT_SAMPLE, slot.center_y());
    if x as u32 >= view.width() || y as u32 >= view.height() {
        return Slot::Empty;
    }
    let px = view.get_pixel(x as u32, y as u32);
    if is_region_bar(px) {
        Slot::Region
    } else if is_picked_row(px) {
        Slot::Picked
    } else if count(view, text_area(anchor, i), is_grey_ink) > 0 {
        Slot::Row
    } else {
        Slot::Empty
    }
}

/// Where a slot's text can be: past the icons and the bar's minus box.
fn text_area(anchor: &Anchor, i: usize) -> Rect {
    let slot = anchor.slot(i);
    Rect::from_edges(anchor.left + ICONS.1 - 30.0, slot.y, slot.right(), slot.bottom())
}

fn ink_of(slot: Slot) -> fn(&Rgba<u8>) -> bool {
    match slot {
        Slot::Region | Slot::Picked => is_white,
        Slot::Row | Slot::Empty => is_grey_ink,
    }
}

fn ink_in(view: &Frame, area: Rect, slot: Slot) -> usize {
    count(view, area, ink_of(slot))
}

/// The list's text, keyed black on white slot by slot, so one OCR pass reads it all. On the
/// picked row only what is inside its bar counts: its pale edge reads as an `I`.
fn keyed_list(view: &Frame, anchor: &Anchor, kinds: &[Slot]) -> (Frame, Rect) {
    let area = Rect::from_edges(
        anchor.left + LIST_LEFT,
        anchor.top + FIRST_SLOT,
        anchor.left + LIST_RIGHT,
        anchor.slot(SLOTS - 1).bottom(),
    );
    let starts: Vec<f32> = (0..kinds.len())
        .map(|i| match kinds[i] {
            Slot::Picked => bar_start(view, anchor, i).map_or(text_area(anchor, i).x, |x| x + BAR_EDGE),
            _ => text_area(anchor, i).x,
        })
        .collect();
    let (ax, ay, aw, ah) = area.to_pixels(view.width(), view.height());
    let keyed = Frame::from_fn(aw, ah, |x, y| {
        let (vx, vy) = (ax + x, ay + y);
        let within = (vy as f32 - anchor.top - FIRST_SLOT) / SLOT_PITCH;
        let i = within.floor() as usize;
        let inside = within.fract() * SLOT_PITCH < SLOT_HEIGHT;
        let ink = inside
            && kinds.get(i).is_some_and(|&kind| vx as f32 >= starts[i] && ink_of(kind)(view.get_pixel(vx, vy)));
        if ink { Rgba([0, 0, 0, 255]) } else { Rgba([255, 255, 255, 255]) }
    });
    (keyed, Rect::new(ax as f32, ay as f32, aw as f32, ah as f32))
}

/// Where the picked row's bar starts, from the left along its middle.
fn bar_start(view: &Frame, anchor: &Anchor, i: usize) -> Option<f32> {
    let slot = anchor.slot(i);
    let y = slot.center_y() as u32;
    let (from, to) = (slot.x as u32, (anchor.left + SLOT_SAMPLE) as u32);
    (from..to.min(view.width())).find(|&x| is_picked_row(view.get_pixel(x, y))).map(|x| x as f32)
}

/// The list's bars and rows, and how many in view didn't read.
fn read_list(view: &Frame, anchor: &Anchor, ocr: &Ocr) -> Result<(Vec<LogItem>, u32)> {
    let kinds: Vec<Slot> = (0..SLOTS).map(|i| slot_kind(view, anchor, i)).collect();
    if kinds.iter().all(|&k| k == Slot::Empty) {
        return Ok((Vec::new(), 0));
    }
    let mut unread = 0;
    let (keyed, area) = keyed_list(view, anchor, &kinds);
    let passes =
        PASSES.iter().map(|&(scale, filter)| ocr_lines(ocr, &keyed, scale, filter)).collect::<Result<Vec<_>>>()?;
    let mut items = Vec::new();
    for (i, &kind) in kinds.iter().enumerate() {
        if kind == Slot::Empty {
            continue;
        }
        let slot = anchor.slot(i);
        let texts: Vec<String> = passes.iter().filter_map(|lines| line_in(lines, area, slot)).collect();
        match kind {
            Slot::Region => {
                let parsed: Vec<(String, Option<u32>)> = texts.iter().filter_map(|t| parse_region(t)).collect();
                let names: Vec<String> = parsed.iter().map(|(name, _)| name.clone()).collect();
                let Some(name) = agreed(&names) else {
                    unread += 1;
                    continue;
                };
                items.push(LogItem::Region { name, count: most_common(parsed.iter().filter_map(|(_, c)| *c)) });
            }
            Slot::Row | Slot::Picked => {
                let dots = ends_in_dots(view, text_area(anchor, i), ink_of(kind));
                let titles: Vec<(String, bool)> =
                    texts.iter().map(|t| untruncated(t, dots)).filter(|(t, _)| !t.is_empty()).collect();
                let names: Vec<String> = titles.iter().map(|(t, _)| t.clone()).collect();
                let Some(title) = agreed(&names) else {
                    unread += 1;
                    continue;
                };
                let cut = titles.iter().any(|(_, cut)| *cut);
                let mut readings: Vec<String> = Vec::new();
                for name in names {
                    if name != title && !readings.contains(&name) {
                        readings.push(name);
                    }
                }
                let icon_area = Rect::from_edges(anchor.left + ICONS.0, slot.y, anchor.left + ICONS.1, slot.bottom());
                let checkbox =
                    Rect::from_edges(anchor.left + CHECKBOX.0, slot.y, anchor.left + CHECKBOX.1, slot.bottom());
                let on_helper = (anchor.tab == QuestTab::InProgress && count(view, checkbox, is_checkbox_edge) > 0)
                    .then(|| count(view, checkbox, is_red_check) > 0);
                items.push(LogItem::Quest(LogQuest {
                    title,
                    readings,
                    cut,
                    icon: icon_in(view, icon_area),
                    on_helper,
                    selected: kind == Slot::Picked,
                }));
            }
            Slot::Empty => {}
        }
    }
    Ok((items, unread))
}

/// Of several readings of one line, the one most like the others: two passes that agree outvote
/// a third, and with all three apart the one in the middle wins. Ties go to the earlier pass.
fn agreed(readings: &[String]) -> Option<String> {
    let closeness = |a: &String| readings.iter().map(|b| strsim::normalized_levenshtein(a, b)).sum::<f64>();
    let mut best: Option<(&String, f64)> = None;
    for reading in readings {
        let score = closeness(reading);
        if best.is_none_or(|(_, top)| score > top + 1e-9) {
            best = Some((reading, score));
        }
    }
    best.map(|(r, _)| r.clone())
}

fn most_common(values: impl Iterator<Item = u32>) -> Option<u32> {
    let values: Vec<u32> = values.collect();
    let mut best: Option<(u32, usize)> = None;
    for &v in &values {
        let n = values.iter().filter(|&&w| w == v).count();
        if best.is_none_or(|(_, top)| n > top) {
            best = Some((v, n));
        }
    }
    best.map(|(v, _)| v)
}

/// OCR's lines in `keyed`, enlarged by `scale` with `filter`, with rectangles in `keyed`'s
/// pixels.
fn ocr_lines(ocr: &Ocr, keyed: &Frame, scale: f32, filter: FilterType) -> Result<Vec<Line>> {
    let whole = Rect::new(0.0, 0.0, keyed.width() as f32, keyed.height() as f32);
    let words = ocr.words_in(keyed, whole, scale, |crop| {
        let (w, h) = ((crop.width() as f32 * scale).round() as u32, (crop.height() as f32 * scale).round() as u32);
        preprocess::pad(&imageops::resize(crop, w, h, filter), OCR_PAD)
    })?;
    Ok(group_lines(&words))
}

/// The text of the line OCR found in `slot`, with `area` where the lines were read from.
fn line_in(lines: &[Line], area: Rect, slot: Rect) -> Option<String> {
    let inside = |l: &&Line| {
        let y = area.y + l.rect.center_y();
        y >= slot.y && y < slot.bottom()
    };
    let words: Vec<&str> = lines.iter().filter(inside).map(|l| l.text.as_str()).collect();
    (!words.is_empty()).then(|| words.join(" "))
}

static REGION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.*?)\s*[(\[{]\s*([0-9OoIlSBgq|! ]+?)\s*[)\]}]?$").unwrap());

/// `Henesys (3)` as the region and its count. A bar whose count doesn't read keeps its name.
fn parse_region(text: &str) -> Option<(String, Option<u32>)> {
    let text = text.trim();
    match REGION.captures(text) {
        Some(c) => {
            let name = c[1].trim().to_string();
            (!name.is_empty()).then(|| (name, number(&c[2])))
        }
        None => (!text.is_empty()).then(|| (text.to_string(), None)),
    }
}

/// A title without the dots the game cuts it short with, and whether it was cut.
fn untruncated(text: &str, dots: bool) -> (String, bool) {
    let trimmed = text.trim();
    let bare = trimmed.trim_end_matches(['.', '…', ',', ' ']);
    let cut = dots || bare.len() + 2 <= trimmed.len() || trimmed.ends_with('…');
    (if cut { bare } else { trimmed }.to_string(), cut)
}

/// Whether the text in `area` ends in three narrow marks standing on the baseline.
fn ends_in_dots(view: &Frame, area: Rect, ink: fn(&Rgba<u8>) -> bool) -> bool {
    let columns = ink_columns(view, area, ink);
    if columns.len() < DOTS + 1 {
        return false;
    }
    let Some(baseline) = crate::vision::glyphs::ink_rows(view, area, ink).map(|(_, bottom)| bottom) else { return false };
    columns[columns.len() - DOTS..].iter().all(|&(left, right)| {
        let mark = Rect::new(left as f32, area.y, (right - left) as f32, area.h);
        right - left <= DOT_WIDTH
            && crate::vision::glyphs::ink_rows(view, mark, ink)
                .is_some_and(|(top, bottom)| bottom == baseline && bottom - top <= DOT_WIDTH)
    })
}

/// The icon in a row. The ring's tile tells it from an arrow, which has some of its blue.
fn icon_in(view: &Frame, area: Rect) -> RowIcon {
    let tile = count(view, area, is_icon_tile) >= ICON_TILE;
    if count(view, area, is_bang) >= 8 {
        RowIcon::Bang
    } else if tile && count(view, area, is_ring) >= 24 {
        RowIcon::Ring
    } else if count(view, area, is_arrow) >= 12 {
        RowIcon::Arrow
    } else if tile {
        RowIcon::GreyRing
    } else {
        RowIcon::Unknown
    }
}

/// Whether all three tabs stand on the rule, pink or grey. An NPC shop has one tab on the same
/// rule, and the strip's plain grey beside it.
fn has_tabs(view: &Frame, anchor: &Anchor) -> bool {
    (0..TABS).all(|k| {
        let tab = anchor.at(TAB_SAMPLE.0 + k as f32 * TAB_PITCH, TAB_SAMPLE.1, TAB_SAMPLE.2, TAB_SAMPLE.3);
        let (_, _, w, h) = tab.to_pixels(view.width(), view.height());
        let filled = count(view, tab, |px| is_pink_tab(px) || is_grey_tab(px));
        w * h > 0 && filled * 100 >= (w * h) as usize * TAB_FILLED
    })
}

/// Whether the list's scroll track runs where the Quest window has it. Other windows with tabs on
/// a red rule, such as the Item Inventory, have none there.
fn has_track(view: &Frame, anchor: &Anchor) -> bool {
    let x = (anchor.left + SCROLL_COLUMN) as u32;
    let (top, bottom) = ((anchor.top + TRACK.0) as u32, (anchor.top + TRACK.1) as u32);
    if x >= view.width() || bottom >= view.height() {
        return false;
    }
    let track = (top..bottom).filter(|&y| is_track(view.get_pixel(x, y))).count() as u32;
    track * 10 >= (bottom - top) * TRACK_SHARE
}

/// The scroll thumb's top and bottom, or `None` with nothing to scroll.
fn thumb(view: &Frame, anchor: &Anchor) -> Option<(u32, u32)> {
    let x = (anchor.left + SCROLL_COLUMN) as u32;
    let (top, bottom) = ((anchor.top + TRACK.0) as u32, (anchor.top + TRACK.1) as u32);
    if x >= view.width() || bottom >= view.height() {
        return None;
    }
    let off_track: Vec<u32> = (top..bottom).filter(|&y| !is_track(view.get_pixel(x, y))).collect();
    Some((*off_track.first()?, *off_track.last()? + 1))
}

fn scroll(view: &Frame, anchor: &Anchor) -> Scroll {
    match thumb(view, anchor) {
        None => Scroll { at_top: true, at_end: true },
        Some((top, bottom)) => {
            let (start, end) = (anchor.top + TRACK.0, anchor.top + TRACK.1);
            Scroll { at_top: top as f32 <= start + TRACK_SLACK, at_end: bottom as f32 >= end - TRACK_SLACK }
        }
    }
}

/// The detail pane's blue box, when the pane is open.
fn detail_box(view: &Frame, anchor: &Anchor) -> Option<Rect> {
    let (x, y, w, h) = DETAIL_SEARCH;
    let search = anchor.at(x, y, w, h);
    let blocks = color::find_blocks(view, search, 250, is_detail_blue);
    let top = blocks.into_iter().filter(|b| b.w >= 300.0).min_by(|a, b| a.y.total_cmp(&b.y))?;
    // The title's text splits the box's rows, so its foot is found down its left margin.
    let x = top.x as u32 + 4;
    let foot = (top.y as u32..view.height()).find(|&y| !is_detail_blue(view.get_pixel(x, y)))?;
    let box_ = Rect::new(top.x, top.y, top.w, foot as f32 - top.y);
    (box_.h >= 100.0).then_some(box_)
}

/// The blue box's title side, short of the picture of the NPC.
fn title_side(blue: &Rect) -> Rect {
    Rect::new(blue.x, blue.y, blue.w.min(DETAIL_TITLE_WIDTH), blue.h)
}

/// The pane's text under the blue box, short of its scroll bar.
fn detail_body(anchor: &Anchor, blue: &Rect) -> Rect {
    Rect::from_edges(blue.x, blue.bottom() + 2.0, blue.x + DETAIL_WIDTH, anchor.top + DETAIL_FOOT)
}

static LEVEL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^lev[eo]l\s*([0-9OoIlSB][0-9OoIlSB ]*)").unwrap());
static COMPLETED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)completed\s*on\s*(\d{1,2})\s*\D{1,2}\s*(\d{1,2})\s*\D{1,2}\s*(\d{4})").unwrap());
/// `Maple Leaf 30 / 30`. OCR can split a number, `1 0`, read the slash as an `r`, or lose it; a
/// counter without one, `30 30`, is taken only when no reading has it.
static COUNTER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(.*?[A-Za-z].*?)\s+([0-9OoIlSB](?: ?[0-9OoIlSB])*)\s*[/|\\r]\s*([0-9OoIlSB](?: ?[0-9OoIlSB])*)$").unwrap()
});
static COUNTER_NO_SLASH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(.*?[A-Za-z].*?)\s+([0-9OoIlSB]+)\s+([0-9OoIlSB]+)$").unwrap());
static TALK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)talk\s+to\s+(?:npc\s+)?(.+)$").unwrap());
static ITEM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(.*?[A-Za-z].*?)\s*[xX×]\s*([0-9OoIlSB]+)$").unwrap());
static EXP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\+?\s*([\d,.]+)\s*exp\b").unwrap());
static MESOS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\+?\s*([\d,.]+)\s*mesos?\b").unwrap());

/// A number OCR read, with the letters it takes digits for put back.
fn number(text: &str) -> Option<u32> {
    let text: String = text.chars().map(|c| if matches!(c, 'g' | 'q') { '9' } else { c }).collect();
    digitize(&text).chars().filter(char::is_ascii_digit).collect::<String>().parse().ok()
}

fn read_detail(view: &Frame, anchor: &Anchor, blue: &Rect, ocr: &Ocr) -> Result<Option<QuestDetail>> {
    let (scale, filter) = PASSES[0];
    // The pane's text is bold and smoothed in places, so it is read twice: kept in shades, which
    // reads its words best, and cut to black, which keeps the slash of a counter.
    let title_area = title_side(blue);
    let crop = preprocess::crop(view, title_area);
    let heads = [
        ocr_lines(ocr, &shaded(&crop, |lum| lum.saturating_sub(BLUE_BOX_LUMINANCE)), scale, filter)?,
        ocr_lines(ocr, &preprocess::keep(&crop, is_white), scale, filter)?,
    ];
    let Some(first) = heads[0].first().or(heads[1].first()) else { return Ok(None) };
    let line = Rect::from_edges(
        title_area.x,
        title_area.y + first.rect.y - 4.0,
        title_area.right(),
        title_area.y + first.rect.bottom() + 4.0,
    );
    let (title, cut) = untruncated(trim_bullet(&first.text), ends_in_dots(view, line, is_white));
    if title.is_empty() {
        return Ok(None);
    }
    let mut detail = QuestDetail { title, cut, ..QuestDetail::default() };
    for line in heads.iter().flat_map(|lines| lines.iter().skip(1)) {
        if let Some(c) = LEVEL.captures(line.text.trim()) {
            detail.level = detail.level.or(number(&c[1]));
        } else if let Some(date) = completed_on(&line.text) {
            detail.completed_on.get_or_insert(date);
        }
    }

    let body = detail_body(anchor, blue);
    if body.h <= 0.0 {
        return Ok(Some(detail));
    }
    // The grey boxes: the Quest Summary and the Reward box. A line counts as in one when its
    // middle is. The pictures at their left are left out, which OCR would read as letters.
    let boxes: Vec<Rect> = color::find_blocks(view, body, 200, is_box_grey)
        .into_iter()
        .filter(|b| b.h >= 40.0)
        .map(|b| b.translated(-body.x, -body.y))
        .collect();
    let crop = preprocess::crop(view, body);
    let mut unsure = Vec::new();
    // The boxes alone, read the other ways the list is: OCR can lose a counter's small digits in
    // the whole pane, and a box is quick to read again.
    for b in &boxes {
        let area = Rect::from_edges(body.x + b.x + BOX_ICON, body.y + b.y, body.x + b.right(), body.y + b.bottom());
        let keyed = preprocess::keep(&preprocess::crop(view, area), is_detail_ink);
        for &(scale, filter) in &PASSES[1..] {
            let lines = ocr_lines(ocr, &keyed, scale, filter)?;
            let whole = [Rect::new(0.0, 0.0, keyed.width() as f32, keyed.height() as f32)];
            let (found, guessed) = parse_body(&lines, &whole);
            for counter in found.counters {
                if !detail.counters.iter().any(|c| c.name == counter.name) {
                    detail.counters.push(counter);
                }
            }
            for counter in guessed {
                if !unsure.iter().any(|c: &DetailCounter| c.name == counter.name) {
                    unsure.push(counter);
                }
            }
        }
    }
    for keyed in [shaded(&crop, |lum| 255 - lum), preprocess::keep(&crop, is_detail_ink)] {
        let mut keyed = keyed;
        for b in &boxes {
            let (x, y, w, h) = Rect::new(b.x, b.y, BOX_ICON, b.h).to_pixels(keyed.width(), keyed.height());
            for row in y..y + h {
                for col in x..x + w {
                    keyed.put_pixel(col, row, Rgba([255, 255, 255, 255]));
                }
            }
        }
        let lines = ocr_lines(ocr, &keyed, scale, filter)?;
        let (found, guessed) = parse_body(&lines, &boxes);
        // What one reading missed, the other may have: a counter by its name, the rest whole.
        for counter in found.counters {
            if !detail.counters.iter().any(|c| c.name == counter.name) {
                detail.counters.push(counter);
            }
        }
        for counter in guessed {
            if !unsure.iter().any(|c: &DetailCounter| c.name == counter.name) {
                unsure.push(counter);
            }
        }
        if detail.rewards.is_empty() {
            detail.rewards = found.rewards;
        }
        detail.talk_to = detail.talk_to.or(found.talk_to);
        detail.reward_exp = detail.reward_exp.or(found.reward_exp);
        detail.reward_mesos = detail.reward_mesos.or(found.reward_mesos);
    }
    if detail.counters.is_empty() {
        detail.counters = unsure;
    }
    Ok(Some(detail))
}

/// `Completed on 10/25/2026`, as `2026-10-25`, however OCR read the slashes.
fn completed_on(text: &str) -> Option<String> {
    let c = COMPLETED.captures(text)?;
    let (month, day, year): (u32, u32, u32) = (c[1].parse().ok()?, c[2].parse().ok()?, c[3].parse().ok()?);
    ((1..=12).contains(&month) && (1..=31).contains(&day)).then(|| format!("{year:04}-{month:02}-{day:02}"))
}

/// The counters, whom to talk to and the rewards in the lines under the blue box, with `boxes`
/// the grey boxes in the same pixels. Counters read without their slash come apart.
fn parse_body(lines: &[Line], boxes: &[Rect]) -> (QuestDetail, Vec<DetailCounter>) {
    let mut detail = QuestDetail::default();
    let mut unsure = Vec::new();
    let in_box = |line: &Line| boxes.iter().position(|b| line.rect.center_y() >= b.y && line.rect.center_y() < b.bottom());
    for line in lines {
        let text = line.text.trim();
        let boxed = in_box(line);
        if boxed.is_some()
            && let Some(c) = COUNTER.captures(text)
            && let (Some(have), Some(need)) = (number(&c[2]), number(&c[3]))
        {
            detail.counters.push(DetailCounter { name: clean_name(&c[1]), have, need });
            continue;
        }
        if boxed.is_some()
            && let Some(c) = COUNTER_NO_SLASH.captures(text)
            && let (Some(have), Some(need)) = (number(&c[2]), number(&c[3]))
        {
            unsure.push(DetailCounter { name: clean_name(&c[1]), have, need });
            continue;
        }
        if detail.talk_to.is_none()
            && let Some(c) = TALK.captures(text)
        {
            detail.talk_to = Some(c[1].trim().trim_end_matches('.').to_string());
            continue;
        }
        if boxed.is_none() {
            continue;
        }
        if let Some(c) = EXP.captures(text) {
            detail.reward_exp = number(&c[1]);
        } else if let Some(c) = MESOS.captures(text) {
            detail.reward_mesos = number(&c[1]);
        } else if let Some(c) = ITEM.captures(text)
            && let Some(count) = number(&c[2])
        {
            detail.rewards.push(DetailReward { name: clean_name(&c[1]), count });
        }
    }
    (detail, unsure)
}

/// A grey picture of `img` with dark text on white, for smoothed text: `ink` gives how much a
/// pixel of that luminance is text, from 0 for none.
fn shaded(img: &Frame, ink: impl Fn(u8) -> u8) -> Frame {
    let most = ink(255).max(ink(0)).max(1) as u32;
    Frame::from_fn(img.width(), img.height(), |x, y| {
        let amount = u32::from(ink(preprocess::luminance(img.get_pixel(x, y))));
        let v = 255 - (amount * 255 / most).min(255) as u8;
        Rgba([v, v, v, 255])
    })
}

/// The title less the green dot before it, which OCR can read as a letter.
fn trim_bullet(text: &str) -> &str {
    let text = text.trim();
    match text.split_once(' ') {
        Some((first, rest)) if first.chars().count() == 1 && !first.chars().all(char::is_alphanumeric) => rest,
        Some((first, rest)) if matches!(first, "o" | "a" | "e" | "c" | "0" | "•" | "°") && rest.starts_with(|c: char| c.is_uppercase() || c == '[') => rest,
        _ => text,
    }
}

/// An item or monster name less the icon OCR can read as a mark before it.
fn clean_name(text: &str) -> String {
    let text = text.trim();
    let start = text.find(|c: char| c.is_alphanumeric() || c == '[').unwrap_or(0);
    text[start..].trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);
    const RULE: Rgba<u8> = Rgba([204, 0, 0, 255]);
    const PINK: Rgba<u8> = Rgba([238, 102, 136, 255]);
    const GREY_TAB: Rgba<u8> = Rgba([187, 187, 187, 255]);
    const TAN: Rgba<u8> = Rgba([176, 148, 109, 255]);
    const SLATE: Rgba<u8> = Rgba([156, 176, 200, 255]);
    const PICKED: Rgba<u8> = Rgba([51, 102, 153, 255]);
    const INK: Rgba<u8> = Rgba([51, 51, 51, 255]);
    const TRACK_GREY: Rgba<u8> = Rgba([231, 231, 231, 255]);
    const THUMB: Rgba<u8> = Rgba([51, 102, 136, 255]);
    const BLUE_BOX: Rgba<u8> = Rgba([68, 136, 187, 255]);

    fn fill(frame: &mut Frame, (x, y, w, h): (u32, u32, u32, u32), colour: Rgba<u8>) {
        for row in y..y + h {
            for col in x..x + w {
                frame.put_pixel(col, row, colour);
            }
        }
    }

    /// A made-up Quest window, hand-drawn in the window's flat colours: the rule at (40, 100),
    /// the `active` tab pink, and the slots given, top to bottom.
    fn window(active: usize, slots: &[Slot]) -> Frame {
        let mut frame = Frame::from_pixel(1300, 900, Rgba([90, 120, 60, 255]));
        fill(&mut frame, (30, 40, 500, 760), WHITE);
        for tab in 0..TABS as u32 {
            let colour = if tab as usize == active { PINK } else { GREY_TAB };
            fill(&mut frame, (44 + tab * 145, 66, 138, 30), colour);
        }
        fill(&mut frame, (40, 100, 478, 4), RULE);
        fill(&mut frame, (40 + 450, 100 + 38, 24, 575), TRACK_GREY);
        for (i, slot) in slots.iter().enumerate() {
            let top = 100 + 20 + i as u32 * 42;
            match slot {
                Slot::Region => fill(&mut frame, (52, top, 422, 36), if i % 2 == 0 { TAN } else { SLATE }),
                Slot::Picked => fill(&mut frame, (110, top, 364, 36), PICKED),
                Slot::Row => fill(&mut frame, (120, top + 10, 120, 16), INK),
                Slot::Empty => {}
            }
        }
        frame
    }

    #[test]
    fn the_window_is_the_rule_with_a_pink_tab_on_it_and_the_pink_one_is_the_tab_shown() {
        for (active, tab) in QuestTab::ALL.into_iter().enumerate() {
            let anchor = find(&window(active, &[])).expect("found");
            assert_eq!((anchor.left, anchor.top, anchor.tab), (40.0, 100.0, tab));
        }
    }

    #[test]
    fn a_window_drawn_at_another_ui_size_is_put_back_on_the_grid() {
        let big = window(2, &[Slot::Region, Slot::Row, Slot::Picked]);
        let small = Frame::from_fn(big.width() / 2, big.height() / 2, |x, y| *big.get_pixel(x * 2, y * 2));
        assert!(find(&small).is_none(), "half the size, as drawn");
        let (view, anchor) = locate(&small, None).expect("found once redrawn");
        let view = view.expect("redrawn");
        assert_eq!(anchor.tab, QuestTab::Completed);
        let kinds: Vec<Slot> = (0..3).map(|i| slot_kind(&view, &anchor, i)).collect();
        assert_eq!(kinds, [Slot::Region, Slot::Row, Slot::Picked]);
    }

    #[test]
    fn a_red_line_alone_or_of_the_wrong_length_is_not_the_window() {
        let mut frame = Frame::from_pixel(800, 400, WHITE);
        fill(&mut frame, (40, 100, 478, 4), RULE);
        assert!(find(&frame).is_none(), "no tab on it");
        let mut frame = Frame::from_pixel(800, 400, WHITE);
        fill(&mut frame, (40, 100, 300, 4), RULE);
        fill(&mut frame, (44, 66, 138, 30), PINK);
        assert!(find(&frame).is_none(), "too short");
        assert!(locate(&frame, None).is_none(), "nor stretched to fit at a size the game doesn't draw");
        let mut frame = Frame::from_pixel(1300, 900, WHITE);
        fill(&mut frame, (40, 100, 478, 4), RULE);
        fill(&mut frame, (44, 66, 138, 30), PINK);
        assert!(find(&frame).is_none(), "tabs on a rule with no list's scroll track: another window");
        assert!(!any_rule(&Frame::from_pixel(800, 400, WHITE)));
    }

    #[test]
    fn slots_tell_bars_from_rows_and_the_picked_row() {
        let slots = [Slot::Region, Slot::Picked, Slot::Region, Slot::Row, Slot::Row, Slot::Empty];
        let frame = window(1, &slots);
        let anchor = find(&frame).unwrap();
        let kinds: Vec<Slot> = (0..slots.len()).map(|i| slot_kind(&frame, &anchor, i)).collect();
        assert_eq!(kinds, slots);
    }

    #[test]
    fn a_list_that_fits_shows_its_top_and_end_and_a_thumb_says_where_it_is() {
        let mut frame = window(2, &[Slot::Region]);
        let anchor = find(&frame).unwrap();
        assert_eq!(scroll(&frame, &anchor), Scroll { at_top: true, at_end: true });
        fill(&mut frame, (490, 138, 24, 48), THUMB);
        assert_eq!(scroll(&frame, &anchor), Scroll { at_top: true, at_end: false });
        fill(&mut frame, (490, 138, 24, 48), TRACK_GREY);
        fill(&mut frame, (490, 400, 24, 48), THUMB);
        assert_eq!(scroll(&frame, &anchor), Scroll { at_top: false, at_end: false });
        fill(&mut frame, (490, 400, 24, 48), TRACK_GREY);
        fill(&mut frame, (490, 665, 24, 48), THUMB);
        assert_eq!(scroll(&frame, &anchor), Scroll { at_top: false, at_end: true });
    }

    #[test]
    fn the_detail_box_is_found_beside_the_list() {
        let mut frame = window(1, &[Slot::Region]);
        let anchor = find(&frame).unwrap();
        assert!(detail_box(&frame, &anchor).is_none(), "pane closed");
        fill(&mut frame, (548, 66, 563, 152), BLUE_BOX);
        let found = detail_box(&frame, &anchor).unwrap();
        assert_eq!((found.x, found.y, found.w), (548.0, 66.0, 563.0));
    }

    #[test]
    fn region_bars_read_as_a_name_and_count_even_with_the_digit_misread() {
        assert_eq!(parse_region("Henesys (3)"), Some(("Henesys".into(), Some(3))));
        assert_eq!(parse_region("Citizenship (19)"), Some(("Citizenship".into(), Some(19))));
        assert_eq!(parse_region("Kerning City (l)"), Some(("Kerning City".into(), Some(1))));
        assert_eq!(parse_region("Event (1"), Some(("Event".into(), Some(1))));
        assert_eq!(parse_region("Sleepywood"), Some(("Sleepywood".into(), None)));
    }

    #[test]
    fn a_title_cut_short_loses_its_dots_and_says_so() {
        assert_eq!(untruncated("Asking After Rina, the Town Resi...", false), ("Asking After Rina, the Town Resi".into(), true));
        assert_eq!(untruncated("[Forest of Patience] Sabitra…", false), ("[Forest of Patience] Sabitra".into(), true));
        assert_eq!(untruncated("Trick or Treat!", false), ("Trick or Treat!".into(), false));
        assert_eq!(untruncated("Mr. Wetbottom's Secret Book", false), ("Mr. Wetbottom's Secret Book".into(), false));
        assert_eq!(untruncated("The Marks We Leave Behi", true), ("The Marks We Leave Behi".into(), true), "dots seen, not read");
    }

    #[test]
    fn three_dots_on_the_baseline_are_a_cut() {
        let mut frame = Frame::from_pixel(200, 40, WHITE);
        fill(&mut frame, (10, 8, 30, 20), INK);
        for i in 0..3 {
            fill(&mut frame, (50 + i * 8, 24, 4, 4), INK);
        }
        let area = Rect::new(0.0, 0.0, 200.0, 40.0);
        assert!(ends_in_dots(&frame, area, is_grey_ink));
        let mut plain = Frame::from_pixel(200, 40, WHITE);
        fill(&mut plain, (10, 8, 30, 20), INK);
        fill(&mut plain, (50, 8, 4, 20), INK);
        assert!(!ends_in_dots(&plain, area, is_grey_ink));
    }

    #[test]
    fn row_icons_are_told_apart_by_colour() {
        let area = Rect::new(0.0, 0.0, 40.0, 40.0);
        let icon = |tile: bool, colour| {
            let mut frame = Frame::from_pixel(40, 40, WHITE);
            if tile {
                fill(&mut frame, (6, 6, 24, 24), Rgba([238, 238, 238, 255]));
            }
            fill(&mut frame, (10, 10, 8, 8), colour);
            icon_in(&frame, area)
        };
        assert_eq!(icon(false, Rgba([238, 170, 0, 255])), RowIcon::Bang);
        assert_eq!(icon(true, Rgba([51, 187, 238, 255])), RowIcon::Ring);
        assert_eq!(icon(false, Rgba([51, 187, 238, 255])), RowIcon::Unknown, "the ring's blue without its tile");
        assert_eq!(icon(false, Rgba([0, 85, 170, 255])), RowIcon::Arrow);
        assert_eq!(icon(true, Rgba([204, 204, 204, 255])), RowIcon::GreyRing);
        assert_eq!(icon(false, INK), RowIcon::Unknown);
    }

    #[test]
    fn the_rows_text_is_flat_grey_and_icons_edges_are_not() {
        assert!(is_grey_ink(&INK));
        assert!(is_grey_ink(&Rgba([102, 102, 102, 255])));
        assert!(!is_grey_ink(&Rgba([34, 68, 119, 255])), "an arrow's dark edge");
        assert!(!is_grey_ink(&Rgba([153, 153, 153, 255])), "the checkbox");
        assert!(!is_grey_ink(&WHITE));
    }

    #[test]
    fn detail_lines_parse() {
        let level = LEVEL.captures("Level 12+").unwrap();
        assert_eq!(number(&level[1]), Some(12));
        assert_eq!(completed_on("Completed on 10/25/2026..."), Some("2026-10-25".into()));
        assert_eq!(number(&LEVEL.captures("Level 1 0+").unwrap()[1]), Some(10));
        let counter = COUNTER.captures("Maple Leaf 30 / 30").unwrap();
        assert_eq!((&counter[1], number(&counter[2]), number(&counter[3])), ("Maple Leaf", Some(30), Some(30)));
        let split = COUNTER.captures("Snail Shell 1 0 / 1 0").unwrap();
        assert_eq!((&split[1], number(&split[2]), number(&split[3])), ("Snail Shell", Some(10), Some(10)));
        assert!(COUNTER.captures("Maple Leaf 30").is_none(), "one number is no counter");
        assert!(COUNTER_NO_SLASH.captures("Maple Leaf 30").is_none());
        let slash_as_r = COUNTER.captures("Maple Leaf 30 r 30").unwrap();
        assert_eq!((number(&slash_as_r[2]), number(&slash_as_r[3])), (Some(30), Some(30)));
        let bare = COUNTER_NO_SLASH.captures("Maple Leaf 30 30").unwrap();
        assert_eq!((number(&bare[2]), number(&bare[3])), (Some(30), Some(30)));
        assert!(COUNTER.captures("Lv. 10 and above").is_none());
        assert_eq!(completed_on("Completed on 10!25$2026..."), Some("2026-10-25".into()));
        assert_eq!(completed_on("Completed on 13/25/2026"), None);
        assert_eq!(&TALK.captures("! Talk to NPC Community Board").unwrap()[1], "Community Board");
        let item = ITEM.captures("Adventurer's Journal x1").unwrap();
        assert_eq!((&item[1], number(&item[2])), ("Adventurer's Journal", Some(1)));
        assert_eq!(number(&EXP.captures("1,200 EXP").unwrap()[1]), Some(1200));
        assert_eq!(number(&MESOS.captures("500 mesos").unwrap()[1]), Some(500));
    }

    #[test]
    fn the_green_dot_before_a_detail_title_is_dropped() {
        assert_eq!(trim_bullet("o [Event] The Journey Be..."), "[Event] The Journey Be...");
        assert_eq!(trim_bullet("• Asking After Rina, the..."), "Asking After Rina, the...");
        assert_eq!(trim_bullet("Arwen and the Glass Shoe"), "Arwen and the Glass Shoe");
        assert_eq!(trim_bullet("O Snail"), "O Snail", "a word, not a dot, before a capital is kept");
    }

    #[test]
    fn a_glance_changes_with_the_tab_and_a_read_happens_once_it_settles() {
        let frame = window(1, &[Slot::Region, Slot::Row]);
        let first = glance(&frame, None).unwrap();
        assert_eq!(first.tab, QuestTab::InProgress);
        let other = glance(&window(2, &[Slot::Region, Slot::Row]), None).unwrap();
        assert_ne!(first, other);
        let mut watch = Watch::default();
        assert!(!watch.settled(Some(first.clone())));
        assert!(watch.settled(Some(first.clone())));
        assert!(!watch.settled(Some(first.clone())), "already read");
        assert!(!watch.settled(None));
        assert!(!watch.settled(Some(first.clone())));
        assert!(watch.settled(Some(first)), "opened again, read again");
        assert!(glance(&Frame::from_pixel(600, 400, WHITE), None).is_none());
    }
}
