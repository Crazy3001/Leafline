//! What the capture did to the pattern: where the pattern is in a delivered frame, and how far
//! each part of it is from what was drawn.

use image::Rgba;
use serde::Serialize;

use super::pattern::{
    self, Area, BLOCK_H, BLOCK_W, CODE_BLOCKS, COLOUR_RAMPS, FONT_SCALE, FontKind, GLYPHS_AREA, GREY_RAMP,
    HEIGHT, LINE_BOXES, LINE_COLOURS, LINE_INSET, LINES_AREA, Part, RAMP_STEP, RAMP_X, ReadBy, STRIPS, SWATCHES,
    SWATCHES_AREA, WIDTH, WindowInfo,
};
use crate::readers::{gamefont, pickup, quest, questfont};
use crate::vision::{Frame, Rect};

/// A line's colour is taken to reach a neighbouring pixel when it moves that pixel this far
/// from the background in some channel. The readers' colour tests allow 8 to 16.
pub const BLEED_LEVEL: u8 = 12;
/// How far from a line its colour is looked for.
const BLEED_REACH: u32 = 6;

/// How a frame compared with the pattern.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Measure {
    pub frame_width: u32,
    pub frame_height: u32,
    /// What was found, or `None` with [`Measure::problem`] saying why not.
    pub found: Option<Found>,
    pub problem: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    /// Where the pattern's top left corner is in the frame: anything above or left of it is the
    /// window's frame or title bar.
    pub x: u32,
    pub y: u32,
    /// The window's size and scale, from its code strip.
    pub window: WindowInfo,
    /// Red, green and blue over the whole pattern.
    pub channels: [ChannelError; 3],
    pub parts: Vec<PartError>,
    pub ramp: RampCheck,
    pub swatches: Vec<SwatchCheck>,
    pub lines: Vec<LineCheck>,
    pub bleed: Bleed,
    pub glyphs: Vec<GlyphCheck>,
}

/// How far one channel is from what was drawn, in levels of 255.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct ChannelError {
    pub mean: f32,
    pub largest: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct PartError {
    pub part: Part,
    pub channels: [ChannelError; 3],
}

/// The grey ramp: how black and white came through, and the largest step error. A capture that
/// squeezes the range (video levels, 16 to 235) and never stretches it back shows a raised black
/// and a lowered white.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RampCheck {
    pub black: u8,
    pub white: u8,
    pub largest: u8,
    pub squeezed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SwatchCheck {
    pub name: &'static str,
    pub expected: [u8; 3],
    pub seen: [u8; 3],
    /// The largest channel difference.
    pub off: u8,
}

/// A 1-px line: its colour as seen, and how many pixels beside it took on some of its colour.
#[derive(Debug, Clone, Serialize)]
pub struct LineCheck {
    pub colour: [u8; 3],
    pub ground: [u8; 3],
    pub vertical: bool,
    pub seen: [u8; 3],
    pub off: u8,
    /// Pixels on the worse side.
    pub bleed: u32,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct Bleed {
    pub largest: u32,
    /// The median over the lines.
    pub typical: u32,
}

/// A strip of the game's font read back from the frame.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlyphCheck {
    pub name: &'static str,
    pub read_by: ReadBy,
    pub expected: &'static str,
    pub read: Option<String>,
    /// Font pixels in the text's box, and how many of them came back the wrong way (ink taken for
    /// background or the other way round), by the nearest-colour cut.
    pub font_pixels: u32,
    pub flipped: u32,
}

impl GlyphCheck {
    pub fn reads(&self) -> bool {
        self.read.as_deref() == Some(self.expected)
    }
}

/// Finds the pattern in `frame` and measures it. `hint` is where it was last time: tried first.
pub fn measure(frame: &Frame, hint: Option<(u32, u32)>) -> Measure {
    let (frame_width, frame_height) = frame.dimensions();
    let missing = |problem: String| Measure { frame_width, frame_height, found: None, problem: Some(problem) };
    if frame_width < WIDTH || frame_height < HEIGHT {
        return missing(format!(
            "The frame is {frame_width} x {frame_height}, smaller than the pattern ({WIDTH} x {HEIGHT}): the browser \
             scaled the window down, or this isn't the pattern window."
        ));
    }
    let Some(((x, y), window)) = locate(frame, hint) else {
        return missing(
            "The pattern isn't in this frame at full size. Share the pattern window, keep it uncovered and at 100% zoom."
                .into(),
        );
    };
    Measure { frame_width, frame_height, found: Some(compare(frame, x, y, window)), problem: None }
}

