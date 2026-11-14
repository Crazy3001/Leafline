//! Link cards popped out of a window into one of their own, so they can sit anywhere on the
//! screen, outside the overlay, beside the game. Each is a small always-on-top window like a
//! pinned panel, showing one card; closing it is the window's own close button.

use std::sync::atomic::{AtomicU32, Ordering};

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use super::overlay;
use super::pins;
use super::settings::SettingsState;
use crate::error::{Error, Result};

/// A card window's size, in logical pixels, before the size setting.
const WIDTH: f64 = 380.0;
const HEIGHT: f64 = 540.0;

static NEXT: AtomicU32 = AtomicU32::new(1);

/// Whether a page path is one this command opens: `card/<kind>/<id or n-name>`, in the
/// characters a link's path can carry. Nothing else can be loaded through it.
fn card_path(path: &str) -> bool {
    let mut parts = path.split('/');
    parts.next() == Some("card")
        && parts.next().is_some_and(|kind| ["item", "mob", "npc", "map", "quest", "skill"].contains(&kind))
        && parts.next().is_some_and(|key| !key.is_empty())
        && parts.next().is_none()
        && path.chars().all(|c| c.is_ascii_alphanumeric() || "/-_.~%".contains(c))
}

/// Opens a card in a window of its own at screen position `x`, `y` (logical pixels).
#[tauri::command]
pub fn pop_out_card(app: AppHandle, path: String, x: f64, y: f64) -> Result<()> {
    if !card_path(&path) {
        return Err(Error::Invalid(format!("not a card: {path}")));
    }
    let settings = app.state::<SettingsState>().get();
    // Named like a pinned panel, so it hides, shows, sizes and lets clicks through with them.
    let label = format!("{}card-{}", pins::PREFIX, NEXT.fetch_add(1, Ordering::Relaxed));
    let window = WebviewWindowBuilder::new(&app, label, WebviewUrl::App(path.into()))
        .title("Leafline · Card")
        .inner_size(WIDTH * settings.scale, HEIGHT * settings.scale)
        .position(x, y)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .always_on_top(settings.always_on_top)
        .skip_taskbar(true)
        .maximizable(false)
        .focused(false)
        .focusable(!settings.keep_game_focus)
        .visible(!pins::hidden(&app))
        .additional_browser_args(super::BROWSER_ARGS)
        .build()?;
    window.set_zoom(settings.scale)?;
    if overlay::status(&app)?.click_through {
        window.set_ignore_cursor_events(true)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::card_path;

    #[test]
    fn opens_only_card_pages() {
        assert!(card_path("card/item/2000001"));
        assert!(card_path("card/mob/n-Red%20Snail"));
        assert!(!card_path("card/settings/1"), "not a card kind");
        assert!(!card_path("settings"));
        assert!(!card_path("card/item/1/extra"));
        assert!(!card_path("card/item/<script>"));
        assert!(!card_path("card/item/"));
    }
}
