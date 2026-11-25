//! Sell or keep: the Item Inventory's open tab read, each item named by its picture and given
//! what says to keep it (see `service::bag`, which the website shares). The window marks the rest
//! to sell and adds up what the bag is worth at NPC prices. On a hotkey, from one picture of the
//! game; and while live reading runs, from its pictures once the tab has read the same twice in a
//! row, and again only when what it shows changes ([`BagFollow`], as the website does). The
//! player's switch for the second is `live.bagCheck`.
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
use crate::pipeline::Analysis;
use crate::readers::inventory::{self, TabWatch};
use crate::session::SessionSummary;
use crate::vision::Frame;
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
    start(app, Ask::Hotkey).map(drop)
}

/// "Look further": tries every item of the tab last read.
#[tauri::command]
pub fn bag_look_further(app: AppHandle) -> Result<()> {
    let last = app.state::<BagState>().last.lock().expect("bag lock").clone();
    let Some(read) = last else {
        return Err(Error::Invalid("Check your bag first.".into()));
    };
    start(&app, Ask::Further(read)).map(drop)
}

/// Follows the Item Inventory across live pictures, as the website does: a tab is checked once it
/// reads the same twice in a row (held still, not being scrolled or rearranged), and again only
/// when what it shows changes (`inventory::TabWatch`).
#[derive(Default)]
pub struct BagFollow {
    watch: TabWatch,
}

impl BagFollow {
    pub fn follow(&mut self, app: &AppHandle, frame: &Frame, analysis: &Analysis, session: &SessionSummary) {
        let Some(seen) = self.watch.settled_in(frame, inventory::read) else { return };
        let hud = analysis.hud.as_ref();
        let read = Read {
            inventory: seen.clone(),
            character: hud.and_then(|h| h.name.clone()).or_else(|| session.name.clone()),
            level: hud.and_then(|h| h.level).or(session.level),
            video: false,
        };
        match start(app, Ask::Seen(read)) {
            Ok(true) => self.watch.checked(seen),
            // A check is under way: this tab is tried again at the next reading.
            Ok(false) => {}
            Err(err) => report::note(format!("Couldn't check the bag: {err}")),
        }
    }
}

/// What a check is for.
enum Ask {
    /// The hotkey: a picture of its own, the overlay brought up.
    Hotkey,
    /// "Look further" on the tab last read: every item tried.
    Further(Read),
    /// A tab live reading saw held still: quietly, the result in place.
    Seen(Read),
}

/// The last check, for a window that opens after it.
#[tauri::command]
pub fn last_bag(state: State<BagState>) -> Option<BagCheck> {
    state.result.lock().expect("bag lock").clone()
}

/// Starts a check beside whatever asked for it. `false` when one is already under way.
fn start(app: &AppHandle, ask: Ask) -> Result<bool> {
    let state = app.state::<BagState>();
    if state.busy.swap(true, Ordering::SeqCst) {
        return Ok(false);
    }
    let busy = BagCheck { busy: true, ..state.result.lock().expect("bag lock").clone().unwrap_or_default() };
    let _ = app.emit(BAG_EVENT, &busy);
    let handle = app.clone();
    let wide = matches!(ask, Ask::Further(_));
    let show = !matches!(ask, Ask::Seen(_));
    let spawned = std::thread::Builder::new().name("bag-check".into()).spawn(move || {
        let result = (|| {
            let read = match ask {
                Ask::Further(read) | Ask::Seen(read) => read,
                Ask::Hotkey => read_bag(&handle)?,
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
        if show && let Err(err) = overlay::show_full(&handle) {
            report::note(err.to_string());
        }
        let _ = handle.emit(BAG_EVENT, &check);
    });
    if let Err(err) = spawned {
        state.busy.store(false, Ordering::SeqCst);
        return Err(err.into());
    }
    Ok(true)
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

