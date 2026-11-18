//! The capture test's pattern: what the program's "Capture test pattern" window shows, and what
//! the website's capture test page draws again to compare with the frames the browser delivers.
//!
//! It is made here, the same way on both sides, so any difference is the capture's. It holds a
//! code strip (the window's size and scale, so the page knows what it is looking at), a grey ramp
//! and red, green and blue ramps, swatches of colours the readers look for, 1-px coloured lines
//! on grey and on black, and lines of the game's pixel fonts on the backgrounds they appear on.
//! The fonts are the readers' own glyph tables (allowed 2026-11-15); everything else is drawn
//! here. No picture of the game is used.

use image::Rgba;
use serde::{Deserialize, Serialize};

use crate::readers::gamefont::Font;
use crate::readers::{pickup, questfont};
use crate::vision::Frame;

/// The pattern's size in screen pixels. The window shows it pixel for pixel.
pub const WIDTH: u32 = 1280;
pub const HEIGHT: u32 = 720;
/// The grey everything sits on.
pub const BACKGROUND: [u8; 3] = [90, 90, 90];

/// What the pattern window says about itself in its code strip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    /// The window's inside, in screen pixels.
    pub width: u32,
    pub height: u32,
    /// The window's display scale in thousandths: 1250 at 125%.
    pub scale_milli: u32,
}

/// The parts of the pattern, for errors reported part by part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Part {
    Code,
    GreyRamp,
    ColourRamps,
    Swatches,
    Lines,
    Glyphs,
}

/// A rectangle of the pattern, in its own pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Area {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl Area {
    pub const fn new(x: u32, y: u32, w: u32, h: u32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

// The code strip: two guard blocks, 64 bits (white is 1), two guard blocks.
pub(super) const CODE_X: u32 = 8;
pub(super) const CODE_Y: u32 = 8;
pub(super) const BLOCK_W: u32 = 12;
pub(super) const BLOCK_H: u32 = 16;
pub(super) const CODE_BLOCKS: u32 = 68;
const MAGIC: u64 = 0xA5C;
const VERSION: u64 = 1;

/// The bits the code strip shows for `info`.
pub fn encode(info: WindowInfo) -> u64 {
    let field = |v: u32| u64::from(v.min(0xFFFF));
    (MAGIC << 52) | (VERSION << 48) | (field(info.width) << 32) | (field(info.height) << 16) | field(info.scale_milli)
}

/// The window's description, when `bits` are a pattern's code.
pub fn decode(bits: u64) -> Option<WindowInfo> {
    (bits >> 52 == MAGIC && (bits >> 48) & 0xF == VERSION).then(|| WindowInfo {
        width: ((bits >> 32) & 0xFFFF) as u32,
        height: ((bits >> 16) & 0xFFFF) as u32,
        scale_milli: (bits & 0xFFFF) as u32,
    })
}

/// The code strip's blocks, white or not, left to right.
pub(super) fn code_blocks(bits: u64) -> Vec<bool> {
    let mut blocks = vec![true, false];
    blocks.extend((0..64).rev().map(|i| (bits >> i) & 1 == 1));
    blocks.extend([false, true]);
    blocks
}

pub(super) fn block_area(i: u32) -> Area {
    Area::new(CODE_X + i * BLOCK_W, CODE_Y, BLOCK_W, BLOCK_H)
}

// The grey ramp: 256 steps 4 px wide. The colour ramps below it: red, green, blue.
pub(super) const RAMP_X: u32 = 8;
pub(super) const RAMP_STEP: u32 = 4;
pub(super) const GREY_RAMP: Area = Area::new(RAMP_X, 40, 256 * RAMP_STEP, 48);
pub(super) const COLOUR_RAMPS: Area = Area::new(RAMP_X, 96, 256 * RAMP_STEP, 48);
const COLOUR_RAMP_H: u32 = 16;

/// A colour the readers look for, or a plain one to compare with.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Swatch {
    pub name: &'static str,
    pub rgb: [u8; 3],
}

