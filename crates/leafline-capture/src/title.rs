//! The minimap title on the website, which has no Windows OCR: its words by ocrs (a Rust OCR
//! engine whose recognition model the page downloads once), its trailing roman numeral by
//! counting the thin bars at the end of the line, and the result settled against the known maps
//! the way the program settles its own readings. Only a known map counts, so a misread reads as
//! no map rather than a wrong one.
//!
//! Measured on 1,043 saved frames with a title (the program's and a browser's): 1,041 settle on
//! the same map as the program's reading, none on another; the two misses are `Line 1 <Area 1>`.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use image::{Rgba, RgbImage, imageops};
use leafline_core::mapdata::atlas::Atlas;
use leafline_core::readers::map::{self, MapReading};
use leafline_core::vision::{Frame, preprocess};
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;
use rten_imageproc::{RectF, RotatedRect};

/// The share of a line's height below which a full-height glyph is a numeral's bar (an I).
const BAR_WIDTH: f32 = 0.35;
/// Room around each line handed to the recogniser, in pixels.
const LINE_PAD: f32 = 3.0;
/// White space around the title, in pixels.
const MARGIN: u32 = 20;

pub struct TitleReader {
    engine: OcrEngine,
    /// The last title's picture (as ink) and what it read as, so an unchanged title isn't read
    /// again.
    last: Option<(u64, Option<RawTitle>)>,
}

/// A title as read, before it is settled against the known maps.
#[derive(Debug, Clone, PartialEq)]
pub struct RawTitle {
    pub street: Option<String>,
    pub name: String,
    pub collapsed: bool,
}

impl TitleReader {
    /// From the bytes of ocrs's text recognition model (`text-recognition.onnx`).
    pub fn new(recognition: Vec<u8>) -> Result<Self, String> {
        let model = Model::load(recognition).map_err(|e| e.to_string())?;
        let engine = OcrEngine::new(OcrEngineParams { recognition_model: Some(model), ..Default::default() })
            .map_err(|e| e.to_string())?;
        Ok(Self { engine, last: None })
    }

    /// The title in `frame`, as read; `None` without a minimap or when nothing could be read.
    pub fn read(&mut self, frame: &Frame) -> Option<RawTitle> {
        let (area, collapsed) = map::title_area(frame)?;
        let (x, y, w, h) = area.to_pixels(frame.width(), frame.height());
        if w == 0 || h == 0 {
            return None;
        }
        let crop = imageops::crop_imm(frame, x, y, w, h).to_image();
        let is_ink = |p: &Rgba<u8>| {
            let l = preprocess::luminance(p);
            if collapsed { l <= 90 } else { l >= 200 }
        };
        let key = {
            let mut hasher = DefaultHasher::new();
            (w, h, collapsed).hash(&mut hasher);
            for p in crop.pixels() {
                is_ink(p).hash(&mut hasher);
            }
            hasher.finish()
        };
        if let Some((last, title)) = &self.last
            && *last == key
        {
            return title.clone();
        }
        let title = self.recognise(frame, &crop, collapsed, is_ink);
        self.last = Some((key, title.clone()));
        title
    }

