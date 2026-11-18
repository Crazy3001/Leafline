//! Connected blobs of pixels, for finding UI elements by shape.

/// One connected blob's bounding box and how many pixels it holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Blob {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub area: u32,
}

/// A yes-or-no picture.
#[derive(Debug, Clone)]
pub struct Bitmap {
    width: u32,
    height: u32,
    bits: Vec<bool>,
}

impl Bitmap {
    pub fn from_fn(width: u32, height: u32, on: impl Fn(u32, u32) -> bool) -> Self {
        let bits = (0..height).flat_map(|y| (0..width).map(move |x| (x, y))).map(|(x, y)| on(x, y)).collect();
        Self { width, height, bits }
    }

    pub fn get(&self, x: u32, y: u32) -> bool {
        x < self.width && y < self.height && self.bits[(y * self.width + x) as usize]
    }

    /// Fills gaps a pixel or two wide, such as those letters leave in a button.
    pub fn closed(&self) -> Self {
        let any_around = |x: u32, y: u32, test: &dyn Fn(i64, i64) -> bool| {
            (-1..=1).any(|dy| (-1..=1).any(|dx| test(i64::from(x) + dx, i64::from(y) + dy)))
        };
        let on = |map: &Self, x: i64, y: i64| {
            x >= 0 && y >= 0 && x < i64::from(map.width) && y < i64::from(map.height) && map.get(x as u32, y as u32)
        };
        let grown = Self::from_fn(self.width, self.height, |x, y| any_around(x, y, &|nx, ny| on(self, nx, ny)));
        Self::from_fn(self.width, self.height, |x, y| !any_around(x, y, &|nx, ny| !on(&grown, nx, ny)))
    }

    /// The connected blobs, joined by edges and corners.
    pub fn blobs(&self) -> Vec<Blob> {
        struct Run {
            left: u32,
            right: u32,
            y: u32,
        }
        fn find(parents: &mut [usize], mut i: usize) -> usize {
            while parents[i] != i {
                parents[i] = parents[parents[i]];
                i = parents[i];
            }
            i
        }

        let mut runs: Vec<Run> = Vec::new();
        let mut parents: Vec<usize> = Vec::new();
        let mut previous = 0..0;
        for y in 0..self.height {
            let start = runs.len();
            let mut x = 0;
            while x < self.width {
                if !self.get(x, y) {
                    x += 1;
                    continue;
                }
                let left = x;
                while x < self.width && self.get(x, y) {
                    x += 1;
                }
                let id = runs.len();
                runs.push(Run { left, right: x, y });
                parents.push(id);
                for above in previous.clone() {
                    if runs[above].left <= x && left <= runs[above].right {
                        let (a, b) = (find(&mut parents, above), find(&mut parents, id));
                        parents[a] = b;
                    }
                }
            }
            previous = start..runs.len();
        }

        let mut blobs: std::collections::HashMap<usize, (u32, u32, u32, u32, u32)> = std::collections::HashMap::new();
        for (i, run) in runs.iter().enumerate() {
            let root = find(&mut parents, i);
            let blob = blobs.entry(root).or_insert((run.left, run.y, run.right, run.y + 1, 0));
            blob.0 = blob.0.min(run.left);
            blob.1 = blob.1.min(run.y);
            blob.2 = blob.2.max(run.right);
            blob.3 = blob.3.max(run.y + 1);
            blob.4 += run.right - run.left;
        }
        blobs.into_values().map(|(x0, y0, x1, y1, area)| Blob { x: x0, y: y0, w: x1 - x0, h: y1 - y0, area }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bitmap(rows: &[&str]) -> Bitmap {
        Bitmap::from_fn(rows[0].len() as u32, rows.len() as u32, |x, y| rows[y as usize].as_bytes()[x as usize] == b'#')
    }

    #[test]
    fn finds_blobs_joined_at_corners() {
        let mut blobs = bitmap(&["##...#", ".#...#", "..#...", "......", "###..."]).blobs();
        blobs.sort_by_key(|b| (b.y, b.x));
        assert_eq!(
            blobs,
            vec![
                Blob { x: 0, y: 0, w: 3, h: 3, area: 4 },
                Blob { x: 5, y: 0, w: 1, h: 2, area: 2 },
                Blob { x: 0, y: 4, w: 3, h: 1, area: 3 },
            ]
        );
    }

    #[test]
    fn a_u_shape_is_one_blob() {
        assert_eq!(bitmap(&["#.#", "#.#", "###"]).blobs().len(), 1);
    }

    #[test]
    fn closing_fills_a_small_gap() {
        let closed = bitmap(&[".......", ".##.##.", ".##.##.", "......."]).closed();
        assert!(closed.get(3, 1));
        assert_eq!(closed.blobs().len(), 1);
        assert!(!closed.get(0, 0));
    }
}