/// Taken from the readers' own colour tests (`readers/`) and clean captures of the game. The
/// MP and EXP gauge colours are the readers' test colours, not measured ones.
pub const SWATCHES: [Swatch; 19] = [
    Swatch { name: "HP gauge red", rgb: [202, 4, 39] },
    Swatch { name: "MP gauge blue", rgb: [40, 60, 220] },
    Swatch { name: "EXP gauge yellow", rgb: [220, 220, 40] },
    Swatch { name: "Minimap WORLD button", rgb: [221, 153, 17] },
    Swatch { name: "WORLD button, Default Ratio", rgb: [216, 141, 2] },
    Swatch { name: "Minimap name band", rgb: [153, 187, 204] },
    Swatch { name: "Collapsed minimap bar", rgb: [221, 221, 221] },
    Swatch { name: "Upgraded item amber", rgb: [255, 170, 0] },
    Swatch { name: "Tooltip name bullet", rgb: [85, 204, 255] },
    Swatch { name: "NPC name yellow", rgb: [251, 252, 0] },
    Swatch { name: "Pickup Log white", rgb: [246, 246, 245] },
    Swatch { name: "Tooltip panel", rgb: [74, 76, 120] },
    Swatch { name: "Quest Helper panel", rgb: [200, 200, 190] },
    Swatch { name: "Inventory ground", rgb: [225, 223, 213] },
    Swatch { name: "Red", rgb: [255, 0, 0] },
    Swatch { name: "Green", rgb: [0, 255, 0] },
    Swatch { name: "Blue", rgb: [0, 0, 255] },
    Swatch { name: "White", rgb: [255, 255, 255] },
    Swatch { name: "Black", rgb: [0, 0, 0] },
];
pub(super) const SWATCH_Y: u32 = 160;
pub(super) const SWATCH_SIZE: u32 = 56;
const SWATCH_PITCH: u32 = 64;

pub(super) fn swatch_area(i: usize) -> Area {
    Area::new(8 + i as u32 * SWATCH_PITCH, SWATCH_Y, SWATCH_SIZE, SWATCH_SIZE)
}
pub(super) const SWATCHES_AREA: Area = Area::new(8, SWATCH_Y, 19 * SWATCH_PITCH - 8, SWATCH_SIZE);

/// The colours of the 1-px lines: pure ones first, then colours the readers look for.
pub const LINE_COLOURS: [[u8; 3]; 16] = [
    [255, 0, 0],
    [0, 255, 0],
    [0, 0, 255],
    [255, 255, 0],
    [0, 255, 255],
    [255, 0, 255],
    [255, 255, 255],
    [0, 0, 0],
    [202, 4, 39],
    [220, 220, 40],
    [40, 60, 220],
    [255, 170, 0],
    [85, 204, 255],
    [251, 252, 0],
    [80, 200, 60],
    [246, 246, 245],
];

/// A box of 1-px lines, one of each of [`LINE_COLOURS`].
#[derive(Debug, Clone, Copy)]
pub struct LineBox {
    pub area: Area,
    pub ground: [u8; 3],
    pub vertical: bool,
}

pub(super) const LINE_BOXES: [LineBox; 3] = [
    LineBox { area: Area::new(8, 232, 512, 160), ground: [128, 128, 128], vertical: true },
    LineBox { area: Area::new(536, 232, 512, 160), ground: [0, 0, 0], vertical: true },
    LineBox { area: Area::new(1064, 232, 208, 160), ground: [128, 128, 128], vertical: false },
];
pub(super) const LINES_AREA: Area = Area::new(8, 232, 1264, 160);
/// How far a line's ends stay from its box's edge.
pub(super) const LINE_INSET: u32 = 4;