    fn recognise(&self, frame: &Frame, crop: &Frame, collapsed: bool, is_ink: impl Fn(&Rgba<u8>) -> bool) -> Option<RawTitle> {
        // The title as drawn (the model reads pictures, not cut-outs), on its own background.
        let fill = crop.get_pixel(0, 0).0;
        let mut padded = RgbImage::from_pixel(crop.width() + 2 * MARGIN, crop.height() + 2 * MARGIN, image::Rgb([fill[0], fill[1], fill[2]]));
        for (px, py, p) in crop.enumerate_pixels() {
            padded.put_pixel(px + MARGIN, py + MARGIN, image::Rgb([p.0[0], p.0[1], p.0[2]]));
        }
        // Each line where its letters are, which spares the detection model.
        let inked_row = |y: u32| (0..crop.width()).any(|x| is_ink(crop.get_pixel(x, y)));
        let mut rows: Vec<(u32, u32)> = Vec::new();
        let mut start = None;
        for y in 0..=crop.height() {
            match (y < crop.height() && inked_row(y), start) {
                (true, None) => start = Some(y),
                (false, Some(top)) => {
                    rows.push((top, y));
                    start = None;
                }
                _ => {}
            }
        }
        let tallest = rows.iter().map(|(t, b)| b - t).max().unwrap_or(0);
        rows.retain(|(t, b)| (b - t) * 2 >= tallest);
        let pad = MARGIN as f32;
        let lines: Vec<Vec<RotatedRect>> = rows
            .iter()
            .filter_map(|&(top, bottom)| {
                let inked = |x: u32| (top..bottom).any(|y| is_ink(crop.get_pixel(x, y)));
                let left = (0..crop.width()).find(|&x| inked(x))? as f32;
                let right = (0..crop.width()).rev().find(|&x| inked(x))? as f32 + 1.0;
                let rect = RectF::from_tlbr(
                    top as f32 + pad - LINE_PAD,
                    left + pad - LINE_PAD,
                    bottom as f32 + pad + LINE_PAD,
                    right + pad + LINE_PAD,
                );
                Some(vec![RotatedRect::from_rect(rect)])
            })
            .collect();
        if lines.is_empty() {
            return None;
        }
        let source = ImageSource::from_bytes(padded.as_raw(), padded.dimensions()).ok()?;
        let input = self.engine.prepare_input(source).ok()?;
        let texts: Vec<String> = self
            .engine
            .recognize_text(&input, &lines)
            .ok()?
            .into_iter()
            .flatten()
            .map(|line| line.to_string().trim().to_string())
            .filter(|t| !t.is_empty())
            .collect();
        let (street, name) = if collapsed {
            let text = texts.join(" ");
            match text.split_once(':') {
                Some((street, name)) => (Some(street.trim().to_string()), name.trim().to_string()),
                None => (None, text.trim().to_string()),
            }
        } else {
            match texts.as_slice() {
                [] => return None,
                [name] => (None, name.clone()),
                [street, rest @ ..] => (Some(street.clone()), rest.join(" ")),
            }
        };
        let name = with_numeral(&name, frame);
        (name.len() >= 2).then_some(RawTitle { street, name, collapsed })
    }
}

/// `name` with its trailing roman numeral as counted from the picture: after the title's last
/// word gap, a run of thin, full-height bars is that many I's. OCR reads them poorly (`Rocky Road
/// III` as `Rocky Roarl 1`), and they are what tells a map from its neighbours.
fn with_numeral(name: &str, frame: &Frame) -> String {
    let base = name.trim_end_matches(|c: char| "Il1|!?[]{}".contains(c) || c.is_whitespace());
    let base = base.strip_suffix(" I").unwrap_or(base).trim_end().to_string();
    let Some(title) = map::title_glyphs(frame) else { return base };
    let Some(line) = title.lines.last() else { return base };
    let height = line.iter().map(|(_, m)| m.height()).max().unwrap_or(0) as f32;
    let start = line.iter().rposition(|(gap, _)| *gap as f32 >= 0.25 * height).unwrap_or(0);
    let word = &line[start..];
    let bars = !word.is_empty()
        && word.len() <= 4
        && word.iter().all(|(_, m)| {
            let inked = (0..m.height()).filter(|&y| (0..m.width()).any(|x| m.get(x, y))).count();
            (m.width() as f32) <= BAR_WIDTH * height && inked as f32 >= 0.6 * m.height() as f32
        });
    if bars { format!("{base} {}", "I".repeat(word.len())) } else { base }
}

/// A title settled against the known maps, as the program settles its own: the map's own name
/// and street, whether it is a town, and what each of its monsters gives.
pub struct Settled {
    pub reading: MapReading,
    pub town: bool,
    pub kill_exp: Vec<u32>,
}

pub fn settle(atlas: &Atlas, title: &RawTitle, rect: leafline_core::vision::Rect) -> Option<Settled> {
    let known = atlas.settled(&title.name, title.street.as_deref())?;
    let town = atlas.is_town(known.id);
    let kill_exp = atlas.card(known.id).map(|c| c.mobs.iter().map(|m| m.exp).collect()).unwrap_or_default();
    Some(Settled {
        reading: MapReading { street: known.street, name: known.name, collapsed: title.collapsed, rect, twin: None },
        town,
        kill_exp,
    })
}
