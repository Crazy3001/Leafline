//! Finding solid blocks of a colour (HUD bars, highlighted labels) without fixed coordinates.

use image::Rgba;

use super::{Frame, Rect};

/// Bounding boxes of horizontal runs of `matches` pixels at least `min_run` long, merged
/// across consecutive rows. Only rows inside `region` are scanned.
pub fn find_blocks(frame: &Frame, region: Rect, min_run: u32, matches: impl Fn(&Rgba<u8>) -> bool) -> Vec<Rect> {
    find_blocks_at(frame, region, min_run, |x, y| matches(frame.get_pixel(x, y)))
}

/// [`find_blocks`] with a test that is given each pixel's position, so it can look at the
/// pixels around it.
pub fn find_blocks_at(frame: &Frame, region: Rect, min_run: u32, matches: impl Fn(u32, u32) -> bool) -> Vec<Rect> {
    let (rx, ry, rw, rh) = region.to_pixels(frame.width(), frame.height());
    let mut open: Vec<Block> = Vec::new();
    let mut done: Vec<Block> = Vec::new();

    for y in ry..ry + rh {
        let runs = row_runs(y, rx, rx + rw, min_run, &matches);
        let (continuing, ended): (Vec<Block>, Vec<Block>) =
            open.drain(..).partition(|block| runs.iter().any(|&(left, right)| block.overlaps(left, right)));
        done.extend(ended);
        open = continuing;

        for (left, right) in runs {
            match open.iter_mut().find(|block| block.overlaps(left, right) && block.bottom < y) {
                Some(block) => block.extend(left, right, y),
                None => open.push(Block { left, right, top: y, bottom: y }),
            }
        }
    }
    done.extend(open);
    done.into_iter().map(Block::rect).collect()
}

fn row_runs(y: u32, from: u32, to: u32, min_run: u32, matches: &impl Fn(u32, u32) -> bool) -> Vec<(u32, u32)> {
    let mut runs = Vec::new();
    let mut start = None;
    for x in from..=to {
        let hit = x < to && matches(x, y);
        match (hit, start) {
            (true, None) => start = Some(x),
            (false, Some(s)) => {
                if x - s >= min_run {
                    runs.push((s, x));
                }
                start = None;
            }
            _ => {}
        }
    }
    runs
}

struct Block {
    left: u32,
    right: u32,
    top: u32,
    bottom: u32,
}

impl Block {
    fn overlaps(&self, left: u32, right: u32) -> bool {
        left < self.right && right > self.left
    }

    fn extend(&mut self, left: u32, right: u32, y: u32) {
        self.left = self.left.min(left);
        self.right = self.right.max(right);
        self.bottom = y;
    }

    fn rect(self) -> Rect {
        Rect::from_edges(self.left as f32, self.top as f32, self.right as f32, (self.bottom + 1) as f32)
    }
}

/// Brightness and the two colour-difference channels (BT.601, full range): `(y, cb, cr)`.
fn ycbcr([r, g, b]: [f32; 3]) -> (f32, f32, f32) {
    (
        0.299 * r + 0.587 * g + 0.114 * b,
        -0.168_736 * r - 0.331_264 * g + 0.5 * b,
        0.5 * r - 0.418_688 * g - 0.081_312 * b,
    )
}

/// Whether `px` is `target` as a browser's window share delivers it. Browsers send a shared window
/// as video (Edge: I420, BT.601), which keeps brightness for every pixel but colour only for each
/// 2 x 2 block, and smooths it into the next ones: beside white lettering, the WORLD button's
/// (221, 153, 17) arrives as (205, 155, 52). So brightness must match within `brightness`, while the
/// colour may be weakened down to 45% (or strengthened to 125%) of the target's, but not turned.
/// Measured on Edge 157 over 237 frames: 99% of the button's pixels pass, 60% pass a plain match.
pub fn like_through_video(px: &Rgba<u8>, target: [u8; 3], brightness: f32) -> bool {
    let [r, g, b, _] = px.0;
    let (y, cb, cr) = ycbcr([r, g, b].map(f32::from));
    let (ty, tcb, tcr) = ycbcr(target.map(f32::from));
    if (y - ty).abs() > brightness {
        return false;
    }
    let strength = tcb * tcb + tcr * tcr;
    if strength < 1.0 {
        // A grey: some colour bleeds in from its neighbours.
        return cb.hypot(cr) <= 12.0;
    }
    let along = (cb * tcb + cr * tcr) / strength;
    let across = (cb - along * tcb).hypot(cr - along * tcr);
    (0.45..=1.25).contains(&along) && across <= 0.35 * strength.sqrt() + 6.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_colour_weakened_by_video_still_matches_but_another_does_not() {
        let orange = [221, 153, 17];
        for seen in [[221, 153, 17], [205, 155, 52], [211, 155, 39], [208, 154, 47]] {
            assert!(like_through_video(&Rgba([seen[0], seen[1], seen[2], 255]), orange, 12.0), "{seen:?}");
        }
        // Too bright (a letter's edge), grey, and a red of the same brightness.
        for other in [[255, 220, 141], [158, 158, 158], [255, 120, 120]] {
            assert!(!like_through_video(&Rgba([other[0], other[1], other[2], 255]), orange, 12.0), "{other:?}");
        }
    }

    #[test]
    fn finds_a_red_block() {
        let frame = Frame::from_fn(40, 20, |x, y| {
            if (5..30).contains(&x) && (8..12).contains(&y) { Rgba([250, 10, 10, 255]) } else { Rgba([0, 0, 0, 255]) }
        });
        let blocks = find_blocks(&frame, Rect::new(0.0, 0.0, 40.0, 20.0), 10, |p| p.0[0] > 200);
        assert_eq!(blocks, vec![Rect::new(5.0, 8.0, 25.0, 4.0)]);
    }
}
