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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_a_red_block() {
        let frame = Frame::from_fn(40, 20, |x, y| {
            if (5..30).contains(&x) && (8..12).contains(&y) { Rgba([250, 10, 10, 255]) } else { Rgba([0, 0, 0, 255]) }
        });
        let blocks = find_blocks(&frame, Rect::new(0.0, 0.0, 40.0, 20.0), 10, |p| p.0[0] > 200);
        assert_eq!(blocks, vec![Rect::new(5.0, 8.0, 25.0, 4.0)]);
    }
}
