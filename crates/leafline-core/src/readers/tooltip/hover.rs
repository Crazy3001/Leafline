//! When a tooltip counts as hovered on purpose, for the readers that read item tooltips from live
//! pictures without a hotkey (the website's and the program's live loops). A tooltip is read once
//! it has stood in one place for [`HOLD`] seconds: the mouse resting on an item, not passing over
//! it on the way to another. Its place is its name's bullet ([`super::name_lines`]).

use super::ItemTooltip;
use crate::vision::Rect;

/// How long a tooltip must stay in one place before it is read, in seconds: long enough to pass
/// over items on the way to another, short enough to feel prompt.
pub const HOLD: f64 = 0.6;
/// How far a tooltip may move, in pixels, and still be the same one held.
pub const HELD_WITHIN: f32 = 3.0;

/// Whether two looks found tooltips in the same places, give or take a few pixels.
pub fn same_places(a: &[Rect], b: &[Rect]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| (a.x - b.x).abs() <= HELD_WITHIN && (a.y - b.y).abs() <= HELD_WITHIN)
}

/// Where the tooltips stood at the last looks, since when (in seconds), and whether they have
/// been read: each resting place is read once.
#[derive(Debug, Default)]
pub struct Rest {
    at: Option<(Vec<Rect>, f64)>,
    read: bool,
}

impl Rest {
    /// A new picture showed tooltips at `places` (none: no tooltip), at `at` seconds.
    pub fn seen(&mut self, places: Vec<Rect>, at: f64) {
        if places.is_empty() {
            *self = Self::default();
        } else if !self.at.as_ref().is_some_and(|(held, _)| same_places(held, &places)) {
            *self = Self { at: Some((places, at)), read: false };
        }
    }

    /// Whether the tooltips are to be read at `at` seconds: once, when they have stood still for
    /// [`HOLD`]. Asked with no new picture too, when the scene behind them is still.
    pub fn due(&mut self, at: f64) -> bool {
        let due = !self.read && self.at.as_ref().is_some_and(|(_, since)| at - since >= HOLD);
        self.read |= due;
        due
    }

    /// The read couldn't start (another was under way): try again at the next look.
    pub fn again(&mut self) {
        self.read = false;
    }
}

/// Whether two sets of item tooltips would show the same cards.
pub fn same_cards(a: &[ItemTooltip], b: &[ItemTooltip]) -> bool {
    let key = |items: &[ItemTooltip]| {
        items.iter().map(|t| (t.name.clone(), t.upgrades, t.upgraded, t.stats.clone(), t.remaining_enhancements)).collect::<Vec<_>>()
    };
    key(a) == key(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tooltip_a_pixel_or_two_off_is_held_still() {
        let at = |x: f32, y: f32| Rect { x, y, w: 5.0, h: 5.0 };
        assert!(same_places(&[at(100.0, 50.0)], &[at(102.0, 47.0)]));
        assert!(!same_places(&[at(100.0, 50.0)], &[at(104.0, 50.0)]));
        assert!(!same_places(&[at(100.0, 50.0)], &[at(100.0, 50.0), at(300.0, 50.0)]), "a compared item came up");
    }

    fn place(x: f32) -> Vec<Rect> {
        vec![Rect { x, y: 200.0, w: 5.0, h: 5.0 }]
    }

    #[test]
    fn a_tooltip_is_read_once_it_has_held_still() {
        let mut rest = Rest::default();
        rest.seen(place(100.0), 0.0);
        assert!(!rest.due(0.0));
        rest.seen(place(101.0), 0.25);
        assert!(!rest.due(0.25), "not yet held long enough");
        rest.seen(place(100.0), 0.75);
        assert!(rest.due(0.75));
        assert!(!rest.due(1.0), "read once");
        rest.seen(place(100.0), 1.25);
        assert!(!rest.due(1.25), "still the same hover");
    }

    #[test]
    fn a_tooltip_passed_over_is_not_read() {
        let mut rest = Rest::default();
        rest.seen(place(100.0), 0.0);
        rest.seen(place(160.0), 0.5);
        assert!(!rest.due(0.75), "moved on to another item");
        assert!(rest.due(1.1), "held on the second");
        rest.seen(Vec::new(), 1.35);
        rest.seen(place(160.0), 1.6);
        assert!(!rest.due(1.85), "hovered again: a fresh hold");
        assert!(rest.due(2.2));
    }

    #[test]
    fn a_still_picture_keeps_the_hold_going() {
        let mut rest = Rest::default();
        rest.seen(place(100.0), 0.0);
        // No new picture comes (the game drew nothing new), so only time passes.
        assert!(rest.due(0.7));
    }

    #[test]
    fn a_hover_that_couldnt_be_read_is_tried_again() {
        let mut rest = Rest::default();
        rest.seen(place(100.0), 0.0);
        assert!(rest.due(0.7));
        rest.again();
        assert!(rest.due(0.95));
    }
}
