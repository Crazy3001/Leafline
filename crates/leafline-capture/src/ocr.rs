//! The website's text recognition, which has no Windows OCR: ocrs (a Rust OCR engine) with only
//! its recognition model, which the page downloads once. Lines are found from the picture itself
//! (rows of ink), which spares the detection model; each reader says what its ink looks like.

use image::{Rgba, RgbImage, imageops};
use leafline_core::vision::{Frame, Rect};
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;
use rten_imageproc::{RectF, RotatedRect};

/// Room around each line handed to the recogniser, in pixels.
const LINE_PAD: f32 = 3.0;
/// Plain background around the picture, in pixels.
const MARGIN: u32 = 20;

pub struct Recogniser {
    engine: OcrEngine,
}

/// A line of ink found in a picture, in the picture's pixels: rows `top..bottom`, columns
/// `left..right`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InkLine {
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
}

impl InkLine {
    pub fn height(&self) -> u32 {
        self.bottom - self.top
    }
}

impl Recogniser {
    /// From the bytes of ocrs's text recognition model (`text-recognition.onnx`).
    pub fn new(recognition: Vec<u8>) -> Result<Self, String> {
        let model = Model::load(recognition).map_err(|e| e.to_string())?;
        let engine = OcrEngine::new(OcrEngineParams { recognition_model: Some(model), ..Default::default() })
            .map_err(|e| e.to_string())?;
        Ok(Self { engine })
    }

    /// The text of each of `lines` in `picture`, in order (an empty string where nothing was
    /// read). The picture is read as drawn, on a margin of its own top-left colour: the model
    /// reads pictures, not cut-outs.
    pub fn read_lines(&self, picture: &Frame, lines: &[InkLine]) -> Vec<String> {
        if lines.is_empty() {
            return Vec::new();
        }
        let fill = picture.get_pixel(0, 0).0;
        let mut padded =
            RgbImage::from_pixel(picture.width() + 2 * MARGIN, picture.height() + 2 * MARGIN, image::Rgb([fill[0], fill[1], fill[2]]));
        for (px, py, p) in picture.enumerate_pixels() {
            padded.put_pixel(px + MARGIN, py + MARGIN, image::Rgb([p.0[0], p.0[1], p.0[2]]));
        }
        let pad = MARGIN as f32;
        let rects: Vec<Vec<RotatedRect>> = lines
            .iter()
            .map(|line| {
                let rect = RectF::from_tlbr(
                    line.top as f32 + pad - LINE_PAD,
                    line.left as f32 + pad - LINE_PAD,
                    line.bottom as f32 + pad + LINE_PAD,
                    line.right as f32 + pad + LINE_PAD,
                );
                vec![RotatedRect::from_rect(rect)]
            })
            .collect();
        let read = || -> Option<Vec<String>> {
            let source = ImageSource::from_bytes(padded.as_raw(), padded.dimensions()).ok()?;
            let input = self.engine.prepare_input(source).ok()?;
            let texts = self.engine.recognize_text(&input, &rects).ok()?;
            Some(texts.into_iter().map(|t| t.map(|l| l.to_string().trim().to_string()).unwrap_or_default()).collect())
        };
        read().unwrap_or_else(|| vec![String::new(); lines.len()])
    }
}

/// `area` of `frame` as a picture of its own.
pub fn crop(frame: &Frame, area: Rect) -> Option<Frame> {
    let (x, y, w, h) = area.to_pixels(frame.width(), frame.height());
    (w > 0 && h > 0).then(|| imageops::crop_imm(frame, x, y, w, h).to_image())
}

/// The lines of ink in `picture`: runs of rows with ink, each trimmed to its inked columns.
/// Runs shorter than half the tallest are dropped as specks and underlines.
pub fn ink_lines(picture: &Frame, is_ink: impl Fn(&Rgba<u8>) -> bool) -> Vec<InkLine> {
    let (width, height) = picture.dimensions();
    let inked_row = |y: u32| (0..width).any(|x| is_ink(picture.get_pixel(x, y)));
    let mut rows: Vec<(u32, u32)> = Vec::new();
    let mut start = None;
    for y in 0..=height {
        match (y < height && inked_row(y), start) {
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
    rows.into_iter()
        .filter_map(|(top, bottom)| {
            let inked = |x: u32| (top..bottom).any(|y| is_ink(picture.get_pixel(x, y)));
            let left = (0..width).find(|&x| inked(x))?;
            let right = (0..width).rev().find(|&x| inked(x))? + 1;
            Some(InkLine { top, bottom, left, right })
        })
        .collect()
}
