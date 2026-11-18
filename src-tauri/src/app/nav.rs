//! "Go here": the one place the player is heading for, shared by every window and kept between
//! runs. The state is shared with the website in `leafline_core::service::nav`.

#[allow(unused_imports)]
use tauri::AppHandle;

#[allow(unused_imports)]
use super::host::AppHost;
pub use leafline_core::service::nav::*;

use tauri::{Emitter, State};

use crate::error::Result;

fn told(app: &AppHandle, (nav, changed): (Nav, bool)) -> Result<Nav> {
    if changed {
        app.emit(NAV_EVENT, &nav)?;
    }
    Ok(nav)
}

#[tauri::command]
pub fn destination(state: State<NavState>) -> Nav {
    state.get()
}

/// Heads for a place, or with none, cancels.
#[tauri::command]
pub fn set_destination(app: AppHandle, state: State<NavState>, destination: Option<Destination>) -> Result<Nav> {
    told(&app, state.set_destination(AppHost::files_of(&app), destination)?)
}

/// The overlay reached the place: clears it if it is still the one set.
#[tauri::command]
pub fn arrive_at(app: AppHandle, state: State<NavState>, destination: Destination) -> Result<Nav> {
    told(&app, state.arrive_at(AppHost::files_of(&app), destination)?)
}

#[tauri::command]
pub fn forget_destination(app: AppHandle, state: State<NavState>, destination: Destination) -> Result<Nav> {
    told(&app, state.forget_destination(AppHost::files_of(&app), destination)?)
}
