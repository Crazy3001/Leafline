//! Sell or keep, on a hotkey: one picture of the game, the Item Inventory's open tab read, each
//! item named by its picture and given what says to keep it (see `service::bag`, which the
//! website shares). The window marks the rest to sell and adds up what the bag is worth at NPC
//! prices.
//!
//! Pictures come from OSMS through the picture cache, downloaded the first time and kept on this
//! PC.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter, Manager, State};

use super::host::AppHost;
use super::pictures::PictureState;
use super::readings;
use super::settings::SettingsState;
use super::{live, overlay, report};
use crate::error::{Error, Result};
use crate::readers::inventory;
pub use leafline_core::service::bag::{BAG_EVENT, BagCheck, BagRead as Read};
use leafline_core::service::bag::{self as shared, Pictures};

#[derive(Default)]
pub struct BagState {
    busy: AtomicBool,
    /// The last tab read, kept in memory only, for "Look further".
    last: Mutex<Option<Read>>,
    result: Mutex<Option<BagCheck>>,
}

/// The hotkey: reads the open tab from one picture of the game. The Settings switch for live
/// reading covers this too.
pub fn check(app: &AppHandle) -> Result<()> {
    if !app.state::<SettingsState>().get().live.enabled {
        return super::settings::open(app);
    }
    start(app, None)
}

/// "Look further": tries every item of the tab last read.
#[tauri::command]
pub fn bag_look_further(app: AppHandle) -> Result<()> {
    let last = app.state::<BagState>().last.lock().expect("bag lock").clone();
    let Some(read) = last else {
        return Err(Error::Invalid("Check your bag first.".into()));
    };
    start(&app, Some(read))
}

/// The last check, for a window that opens after it.
#[tauri::command]
pub fn last_bag(state: State<BagState>) -> Option<BagCheck> {
    state.result.lock().expect("bag lock").clone()
}

fn start(app: &AppHandle, again: Option<Read>) -> Result<()> {
    let state = app.state::<BagState>();
    if state.busy.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let busy = BagCheck { busy: true, ..state.result.lock().expect("bag lock").clone().unwrap_or_default() };
    let _ = app.emit(BAG_EVENT, &busy);
    let handle = app.clone();
    std::thread::Builder::new().name("bag-check".into()).spawn(move || {
        let wide = again.is_some();
        let result = (|| {
            let read = match again {
                Some(read) => read,
                None => read_bag(&handle)?,
            };
            *handle.state::<BagState>().last.lock().expect("bag lock") = Some(read.clone());
            name_items(&handle, &read, wide)
        })();
        let check = result.unwrap_or_else(|err| {
            report::note(format!("Couldn't check the bag: {err}"));
            BagCheck::failed(err.to_string())
        });
        let state = handle.state::<BagState>();
        *state.result.lock().expect("bag lock") = Some(check.clone());
        state.busy.store(false, Ordering::SeqCst);
        if let Err(err) = overlay::show_full(&handle) {
            report::note(err.to_string());
        }
        let _ = handle.emit(BAG_EVENT, &check);
    })?;
    Ok(())
}

/// Loads the pictures, downloading the missing ones a few at a time (the cache keeps to its own
/// limit). One that can't be had is left out.
fn load_pictures(pictures: &PictureState, wanted: Vec<(u32, String)>) -> Vec<(u32, image::RgbaImage)> {
    const WORKERS: usize = crate::mapdata::pictures::MAX_DOWNLOADS;
    let chunk = wanted.len().div_ceil(WORKERS).max(1);
    std::thread::scope(|scope| {
        let jobs: Vec<_> = wanted
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || part.iter().filter_map(|(id, url)| Some((*id, pictures.image(url).ok()?))).collect::<Vec<_>>())
            })
            .collect();
        jobs.into_iter().flat_map(|job| job.join().unwrap_or_default()).collect()
    })
}

fn read_bag(app: &AppHandle) -> Result<Read> {
    let frame = live::one_picture(app)?;
    let inventory = inventory::read(&frame)
        .ok_or_else(|| Error::Invalid("No Item Inventory in view. Open it on the tab to check, then press the hotkey.".into()))?;
    let analysis = readings::current_analysis(app);
    let hud = analysis.as_ref().and_then(|a| a.hud.as_ref());
    let session = readings::current_session(app);
    Ok(Read {
        inventory,
        character: hud.and_then(|h| h.name.clone()).or_else(|| session.as_ref().and_then(|s| s.name.clone())),
        level: hud.and_then(|h| h.level).or_else(|| session.as_ref().and_then(|s| s.level)),
        video: false,
    })
}

fn name_items(app: &AppHandle, read: &Read, wide: bool) -> Result<BagCheck> {
    shared::name_items(&AppHost::new(app), &Cache(&app.state::<PictureState>()), read, wide)
}

/// The picture cache, as the shared check takes it.
struct Cache<'a>(&'a PictureState);

impl Pictures for Cache<'_> {
    fn has(&self, url: &str) -> bool {
        self.0.has(url)
    }

    fn load(&self, wanted: Vec<(u32, String)>) -> Vec<(u32, image::RgbaImage)> {
        load_pictures(self.0, wanted)
    }
}