fn luma(px: &Rgba<u8>) -> u8 {
    crate::vision::preprocess::luminance(px)
}

/// The window's description from the code strip at `(x, y)`, if a pattern starts there.
fn read_code(frame: &Frame, x: u32, y: u32) -> Option<WindowInfo> {
    let blocks: Vec<bool> = (0..CODE_BLOCKS)
        .map(|i| {
            let block = pattern::block_area(i);
            let (cx, cy) = (x + block.x + BLOCK_W / 2, y + block.y + BLOCK_H / 2);
            let sum: u32 = (cy - 4..cy + 4).flat_map(|py| (cx - 2..cx + 2).map(move |px| (px, py)))
                .map(|(px, py)| u32::from(luma(frame.get_pixel(px, py))))
                .sum();
            sum > 128 * 32
        })
        .collect();
    let n = blocks.len();
    if blocks[..2] != [true, false] || blocks[n - 2..] != [false, true] {
        return None;
    }
    let bits = blocks[2..n - 2].iter().fold(0u64, |bits, &on| bits << 1 | u64::from(on));
    pattern::decode(bits)
}

/// A few pixels any pattern has, whatever its code: tried at every place before the code is read.
fn could_start(frame: &Frame, x: u32, y: u32) -> bool {
    let at = |px: u32, py: u32| luma(frame.get_pixel(x + px, y + py));
    let ramp = |step: u32| at(RAMP_X + step * RAMP_STEP + 2, GREY_RAMP.y + GREY_RAMP.h / 2);
    let first = pattern::block_area(0);
    // The first block's top left corner exactly, with the grey around it: the block is white.
    at(first.x, first.y) > 200
        && at(first.x - 1, first.y + BLOCK_H / 2) < 150
        && at(first.x + BLOCK_W / 2, first.y - 1) < 150
        && at(first.x + BLOCK_W / 2, first.y + BLOCK_H / 2) > 200
        && at(first.x + BLOCK_W + BLOCK_W / 2, first.y + BLOCK_H / 2) < 55
        && ramp(0) < 40
        && ramp(255) > 215
        && (100..=156).contains(&ramp(128))
}

/// Where the pattern starts in `frame` and what its code says.
pub fn locate(frame: &Frame, hint: Option<(u32, u32)>) -> Option<((u32, u32), WindowInfo)> {
    let (w, h) = frame.dimensions();
    if w < WIDTH || h < HEIGHT {
        return None;
    }
    let (max_x, max_y) = (w - WIDTH, h - HEIGHT);
    if let Some((x, y)) = hint.filter(|&(x, y)| x <= max_x && y <= max_y)
        && could_start(frame, x, y)
        && let Some(info) = read_code(frame, x, y)
    {
        return Some(((x, y), info));
    }
    (0..=max_y)
        .flat_map(|y| (0..=max_x).map(move |x| (x, y)))
        .filter(|&(x, y)| could_start(frame, x, y))
        .find_map(|(x, y)| read_code(frame, x, y).map(|info| ((x, y), info)))
}

#[derive(Default, Clone, Copy)]
struct Sums {
    sum: [u64; 3],
    largest: [u8; 3],
    count: u64,
}

impl Sums {
    fn add(&mut self, expected: &Rgba<u8>, seen: &Rgba<u8>) {
        for c in 0..3 {
            let d = expected.0[c].abs_diff(seen.0[c]);
            self.sum[c] += u64::from(d);
            self.largest[c] = self.largest[c].max(d);
        }
        self.count += 1;
    }

    fn channels(&self) -> [ChannelError; 3] {
        [0, 1, 2].map(|c| ChannelError {
            mean: self.sum[c] as f32 / self.count.max(1) as f32,
            largest: self.largest[c],
        })
    }
}