/// Where line `i` of `b` is: its column, or its row in a box of horizontal lines. Neighbours
/// alternate between even and odd positions, because colour is often kept for pairs of pixels.
pub(super) fn line_at(b: &LineBox, i: usize) -> u32 {
    let i = i as u32;
    if b.vertical { b.area.x + 16 + i * 30 + i % 2 } else { b.area.y + 8 + i * 9 }
}

/// Which of the readers' fonts a strip is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FontKind {
    /// The Quest Helper's objective font.
    Quest,
    /// The Pickup Log's font.
    Pickup,
}

/// What is behind a strip's text.
#[derive(Debug, Clone, Copy)]
pub enum Ground {
    Flat([u8; 3]),
    /// Made-up scenery, brown and green blotches.
    World,
}

/// What the game draws around a strip's letters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    None,
    /// Black, one font pixel down and to the right (the Pickup Log).
    Shadow,
    /// Black all round (the minimap's name).
    Outline,
}

/// How a strip's text is read back from a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ReadBy {
    /// The Quest Helper reader's own ink rule and glyph matcher.
    QuestReader,
    /// The Pickup Log reader: finding the log, its ink cut and its glyph matcher.
    PickupReader,
    /// Each pixel taken as ink when it is nearer the ink's colour than the background's, then the
    /// font's glyph matcher. The game's own reader uses OCR for this text.
    StandIn,
}

/// A line of one of the game's fonts on a background it appears on.
#[derive(Debug, Clone, Copy)]
pub struct Strip {
    pub name: &'static str,
    pub font: FontKind,
    pub text: &'static str,
    pub ink: [u8; 3],
    pub ground: Ground,
    pub edge: Edge,
    pub read_by: ReadBy,
}

pub const STRIPS: [Strip; 8] = [
    Strip {
        name: "Quest Helper objective",
        font: FontKind::Quest,
        text: "23 / 20 Orange Mushroom Cap",
        ink: [0, 0, 0],
        ground: Ground::Flat([200, 200, 190]),
        edge: Edge::None,
        read_by: ReadBy::QuestReader,
    },
    Strip {
        name: "Pickup Log line",
        font: FontKind::Pickup,
        text: "Red Snail Shell x1 earned. (Etc)",
        ink: [246, 246, 245],
        ground: Ground::World,
        edge: Edge::Shadow,
        read_by: ReadBy::PickupReader,
    },
    Strip {
        name: "Tooltip text",
        font: FontKind::Pickup,
        text: "Snail Shell x3",
        ink: [255, 255, 255],
        ground: Ground::Flat([74, 76, 120]),
        edge: Edge::None,
        read_by: ReadBy::StandIn,
    },
    Strip {
        name: "Upgraded item name",
        font: FontKind::Pickup,
        text: "Blue Snail Shell",
        ink: [255, 170, 0],
        ground: Ground::Flat([74, 76, 120]),
        edge: Edge::None,
        read_by: ReadBy::StandIn,
    },
    Strip {
        name: "Minimap name",
        font: FontKind::Pickup,
        text: "Mushroom Park",
        ink: [255, 255, 255],
        ground: Ground::Flat([153, 187, 204]),
        edge: Edge::Outline,
        read_by: ReadBy::StandIn,
    },
    Strip {
        name: "Collapsed minimap title",
        font: FontKind::Quest,
        text: "Mushroom Town",
        ink: [0, 0, 0],
        ground: Ground::Flat([221, 221, 221]),
        edge: Edge::None,
        read_by: ReadBy::StandIn,
    },
    Strip {
        name: "NPC name tag",
        font: FontKind::Pickup,
        text: "Sera",
        ink: [251, 252, 0],
        ground: Ground::Flat([40, 40, 40]),
        edge: Edge::None,
        read_by: ReadBy::StandIn,
    },
    Strip {
        name: "HUD numbers",
        font: FontKind::Quest,
        text: "1234 / 5678",
        ink: [230, 230, 230],
        ground: Ground::Flat([34, 34, 34]),
        edge: Edge::None,
        read_by: ReadBy::StandIn,
    },
];

