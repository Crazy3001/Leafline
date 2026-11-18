//! Map data: the cache, routes, drops and lookups. The logic is shared with the website in
//! `leafline_core::service::mapdata`; this adds the program's own: its downloaders, the daily
//! check in the background, bringing things up in the overlay, and opening links.

#[allow(unused_imports)]
use tauri::AppHandle;

#[allow(unused_imports)]
use super::host::AppHost;
pub use leafline_core::service::mapdata::*;

use std::time::Duration;

use serde::Serialize;
use tauri::{Emitter, Manager, Url};
use tauri_plugin_opener::OpenerExt;

use super::{overlay, pins};
use crate::error::{Error, Result};
use crate::mapdata::fetch::Http;

/// Opens the cache in the app's data folder, with OSMS asked at most four times a second and the
/// wiki once a second. Nothing is downloaded here.
pub fn load(app: &AppHandle) -> Result<MapDataState> {
    let path = app.path().app_data_dir()?.join(FILE_NAME);
    Ok(MapDataState::load(
        path,
        Box::new(Http::new(Duration::from_millis(250))),
        Box::new(Http::new(Duration::from_secs(1))),
    ))
}

/// Once the user has downloaded map data, checks OSMS for changes in the background, at most
/// once a day, and renews the turn-in table and drop list once they are a day old. An update of
/// Leafline that replaced the cache gets back what the user had the same way.
pub fn check_for_updates(app: &AppHandle) {
    if !wants_updates(app.state::<MapDataState>().inner()) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        if let Some(status) = update_in_background(app.state::<MapDataState>().inner()) {
            let _ = app.emit(MAP_DATA_EVENT, &status);
        }
    });
}

/// Opens the player's copy of the travel connections in their editor, making it first from the
/// downloaded connections, or empty if there are none.
#[tauri::command]
pub fn edit_travel_file(app: AppHandle) -> Result<()> {
    let Some(path) = own_travel_file(&AppHost::new(&app))? else { return Ok(()) };
    app.opener().open_path(path.display().to_string(), None::<&str>).map_err(|err| Error::Invalid(err.to_string()))
}

/// Brings a quest up in the Quests tab, from whichever window asked.
#[tauri::command]
pub fn show_quest(app: AppHandle, id: u32) -> Result<()> {
    if !pins::shows_quests(&app) {
        overlay::show_full(&app)?;
    }
    Ok(app.emit(SHOW_QUEST_EVENT, id)?)
}

/// What to bring up in the Lookup tab: one monster, or a search when no monster was matched.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct MonsterLink {
    id: Option<u32>,
    text: Option<String>,
}

/// Brings a monster up in the Lookup tab, from whichever window asked. A pinned Lookup panel
/// shows it by itself; otherwise the overlay has to.
#[tauri::command]
pub fn show_monster(app: AppHandle, id: Option<u32>, text: Option<String>) -> Result<()> {
    if !pins::shows_item(&app) {
        overlay::show_full(&app)?;
    }
    Ok(app.emit(SHOW_MONSTER_EVENT, MonsterLink { id, text })?)
}

/// What to bring up in the Lookup tab: a map or an NPC.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlaceLink {
    mode: String,
    id: u32,
}

/// Brings a map (`maps`) or an NPC (`npcs`) up in the Lookup tab, from whichever window asked.
/// A pinned Lookup panel shows it by itself; otherwise the overlay has to.
#[tauri::command]
pub fn show_place(app: AppHandle, mode: String, id: u32) -> Result<()> {
    if !pins::shows_item(&app) {
        overlay::show_full(&app)?;
    }
    Ok(app.emit(SHOW_PLACE_EVENT, PlaceLink { mode, id })?)
}

/// Sites Leafline may open in the browser: the link-outs and the credits.
const LINK_HOSTS: [&str; 9] = [
    "mapleclassic.wiki",
    "meowdb.com",
    "meso.watch",
    "henesys.gg",
    "osmsdataexplorer.com",
    "creativecommons.org",
    "lucide.dev",
    "tauri.app",
    "svelte.dev",
];

/// Opens one of the card's links in the default browser. Only the sites it links to are allowed.
#[tauri::command]
pub fn open_link(app: AppHandle, url: String) -> Result<()> {
    let parsed = Url::parse(&url).map_err(|err| Error::Invalid(format!("{url}: {err}")))?;
    if !link_allowed(&parsed) {
        return Err(Error::Invalid(format!("Leafline does not open links to {url}")));
    }
    app.opener().open_url(parsed.as_str(), None::<&str>).map_err(|err| Error::Invalid(err.to_string()))
}

/// A site from [`LINK_HOSTS`], or Leafline's own pages on GitHub: its releases and the forms
/// for reporting a problem or suggesting an idea. Nowhere else on GitHub.
fn link_allowed(url: &Url) -> bool {
    if url.scheme() != "https" {
        return false;
    }
    match url.host_str() {
        Some("github.com") => {
            let own = format!("/{}/", super::update::RELEASES_REPO).to_ascii_lowercase();
            url.path().to_ascii_lowercase().starts_with(&own)
        }
        Some(host) => LINK_HOSTS.contains(&host),
        None => false,
    }
}

#[cfg(test)]
mod link_tests {
    use super::*;

    #[test]
    fn only_leaflines_own_github_pages_open() {
        let allowed = |url: &str| link_allowed(&Url::parse(url).unwrap());
        assert!(allowed("https://github.com/Crazy3001/Leafline/releases/tag/v0.1.1"));
        assert!(allowed("https://github.com/Crazy3001/Leafline/issues/new?template=bug_report.yml"));
        assert!(allowed("https://mapleclassic.wiki/index.php?search=Snail"));
        assert!(!allowed("https://github.com/someone/else"));
        assert!(!allowed("https://github.com/Crazy3001/LeaflineFake/issues"));
        assert!(!allowed("http://github.com/Crazy3001/Leafline/issues"));
        assert!(!allowed("https://example.com/"));
    }
}
