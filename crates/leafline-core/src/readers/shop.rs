//! Whether an NPC shop, storage or trade window is open, going by its buttons. Mesos move at
//! these windows without being earned, so the session tracker leaves those changes out.
//!
//! Each of the three has a column of buttons the same size: a shop's green LEAVE STORE over the
//! orange BUY ITEM, storage the same, and a trade's orange TRADE over the green CANCEL. Orange
//! chat lines and lone buttons are not such a column.

use image::{Rgba, imageops};

use crate::vision::Frame;
use crate::vision::blobs::{Bitmap, Blob};

/// The frame is looked at no wider than this, where buttons are still 10 pixels tall or more.
const LOOK_WIDTH: u32 = 1600;

pub fn is_open(frame: &Frame) -> bool {
    let scale = (LOOK_WIDTH as f32 / frame.width() as f32).min(1.0);
    let small;
    let look = if scale < 1.0 {
        small = imageops::thumbnail(frame, LOOK_WIDTH, (frame.height() as f32 * scale).round() as u32);
        &small
    } else {
        frame
    };
    let buttons = |matches: fn(f32, f32, f32) -> bool| -> Vec<Blob> {
        let mask = Bitmap::from_fn(look.width(), look.height(), |x, y| {
            let (h, s, v) = hsv(look.get_pixel(x, y));
            matches(h, s, v)
        });
        mask.closed().blobs().into_iter().filter(is_button).collect()
    };
    let (orange, green) = (buttons(is_orange), buttons(is_green));
    orange.iter().any(|o| green.iter().any(|g| stacked(g, o) || stacked(o, g)))
}

fn is_button(blob: &Blob) -> bool {
    let (w, h) = (blob.w as f32, blob.h as f32);
    h >= 8.0 && (2.5 * h..=7.0 * h).contains(&w) && blob.area as f32 >= 0.6 * w * h
}

/// `lower` is the next button down from `upper`: the same left edge and width, a small gap.
fn stacked(upper: &Blob, lower: &Blob) -> bool {
    let (w, h) = (upper.w as f32, upper.h as f32);
    let gap = lower.y as f32 - (upper.y + upper.h) as f32;
    upper.x.abs_diff(lower.x) as f32 <= 0.08 * w
        && upper.w.abs_diff(lower.w) as f32 <= 0.1 * w
        && (0.0..=0.8 * h).contains(&gap)
}

/// Hue in degrees, saturation and value from 0 to 1.
fn hsv(px: &Rgba<u8>) -> (f32, f32, f32) {
    let [r, g, b] = [px.0[0], px.0[1], px.0[2]].map(|c| f32::from(c) / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let spread = max - min;
    let hue = if spread == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / spread).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / spread + 2.0)
    } else {
        60.0 * ((r - g) / spread + 4.0)
    };
    (hue, if max > 0.0 { spread / max } else { 0.0 }, max)
}

fn is_orange(h: f32, s: f32, v: f32) -> bool {
    (12.0..=32.0).contains(&h) && s >= 0.59 && v >= 0.67
}

fn is_green(h: f32, s: f32, v: f32) -> bool {
    (56.0..=100.0).contains(&h) && s >= 0.24 && v >= 0.55
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORANGE: Rgba<u8> = Rgba([240, 140, 40, 255]);
    const GREEN: Rgba<u8> = Rgba([120, 200, 70, 255]);

    fn with_buttons(buttons: &[(u32, u32, Rgba<u8>)]) -> Frame {
        let mut frame = Frame::from_pixel(1280, 720, Rgba([60, 70, 90, 255]));
        for &(left, top, colour) in buttons {
            for y in top..top + 20 {
                for x in left..left + 90 {
                    frame.put_pixel(x, y, colour);
                }
            }
        }
        frame
    }

    #[test]
    fn a_column_of_green_over_orange_is_a_shop() {
        assert!(is_open(&with_buttons(&[(600, 300, GREEN), (600, 326, ORANGE)])));
    }

    #[test]
    fn a_column_of_orange_over_green_is_a_trade() {
        assert!(is_open(&with_buttons(&[(600, 300, ORANGE), (600, 326, GREEN)])));
    }

    #[test]
    fn lone_buttons_and_nothing_are_no_window() {
        assert!(!is_open(&with_buttons(&[])));
        assert!(!is_open(&with_buttons(&[(600, 300, ORANGE)])));
        assert!(!is_open(&with_buttons(&[(100, 300, GREEN), (600, 326, ORANGE)])));
    }
}
