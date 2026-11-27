//! The Character Info window, for the player's own character's look: the picture of them standing
//! in it becomes their avatar in Leafline. The window opens for other players too, so its name is
//! read first, and the picture is cut out only when that name is the HUD's own. Nothing of another
//! player's window outlives the frame it was in.
//!
//! The window is found by appearance (it can be dragged anywhere): four pink label cells stacked
//! at an even pitch (LEVEL, JOB, FAME, GUILD) with a pale bordered box to their left that holds
//! the character, and the name in a box under that. The box's ground is keyed out from colours
//! sampled along its inside edges, which leaves the character on a transparent ground; the round
//! guild badge in the box's top-right corner is left out. The picture is kept at the game's own
//! pixel size (one pixel per design pixel), so it can be drawn crisply at any size.
//!
//! Measured on frames of the Character Stat window, whose pink labels look the same; the Character
//! Info window itself is still to be checked on real frames (2026-11-26).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Cursor;

use base64::Engine as _;
use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};

use super::gamefont;
use super::hud::HudLayout;
use super::pickup::font::{UNKNOWN, read_glyphs};
use super::ui::{self, CANONICAL_SCALE, Canonical};
use crate::vision::{Frame, Ocr, Rect, color};

// All distances are in pixels at CANONICAL_SCALE (two per design pixel).

/// The label cells' pink, as the Character Stat window draws it.
const LABEL: [u8; 3] = [203, 79, 130];
const LABEL_TOLERANCE: u8 = 12;
const LABELS: usize = 4;
const MIN_LABEL_RUN: u32 = 30;
const LABEL_HEIGHT: (f32, f32) = (18.0, 40.0);
const LABEL_WIDTH: (f32, f32) = (50.0, 160.0);
/// How far apart stacked labels can stand.
const PITCH: (f32, f32) = (22.0, 64.0);
const SLACK: f32 = 3.0;
/// The avatar box stands left of the labels: its right edge at most this far from them.
const BOX_GAP: f32 = 90.0;
/// The box is at least this wide and tall, and at most this wide.
const MIN_BOX_W: u32 = 80;
const MAX_BOX_W: u32 = 360;
const MIN_BOX_H: u32 = 110;
/// The box may reach this far above the first label and below the last.
const BOX_REACH: f32 = 140.0;
/// Two neighbouring pixels this different (summed over red, green and blue) make an edge.
const EDGE: u32 = 60;
/// A border is at most this thick, from the box's outer edge to its ground.
const MAX_BORDER: u32 = 16;
/// The name box, at the avatar box's foot: this tall, and filled this much by its own colour (the
/// letters take the rest).
const MIN_NAME_BOX: u32 = 16;
const MAX_NAME_BOX: u32 = 120;
const NAME_FILL: f32 = 0.55;
/// How far one shade of a gradient can stand from the next few.
const SHADE: u8 = 8;

/// Pixels of the box's ground this close (any channel) to a colour sampled at its edges are keyed
/// out when they join the edge; anywhere in the box when this close to the main one.
const KEY_TOLERANCE: u8 = 16;
const HOLE_TOLERANCE: u8 = 3;
/// How far from the edge colours a shaded ground can drift and still be followed.
const GRADIENT_REACH: u8 = 48;
/// The share of the character's height, from the bottom, where the shadow under their feet lies.
const SHADOW_BAND: u32 = 15;
/// A colour counts as part of the ground with this share of the edge pixels.
const GROUND_SHARE: f32 = 0.04;
/// The ground's main colour must cover this share of the box's inside edge.
const MIN_GROUND: f32 = 0.3;
/// A picture this big (design pixels) can't be a character.
const MAX_LOOK: u32 = 200;
/// Every this many pixels, a picture is checked for the label pink before anything else.
const LOOK_STEP: usize = 6;

/// Where the window's parts stand, in the view it was found in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InfoWindow {
    pub labels: [Rect; LABELS],
    /// The avatar box's ground, inside its border.
    pub portrait: Rect,
    /// Where the name is written, under the avatar box.
    pub name: Rect,
}

/// The name under the avatar as read: by the game's font (unknown glyphs as `?`) and by OCR.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NameRead {
    pub glyphs: Option<String>,
    pub ocr: Option<String>,
}

/// The player's own character as the window shows them, on a transparent ground, one pixel per
/// design pixel.
#[derive(Debug, Clone, PartialEq)]
pub struct Portrait {
    pub image: RgbaImage,
    /// The box's picture before keying, so a still one can be told from a changing one (a cursor
    /// over it, the character animating).
    pub key: u64,
}

/// A character's look to keep: whose, and the picture as PNG in base64.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeenLook {
    pub name: String,
    pub png: String,
    pub width: u32,
    pub height: u32,
}

impl Portrait {
    /// The picture as a look to keep for `name`; `None` if it won't encode.
    pub fn seen(&self, name: &str) -> Option<SeenLook> {
        let mut bytes = Vec::new();
        self.image.write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png).ok()?;
        Some(SeenLook {
            name: name.to_string(),
            png: base64::engine::general_purpose::STANDARD.encode(bytes),
            width: self.image.width(),
            height: self.image.height(),
        })
    }
}

/// Decides when to keep the look: once per opening of the window, at a picture seen twice (a still
/// one, or a frame of its animation that came round again), so a cursor passing over it or a
/// half-drawn frame isn't kept. Only ever holds the player's own window's keys.
#[derive(Debug, Default)]
pub struct LookWatch {
    seen: Vec<u64>,
    kept: bool,
}

