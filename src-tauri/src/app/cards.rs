//! Link cards in windows of their own, so they can sit anywhere on the screen, outside the
//! overlay, beside the game. A coloured name opens its card in the one live card window, moved
//! beside the link. Pinning a card keeps that window where it is, as a pinned panel stays, and
//! the next link opens a new live one. Each is a small window like a pinned panel.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use tauri::{AppHandle, Emitter, LogicalPosition, Manager, WebviewUrl, WebviewWindowBuilder, Window};

use super::overlay;
use super::pins;
use super::settings::SettingsState;
use crate::error::{Error, Result};

/// A card window's size, in logical pixels, before the size setting.
const WIDTH: f64 = 380.0;
const HEIGHT: f64 = 540.0;

/// Which card window links open their cards in, by label, until it is pinned.
#[derive(Default)]
pub struct CardState {
    live: Mutex<Option<String>>,
}

impl CardState {
    fn live(&self) -> Option<String> {
        self.live.lock().expect("cards lock").clone()
    }
}

/// Tells the live card window which card to show now, with its page path.
const SHOW_EVENT: &str = "card-show";

static NEXT: AtomicU32 = AtomicU32::new(1);

/// A new card window's label, named like a pinned panel so it hides, shows, sizes and lets
/// clicks through with them.
fn next_label() -> String {
    format!("{}card-{}", pins::PREFIX, NEXT.fetch_add(1, Ordering::Relaxed))
}

/// Whether a page path is one these commands open: `card/<kind>/<id or n-name>`, in the
/// characters a link's path can carry. Nothing else can be loaded through them.
fn card_path(path: &str) -> bool {
    let mut parts = path.split('/');
    parts.next() == Some("card")
        && parts.next().is_some_and(|kind| ["item", "mob", "npc", "map", "quest", "skill"].contains(&kind))
        && parts.next().is_some_and(|key| !key.is_empty())
        && parts.next().is_none()
        && path.chars().all(|c| c.is_ascii_alphanumeric() || "/-_.~%".contains(c))
}

fn open(app: &AppHandle, label: String, path: String, x: f64, y: f64) -> Result<()> {
    let settings = app.state::<SettingsState>().get();
    let window = WebviewWindowBuilder::new(app, label, WebviewUrl::App(path.into()))
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
        .visible(!pins::hidden(app))
        .additional_browser_args(super::BROWSER_ARGS)
        .build()?;
    window.set_zoom(settings.scale)?;
    if overlay::status(app)?.click_through {
        window.set_ignore_cursor_events(true)?;
    }
    Ok(())
}

/// Shows a card in the live card window at screen position `x`, `y` (logical pixels), opening
/// the window if there is none. Async, as every command that makes a window is: a synchronous
/// one would block the event loop the new window needs, and the app would hang.
#[tauri::command]
pub async fn show_card(app: AppHandle, path: String, x: f64, y: f64) -> Result<()> {
    if !card_path(&path) {
        return Err(Error::Invalid(format!("not a card: {path}")));
    }
    let state = app.state::<CardState>();
    match state.live().and_then(|label| app.get_webview_window(&label)) {
        Some(window) => {
            window.set_position(LogicalPosition::new(x, y))?;
            app.emit_to(window.label(), SHOW_EVENT, &path)?;
            if !pins::hidden(&app) {
                window.show()?;
            }
            Ok(())
        }
        None => {
            let label = next_label();
            *state.live.lock().expect("cards lock") = Some(label.clone());
            open(&app, label, path, x, y)
        }
    }
}

/// Whether the asking window is the live card window, which links send their cards to.
#[tauri::command]
pub fn card_is_live(window: Window, state: tauri::State<CardState>) -> bool {
    state.live().as_deref() == Some(window.label())
}

/// Pins the asking card window where it is: links stop using it, and the next one opens a new
/// live window.
#[tauri::command]
pub fn pin_card(window: Window, state: tauri::State<CardState>) {
    let mut live = state.live.lock().expect("cards lock");
    if live.as_deref() == Some(window.label()) {
        *live = None;
    }
}

/// Opens a card in a window that stays, at screen position `x`, `y` (logical pixels): a pinned
/// card. Async for the same reason as [`show_card`].
#[tauri::command]
pub async fn pop_out_card(app: AppHandle, path: String, x: f64, y: f64) -> Result<()> {
    if !card_path(&path) {
        return Err(Error::Invalid(format!("not a card: {path}")));
    }
    open(&app, next_label(), path, x, y)
}

/// Closes the live card window, as a click elsewhere in Leafline closes a card.
#[tauri::command]
pub async fn close_live_card(app: AppHandle) -> Result<()> {
    if let Some(window) = app.state::<CardState>().live().and_then(|label| app.get_webview_window(&label)) {
        window.close()?;
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