pub(super) const STRIP_Y: u32 = 408;
const STRIP_PITCH: u32 = 38;
const STRIP_W: u32 = 640;
const STRIP_H: u32 = 34;
pub(super) const GLYPHS_AREA: Area = Area::new(8, STRIP_Y, STRIP_W, 7 * STRIP_PITCH + STRIP_H);
/// Screen pixels per font pixel, as at UI Size Ideal Ratio at 1440p.
pub(super) const FONT_SCALE: u32 = 2;
/// Where a strip's text starts, from its box's top left.
const TEXT_X: u32 = 16;
const TEXT_Y: u32 = 6;

pub(super) fn strip_area(i: usize) -> Area {
    Area::new(8, STRIP_Y + i as u32 * STRIP_PITCH, STRIP_W, STRIP_H)
}

/// A strip's text laid out in font pixels: `ink[row][column]`, left to right, and where its top
/// left corner is in the pattern.
pub(super) struct Laid {
    pub ink: Vec<Vec<bool>>,
    pub x: u32,
    pub y: u32,
}

impl Laid {
    pub fn columns(&self) -> usize {
        self.ink.first().map_or(0, Vec::len)
    }

    /// The font pixel's screen pixels.
    pub fn pixels(&self, row: usize, column: usize) -> impl Iterator<Item = (u32, u32)> + use<> {
        let (x, y) = (self.x + column as u32 * FONT_SCALE, self.y + row as u32 * FONT_SCALE);
        (0..FONT_SCALE).flat_map(move |dy| (0..FONT_SCALE).map(move |dx| (x + dx, y + dy)))
    }
}

pub(super) fn font_rows(font: FontKind) -> usize {
    match font {
        FontKind::Quest => questfont::OBJECTIVE.rows,
        FontKind::Pickup => 11,
    }
}

fn glyph(font: FontKind, c: char) -> Option<&'static [&'static str]> {
    match font {
        FontKind::Quest => questfont::OBJECTIVE.glyphs.iter().find(|(g, _)| *g == c).map(|(_, rows)| *rows),
        FontKind::Pickup => pickup::font::all_glyphs().find(|(g, _)| *g == c).map(|(_, rows)| &rows[..]),
    }
}

/// Blank columns after a glyph, and between words, as the game spaces each font (see
/// `questfont` and `pickup::font`).
fn gap_after(font: FontKind, c: char) -> usize {
    let quest: &Font = &questfont::OBJECTIVE;
    match font {
        FontKind::Quest => 1 + quest.narrow.iter().find(|(n, _)| *n == c).map_or(0, |(_, extra)| *extra),
        FontKind::Pickup => 1,
    }
}

fn word_gap(font: FontKind) -> usize {
    match font {
        FontKind::Quest => questfont::OBJECTIVE.space,
        FontKind::Pickup => 5,
    }
}

/// `text` in `font`'s glyphs. Characters the font has no glyph for are left out; a test checks
/// every strip's text has them all.
pub(super) fn lay_out(font: FontKind, text: &str) -> Vec<Vec<bool>> {
    let rows = font_rows(font);
    let mut columns: Vec<Vec<bool>> = Vec::new();
    let mut previous: Option<char> = None;
    for c in text.chars() {
        if c == ' ' {
            // The previous glyph's own gap is there already (wider after a narrow glyph, which
            // the readers allow for); a word gap adds the rest.
            if previous.is_some() {
                columns.extend((0..word_gap(font) - 1).map(|_| vec![false; rows]));
            }
            continue;
        }
        let Some(glyph) = glyph(font, c) else { continue };
        let width = glyph[0].len();
        for x in 0..width {
            columns.push(glyph.iter().map(|row| row.as_bytes()[x] == b'#').collect());
        }
        columns.extend((0..gap_after(font, c)).map(|_| vec![false; rows]));
        previous = Some(c);
    }
    while columns.last().is_some_and(|c| c.iter().all(|&on| !on)) {
        columns.pop();
    }
    (0..rows).map(|r| columns.iter().map(|c| c[r]).collect()).collect()
}

