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
/// Gauge fills end within this many design pixels of the window's bottom edge.
const GAUGE_REACH: f32 = 20.0;
/// Two scales this close are the same scale.
const SAME: f32 = 0.03;
/// Channels at or below this are taken as black in a [`Canonical::sharp`] view: the game's own
/// text darkness at Default Ratio, where it is drawn unsmoothed, and with Filter Applied, which
/// smooths it. Tried on one frame of each: the Quest Helper read every row at these, and lost
/// rows the other way round.
pub const NEAR_BLACK: u8 = 40;
pub const NEAR_BLACK_SMOOTHED: u8 = 70;
/// Of a panel's dark grey pixels, the share that are pure black: about 0.9 at Ideal Ratio, 0.6
/// at Default Ratio and 0.3 with Filter Applied (one frame each).
const SMOOTHED_BELOW: f32 = 0.42;

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

/// Whether fills `pitch` apart, the first ending at `bottom`, could be the HUD's at a size the
/// game draws in this frame. Scenery can match the spacing by chance, but not on the bottom edge.
pub fn fits_gauges(frame: &Frame, pitch: f32, bottom: f32) -> bool {
    let above_edge = frame.height() as f32 - bottom;
    scales(frame, None)
        .iter()
        .any(|scale| (pitch / GAUGE_PITCH - scale).abs() <= SAME * scale && above_edge <= GAUGE_REACH * scale)
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
        Self::drawn(frame, scale, None, sample)
    }

    /// Only the part of the window around `area`, which is quicker to draw than all of it.
    /// Rectangles still map back with [`Self::to_window`].
    pub fn within(frame: &Frame, scale: f32, area: Rect) -> Self {
        Self::drawn(frame, scale, Some(area), sample)
    }

    /// A view of `area` for readers that key on pure black text (the Quest Helper). With Filter
    /// Applied the game blurs its black text, so near-black is taken back to black.
    pub fn sharp(frame: &Frame, scale: f32, area: Rect, near_black: u8) -> Self {
        Self::drawn(frame, scale, Some(area), |frame, x, y| {
            let px = sample(frame, x, y);
            if px.0[..3].iter().all(|&c| c <= near_black) { Rgba([0, 0, 0, 255]) } else { px }
        })
    }

    /// Like [`Self::sharp`], but a design pixel is black when any window pixel centred inside
    /// it is near-black, not only its middle: at 1.875x a one-pixel stroke, such as the bar
    /// across an 8, can sit off the middle of its design pixel, and sampling the middle alone
    /// loses it (8/8 read as 0/0). For cutting out digits only: it thickens letters, which
    /// OCR then reads worse.
    pub fn inked(frame: &Frame, scale: f32, area: Rect, near_black: u8) -> Self {
        let dark = |px: &Rgba<u8>| px.0[..3].iter().all(|&c| c <= near_black);
        Self::drawn(frame, scale, Some(area), |frame, x, y| {
            let half = scale / 2.0;
            // Window pixels whose centres (i + 0.5) fall within [centre - half, centre + half).
            let span = |centre: f32, limit: u32| {
                let first = (centre - half - 0.5).ceil().max(0.0) as u32;
                let end = ((centre + half - 0.5).ceil().max(0.0) as u32).min(limit);
                first..end
            };
            let inked = span(y, frame.height())
                .any(|py| span(x, frame.width()).any(|px| dark(frame.get_pixel(px, py))));
            if inked { Rgba([0, 0, 0, 255]) } else { sample(frame, x, y) }
        })
    }

    fn drawn(frame: &Frame, scale: f32, area: Option<Rect>, sample: impl Fn(&Frame, f32, f32) -> Rgba<u8>) -> Self {
        let (width, height) = (frame.width() as f32, frame.height() as f32);
        // Everything within the window, on the same grid as the design area, which is centred
        // along the bottom edge. Off that grid, each pixel would blend two design pixels.
        let origin = (width - DESIGN_WIDTH * scale) / 2.0;
        let grid_left = origin.rem_euclid(scale);
        let (all_columns, all_rows) = (((width - grid_left) / scale).floor(), (height / scale).floor());
        let grid_top = height - all_rows * scale;
        // The design pixels `area` touches, or all of them.
        let (first_column, last_column, first_row, last_row) = match area {
            Some(area) => (
                ((area.x - grid_left) / scale).floor().clamp(0.0, all_columns),
                ((area.right() - grid_left) / scale).ceil().clamp(0.0, all_columns),
                ((area.y - grid_top) / scale).floor().clamp(0.0, all_rows),
                ((area.bottom() - grid_top) / scale).ceil().clamp(0.0, all_rows),
            ),
            None => (0.0, all_columns, 0.0, all_rows),
        };
        let (columns, rows) = (last_column - first_column, last_row - first_row);
        let (left, top) = (grid_left + first_column * scale, grid_top + first_row * scale);
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

    /// Where a rectangle of the window is in this view.
    pub fn to_view(&self, rect: Rect) -> Rect {
        rect.translated(-self.left, -self.top).scaled(CANONICAL_SCALE / self.scale)
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

/// Whether the game smoothed its UI (UI Size "Default Ratio (Filter Applied)"), judged by how much
/// of the black text in `rect` (a panel with black text on a light ground) is still pure black.
/// `None` when there is too little dark to tell.
pub fn smoothed(frame: &Frame, rect: Rect) -> Option<bool> {
    let (x, y, w, h) = rect.to_pixels(frame.width(), frame.height());
    let (mut pure, mut dark) = (0u32, 0u32);
    for py in y..y + h {
        for px in x..x + w {
            let [r, g, b, _] = frame.get_pixel(px, py).0;
            let (high, low) = (r.max(g).max(b), r.min(g).min(b));
            if high <= 80 && high - low <= 30 {
                dark += 1;
                pure += u32::from(high <= 8);
            }
        }
    }
    (dark >= 200).then(|| (pure as f32 / dark as f32) < SMOOTHED_BELOW)
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
    fn smoothed_text_is_told_from_crisp() {
        // Made-up text strokes on a light panel: crisp ones are pure black, smoothed ones grey.
        let panel = |stroke: [u8; 3]| {
            Frame::from_fn(60, 40, |x, _| {
                if x % 3 == 0 { Rgba([stroke[0], stroke[1], stroke[2], 255]) } else { Rgba([200, 210, 220, 255]) }
            })
        };
        let whole = Rect::new(0.0, 0.0, 60.0, 40.0);
        assert_eq!(smoothed(&panel([0, 0, 0]), whole), Some(false));
        assert_eq!(smoothed(&panel([40, 40, 44]), whole), Some(true));
        assert_eq!(smoothed(&Frame::from_pixel(60, 40, Rgba([200, 210, 220, 255])), whole), None);
    }

    #[test]
    fn a_part_of_the_window_is_drawn_like_the_whole() {
        let scale = 1.875;
        let frame = lit_cell(scale, 1920, 1080, (300, 200));
        let whole = Canonical::new(&frame, scale);
        let part = Canonical::within(&frame, scale, Rect::new(500.0, 300.0, 200.0, 150.0));
        assert!(part.frame.width() < 250 && part.frame.height() < 200);
        let lit = |view: &Canonical| {
            let (x, y) =
                view.frame.enumerate_pixels().find(|(_, _, px)| px.0[0] > 150).map(|(x, y, _)| (x, y)).unwrap();
            view.to_window(Rect::new(x as f32, y as f32, 1.0, 1.0))
        };
        let (a, b) = (lit(&whole), lit(&part));
        assert!((a.x - b.x).abs() < 1e-3 && (a.y - b.y).abs() < 1e-3);
    }

    #[test]
    fn rectangles_map_back_to_the_window() {
        let view = Canonical::new(&Frame::new(2560, 1440), 1.875);
        let rect = view.to_window(Rect::new(0.0, 2.0, 2.0, 2.0));
        assert!((rect.w - 1.875).abs() < 1e-3 && rect.y - view.top - 1.875 < 1e-3);
    }
}
