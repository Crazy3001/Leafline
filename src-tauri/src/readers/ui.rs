//! How big the game draws its UI.
//!
//! The UI is laid out on a 1024 x 768 design area, centred along the window's bottom edge, then
//! drawn bigger: by window height / 768 at UI Size Default, in whole steps at Ideal Ratio. Either
//! way each design pixel covers a fixed patch of the window, so a picture of the UI at any size
//! can be put back on the design grid. That lets one learned font read every size.

use image::Rgba;

use super::hud::HudLayout;
use crate::vision::{Frame, Rect};

pub const DESIGN_WIDTH: f32 = 1024.0;
pub const DESIGN_HEIGHT: f32 = 768.0;
/// The size fonts are learned and read at: the crisp doubled pixels of Ideal Ratio at 1440p.
pub const CANONICAL_SCALE: f32 = 2.0;
/// Gauge starts stand this many design pixels apart.
const GAUGE_PITCH: f32 = 108.0;
/// Two scales this close are the same scale.
const SAME: f32 = 0.03;

/// The scales the UI may be drawn at in this frame, most likely first.
pub fn scales(frame: &Frame, hud: Option<&HudLayout>) -> Vec<f32> {
    let window = (frame.height() as f32 / DESIGN_HEIGHT).max(1.0);
    let mut scales: Vec<f32> = hud.map(measured).into_iter().collect();
    for scale in [window, 2.0, 1.0] {
        if !scales.iter().any(|s| (s - scale).abs() <= SAME * scale) {
            scales.push(scale);
        }
    }
    scales
}

/// Whether gauges this far apart could be the HUD's at a size the game draws in this frame.
pub fn fits_gauge_pitch(frame: &Frame, pitch: f32) -> bool {
    scales(frame, None).iter().any(|scale| (pitch / GAUGE_PITCH - scale).abs() <= SAME * scale)
}

fn measured(hud: &HudLayout) -> f32 {
    (hud.mp.x - hud.hp.x) / GAUGE_PITCH
}

/// The scale of the HUD at `hud`: what the window's size allows that fits the gauges' spacing.
pub fn hud_scale(frame: &Frame, hud: &HudLayout) -> f32 {
    let measured = measured(hud);
    let fits = |scale: f32| (scale - measured).abs() <= 2.0 * SAME * scale;
    let window = (frame.height() as f32 / DESIGN_HEIGHT).max(1.0);
    [window, 2.0, 1.0].into_iter().find(|&s| fits(s)).unwrap_or(measured)
}

/// A view of the frame on the design grid, at [`CANONICAL_SCALE`]. Only the bottom of the window
/// is drawn, which is where the HUD lives.
pub struct Canonical {
    pub frame: Frame,
    scale: f32,
    left: f32,
    top: f32,
}

impl Canonical {
    pub fn new(frame: &Frame, scale: f32) -> Self {
        let (width, height) = (frame.width() as f32, frame.height() as f32);
        // Everything within the window, on the same grid as the design area, which is centred
        // along the bottom edge. Off that grid, each pixel would blend two design pixels.
        let origin = (width - DESIGN_WIDTH * scale) / 2.0;
        let left = origin.rem_euclid(scale);
        let columns = ((width - left) / scale).floor();
        let rows = (height / scale).floor();
        let top = height - rows * scale;
        let block = CANONICAL_SCALE as u32;
        let (out_width, out_height) = (columns as u32 * block, rows as u32 * block);
        let sampled = Frame::from_fn(columns as u32, rows as u32, |x, y| {
            sample(frame, left + (x as f32 + 0.5) * scale, top + (y as f32 + 0.5) * scale)
        });
        let doubled = Frame::from_fn(out_width, out_height, |x, y| *sampled.get_pixel(x / block, y / block));
        Self { frame: doubled, scale, left, top }
    }

    /// Where a rectangle of this view is in the window.
    pub fn to_window(&self, rect: Rect) -> Rect {
        let factor = self.scale / CANONICAL_SCALE;
        rect.scaled(factor).translated(self.left, self.top)
    }

    pub fn layout_to_window(&self, layout: &HudLayout) -> HudLayout {
        HudLayout {
            hp: self.to_window(layout.hp),
            mp: self.to_window(layout.mp),
            exp: self.to_window(layout.exp),
            labels: self.to_window(layout.labels),
            level: layout.level.map(|level| self.to_window(level)),
            identity: self.to_window(layout.identity),
        }
    }
}

/// The colour at a point, blended from the four nearest pixels.
fn sample(frame: &Frame, x: f32, y: f32) -> Rgba<u8> {
    let (x, y) = ((x - 0.5).max(0.0), (y - 0.5).max(0.0));
    let (left, top) = (x.floor(), y.floor());
    let (dx, dy) = (x - left, y - top);
    let (right_most, bottom_most) = (frame.width() - 1, frame.height() - 1);
    let (x0, y0) = ((left as u32).min(right_most), (top as u32).min(bottom_most));
    let (x1, y1) = ((x0 + 1).min(right_most), (y0 + 1).min(bottom_most));
    let (a, b, c, d) =
        (frame.get_pixel(x0, y0), frame.get_pixel(x1, y0), frame.get_pixel(x0, y1), frame.get_pixel(x1, y1));
    let blend = |i: usize| {
        let top = f32::from(a.0[i]) * (1.0 - dx) + f32::from(b.0[i]) * dx;
        let bottom = f32::from(c.0[i]) * (1.0 - dx) + f32::from(d.0[i]) * dx;
        (top * (1.0 - dy) + bottom * dy).round() as u8
    };
    Rgba([blend(0), blend(1), blend(2), 255])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One made-up design pixel lit, drawn at `scale` the way the game smooths its picture.
    fn lit_cell(scale: f32, width: u32, height: u32, cell: (u32, u32)) -> Frame {
        let rows = (height as f32 / scale).floor();
        let left = ((width as f32 - DESIGN_WIDTH * scale) / 2.0).rem_euclid(scale);
        let top = height as f32 - rows * scale;
        Frame::from_fn(width, height, |x, y| {
            let (dx, dy) = ((x as f32 + 0.5 - left) / scale, (y as f32 + 0.5 - top) / scale);
            let lit = dx.floor() as u32 == cell.0 && dy.floor() as u32 == cell.1;
            if lit { Rgba([255, 255, 255, 255]) } else { Rgba([0, 0, 0, 255]) }
        })
    }

    #[test]
    fn a_design_pixel_comes_back_doubled_at_any_scale() {
        for scale in [1.875, 1.40625, 2.0, 1.0] {
            let (width, height) = ((1024.0 * scale) as u32, (768.0 * scale) as u32);
            let view = Canonical::new(&lit_cell(scale, width, height, (10, 20)), scale);
            assert!(view.frame.get_pixel(20, 40).0[0] > 150, "scale {scale}");
            assert!(view.frame.get_pixel(21, 41).0[0] > 150, "scale {scale}");
            assert!(view.frame.get_pixel(26, 40).0[0] < 100, "scale {scale}");
            assert!(view.frame.get_pixel(20, 46).0[0] < 100, "scale {scale}");
        }
    }

    #[test]
    fn rectangles_map_back_to_the_window() {
        let view = Canonical::new(&Frame::new(2560, 1440), 1.875);
        let rect = view.to_window(Rect::new(0.0, 2.0, 2.0, 2.0));
        assert!((rect.w - 1.875).abs() < 1e-3 && rect.y - view.top - 1.875 < 1e-3);
    }
}
