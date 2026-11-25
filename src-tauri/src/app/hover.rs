//! Item cards from a held hover, as the website reads them (`leafline-capture`'s
//! `LiveReader::hover`): the live loop shows each picture it takes here, a tooltip that stays in
//! one place for `tooltip::hover::HOLD` seconds is read once, by the same reader as the Read item
//! hotkey, and its card stays until another is read. A tooltip passed over on the way to another
//! item isn't read, and one that names nothing leaves the last card as it was. Tooltips are found
//! by the picture alone (`tooltip::name_lines`), so looking costs no OCR; the read itself runs
//! beside the live loop. The player's switch is `live.itemTooltips`.

use std::sync::Arc;
use std::time::Instant;

use tauri::AppHandle;

use super::live;
use crate::readers::tooltip::{self, hover::Rest};
use crate::vision::{Frame, Rect};

/// Follows the tooltips across live pictures.
pub struct ItemHover {
    rest: Rest,
    /// The latest picture with the tooltips held, read once they have stood still long enough.
    frame: Option<Arc<Frame>>,
    /// Where [`Rest`]'s seconds count from.
    origin: Instant,
}

impl Default for ItemHover {
    fn default() -> Self {
        Self { rest: Rest::default(), frame: None, origin: Instant::now() }
    }
}

impl ItemHover {
    /// One look at the game: `frame` is a new picture, or `None` when the game hasn't drawn a new
    /// one since the last (a tooltip held over a still scene).
    pub fn look(&mut self, app: &AppHandle, frame: Option<&Arc<Frame>>) {
        let at = self.origin.elapsed().as_secs_f64();
        if let Some(frame) = frame {
            let places: Vec<Rect> = tooltip::name_lines(frame).iter().map(|n| n.bullet).collect();
            self.frame = (!places.is_empty()).then(|| frame.clone());
            self.rest.seen(places, at);
        }
        if self.rest.due(at)
            && let Some(frame) = self.frame.clone()
            && !live::read_hovered(app, frame)
        {
            // An item check is already under way: this hover is read at the next look.
            self.rest.again();
        }
    }

    /// Lets go of the last picture, with the switch off.
    pub fn forget(&mut self) {
        self.rest = Rest::default();
        self.frame = None;
    }
}
