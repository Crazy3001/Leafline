//! The party size: how many HP bars stand in the party list.
//!
//! Each member has a bar, a dark frame with a light rim inside it, and the bars stand in an
//! evenly spaced column. It goes by their shape and never reads text, so other players' names are
//! not looked at. No list in view, or a single bar, means the player is on their own.

use super::hud::HudLayout;
use super::ui;
use crate::vision::Frame;
use crate::vision::blobs::{Bitmap, Blob};

/// A bar's outer size, and the step from one bar to the next, in UI pixels.
const BAR_WIDTH: (f32, f32) = (62.0, 80.0);
const BAR_HEIGHT: (f32, f32) = (10.0, 13.8);
const BAR_PITCH: (f32, f32) = (15.0, 19.5);
const MAX_MEMBERS: usize = 6;
/// The frame is darker than this, and the rim just inside it lighter than this.
const DARK: u8 = 70;
const RIM: u8 = 150;
/// Share of the rim that must be light, and how much of the inside may be dark.
const RIM_SHARE: f32 = 0.6;
const DARK_SHARE: f32 = 0.2;

/// The party's size, the player included: 1 when no party can be told.
pub fn size(frame: &Frame, hud: Option<&HudLayout>) -> u32 {
    let value = |x: u32, y: u32| frame.get_pixel(x, y).0[..3].iter().copied().max().unwrap_or(0);
    let dark = Bitmap::from_fn(frame.width(), frame.height(), |x, y| value(x, y) < DARK);
    let blobs: Vec<Blob> = dark.blobs().into_iter().filter(|b| b.w > 3 * b.h).collect();

    let mut counts = ui::scales(frame, hud).into_iter().filter_map(|scale| {
        let bars: Vec<Blob> = blobs.iter().copied().filter(|b| is_bar(frame, b, scale)).collect();
        let column = longest_column(&bars, scale);
        (column >= 2).then_some(column)
    });
    match counts.next() {
        Some(count) if count <= MAX_MEMBERS => count as u32,
        _ => 1,
    }
}

fn is_bar(frame: &Frame, blob: &Blob, scale: f32) -> bool {
    let (w, h) = (blob.w as f32 / scale, blob.h as f32 / scale);
    if !(BAR_WIDTH.0..=BAR_WIDTH.1).contains(&w) || !(BAR_HEIGHT.0..=BAR_HEIGHT.1).contains(&h) {
        return false;
    }
    let value = |x: u32, y: u32| frame.get_pixel(x, y).0[..3].iter().copied().max().unwrap_or(0);
    // The frame and its rim are about this thick.
    let edge = (2.2 * scale).ceil() as u32 + 1;
    if blob.h <= 2 * edge + 2 || blob.w <= 2 * edge {
        return false;
    }
    let (mut inside, mut dark) = (0u32, 0u32);
    for y in blob.y + edge..blob.y + blob.h - edge {
        for x in blob.x + edge..blob.x + blob.w - edge {
            inside += 1;
            dark += u32::from(value(x, y) < DARK);
        }
    }
    if inside == 0 || dark as f32 > DARK_SHARE * inside as f32 {
        return false;
    }
    let columns = blob.x + blob.w / 4..blob.x + blob.w - blob.w / 4;
    let lightest = |rows: std::ops::Range<u32>, x: u32| rows.map(|y| value(x, y)).max().unwrap_or(0);
    let (mut rim, mut light) = (0u32, 0u32);
    for x in columns {
        for rows in [blob.y + 1..blob.y + edge, blob.y + blob.h - edge..blob.y + blob.h - 1] {
            rim += 1;
            light += u32::from(lightest(rows, x) >= RIM);
        }
    }
    light as f32 >= RIM_SHARE * rim as f32
}

/// How many bars stand in the longest evenly spaced column.
fn longest_column(bars: &[Blob], scale: f32) -> usize {
    let tolerance = (2.0 * scale).max(2.0);
    let mut bars = bars.to_vec();
    bars.sort_by_key(|b| b.y);
    let mut best = 0;
    for (i, first) in bars.iter().enumerate() {
        let lined_up = bars[i..]
            .iter()
            .filter(|b| b.x.abs_diff(first.x) as f32 <= tolerance && b.w.abs_diff(first.w) as f32 <= tolerance);
        let (mut last, mut pitch, mut count) = (first.y, None::<f32>, 1);
        for bar in lined_up.skip(1) {
            let step = (bar.y - last) as f32;
            let fits = match pitch {
                Some(pitch) => (step - pitch).abs() <= tolerance,
                None => (BAR_PITCH.0 * scale..=BAR_PITCH.1 * scale).contains(&step),
            };
            if !fits {
                break;
            }
            pitch = Some(step);
            last = bar.y;
            count += 1;
        }
        best = best.max(count);
    }
    best
}

#[cfg(test)]
mod tests {
    use image::Rgba;

    use super::*;

    /// A frame of 2x UI with made-up HP bars drawn down a column.
    fn party_frame(bars: u32) -> Frame {
        let mut frame = Frame::from_pixel(1280, 720, Rgba([120, 160, 200, 255]));
        for i in 0..bars {
            let (left, top) = (900, 100 + i * 34);
            for y in top..top + 24 {
                for x in left..left + 140 {
                    let ring = x - left < 2 || left + 140 - x <= 2 || y - top < 2 || top + 24 - y <= 2;
                    let rim = x - left < 5 || left + 140 - x <= 5 || y - top < 5 || top + 24 - y <= 5;
                    let colour = match (ring, rim) {
                        (true, _) => Rgba([20, 20, 24, 255]),
                        (_, true) => Rgba([235, 235, 235, 255]),
                        _ => Rgba([200, 30, 40, 255]),
                    };
                    frame.put_pixel(x, y, colour);
                }
            }
        }
        frame
    }

    #[test]
    fn counts_the_bars_in_a_column() {
        // No HUD in these frames: 2x is one of the sizes tried.
        assert_eq!(size(&party_frame(4), None), 4);
        assert_eq!(size(&party_frame(2), None), 2);
    }

    #[test]
    fn no_list_or_one_bar_is_solo() {
        assert_eq!(size(&party_frame(0), None), 1);
        assert_eq!(size(&party_frame(1), None), 1);
    }
}