/// Strip `i`'s text, laid out where the pattern draws it. The Pickup Log's line is right-aligned
/// in its box, as the game draws the log.
pub(super) fn laid(i: usize) -> Laid {
    let strip = &STRIPS[i];
    let area = strip_area(i);
    let ink = lay_out(strip.font, strip.text);
    let width = ink.first().map_or(0, Vec::len) as u32 * FONT_SCALE;
    let x = if strip.read_by == ReadBy::PickupReader { area.x + area.w - TEXT_X - width } else { area.x + TEXT_X };
    Laid { ink, x, y: area.y + TEXT_Y }
}

/// Made-up scenery: brown and green blotches, the same everywhere the pattern is drawn.
fn world(x: u32, y: u32) -> [u8; 3] {
    let n = (x / 7 * 31 + y / 5 * 17 + (x * y) % 13) % 64;
    [60 + n as u8, 50 + (n / 2) as u8, 20 + (n / 3) as u8]
}

/// The pattern for a window described by `info`.
pub fn draw(info: WindowInfo) -> Frame {
    render(info, true)
}

/// The pattern with the strips' text left out but their backgrounds, shadows and outlines kept:
/// what is under each letter.
pub(super) fn draw_under(info: WindowInfo) -> Frame {
    render(info, false)
}

fn render(info: WindowInfo, text: bool) -> Frame {
    let mut frame = Frame::from_pixel(WIDTH, HEIGHT, rgba(BACKGROUND));
    let fill = |frame: &mut Frame, area: Area, colour: [u8; 3]| {
        for y in area.y..area.y + area.h {
            for x in area.x..area.x + area.w {
                frame.put_pixel(x, y, rgba(colour));
            }
        }
    };

    for (i, white) in code_blocks(encode(info)).into_iter().enumerate() {
        fill(&mut frame, block_area(i as u32), if white { [255; 3] } else { [0; 3] });
    }

    for step in 0..256u32 {
        let x = RAMP_X + step * RAMP_STEP;
        let v = step as u8;
        fill(&mut frame, Area::new(x, GREY_RAMP.y, RAMP_STEP, GREY_RAMP.h), [v; 3]);
        for (c, colour) in [[v, 0, 0], [0, v, 0], [0, 0, v]].into_iter().enumerate() {
            let y = COLOUR_RAMPS.y + c as u32 * COLOUR_RAMP_H;
            fill(&mut frame, Area::new(x, y, RAMP_STEP, COLOUR_RAMP_H), colour);
        }
    }

    for (i, swatch) in SWATCHES.iter().enumerate() {
        fill(&mut frame, swatch_area(i), swatch.rgb);
    }

    for b in &LINE_BOXES {
        fill(&mut frame, b.area, b.ground);
        for (i, &colour) in LINE_COLOURS.iter().enumerate() {
            let at = line_at(b, i);
            let line = if b.vertical {
                Area::new(at, b.area.y + LINE_INSET, 1, b.area.h - 2 * LINE_INSET)
            } else {
                Area::new(b.area.x + LINE_INSET, at, b.area.w - 2 * LINE_INSET, 1)
            };
            fill(&mut frame, line, colour);
        }
    }

    for (i, strip) in STRIPS.iter().enumerate() {
        let area = strip_area(i);
        match strip.ground {
            Ground::Flat(colour) => fill(&mut frame, area, colour),
            Ground::World => {
                for y in area.y..area.y + area.h {
                    for x in area.x..area.x + area.w {
                        frame.put_pixel(x, y, rgba(world(x, y)));
                    }
                }
            }
        }
        let laid = laid(i);
        let rows = laid.ink.len() as isize;
        let columns = laid.columns() as isize;
        let on = |r: isize, c: isize| r >= 0 && c >= 0 && r < rows && c < columns && laid.ink[r as usize][c as usize];
        // The shadow and outline go one font pixel past the text's box.
        for r in -1..=rows {
            for c in -1..=columns {
                let edge = match strip.edge {
                    Edge::None => false,
                    Edge::Shadow => on(r - 1, c - 1),
                    Edge::Outline => (-1..=1).any(|dr| (-1..=1).any(|dc| on(r + dr, c + dc))),
                };
                let colour = if text && on(r, c) {
                    strip.ink
                } else if edge && !on(r, c) {
                    // The Pickup Log's shadow is very nearly black: `020201`.
                    if strip.edge == Edge::Shadow { [2, 2, 1] } else { [0, 0, 0] }
                } else {
                    continue;
                };
                let (x, y) = (laid.x as isize + c * FONT_SCALE as isize, laid.y as isize + r * FONT_SCALE as isize);
                for dy in 0..FONT_SCALE as isize {
                    for dx in 0..FONT_SCALE as isize {
                        frame.put_pixel((x + dx) as u32, (y + dy) as u32, rgba(colour));
                    }
                }
            }
        }
    }
    frame
}

