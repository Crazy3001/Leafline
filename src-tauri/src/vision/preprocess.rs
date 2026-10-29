//! Image clean-up applied to a region before it is handed to OCR.

use image::{Rgba, imageops};

use super::{Frame, Rect};

const WHITE: Rgba<u8> = Rgba([255, 255, 255, 255]);
const BLACK: Rgba<u8> = Rgba([0, 0, 0, 255]);

pub fn crop(frame: &Frame, rect: Rect) -> Frame {
    let (x, y, w, h) = rect.to_pixels(frame.width(), frame.height());
    imageops::crop_imm(frame, x, y, w.max(1), h.max(1)).to_image()
}

/// Enlarges small text so the OCR engine has enough pixels per glyph.
pub fn upscale(img: &Frame, factor: f32) -> Frame {
    let w = ((img.width() as f32) * factor).round().max(1.0) as u32;
    let h = ((img.height() as f32) * factor).round().max(1.0) as u32;
    imageops::resize(img, w, h, imageops::FilterType::CatmullRom)
}

/// Adds a white margin; OCR engines read text touching the image edge poorly.
pub fn pad(img: &Frame, margin: u32) -> Frame {
    let mut out = Frame::from_pixel(img.width() + 2 * margin, img.height() + 2 * margin, WHITE);
    imageops::replace(&mut out, img, i64::from(margin), i64::from(margin));
    out
}

pub fn luminance(px: &Rgba<u8>) -> u8 {
    let [r, g, b, _] = px.0;
    ((u32::from(r) * 299 + u32::from(g) * 587 + u32::from(b) * 114) / 1000) as u8
}

/// Spread between the strongest and weakest channel: 0 for greys, high for vivid colours.
pub fn saturation(px: &Rgba<u8>) -> u8 {
    let [r, g, b, _] = px.0;
    r.max(g).max(b) - r.min(g).min(b)
}

/// Colour keying: renders pixels matching `is_text` as black on white, which OCR engines
/// read far better than coloured text over a busy background.
pub fn keep(img: &Frame, is_text: impl Fn(&Rgba<u8>) -> bool) -> Frame {
    Frame::from_fn(img.width(), img.height(), |x, y| if is_text(img.get_pixel(x, y)) { BLACK } else { WHITE })
}

/// Keeps bright, near-grey pixels (white UI text) and drops everything else, including
/// colourful scenery showing through translucent panels.
pub fn isolate_light_text(img: &Frame, min_luminance: u8, max_saturation: u8) -> Frame {
    keep(img, |px| luminance(px) >= min_luminance && saturation(px) <= max_saturation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_text_is_kept_and_colour_dropped() {
        let pixels = [Rgba([250, 250, 250, 255]), Rgba([20, 20, 30, 255]), Rgba([250, 150, 60, 255])];
        let img = Frame::from_fn(3, 1, |x, _| pixels[x as usize]);
        let out = isolate_light_text(&img, 150, 70);
        assert_eq!(out.get_pixel(0, 0), &BLACK);
        assert_eq!(out.get_pixel(1, 0), &WHITE);
        assert_eq!(out.get_pixel(2, 0), &WHITE);
    }

    #[test]
    fn pad_adds_white_margin() {
        let img = Frame::from_pixel(2, 2, BLACK);
        let out = pad(&img, 3);
        assert_eq!(out.dimensions(), (8, 8));
        assert_eq!(out.get_pixel(0, 0), &WHITE);
        assert_eq!(out.get_pixel(3, 3), &BLACK);
    }
}