/// Pictures of one opening remembered, to see one come round again.
const REMEMBERED: usize = 8;

impl LookWatch {
    /// Takes this read's portrait, `None` when the window isn't open or isn't the player's own,
    /// and gives it back when it is the one to keep.
    pub fn follow(&mut self, portrait: Option<Portrait>) -> Option<Portrait> {
        let Some(portrait) = portrait else {
            *self = Self::default();
            return None;
        };
        let again = self.seen.contains(&portrait.key);
        if !again {
            if self.seen.len() == REMEMBERED {
                self.seen.remove(0);
            }
            self.seen.push(portrait.key);
        }
        if again && !self.kept {
            self.kept = true;
            return Some(portrait);
        }
        None
    }
}

/// The player's own character from the Character Info window in `frame`, when it is open and its
/// name is `own_name` (the HUD's). `ocr` reads the name where the game's font can't.
pub fn read(frame: &Frame, hud: Option<&HudLayout>, own_name: &str, ocr: Option<&Ocr>) -> Option<Portrait> {
    read_with(frame, hud, own_name, |view, area| read_name(view, area, ocr))
}

/// [`read`] with the name read by `name_of` (for tests).
pub fn read_with(
    frame: &Frame,
    hud: Option<&HudLayout>,
    own_name: &str,
    name_of: impl Fn(&Frame, Rect) -> NameRead,
) -> Option<Portrait> {
    if own_name.trim().is_empty() || !any_label(frame) {
        return None;
    }
    let scales: Vec<f32> = match hud {
        Some(layout) => vec![ui::hud_scale(frame, layout)],
        None => ui::scales(frame, None),
    };
    for scale in scales {
        let exact = (scale - CANONICAL_SCALE).abs() <= 0.005;
        let canonical = (!exact).then(|| Canonical::new(frame, scale));
        let view = canonical.as_ref().map_or(frame, |c| &c.frame);
        for window in find(view) {
            // The name first: another player's window goes no further.
            if !names_match(&name_of(view, window.name), own_name) {
                continue;
            }
            let in_window = canonical.as_ref().map_or(window.portrait, |c| c.to_window(window.portrait));
            return cut(frame, scale, in_window);
        }
    }
    None
}

/// Whether anything in `frame` has the labels' pink.
fn any_label(frame: &Frame) -> bool {
    (0..frame.height()).step_by(LOOK_STEP).any(|y| (0..frame.width()).step_by(LOOK_STEP).any(|x| is_label(frame.get_pixel(x, y))))
}

fn near(px: &Rgba<u8>, colour: [u8; 3], tolerance: u8) -> bool {
    px.0.iter().zip(colour).all(|(a, b)| a.abs_diff(b) <= tolerance)
}

fn is_label(px: &Rgba<u8>) -> bool {
    near(px, LABEL, LABEL_TOLERANCE)
}

fn luminance(px: &Rgba<u8>) -> u8 {
    let [r, g, b, _] = px.0;
    ((u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114) / 1000) as u8
}

fn rgb(px: &Rgba<u8>) -> [u8; 3] {
    [px.0[0], px.0[1], px.0[2]]
}

fn distance(a: [u8; 3], b: [u8; 3]) -> u32 {
    a.iter().zip(b).map(|(p, q)| u32::from(p.abs_diff(q))).sum()
}

fn close(a: [u8; 3], b: [u8; 3], tolerance: u8) -> bool {
    a.iter().zip(b).all(|(p, q)| p.abs_diff(q) <= tolerance)
}

/// Every Character Info window `view` shows (at most a few: the player's own and another's).
pub fn find(view: &Frame) -> Vec<InfoWindow> {
    label_stacks(view).into_iter().filter_map(|labels| window_at(view, labels)).collect()
}

/// Four pink cells of a size, one above the other at an even pitch, top first. The Character Stat
/// window's labels make stacks too; it has no avatar box beside them.
fn label_stacks(view: &Frame) -> Vec<[Rect; LABELS]> {
    let whole = Rect::new(0.0, 0.0, view.width() as f32, view.height() as f32);
    let mut blocks = color::find_blocks(view, whole, MIN_LABEL_RUN, is_label);
    blocks.retain(|b| (LABEL_HEIGHT.0..=LABEL_HEIGHT.1).contains(&b.h) && (LABEL_WIDTH.0..=LABEL_WIDTH.1).contains(&b.w));
    blocks.sort_by(|a, b| a.y.total_cmp(&b.y));
    let mut stacks = Vec::new();
    for (i, first) in blocks.iter().enumerate() {
        let column: Vec<Rect> = blocks[i + 1..]
            .iter()
            .filter(|b| (b.x - first.x).abs() <= SLACK && (b.w - first.w).abs() <= 2.0 * SLACK)
            .copied()
            .collect();
        let Some(second) = column.first() else { continue };
        let pitch = second.y - first.y;
        if !(PITCH.0..=PITCH.1).contains(&pitch) {
            continue;
        }
        let at = |k: usize| column.iter().find(|b| (b.y - (first.y + k as f32 * pitch)).abs() <= SLACK).copied();
        if let (Some(third), Some(fourth)) = (at(2), at(3)) {
            stacks.push([*first, *second, third, fourth]);
        }
    }
    stacks
}