fn rgba([r, g, b]: [u8; 3]) -> Rgba<u8> {
    Rgba([r, g, b, 255])
}

#[cfg(test)]
mod tests {
    use super::*;

    const INFO: WindowInfo = WindowInfo { width: 1280, height: 720, scale_milli: 1250 };

    #[test]
    fn the_code_says_what_window_it_is() {
        assert_eq!(decode(encode(INFO)), Some(INFO));
        assert_eq!(decode(encode(INFO) ^ (1 << 60)), None, "a pattern's code starts with its mark");
        assert_eq!(code_blocks(encode(INFO)).len() as u32, CODE_BLOCKS);
    }

    #[test]
    fn every_part_fits_the_pattern() {
        let fits = |a: Area| a.x + a.w <= WIDTH && a.y + a.h <= HEIGHT;
        assert!(fits(block_area(CODE_BLOCKS - 1)));
        assert!(fits(GREY_RAMP) && fits(COLOUR_RAMPS) && fits(SWATCHES_AREA) && fits(LINES_AREA) && fits(GLYPHS_AREA));
        assert!(fits(swatch_area(SWATCHES.len() - 1)));
        for b in &LINE_BOXES {
            assert!(line_at(b, LINE_COLOURS.len() - 1) + 7 < if b.vertical { b.area.x + b.area.w } else { b.area.y + b.area.h });
        }
        for i in 0..STRIPS.len() {
            let (area, laid) = (strip_area(i), laid(i));
            assert!(fits(area));
            let right = laid.x + laid.columns() as u32 * FONT_SCALE + FONT_SCALE;
            let bottom = laid.y + laid.ink.len() as u32 * FONT_SCALE + FONT_SCALE;
            assert!(laid.x >= area.x + FONT_SCALE && right <= area.x + area.w && bottom <= area.y + area.h, "{}", STRIPS[i].name);
        }
    }

    #[test]
    fn every_strip_has_all_its_glyphs() {
        for strip in &STRIPS {
            for c in strip.text.chars().filter(|&c| c != ' ') {
                assert!(glyph(strip.font, c).is_some(), "{}: no glyph for {c:?}", strip.name);
            }
        }
    }

    #[test]
    fn draws_the_same_every_time() {
        assert_eq!(draw(INFO), draw(INFO));
        assert_ne!(draw(INFO), draw(WindowInfo { width: 1281, ..INFO }), "the code strip differs");
        assert_ne!(draw(INFO), draw_under(INFO), "the text is left out underneath");
    }
}