fn compare(frame: &Frame, ox: u32, oy: u32, window: WindowInfo) -> Found {
    let expected = pattern::draw(window);
    let under = pattern::draw_under(window);
    let seen = |x: u32, y: u32| *frame.get_pixel(ox + x, oy + y);

    let code_area = Area::new(pattern::CODE_X, pattern::CODE_Y, CODE_BLOCKS * BLOCK_W, BLOCK_H);
    let parts: [(Part, Area); 6] = [
        (Part::Code, code_area),
        (Part::GreyRamp, GREY_RAMP),
        (Part::ColourRamps, COLOUR_RAMPS),
        (Part::Swatches, SWATCHES_AREA),
        (Part::Lines, LINES_AREA),
        (Part::Glyphs, GLYPHS_AREA),
    ];
    let mut whole = Sums::default();
    let mut by_part = [Sums::default(); 6];
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let (e, s) = (expected.get_pixel(x, y), seen(x, y));
            whole.add(e, &s);
            if let Some(i) = parts.iter().position(|(_, a)| a.contains(x, y)) {
                by_part[i].add(e, &s);
            }
        }
    }

    let mean_in = |area: Area| -> [u8; 3] {
        let mut sum = [0u64; 3];
        for y in area.y..area.y + area.h {
            for x in area.x..area.x + area.w {
                let px = seen(x, y);
                (0..3).for_each(|c| sum[c] += u64::from(px.0[c]));
            }
        }
        let n = u64::from(area.w * area.h).max(1);
        sum.map(|s| ((s + n / 2) / n) as u8)
    };
    let off = |a: [u8; 3], b: [u8; 3]| (0..3).map(|c| a[c].abs_diff(b[c])).max().unwrap_or(0);

    // The grey ramp, from the middle of each step.
    let steps: Vec<u8> = (0..256u32)
        .map(|step| {
            let area = Area::new(RAMP_X + step * RAMP_STEP + 1, GREY_RAMP.y + 4, RAMP_STEP - 2, GREY_RAMP.h - 8);
            let [r, g, b] = mean_in(area);
            luma(&Rgba([r, g, b, 255]))
        })
        .collect();
    let largest = steps.iter().enumerate().map(|(i, &v)| (i as u8).abs_diff(v)).max().unwrap_or(0);
    let ramp = RampCheck { black: steps[0], white: steps[255], largest, squeezed: steps[0] >= 10 && steps[255] <= 245 };

    let swatches = SWATCHES
        .iter()
        .enumerate()
        .map(|(i, swatch)| {
            let a = pattern::swatch_area(i);
            let seen = mean_in(Area::new(a.x + 8, a.y + 8, a.w - 16, a.h - 16));
            SwatchCheck { name: swatch.name, expected: swatch.rgb, seen, off: off(seen, swatch.rgb) }
        })
        .collect();

    let mut lines = Vec::new();
    for b in &LINE_BOXES {
        for (i, &colour) in LINE_COLOURS.iter().enumerate() {
            if colour == b.ground {
                continue;
            }
            let at = pattern::line_at(b, i);
            // The line's middle, away from its ends, and the pixels beside it at `d`.
            let beside = |d: i32| -> [u8; 3] {
                let to = (at as i32 + d) as u32;
                if b.vertical {
                    mean_in(Area::new(to, b.area.y + LINE_INSET + 8, 1, b.area.h - 2 * LINE_INSET - 16))
                } else {
                    mean_in(Area::new(b.area.x + LINE_INSET + 8, to, b.area.w - 2 * LINE_INSET - 16, 1))
                }
            };
            let line = beside(0);
            let reach = |sign: i32| {
                (1..=BLEED_REACH as i32).take_while(|&d| off(beside(sign * d), b.ground) > BLEED_LEVEL).count() as u32
            };
            lines.push(LineCheck {
                colour,
                ground: b.ground,
                vertical: b.vertical,
                seen: line,
                off: off(line, colour),
                bleed: reach(-1).max(reach(1)),
            });
        }
    }
    let mut spread: Vec<u32> = lines.iter().map(|l| l.bleed).collect();
    spread.sort_unstable();
    let bleed = Bleed {
        largest: spread.last().copied().unwrap_or(0),
        typical: spread.get(spread.len() / 2).copied().unwrap_or(0),
    };

    let glyphs = (0..STRIPS.len()).map(|i| read_strip(frame, ox, oy, &under, i)).collect();

    Found {
        x: ox,
        y: oy,
        window,
        channels: whole.channels(),
        parts: parts.iter().zip(by_part).map(|((part, _), sums)| PartError { part: *part, channels: sums.channels() }).collect(),
        ramp,
        swatches,
        lines,
        bleed,
        glyphs,
    }
}