/// The longest run of rows from `top` to `bottom` where column `x` and the one right of it differ:
/// a vertical edge. Its first row and the row after its last.
fn edge_run(view: &Frame, x: u32, top: u32, bottom: u32) -> (u32, u32) {
    let mut best = (top, top);
    let mut start = None;
    for y in top..bottom {
        let edge = distance(rgb(view.get_pixel(x, y)), rgb(view.get_pixel(x + 1, y))) >= EDGE;
        match (edge, start) {
            (true, None) => start = Some(y),
            (false, Some(s)) => {
                if y - s > best.1 - best.0 {
                    best = (s, y);
                }
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start
        && bottom - s > best.1 - best.0
    {
        best = (s, bottom);
    }
    best
}

/// The window around a stack of labels, when an avatar box stands to their left.
fn window_at(view: &Frame, labels: [Rect; LABELS]) -> Option<InfoWindow> {
    let (width, height) = (view.width(), view.height());
    let left_of_labels = labels[0].x.floor() as i64 - 2;
    let top = (labels[0].y - BOX_REACH).max(0.0) as u32;
    let bottom = (labels[LABELS - 1].bottom() + BOX_REACH).min(height as f32) as u32;
    if left_of_labels < MIN_BOX_W as i64 || bottom <= top + MIN_BOX_H {
        return None;
    }
    let long = |run: (u32, u32)| run.1 - run.0 >= MIN_BOX_H;
    // The box's right edge: the first long vertical edge left of the labels.
    let nearest = (left_of_labels - BOX_GAP as i64).max(0) as u32;
    let right = (nearest..left_of_labels as u32).rev().find_map(|x| {
        let run = edge_run(view, x, top, bottom);
        long(run).then_some((x, run))
    })?;
    // Its left edge: the next one leftwards that spans about the same rows (a long straight edge
    // in the character, a staff say, spans fewer).
    let farthest = right.0.saturating_sub(MAX_BOX_W);
    let right_len = right.1.1 - right.1.0;
    let left = (farthest..right.0.saturating_sub(MIN_BOX_W)).rev().find_map(|x| {
        let run = edge_run(view, x, top, bottom);
        let shared = run.1.min(right.1.1).saturating_sub(run.0.max(right.1.0));
        (long(run) && shared * 5 >= right_len * 4).then_some((x, run))
    })?;
    // The guild badge can cut the right edge short at the top, so the left edge gives the height.
    let outer = Rect::from_edges(
        (left.0 + 1) as f32,
        left.1.0.min(right.1.0) as f32,
        (right.0 + 1) as f32,
        left.1.1.max(right.1.1) as f32,
    );
    let (portrait, name) = parts_of(view, outer)?;
    (outer.right() <= width as f32).then_some(InfoWindow { labels, portrait, name })
}

/// The most common colour among `pixels`, and its share.
fn commonest(pixels: impl Iterator<Item = [u8; 3]>) -> Option<([u8; 3], f32)> {
    let mut counts: Vec<([u8; 3], usize)> = Vec::new();
    let mut total = 0;
    for px in pixels {
        total += 1;
        match counts.iter_mut().find(|(c, _)| *c == px) {
            Some((_, n)) => *n += 1,
            None => counts.push((px, 1)),
        }
    }
    let (colour, n) = counts.into_iter().max_by_key(|(_, n)| *n)?;
    Some((colour, n as f32 / total as f32))
}

/// The box's ground inside its border, and the name box at its foot. Each side moves in past
/// lines that aren't the ground's colour; the name box is the tallest band of rows near the bottom
/// that one colour other than the ground fills (white letters on grey in the game). `None` when the
/// inside isn't mostly one colour, so it isn't the avatar box, or has no name box.
fn parts_of(view: &Frame, outer: Rect) -> Option<(Rect, Rect)> {
    let (x0, y0, w, h) = outer.to_pixels(view.width(), view.height());
    if w < MIN_BOX_W || h < MIN_BOX_H {
        return None;
    }
    let (x1, y1) = (x0 + w - 1, y0 + h - 1);
    let ring = MAX_BORDER;
    // The ground's colour, from the top two thirds of a ring well inside the border (the name box
    // fills the foot).
    let low = y0 + 2 * h / 3;
    let ring_pixels = (x0 + ring..=x1 - ring)
        .map(|x| (x, y0 + ring))
        .chain((y0 + ring..=low).flat_map(|y| [(x0 + ring, y), (x1 - ring, y)]))
        .map(|(x, y)| rgb(view.get_pixel(x, y)));
    let (ground, share) = commonest(ring_pixels)?;
    if share < MIN_GROUND {
        return None;
    }
    // A line is the ground once most of its middle is the ground's colour.
    let is_ground = |pixels: Vec<[u8; 3]>| {
        let n = pixels.len().max(1);
        pixels.into_iter().filter(|&px| close(px, ground, KEY_TOLERANCE)).count() * 2 >= n
    };
    let (mid_x, mid_y) = ((x0 + w / 4)..(x1 - w / 4), (y0 + h / 4)..(y0 + h / 2));
    let column = |x: u32| is_ground(mid_y.clone().map(|y| rgb(view.get_pixel(x, y))).collect());
    let row = |y: u32| is_ground(mid_x.clone().map(|x| rgb(view.get_pixel(x, y))).collect());
    let left = (x0..x0 + ring).find(|&x| column(x))?;
    let right = (x1 - ring..=x1).rev().find(|&x| column(x))?;
    let top = (y0..y0 + ring).find(|&y| row(y))?;
    // The name box: rows across the box's width that one colour other than the ground fills.
    let across = (x0 + ring)..(x1 - ring);
    // The game shades it top to bottom, so a row counts the shades near its commonest.
    let boxed = |y: u32| {
        let pixels: Vec<[u8; 3]> = across.clone().map(|x| rgb(view.get_pixel(x, y))).collect();
        commonest(pixels.iter().copied()).is_some_and(|(colour, _)| {
            let near = pixels.iter().filter(|&&px| close(px, colour, SHADE)).count();
            near as f32 >= NAME_FILL * pixels.len() as f32 && !close(colour, ground, KEY_TOLERANCE)
        })
    };
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for y in y1.saturating_sub(MAX_NAME_BOX)..=y1 {
        if !boxed(y) {
            continue;
        }
        match runs.last_mut() {
            Some(run) if run.1 + 1 == y => run.1 = y,
            _ => runs.push((y, y)),
        }
    }
    let (name_top, name_bottom) = runs.into_iter().filter(|(t, b)| b - t + 1 >= MIN_NAME_BOX).max_by_key(|(t, b)| b - t)?;
    if name_top <= top + MIN_BOX_H / 2 {
        return None;
    }
    let portrait = Rect::from_edges(left as f32, top as f32, (right + 1) as f32, name_top as f32);
    let name = Rect::from_edges(left as f32, name_top as f32, (right + 1) as f32, (name_bottom + 1) as f32);
    Some((portrait, name))
}

/// The name under the avatar, read by the game's font and by OCR.
pub fn read_name(view: &Frame, area: Rect, ocr: Option<&Ocr>) -> NameRead {
    let (x0, y0, w, h) = area.to_pixels(view.width(), view.height());
    if w == 0 || h == 0 {
        return NameRead::default();
    }
    let Some((ground, _)) = commonest((y0..y0 + h).flat_map(|y| (x0..x0 + w).map(move |x| (x, y))).map(|(x, y)| rgb(view.get_pixel(x, y))))
    else {
        return NameRead::default();
    };
    // Light letters on a dark box, or dark on light: by the box's middle brightness. A row the ink
    // spans most of is a rule along the box's edge, not letters.
    let mut lums: Vec<u8> = (y0..y0 + h).flat_map(|y| (x0..x0 + w).map(move |x| (x, y))).map(|(x, y)| luminance(view.get_pixel(x, y))).collect();
    lums.sort_unstable();
    let dark_box = lums[lums.len() / 2] < 160;
    let ink = move |px: &Rgba<u8>| {
        let (l, [r, g, b]) = (luminance(px), rgb(px));
        let grey = r.max(g).max(b) - r.min(g).min(b) <= 40;
        if dark_box { l >= 230 && grey } else { l <= 70 }
    };
    let inked = |y: u32| {
        let n = (x0..x0 + w).filter(|&x| ink(view.get_pixel(x, y))).count() as u32;
        n > 0 && n * 10 < w * 6
    };
    // The tallest run of inked rows is the name (a box's border is one or two rows).
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for y in y0..y0 + h {
        if !inked(y) {
            continue;
        }
        match runs.last_mut() {
            Some(run) if y - run.1 <= 2 => run.1 = y + 1,
            _ => runs.push((y, y + 1)),
        }
    }
    let Some(&(top, bottom)) = runs.iter().max_by_key(|(t, b)| b - t) else { return NameRead::default() };
    if bottom - top < 8 {
        return NameRead::default();
    }
    let band = Rect::from_edges(area.x, top as f32 - 6.0, area.right(), bottom as f32 + 6.0);
    let glyphs = gamefont::ink_grid(view, band, 2, ink)
        .and_then(|grid| gamefont::line_rows(&NAME_FONT, &grid, grid.len() / 2))
        .map(|rows| gamefont::first_stretch(&rows, 12))
        .map(|rows| read_glyphs(&rows))
        .filter(|text| !text.is_empty());
    let ocr = ocr.and_then(|ocr| ocr_name(view, band, ground, ocr));
    NameRead { glyphs, ocr }
}

/// The font the HUD's name is drawn in, whose glyphs the Pickup Log's reader knows.
const NAME_FONT: gamefont::Font = gamefont::Font { glyphs: &[], rows: 11, baseline: 8, space: 4, narrow: &[] };

/// OCR reads a small line best enlarged, with a margin, as dark text on light.
const OCR_SCALE: u32 = 2;
const OCR_PAD: u32 = 24;

fn ocr_name(view: &Frame, band: Rect, ground: [u8; 3], ocr: &Ocr) -> Option<String> {
    let (x0, y0, w, h) = band.to_pixels(view.width(), view.height());
    let dark_ground = ground.iter().map(|&c| u32::from(c)).sum::<u32>() < 3 * 128;
    let (pw, ph) = (w * OCR_SCALE + 2 * OCR_PAD, h * OCR_SCALE + 2 * OCR_PAD);
    let picture = Frame::from_fn(pw, ph, |x, y| {
        let inside = x >= OCR_PAD && y >= OCR_PAD && x < pw - OCR_PAD && y < ph - OCR_PAD;
        let px = if inside {
            *view.get_pixel(x0 + (x - OCR_PAD) / OCR_SCALE, y0 + (y - OCR_PAD) / OCR_SCALE)
        } else {
            Rgba([ground[0], ground[1], ground[2], 255])
        };
        if dark_ground { Rgba([255 - px.0[0], 255 - px.0[1], 255 - px.0[2], 255]) } else { px }
    });
    let words = ocr.words(&picture).ok()?;
    let text: String = words.iter().flat_map(|w| w.text.chars()).filter(|c| c.is_ascii_alphanumeric()).collect();
    (!text.is_empty()).then_some(text)
}

/// Whether the name read is `own` (the HUD's name). The game's font, where it reads every glyph,
/// must match exactly (letters' case aside); otherwise OCR's reading must, with the letters it
/// mixes up (`0` and `O`, `1`, `I` and `l`) taken as one, and where the font did read a glyph it
/// must agree.
pub fn names_match(read: &NameRead, own: &str) -> bool {
    let own = own.trim();
    if own.is_empty() {
        return false;
    }
    if let Some(glyphs) = &read.glyphs
        && !glyphs.contains(UNKNOWN)
    {
        return glyphs.eq_ignore_ascii_case(own);
    }
    let Some(ocr) = &read.ocr else { return false };
    if fold(ocr) != fold(own) {
        return false;
    }
    match &read.glyphs {
        Some(glyphs) if glyphs.chars().count() == own.chars().count() => {
            glyphs.chars().zip(own.chars()).all(|(g, o)| g == UNKNOWN || g.eq_ignore_ascii_case(&o))
        }
        _ => true,
    }
}

/// A name with OCR's usual mix-ups taken as one letter.
fn fold(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| match c.to_ascii_lowercase() {
            '0' => 'o',
            '1' | 'i' => 'l',
            '5' => 's',
            c => c,
        })
        .collect()
}

/// The character in `ground` (the avatar box's ground, in window pixels at `scale`) on a
/// transparent ground: the box redrawn one pixel per design pixel, the ground keyed out from the
/// colours along its edges, the guild badge left out and the rest trimmed.
pub fn cut(frame: &Frame, scale: f32, ground: Rect) -> Option<Portrait> {
    let drawn = Canonical::within(frame, scale, ground);
    let inside = drawn.to_view(ground);
    let block = CANONICAL_SCALE as u32;
    let (x0, y0, w, h) = inside.to_pixels(drawn.frame.width(), drawn.frame.height());
    let (x0, y0) = (x0.div_ceil(block) * block, y0.div_ceil(block) * block);
    let (cols, rows) = (w / block, h / block);
    if cols < 8 || rows < 8 {
        return None;
    }
    let picture = RgbaImage::from_fn(cols, rows, |x, y| {
        let (px, py) = ((x0 + x * block).min(drawn.frame.width() - 1), (y0 + y * block).min(drawn.frame.height() - 1));
        let [r, g, b, _] = drawn.frame.get_pixel(px, py).0;
        Rgba([r, g, b, 255])
    });
    let key = {
        let mut hasher = DefaultHasher::new();
        picture.as_raw().hash(&mut hasher);
        hasher.finish()
    };
    let mut image = key_out(picture)?;
    drop_badge(&mut image);
    let image = trim(&image)?;
    (image.width() <= MAX_LOOK && image.height() <= MAX_LOOK).then_some(Portrait { image, key })
}

/// The picture with its ground transparent: colours common along its edges, wherever they join
/// the edge, and the main one also where it shows through inside (between an arm and the body).
fn key_out(mut picture: RgbaImage) -> Option<RgbaImage> {
    let (w, h) = picture.dimensions();
    let edge: Vec<(u32, u32)> =
        (0..w).flat_map(|x| [(x, 0), (x, h - 1)]).chain((0..h).flat_map(|y| [(0, y), (w - 1, y)])).collect();
    let mut counts: Vec<([u8; 3], usize)> = Vec::new();
    for &(x, y) in &edge {
        let px = rgb(picture.get_pixel(x, y));
        match counts.iter_mut().find(|(c, _)| *c == px) {
            Some((_, n)) => *n += 1,
            None => counts.push((px, 1)),
        }
    }
    counts.sort_by_key(|c| std::cmp::Reverse(c.1));
    let main = counts.first()?.0;
    let palette: Vec<[u8; 3]> =
        counts.iter().filter(|(_, n)| *n as f32 >= GROUND_SHARE * edge.len() as f32).map(|(c, _)| *c).collect();
    let is_ground = |px: [u8; 3]| palette.iter().any(|&c| close(px, c, KEY_TOLERANCE));
    // The game shades the ground (lighter towards the top), so the fill also follows small steps
    // from one ground pixel to the next while they stay pale like the ground. The character's dark
    // outline stops it.
    let pale = |px: [u8; 3]| palette.iter().any(|&c| close(px, c, GRADIENT_REACH));
    let mut clear = vec![false; (w * h) as usize];
    let mut stack: Vec<(u32, u32, [u8; 3])> = edge
        .into_iter()
        .map(|(x, y)| (x, y, rgb(picture.get_pixel(x, y))))
        .filter(|&(_, _, px)| is_ground(px))
        .collect();
    while let Some((x, y, from)) = stack.pop() {
        let i = (y * w + x) as usize;
        let px = rgb(picture.get_pixel(x, y));
        if clear[i] || !(is_ground(px) || (close(px, from, SHADE) && pale(px))) {
            continue;
        }
        clear[i] = true;
        if x > 0 {
            stack.push((x - 1, y, px));
        }
        if y > 0 {
            stack.push((x, y - 1, px));
        }
        if x + 1 < w {
            stack.push((x + 1, y, px));
        }
        if y + 1 < h {
            stack.push((x, y + 1, px));
        }
    }
    for y in 0..h {
        for x in 0..w {
            let px = picture.get_pixel_mut(x, y);
            if clear[(y * w + x) as usize] || close(rgb(px), main, HOLE_TOLERANCE) {
                *px = Rgba([0, 0, 0, 0]);
            }
        }
    }
    drop_shadow(&mut picture, main);
    Some(picture)
}

/// Clears the shadow the game draws under the character's feet: the ground darkened, each channel
/// the ground's times one factor. Only near the bottom, where it lies; a grey in the character
/// (neutral, not the ground's warm tint) stays.
fn drop_shadow(picture: &mut RgbaImage, ground: [u8; 3]) {
    let (w, h) = picture.dimensions();
    let Some(bottom) = (0..h).rev().find(|&y| (0..w).any(|x| picture.get_pixel(x, y).0[3] != 0)) else { return };
    let Some(top) = (0..h).find(|&y| (0..w).any(|x| picture.get_pixel(x, y).0[3] != 0)) else { return };
    let band = ((bottom - top + 1) * SHADOW_BAND / 100).max(4);
    let total = ground.iter().map(|&c| u32::from(c)).sum::<u32>().max(1) as f32;
    let shaded = |px: [u8; 3]| {
        let k = px.iter().map(|&c| u32::from(c)).sum::<u32>() as f32 / total;
        (0.35..0.95).contains(&k) && px.iter().zip(ground).all(|(&c, g)| (f32::from(c) - k * f32::from(g)).abs() <= 3.0)
    };
    for y in bottom.saturating_sub(band)..=bottom {
        for x in 0..w {
            let px = picture.get_pixel_mut(x, y);
            if px.0[3] != 0 && shaded(rgb(px)) {
                *px = Rgba([0, 0, 0, 0]);
            }
        }
    }
}

/// A group of touching opaque pixels: its bounds (right and bottom exclusive) and size.
#[derive(Debug, Clone, Copy)]
struct Piece {
    left: u32,
    top: u32,
    right: u32,
    bottom: u32,
    pixels: usize,
}

fn pieces(image: &RgbaImage) -> (Vec<Piece>, Vec<usize>) {
    let (w, h) = image.dimensions();
    let mut label = vec![usize::MAX; (w * h) as usize];
    let mut found = Vec::new();
    for start in 0..(w * h) {
        let (sx, sy) = (start % w, start / w);
        if label[start as usize] != usize::MAX || image.get_pixel(sx, sy).0[3] == 0 {
            continue;
        }
        let id = found.len();
        let mut piece = Piece { left: sx, top: sy, right: sx + 1, bottom: sy + 1, pixels: 0 };
        let mut stack = vec![(sx, sy)];
        label[start as usize] = id;
        while let Some((x, y)) = stack.pop() {
            piece.pixels += 1;
            piece.left = piece.left.min(x);
            piece.top = piece.top.min(y);
            piece.right = piece.right.max(x + 1);
            piece.bottom = piece.bottom.max(y + 1);
            for dy in -1i64..=1 {
                for dx in -1i64..=1 {
                    let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                    if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 {
                        continue;
                    }
                    let n = (ny as u32 * w + nx as u32) as usize;
                    if label[n] == usize::MAX && image.get_pixel(nx as u32, ny as u32).0[3] != 0 {
                        label[n] = id;
                        stack.push((nx as u32, ny as u32));
                    }
                }
            }
        }
        found.push(piece);
    }
    (found, label)
}

/// Clears what isn't the character: the round guild badge on its own in the box's top-right
/// corner, and specks well away from the character. A pet standing beside them stays.
fn drop_badge(image: &mut RgbaImage) {
    let (w, h) = image.dimensions();
    let (found, label) = pieces(image);
    let Some(main) = found.iter().copied().max_by_key(|p| p.pixels) else { return };
    let main_h = main.bottom - main.top;
    let keep: Vec<bool> = found
        .iter()
        .map(|p| {
            let touches = p.left <= main.right + 2 && p.right + 2 >= main.left && p.top <= main.bottom + 2 && p.bottom + 2 >= main.top;
            let (pw, ph) = (p.right - p.left, p.bottom - p.top);
            let cornered = p.left * 10 >= w * 6 && p.bottom * 10 <= h * 5;
            let round = pw * 10 >= ph * 6 && ph * 10 >= pw * 6;
            let badge = cornered && round && p.top < main.top + main_h / 3;
            let grounded = p.bottom * 4 >= main.top + 3 * main.bottom && p.pixels >= 12;
            !badge && (touches || grounded) && p.pixels >= 3
        })
        .collect();
    for y in 0..h {
        for x in 0..w {
            let id = label[(y * w + x) as usize];
            if id != usize::MAX && !keep[id] {
                image.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
    }
}

/// The picture cut down to what isn't transparent; `None` when nothing is left.
fn trim(image: &RgbaImage) -> Option<RgbaImage> {
    let (w, h) = image.dimensions();
    let opaque = |x: u32, y: u32| image.get_pixel(x, y).0[3] != 0;
    let left = (0..w).find(|&x| (0..h).any(|y| opaque(x, y)))?;
    let right = (0..w).rev().find(|&x| (0..h).any(|y| opaque(x, y)))?;
    let top = (0..h).find(|&y| (0..w).any(|x| opaque(x, y)))?;
    let bottom = (0..h).rev().find(|&y| (0..w).any(|x| opaque(x, y)))?;
    let (cw, ch) = (right - left + 1, bottom - top + 1);
    // A few pixels aren't a character.
    if cw < 6 || ch < 10 {
        return None;
    }
    Some(image::imageops::crop_imm(image, left, top, cw, ch).to_image())
}

#[cfg(test)]
mod tests {
    use super::*;

    // A made-up window in the game's style, drawn by hand: pink label cells, a pale bordered box
    // holding a little stick figure in a dark outline and a round badge, and a name box. No game
    // picture is used.

    const BODY: [u8; 3] = [228, 229, 230];
    const BORDER: [u8; 3] = [91, 123, 158];
    const GROUND: [u8; 3] = [224, 223, 212];
    const NAME_GREY: [u8; 3] = [130, 130, 130];
    const OUTLINE: [u8; 3] = [30, 24, 20];
    const SHIRT: [u8; 3] = [60, 140, 90];
    const SKIN: [u8; 3] = [250, 214, 180];
    const BADGE: [u8; 3] = [200, 160, 40];

    fn fill(frame: &mut Frame, x: u32, y: u32, w: u32, h: u32, c: [u8; 3]) {
        for yy in y..y + h {
            for xx in x..x + w {
                frame.put_pixel(xx, yy, Rgba([c[0], c[1], c[2], 255]));
            }
        }
    }

    /// Design pixels, drawn 2 x 2.
    fn dfill(frame: &mut Frame, (ox, oy): (u32, u32), x: u32, y: u32, w: u32, h: u32, c: [u8; 3]) {
        fill(frame, ox + 2 * x, oy + 2 * y, 2 * w, 2 * h, c);
    }

    /// A 1280 x 720 picture of the scene with the window at (`wx`, `wy`), laid out as the game's:
    /// the avatar box's ground lighter in a band near the top, the figure's shadow under its feet,
    /// the round badge top right and the name box (white letters on grey) inside the box's foot.
    /// The box's top-left corner is returned.
    fn scene(wx: u32, wy: u32, badge: bool) -> (Frame, (u32, u32)) {
        let mut frame = Frame::from_pixel(1280, 720, Rgba([120, 170, 220, 255]));
        fill(&mut frame, wx, wy, 520, 360, BODY);
        // The avatar box: a 2-pixel border round a pale ground, 90 x 130 design pixels.
        let (bx, by) = (wx + 20, wy + 40);
        let ground = (bx + 4, by + 4);
        fill(&mut frame, bx, by, 180, 260, BORDER);
        fill(&mut frame, bx + 4, by + 4, 172, 252, GROUND);
        for row in 4..30u32 {
            let bump = 30 - (row.abs_diff(17) * 30 / 13) as u8;
            dfill(&mut frame, ground, 0, row, 86, 1, GROUND.map(|c| c.saturating_add(bump)));
        }
        // A figure: an outlined head, a body and legs, with its shadow under its feet.
        let (fx, fy) = (bx + 4 + 2 * 30, by + 4 + 2 * 30);
        dfill(&mut frame, (fx, fy), 0, 60, 24, 4, GROUND.map(|c| (f32::from(c) * 0.6).round() as u8));
        dfill(&mut frame, (fx, fy), 0, 0, 22, 20, OUTLINE);
        dfill(&mut frame, (fx, fy), 1, 1, 20, 18, SKIN);
        dfill(&mut frame, (fx, fy), 3, 20, 16, 30, OUTLINE);
        dfill(&mut frame, (fx, fy), 4, 21, 14, 28, SHIRT);
        dfill(&mut frame, (fx, fy), 4, 50, 5, 12, OUTLINE);
        dfill(&mut frame, (fx, fy), 13, 50, 5, 12, OUTLINE);
        if badge {
            // A round badge in the top-right corner.
            let (cx, cy) = (bx + 4 + 2 * 76, by + 4 + 2 * 10);
            for y in 0..24 {
                for x in 0..24 {
                    let (dx, dy) = (x as i32 - 12, y as i32 - 12);
                    if dx * dx + dy * dy <= 121 {
                        frame.put_pixel(cx + x - 12, cy + y - 12, Rgba([BADGE[0], BADGE[1], BADGE[2], 255]));
                    }
                }
            }
        }
        // The name box inside the box's foot: grey, with a white bar standing for the letters.
        fill(&mut frame, bx + 8, by + 212, 164, 36, NAME_GREY);
        fill(&mut frame, bx + 60, by + 222, 60, 14, [255, 255, 255]);
        // The labels to the right: four pink cells, 88 x 28 at a pitch of 36.
        for i in 0..4 {
            fill(&mut frame, bx + 220, by + 60 + 36 * i, 88, 28, LABEL);
        }
        (frame, (bx, by))
    }

    fn named(name: &'static str) -> impl Fn(&Frame, Rect) -> NameRead {
        move |_, _| NameRead { glyphs: Some(name.into()), ocr: None }
    }

    #[test]
    fn finds_the_window_by_its_labels_and_box() {
        let (frame, (bx, by)) = scene(300, 120, true);
        let found = find(&frame);
        assert_eq!(found.len(), 1, "{found:?}");
        let window = found[0];
        assert_eq!((window.portrait.x, window.portrait.y), ((bx + 4) as f32, (by + 4) as f32));
        assert_eq!((window.portrait.w, window.portrait.h), (172.0, 208.0));
        assert_eq!((window.name.y, window.name.h), ((by + 212) as f32, 36.0));
    }

    /// Saved frames of the real window (`test-screenshots/charinfo`, beside the repository and never
    /// committed: they are game pictures). Runs only where they are. The name is the HUD's own, read
    /// from the frame; on Windows the name box must read as it with Windows OCR.
    #[test]
    fn reads_saved_frames_of_the_real_window() {
        let Some(folder) = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .map(|a| a.join("test-screenshots").join("charinfo"))
            .find(|p| p.is_dir())
        else {
            return;
        };
        for entry in std::fs::read_dir(&folder).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "png") {
                continue;
            }
            let frame = crate::vision::load(&path).unwrap();
            let layout = super::super::hud::locate(&frame).expect("the HUD");
            let own = super::super::hud::identity_lines(&frame, &layout).get(1).map(|(t, _)| t.clone()).expect("the HUD's name");
            let windows = find(&frame);
            assert_eq!(windows.len(), 1, "{path:?}");
            #[cfg(windows)]
            {
                let ocr = Ocr::new().unwrap();
                let read = read_name(&frame, windows[0].name, Some(&ocr));
                assert!(names_match(&read, &own), "{path:?}: {read:?}");
                assert!(!names_match(&read, "Somebodyelse"), "{path:?}");
            }
            let portrait = read_with(&frame, Some(&layout), &own, |_, _| NameRead { glyphs: Some(own.clone()), ocr: None })
                .unwrap_or_else(|| panic!("{path:?}: no picture"));
            let (w, h) = portrait.image.dimensions();
            assert!((24..=100).contains(&w) && (40..=110).contains(&h), "{path:?}: {w} x {h}");
            let clear = portrait.image.pixels().filter(|p| p.0[3] == 0).count();
            assert!(clear * 4 >= (w * h) as usize, "{path:?}: the ground wasn't keyed out");            println!("{}: {w} x {h}", path.display());
        }
    }

    #[test]
    fn the_stat_windows_labels_alone_are_not_the_window() {
        let mut frame = Frame::from_pixel(1280, 720, Rgba([120, 170, 220, 255]));
        fill(&mut frame, 300, 100, 380, 500, BODY);
        for i in 0..5 {
            fill(&mut frame, 320, 160 + 36 * i, 88, 28, LABEL);
        }
        assert!(find(&frame).is_empty());
    }

    #[test]
    fn cuts_the_character_out_on_a_clear_ground_without_the_badge() {
        let (frame, _) = scene(300, 120, true);
        let portrait = read_with(&frame, None, "Mossbank", named("Mossbank")).expect("own window");
        let image = &portrait.image;
        // The figure is 22 design pixels wide and 62 tall: the badge, the lighter band and the
        // shadow are left out.
        assert_eq!(image.dimensions(), (22, 62));
        assert_eq!(image.get_pixel(0, 0).0, [OUTLINE[0], OUTLINE[1], OUTLINE[2], 255]);
        assert_eq!(image.get_pixel(5, 10).0, [SKIN[0], SKIN[1], SKIN[2], 255]);
        // Between the legs, the ground is clear.
        assert_eq!(image.get_pixel(11, 61).0[3], 0);
        assert!(image.pixels().all(|p| p.0[3] == 0 || rgb(p) != BADGE));
        let seen = portrait.seen("Mossbank").expect("png");
        assert_eq!((seen.width, seen.height), (22, 62));
        assert!(!seen.png.is_empty());
    }

    #[test]
    fn another_players_window_gives_nothing() {
        let (frame, _) = scene(300, 120, false);
        assert!(read_with(&frame, None, "Mossbank", named("Fernwick")).is_none());
        let unread = |_: &Frame, _: Rect| NameRead::default();
        assert!(read_with(&frame, None, "Mossbank", unread).is_none());
        assert!(read_with(&frame, None, "", named("")).is_none());
    }

    #[test]
    fn the_name_must_match_the_huds() {
        let glyphs = |t: &str| NameRead { glyphs: Some(t.into()), ocr: None };
        let ocr = |g: Option<&str>, t: &str| NameRead { glyphs: g.map(str::to_string), ocr: Some(t.into()) };
        assert!(names_match(&glyphs("Mossbank"), "Mossbank"));
        assert!(names_match(&glyphs("mossbank"), "Mossbank"));
        assert!(!names_match(&glyphs("Mossbanks"), "Mossbank"));
        // A full reading by the font decides, whatever OCR says.
        assert!(!names_match(&ocr(Some("Fernwick"), "Mossbank"), "Mossbank"));
        // OCR's mix-ups are forgiven; the font's known glyphs must still agree.
        assert!(names_match(&ocr(None, "Moss8ank"), "Moss8ank"));
        assert!(names_match(&ocr(Some("M?ssbank"), "M0ssbank"), "Mossbank"));
        assert!(names_match(&ocr(None, "Fern1wick"), "Ferniwick"));
        assert!(!names_match(&ocr(Some("F?ssbank"), "Mossbank"), "Mossbank"));
        assert!(!names_match(&ocr(None, "Mossbanc"), "Mossbank"));
        assert!(!names_match(&NameRead::default(), "Mossbank"));
    }

    #[test]
    fn keeps_a_picture_seen_twice_once_per_opening() {
        let picture = |key: u64| Some(Portrait { image: RgbaImage::new(1, 1), key });
        let mut watch = LookWatch::default();
        assert!(watch.follow(picture(1)).is_none(), "seen once");
        assert!(watch.follow(picture(2)).is_none(), "a cursor over it");
        assert!(watch.follow(picture(1)).is_some(), "came round again");
        assert!(watch.follow(picture(1)).is_none(), "kept already");
        assert!(watch.follow(None).is_none(), "closed");
        assert!(watch.follow(picture(1)).is_none());
        assert!(watch.follow(picture(1)).is_some(), "opened again");
    }
}
