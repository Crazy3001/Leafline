//! Live capture: pictures of the game window, taken the way OBS and Discord take them.
//!
//! On Windows this is Windows Graphics Capture. It asks the desktop compositor for what the
//! window shows, so nothing is ever sent to the game: no input, no window messages, no handle
//! to its process. Windows in front of the game (Leafline's own included) are not in the
//! picture, and the mouse cursor is left out.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows as platform;

#[cfg(not(windows))]
mod unsupported;
#[cfg(not(windows))]
use self::unsupported as platform;

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::vision::Frame;

/// How long a single picture may take to arrive, and how long each try waits.
const SINGLE_PICTURE_WAIT: Duration = Duration::from_secs(2);
const SINGLE_PICTURE_TRY: Duration = Duration::from_millis(300);

/// A top-level window that could be the game.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameWindow {
    /// Only valid while the window exists; pick it again by [`WindowChoice`] after that.
    pub id: isize,
    pub title: String,
    pub class: String,
    /// The size of what would be captured, in physical pixels.
    pub width: u32,
    pub height: u32,
    /// Looks like the MapleStory client by its title or window class.
    pub likely_game: bool,
}

/// How the player's chosen game window is recognised again later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowChoice {
    pub title: String,
    pub class: String,
}

impl GameWindow {
    pub fn choice(&self) -> WindowChoice {
        WindowChoice { title: self.title.clone(), class: self.class.clone() }
    }
}

/// Whether a title and window class look like the game client. Browser tabs and the like
/// that mention MapleStory have a ` - App name` suffix, so those are left out.
pub fn looks_like_game(title: &str, class: &str) -> bool {
    let title = title.trim();
    class.eq_ignore_ascii_case("MapleStoryClass")
        || (title.to_ascii_lowercase().starts_with("maplestory") && !title.contains(" - "))
}

/// Visible windows big enough to be the game, likely ones first. Leafline's own are left out.
pub fn windows() -> Result<Vec<GameWindow>> {
    let mut found = platform::windows()?;
    found.sort_by_key(|w| (!w.likely_game, std::cmp::Reverse(w.width * w.height)));
    Ok(found)
}

/// The window to capture: the one the player chose if it is open, else the only likely one.
pub fn find(choice: Option<&WindowChoice>) -> Result<Option<GameWindow>> {
    let found = windows()?;
    if let Some(choice) = choice {
        // The title may change (some clients add the world or channel), so the class decides
        // when the title alone does not.
        let exact = found.iter().find(|w| w.title == choice.title && w.class == choice.class);
        let same_class = || {
            let mut matches = found.iter().filter(|w| w.class == choice.class && w.likely_game);
            let first = matches.next();
            if matches.next().is_none() { first } else { None }
        };
        if let Some(window) = exact.or_else(same_class) {
            return Ok(Some(window.clone()));
        }
    }
    let mut likely = found.into_iter().filter(|w| w.likely_game);
    let first = likely.next();
    Ok(if likely.next().is_none() { first } else { None })
}

/// What happened when a picture was asked for.
pub enum Grab {
    Frame(Frame),
    /// Nothing has changed since the last picture. Windows only delivers changes.
    Unchanged,
    /// The window is minimised, so it shows nothing.
    Minimized,
    /// No picture has arrived yet. The game may be loading or not drawing.
    Nothing,
    /// The window was closed.
    Closed,
}

/// One picture of `window`, with the capture closed again afterwards.
pub fn picture(window: &GameWindow) -> Result<Frame> {
    let mut capturer = Capturer::new(window)?;
    let deadline = Instant::now() + SINGLE_PICTURE_WAIT;
    loop {
        match capturer.grab(SINGLE_PICTURE_TRY)? {
            Grab::Frame(frame) => return Ok(frame),
            Grab::Minimized => return Err(Error::Invalid("The game is minimised.".into())),
            Grab::Closed => return Err(Error::Invalid("The game window closed.".into())),
            Grab::Unchanged | Grab::Nothing if Instant::now() < deadline => {}
            Grab::Unchanged | Grab::Nothing => {
                return Err(Error::Invalid("The game window hasn't drawn a picture yet.".into()));
            }
        }
    }
}

/// A capture of one window, kept open between pictures.
pub struct Capturer {
    inner: platform::Capturer,
}

impl Capturer {
    pub fn new(window: &GameWindow) -> Result<Self> {
        Ok(Self { inner: platform::Capturer::new(window.id)? })
    }

    /// The window's client area as it looks now, without title bar or borders. Waits up to
    /// `wait` for a fresh picture before settling for the last one.
    pub fn grab(&mut self, wait: Duration) -> Result<Grab> {
        self.inner.grab(wait)
    }

    /// Whether Windows HDR is on for the display the window is on. HDR washes captures out,
    /// so readings are unreliable until it is turned off and the game restarted.
    pub fn hdr(&self) -> bool {
        self.inner.hdr()
    }
}

#[cfg(test)]
mod tests {
    use super::looks_like_game;

    #[test]
    fn recognises_the_client_but_not_browser_tabs() {
        assert!(looks_like_game("MapleStory", "Anything"));
        assert!(looks_like_game("MapleStory Classic World", "Anything"));
        assert!(looks_like_game("Some title", "MapleStoryClass"));
        assert!(!looks_like_game("MapleStory Classic guide - Google Chrome", "Chrome_WidgetWin_1"));
        assert!(!looks_like_game("Leafline", "Tauri Window"));
    }
}