/// Strip `i` read back from the frame, by the reader its [`ReadBy`] names.
fn read_strip(frame: &Frame, ox: u32, oy: u32, under: &Frame, i: usize) -> GlyphCheck {
    let strip = &STRIPS[i];
    let laid = pattern::laid(i);
    let area = pattern::strip_area(i);
    let crop = Frame::from_fn(area.w, area.h, |x, y| *frame.get_pixel(ox + area.x + x, oy + area.y + y));

    // The nearest-colour cut, for the stand-in reader and the count of flipped font pixels.
    let dist = |a: &Rgba<u8>, b: &Rgba<u8>| (0..3).map(|c| (i32::from(a.0[c]) - i32::from(b.0[c])).pow(2)).sum::<i32>();
    let rows = laid.ink.len();
    let cut: Vec<Vec<bool>> = (0..rows)
        .map(|r| {
            (0..laid.columns())
                .map(|c| {
                    let inked = laid
                        .pixels(r, c)
                        .filter(|&(x, y)| {
                            let s = frame.get_pixel(ox + x, oy + y);
                            let ink = Rgba([strip.ink[0], strip.ink[1], strip.ink[2], 255]);
                            dist(s, &ink) < dist(s, under.get_pixel(x, y))
                        })
                        .count();
                    inked * 4 >= 3 * (FONT_SCALE * FONT_SCALE) as usize
                })
                .collect()
        })
        .collect();
    let flipped = (0..rows)
        .flat_map(|r| (0..laid.columns()).map(move |c| (r, c)))
        .filter(|&(r, c)| cut[r][c] != laid.ink[r][c])
        .count() as u32;

    let read = match strip.read_by {
        ReadBy::QuestReader => {
            let band = Rect::new(0.0, 0.0, area.w as f32, area.h as f32);
            gamefont::ink_grid(&crop, band, FONT_SCALE, quest::is_text)
                .and_then(|grid| gamefont::line_rows(&questfont::OBJECTIVE, &grid, grid.len() / 2))
                .map(|rows| gamefont::read_line(&questfont::OBJECTIVE, &rows))
        }
        ReadBy::PickupReader => {
            let whole = (0, 0, crop.width(), crop.height());
            pickup::look::locate(&crop, pickup::look::FULL, whole)
                .and_then(|layout| pickup::look::line_ink(&crop, &layout, 0))
                .and_then(|ink| pickup::font::read_line(&ink))
        }
        ReadBy::StandIn => match strip.font {
            FontKind::Quest => Some(gamefont::read_line(&questfont::OBJECTIVE, &cut)),
            // The Pickup Log's font is read right to left.
            FontKind::Pickup => {
                let reversed: Vec<Vec<bool>> = cut.iter().map(|row| row.iter().rev().copied().collect()).collect();
                pickup::font::read_line(&reversed)
            }
        },
    };

    GlyphCheck {
        name: strip.name,
        read_by: strip.read_by,
        expected: strip.text,
        read,
        font_pixels: (rows * laid.columns()) as u32,
        flipped,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    const INFO: WindowInfo = WindowInfo { width: 1280, height: 752, scale_milli: 1250 };

    /// `pattern` inside a bigger frame at `(x, y)`, as a shared window with a title bar arrives.
    fn framed(pattern: &Frame, x: u32, y: u32) -> Frame {
        Frame::from_fn(pattern.width() + x + 9, pattern.height() + y + 9, |px, py| {
            if px >= x && py >= y && px - x < pattern.width() && py - y < pattern.height() {
                *pattern.get_pixel(px - x, py - y)
            } else {
                Rgba([32, 32, 36, 255])
            }
        })
    }

    /// What video coding's 4:2:0 does: the picture turned to luma and colour (BT.709, video
    /// levels), the colour kept once per 2 x 2 pixels, then turned back. As Chrome's capture does
    /// when it hands over NV12 or I420 frames.
    pub(crate) fn yuv420(frame: &Frame) -> Frame {
        let (w, h) = frame.dimensions();
        let to_yuv = |p: &Rgba<u8>| {
            let [r, g, b] = [0, 1, 2].map(|c| f32::from(p.0[c]) / 255.0);
            let y = 0.2126 * r + 0.7152 * g + 0.0722 * b;
            (16.0 + 219.0 * y, 128.0 + 224.0 * (b - y) / 1.8556, 128.0 + 224.0 * (r - y) / 1.5748)
        };
        let quant = |v: f32| v.round().clamp(0.0, 255.0);
        let luma: Vec<f32> = frame.pixels().map(|p| quant(to_yuv(p).0)).collect();
        let (cw, ch) = (w.div_ceil(2), h.div_ceil(2));
        let mut chroma = vec![(0.0f32, 0.0f32); (cw * ch) as usize];
        for cy in 0..ch {
            for cx in 0..cw {
                let mut sum = (0.0, 0.0);
                let mut n = 0.0;
                for (x, y) in [(0, 0), (1, 0), (0, 1), (1, 1)].map(|(dx, dy)| (cx * 2 + dx, cy * 2 + dy)) {
                    if x < w && y < h {
                        let (_, u, v) = to_yuv(frame.get_pixel(x, y));
                        sum = (sum.0 + u, sum.1 + v);
                        n += 1.0;
                    }
                }
                chroma[(cy * cw + cx) as usize] = (quant(sum.0 / n), quant(sum.1 / n));
            }
        }
        Frame::from_fn(w, h, |x, y| {
            let yy = (luma[(y * w + x) as usize] - 16.0) / 219.0;
            let (u, v) = chroma[((y / 2) * cw + x / 2) as usize];
            let (u, v) = ((u - 128.0) / 224.0 * 1.8556, (v - 128.0) / 224.0 * 1.5748);
            let r = yy + v;
            let b = yy + u;
            let g = (yy - 0.2126 * r - 0.0722 * b) / 0.7152;
            let px = |c: f32| (c * 255.0).round().clamp(0.0, 255.0) as u8;
            Rgba([px(r), px(g), px(b), 255])
        })
    }

    #[test]
    fn finds_the_pattern_below_a_title_bar_and_reads_its_code() {
        let frame = framed(&pattern::draw(INFO), 7, 31);
        let measure = measure(&frame, None);
        let found = measure.found.expect("found");
        assert_eq!((found.x, found.y, found.window), (7, 31, INFO));
        assert_eq!(locate(&frame, Some((7, 31))).map(|(at, _)| at), Some((7, 31)), "the hint is tried first");
    }

    #[test]
    fn a_perfect_capture_has_no_error_and_reads_every_strip() {
        let found = measure(&framed(&pattern::draw(INFO), 0, 0), None).found.expect("found");
        assert!(found.channels.iter().all(|c| c.largest == 0), "{:?}", found.channels);
        assert_eq!((found.ramp.black, found.ramp.white, found.ramp.largest, found.ramp.squeezed), (0, 255, 0, false));
        assert!(found.swatches.iter().all(|s| s.off == 0));
        assert_eq!((found.bleed.largest, found.bleed.typical), (0, 0));
        for glyph in &found.glyphs {
            assert!(glyph.reads(), "{} read as {:?}", glyph.name, glyph.read);
            assert_eq!(glyph.flipped, 0, "{}", glyph.name);
        }
    }

    #[test]
    fn a_frame_too_small_or_without_the_pattern_says_so() {
        let small = Frame::from_pixel(800, 600, Rgba([0, 0, 0, 255]));
        assert!(measure(&small, None).problem.unwrap().contains("smaller"));
        let blank = Frame::from_pixel(1400, 800, Rgba([90, 90, 90, 255]));
        assert!(measure(&blank, None).problem.unwrap().contains("isn't in this frame"));
    }

    /// What 4:2:0 is expected to do to the pattern, printed for the record:
    /// `cargo test -p leafline-core yuv -- --nocapture`.
    #[test]
    fn yuv_420_loses_colour_beside_thin_lines_but_keeps_greys() {
        let frame = yuv420(&framed(&pattern::draw(INFO), 0, 31));
        let found = measure(&frame, None).found.expect("4:2:0 keeps the code strip");
        assert!(found.ramp.largest <= 2, "greys survive: {:?}", found.ramp);
        assert!(found.bleed.largest >= 1, "colour spreads beside 1-px lines");
        println!("channels {:?}", found.channels);
        println!("ramp {:?}  bleed {:?}", found.ramp, found.bleed);
        for s in &found.swatches {
            println!("swatch {:<28} {:?} -> {:?} off {}", s.name, s.expected, s.seen, s.off);
        }
        for l in &found.lines {
            println!("line {:?} on {:?} ({}) -> {:?} off {} bleed {}", l.colour, l.ground, if l.vertical { "v" } else { "h" }, l.seen, l.off, l.bleed);
        }
        for g in &found.glyphs {
            println!("glyphs {:<26} {:?} flipped {}/{}", g.name, g.read, g.flipped, g.font_pixels);
        }
    }
}
