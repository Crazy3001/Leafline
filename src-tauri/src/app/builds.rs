//! The build planner. The logic is shared with the website in `leafline_core::service::builds`;
//! this adds bringing an item up in the overlay and opening links and the planner's window.

#[allow(unused_imports)]
use tauri::AppHandle;

#[allow(unused_imports)]
use super::host::AppHost;
pub use leafline_core::service::builds::*;

use serde::Serialize;
use tauri::{Emitter, Url};
use tauri_plugin_opener::OpenerExt;

use super::{overlay, pins};
use crate::error::{Error, Result};

/// What to bring up in the Item tab: one item, or a search when no item was matched.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ItemLink {
    id: Option<u32>,
    text: Option<String>,
}

/// Brings an item up in the Item tab, from whichever window asked. A pinned Item panel shows it
/// by itself; otherwise the overlay has to.
#[tauri::command]
pub fn show_item(app: AppHandle, id: Option<u32>, text: Option<String>) -> Result<()> {
    if !pins::shows_item(&app) {
        overlay::show_full(&app)?;
    }
    Ok(app.emit(SHOW_ITEM_EVENT, ItemLink { id, text })?)
}

/// Opens the guide link the player typed for a build. It is theirs, so any web address will do.
#[tauri::command]
pub fn open_guide(app: AppHandle, url: String) -> Result<()> {
    let parsed = Url::parse(url.trim()).map_err(|err| Error::Invalid(format!("That isn't a web address: {err}")))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(Error::Invalid("Guide links must be web addresses.".into()));
    }
    app.opener().open_url(parsed.as_str(), None::<&str>).map_err(|err| Error::Invalid(err.to_string()))
}

/// Opens the build planner: a tab of the desk view.
#[tauri::command]
pub async fn show_build(app: AppHandle) -> Result<()> {
    super::desk::open_tab(&app, "planner")
}
