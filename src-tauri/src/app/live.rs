//! Live reading: the game window captured every few seconds and run through the pipeline and
//! the session tracker. It stays off until the player turns it on in Settings, and only ever looks at the window; see [`crate::capture`]. Each session saves itself
//! as it goes, so a crash or a closed window costs half a minute of it at most.
//!
//! The game's windows are read from these same pictures while they are open, each with its switch
//! in Settings, as the website reads them: an item tooltip the mouse rests on (see
//! [`super::hover`]), the Character Stat and Skill windows, the Item Inventory's open tab for sell
//! or keep (see [`super::bag::BagFollow`]) and the Quest window. The hotkeys read the same from a
//! picture of their own, whether live reading is running or not.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use super::host::AppHost;
use super::mapdata::MapDataState;
use super::records::{self, HISTORY_EVENT, UserDataState};
use super::builds;
use super::fighter;
use super::report;
use super::settings::{self, SettingsState};
use super::bag::BagFollow;
use super::hover::ItemHover;
use super::questlog::QuestLogFollow;
use super::{overlay, pins, readings, recovery};
use crate::capture::{self, Capturer, GameWindow, Grab};
use crate::sight::{Reader, SIGHT_EVENT, Sight};
use crate::build::catalog::Catalog;
use crate::error::{Error, Result};
use crate::loot::Pickup;
use crate::mapdata::QuestTitles;
use crate::mapdata::loot::name_item;
use crate::pipeline;
use crate::readers::character::{self, Watch};
use crate::readers::charinfo::{self, LookWatch};
use crate::readers::npc;
use crate::readers::pickup::{self, PickupReader};
use crate::readers::tooltip::ItemTooltip;
use crate::readers::tooltip::hover::same_cards;
use crate::session::{Sample, SessionSummary, SessionTracker, TrackerConfig};
use crate::vision::{Frame, Ocr};

/// Emitted to every window when the live status changes.
pub const LIVE_EVENT: &str = "live";
/// Emitted with the path when a live picture is saved.
pub const FRAME_SAVED_EVENT: &str = "frame-saved";
/// Emitted to every window when an item check ends.
pub const ITEM_READ_EVENT: &str = "item-read";
/// Emitted to every window when the character's windows have been read.
pub const CHARACTER_READ_EVENT: &str = "character-read";

/// How long a grab waits for the game to draw a fresh picture.
const FRESH_FRAME_WAIT: Duration = Duration::from_millis(300);
/// How often the Pickup Log and item tooltips are looked at between readings. A new line starts fading at once and
/// six can come in half a second, as the recording of 2026-11-15 showed; four looks a second saw
/// every one of them at nearly full brightness. The one-picture-a-second limit was lifted for
/// this on 2026-11-15. Only the log's corner of each picture is read.
const PICKUP_EVERY: Duration = Duration::from_millis(250);
/// The least time between two pictures of the game, live or for an item check.
const MIN_PICTURE_GAP: Duration = Duration::from_secs(1);
/// How long a save takes the item check's picture over the last live one, so a tooltip that
/// read badly can be kept: live pictures come seconds apart and rarely catch one.
const ITEM_PICTURE_KEPT: Duration = Duration::from_secs(15);
/// How often HDR is checked again, in readings, in case the game moves to another display.
const HDR_CHECK_EVERY: u64 = 30;
/// Active seconds before a session is worth keeping, and how often it is saved from then on. A
/// quick login to sell or chat isn't a training session.
const SAVE_AFTER_ACTIVE: f64 = 300.0;
/// A break this long with no EXP ends the session; the next gain starts a new one. Paused by hand
/// is left alone: the player said they are coming back.
const BREAK_ENDS_SESSION: Duration = Duration::from_secs(30 * 60);
const SAVE_EVERY: Duration = Duration::from_secs(30);
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "state")]
pub enum LiveStatus {
    #[default]
    Off,
    /// Running, but with nothing to read right now.
    Waiting { reason: String },
    Reading {
        window: String,
        width: u32,
        height: u32,
        /// Readings so far this session.
        frames: u64,
        /// How long the last reading took.
        read_ms: f32,
        /// Windows HDR is on for the game's display, which washes the picture out.
        hdr: bool,
        /// The game draws its interface at a scale that isn't a whole number (UI Size Default
        /// Ratio), smoothing its text, which reads less well than Ideal Ratio.
        smoothed_ui: bool,
        /// Tracking is paused by hand.
        held: bool,
    },
    /// Stopped by an error.
    Failed { message: String },
}

/// How an item check went.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ItemRead {
    /// Item tooltips found, the compared item included.
    found: usize,
    /// Why no picture could be read.
    problem: Option<String>,
}

#[derive(Default)]
pub struct LiveState {
    running: AtomicBool,
    stop: AtomicBool,
    /// An item check is under way.
    reading_item: AtomicBool,
    /// The stat and skill windows are being read.
    reading_character: AtomicBool,
    /// The last item check's picture, and when it was taken.
    item_picture: Mutex<Option<(Instant, Arc<Frame>)>>,
    /// When the last picture was taken, for [`MIN_PICTURE_GAP`].
    last_picture: Mutex<Option<Instant>>,
    /// Tracking is paused by hand; pictures are still read.
    held: AtomicBool,
    /// A new session was asked for.
    reset: AtomicBool,
    status: Mutex<LiveStatus>,
    /// The last picture read, kept in memory only, for "Save frame".
    frame: Mutex<Option<Arc<Frame>>>,
    /// What each reader has read since reading started, and what was left out (in memory only).
    sight: Mutex<Sight>,
}

/// Notes that `reader` gave a reading just now, for "What Leafline sees".
pub fn see(app: &AppHandle, reader: Reader) {
    app.state::<LiveState>().sight.lock().expect("live lock").saw(reader, unix_seconds(SystemTime::now()));
}

fn publish_sight(app: &AppHandle) {
    let sight = app.state::<LiveState>().sight.lock().expect("live lock").clone();
    let _ = app.emit(SIGHT_EVENT, &sight);
}

/// What Leafline sees: each reader's last reading, the last picture, the readings left out.
#[tauri::command]
pub fn reader_sight(app: AppHandle) -> Sight {
    app.state::<LiveState>().sight.lock().expect("live lock").clone()
}

pub fn running(app: &AppHandle) -> bool {
    app.state::<LiveState>().running.load(Ordering::SeqCst)
}

/// Starts reading the game window in the background. A new session starts with it.
pub fn start(app: &AppHandle) -> Result<()> {
    if !app.state::<SettingsState>().get().live.enabled {
        return Err(Error::Invalid("Turn on live reading in Settings first.".into()));
    }
    let state = app.state::<LiveState>();
    if state.running.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    state.stop.store(false, Ordering::SeqCst);
    set_status(app, LiveStatus::Waiting { reason: "Starting…".into() });
    let handle = app.clone();
    std::thread::Builder::new().name("live-reading".into()).spawn(move || {
        let status = match read_until_stopped(&handle) {
            Ok(()) => LiveStatus::Off,
            Err(err) => LiveStatus::Failed { message: err.to_string() },
        };
        handle.state::<LiveState>().running.store(false, Ordering::SeqCst);
        set_status(&handle, status);
    })?;
    Ok(())
}

/// Stops after the reading in progress. The session stays on the overlay, ready to save.
pub fn stop(app: &AppHandle) {
    app.state::<LiveState>().stop.store(true, Ordering::SeqCst);
}

/// The hotkey: starts or stops, or opens Settings if live reading hasn't been turned on.
pub fn toggle(app: &AppHandle) -> Result<()> {
    if running(app) {
        stop(app);
        Ok(())
    } else if app.state::<SettingsState>().get().live.enabled {
        start(app)
    } else {
        settings::open(app)
    }
}

pub fn status(app: &AppHandle) -> LiveStatus {
    app.state::<LiveState>().status.lock().expect("live lock").clone()
}

/// The hotkey: reads the hovered item from one picture of the game, whether live reading is
/// running or not. The Settings switch covers this too, so it opens Settings while that is off.
pub fn read_item(app: &AppHandle) -> Result<()> {
    if !app.state::<SettingsState>().get().live.enabled {
        return settings::open(app);
    }
    if app.state::<LiveState>().reading_item.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let handle = app.clone();
    std::thread::Builder::new().name("item-read".into()).spawn(move || {
        let read = read_item_once(&handle);
        handle.state::<LiveState>().reading_item.store(false, Ordering::SeqCst);
        let outcome = match read {
            Ok((width, height, items)) => {
                let found = items.len();
                readings::publish_items(&handle, width, height, items);
                ItemRead { found, problem: None }
            }
            Err(err) => {
                report::note(format!("Couldn't read the item: {err}"));
                ItemRead { found: 0, problem: Some(err.to_string()) }
            }
        };
        // A pinned Item panel shows the result itself; otherwise the overlay has to.
        if !pins::shows_item(&handle)
            && let Err(err) = overlay::show_full(&handle)
        {
            report::note(err.to_string());
        }
        let _ = handle.emit(ITEM_READ_EVENT, &outcome);
    })?;
    Ok(())
}

fn read_item_once(app: &AppHandle) -> Result<(u32, u32, Vec<ItemTooltip>)> {
    let frame = Arc::new(one_picture(app)?);
    *app.state::<LiveState>().item_picture.lock().expect("live lock") = Some((Instant::now(), frame.clone()));
    let items = pipeline::read_items(&frame, &Ocr::new()?)?;
    Ok((frame.width(), frame.height(), items))
}

/// Reads the item tooltips in `frame`, which the mouse has rested on, as the Read item hotkey does
/// but quietly: the card changes in place, and only when the items differ from those on show; a
/// tooltip that names nothing leaves the last card as it was. The overlay isn't brought up. Runs
/// beside the live loop, so the Pickup Log is still looked at meanwhile. `false` when an item
/// check is already under way.
pub(super) fn read_hovered(app: &AppHandle, frame: Arc<Frame>) -> bool {
    let state = app.state::<LiveState>();
    if state.reading_item.swap(true, Ordering::SeqCst) {
        return false;
    }
    let handle = app.clone();
    let spawned = std::thread::Builder::new().name("item-hover".into()).spawn(move || {
        let read = Ocr::new().and_then(|ocr| pipeline::read_items(&frame, &ocr));
        let state = handle.state::<LiveState>();
        state.reading_item.store(false, Ordering::SeqCst);
        match read {
            Ok(items) if !items.is_empty() => {
                // Kept for "Save item picture", as the hotkey's is.
                *state.item_picture.lock().expect("live lock") = Some((Instant::now(), frame.clone()));
                let shown = readings::current_analysis(&handle).map(|a| a.tooltips).unwrap_or_default();
                if same_cards(&shown, &items) {
                    return;
                }
                let found = items.len();
                readings::publish_items(&handle, frame.width(), frame.height(), items);
                let _ = handle.emit(ITEM_READ_EVENT, &ItemRead { found, problem: None });
            }
            Ok(_) => {}
            Err(err) => report::note(format!("Couldn't read the hovered item: {err}")),
        }
    });
    if let Err(err) = spawned {
        state.reading_item.store(false, Ordering::SeqCst);
        report::note(format!("Couldn't read the hovered item: {err}"));
    }
    true
}

/// One picture of the game, for something the player asked to read.
pub(super) fn one_picture(app: &AppHandle) -> Result<Frame> {
    let choice = app.state::<SettingsState>().get().live;
    let window = capture::find(choice.window.as_ref())?
        .ok_or_else(|| Error::Invalid("Leafline can't find the game window. Pick it in Settings.".into()))?;
    wait_for_picture(&app.state::<LiveState>());
    capture::picture(&window)
}

/// The hotkey: reads the stat and skill windows from one picture of the game and moves the
/// build's starting point to match. The Settings switch covers this too.
pub fn read_character(app: &AppHandle) -> Result<()> {
    if !app.state::<SettingsState>().get().live.enabled {
        return settings::open(app);
    }
    if app.state::<LiveState>().reading_character.swap(true, Ordering::SeqCst) {
        return Ok(());
    }
    let handle = app.clone();
    std::thread::Builder::new().name("character-read".into()).spawn(move || {
        let read = one_picture(&handle).and_then(|frame| {
            let scan = pipeline::read_character(&frame, &Ocr::new()?)?;
            builds::catch_up(&AppHost::new(&handle), scan)
        });
        handle.state::<LiveState>().reading_character.store(false, Ordering::SeqCst);
        let outcome = read.unwrap_or_else(|err| {
            report::note(format!("Couldn't read the character's windows: {err}"));
            builds::CharacterRead::failed(err.to_string())
        });
        // The desk view shows the result in its Character column.
        if !super::desk::showing(&handle)
            && let Err(err) = overlay::show_full(&handle)
        {
            report::note(err.to_string());
        }
        let _ = handle.emit(CHARACTER_READ_EVENT, &outcome);
    })?;
    Ok(())
}

/// Holds a new picture back until [`MIN_PICTURE_GAP`] has passed, and claims this moment for it.
fn wait_for_picture(state: &LiveState) {
    let mut last = state.last_picture.lock().expect("live lock");
    std::thread::sleep(gap_left(*last, Instant::now()));
    *last = Some(Instant::now());
}

fn gap_left(last: Option<Instant>, now: Instant) -> Duration {
    last.map_or(Duration::ZERO, |at| MIN_PICTURE_GAP.saturating_sub(now.saturating_duration_since(at)))
}

/// Moves the build along when the stat or skill window in this picture has changed. The player's
/// name has to be known: a window can't be matched to a build without it.
fn follow_windows(app: &AppHandle, watch: &mut Watch, frame: &Frame, analysis: &pipeline::Analysis, ocr: &Ocr) {
    let Some(layout) = &analysis.hud_layout else { return };
    let hud = analysis.hud.as_ref();
    let glance = character::glance(frame, layout, hud.and_then(|h| h.level));
    let name = hud.and_then(|h| h.name.clone());
    if !watch.settled(glance) || name.is_none() {
        return;
    }
    let read = character::read(frame, Some(layout), ocr).and_then(|windows| {
        builds::catch_up(&AppHost::new(app), pipeline::CharacterScan { name, level: hud.and_then(|h| h.level), windows })
    });
    if read.is_ok() {
        see(app, Reader::CharacterWindows);
    }
    match read {
        Ok(read) if read.changed() => {
            let _ = app.emit(CHARACTER_READ_EVENT, &read.on_its_own());
        }
        Ok(_) => {}
        Err(err) => report::note(format!("Couldn't read the character's windows: {err}")),
    }
}

/// Keeps the character's look from the Character Info window, once per opening, at a picture seen
/// twice. Only when the name under the picture is the HUD's own: another player's window gives
/// nothing, and nothing of it is kept past the frame.
fn follow_look(app: &AppHandle, watch: &mut LookWatch, frame: &Frame, analysis: &pipeline::Analysis, ocr: &Ocr) {
    let own = analysis.hud.as_ref().and_then(|h| h.name.as_deref());
    let portrait = own.and_then(|own| charinfo::read(frame, analysis.hud_layout.as_ref(), own, Some(ocr)));
    let Some(kept) = watch.follow(portrait) else { return };
    let Some(look) = own.and_then(|own| kept.seen(own)) else { return };
    if let Err(err) = records::keep_look(&AppHost::new(app), look) {
        report::note(format!("Couldn't keep the character's look: {err}"));
    }
}

/// Pauses or resumes tracking. Paused time doesn't count.
pub fn set_held(app: &AppHandle, held: bool) {
    app.state::<LiveState>().held.store(held, Ordering::SeqCst);
    show_controls(app);
}

pub fn toggle_held(app: &AppHandle) {
    set_held(app, !app.state::<LiveState>().held.load(Ordering::SeqCst));
}

/// Starts a new session. The one before it stays in the log.
pub fn reset(app: &AppHandle) {
    if running(app) {
        app.state::<LiveState>().reset.store(true, Ordering::SeqCst);
    } else {
        recovery::clear(app);
        readings::publish_session(app, &SessionSummary::default());
    }
}

/// Shows a change of the controls right away, without waiting for a new picture.
fn show_controls(app: &AppHandle) {
    let state = app.state::<LiveState>();
    let mut current = state.status.lock().expect("live lock");
    if let LiveStatus::Reading { held, .. } = &mut *current {
        *held = state.held.load(Ordering::SeqCst);
        let _ = app.emit(LIVE_EVENT, &*current);
    }
}

/// Keeps the session in the log as it goes.
#[derive(Default)]
struct Keeper {
    id: Option<i64>,
    saved_at: Option<Instant>,
    /// When the tracker's state was last written for recovery.
    checkpoint_at: Option<Instant>,
}

impl Keeper {
    /// The keeper for a session that has just ended or been put aside: its recovery snapshot
    /// goes, so the next start doesn't pick it up.
    fn fresh(app: &AppHandle) -> Self {
        recovery::clear(app);
        Self::default()
    }

    /// Writes the tracker's state so a crash or a restart can carry on with it. Every half
    /// minute with the save, or at once when `force`d.
    fn checkpoint(&mut self, app: &AppHandle, tracker: &SessionTracker, force: bool) {
        if !force && self.checkpoint_at.is_some_and(|at| at.elapsed() < SAVE_EVERY) {
            return;
        }
        self.checkpoint_at = Some(Instant::now());
        if tracker.summary().frames == 0 {
            return;
        }
        recovery::write(app, self.id, tracker);
    }

    fn due(&self) -> bool {
        self.saved_at.is_none_or(|at| at.elapsed() >= SAVE_EVERY)
    }

    fn save(&mut self, app: &AppHandle, session: &SessionSummary) {
        if session.active_seconds < SAVE_AFTER_ACTIVE {
            return;
        }
        self.saved_at = Some(Instant::now());
        let now = records::now();
        let saved = app.state::<UserDataState>().with(|db| match self.id {
            Some(id) => db.update_session(id, session, now).map(|()| id),
            None => db.save_session(session, "", now),
        });
        match saved {
            Ok(id) => {
                self.id = Some(id);
                let _ = app.emit(HISTORY_EVENT, ());
            }
            Err(err) => report::note(format!("Couldn't save the session: {err}")),
        }
    }
}

/// Puts a name read with a letter or two wrong back to the character it is. The name's font is
/// small, and at smaller window sizes OCR reads `Swiftleaf` as `Swiftleat`, which would
/// start a new session and keep mesos under a character that doesn't exist. A known name of the
/// same length that differs in at most two letters is taken as the one meant; a longer or
/// shorter name (`Swiftleaf2`, an alt) is another character.
struct NameSnap {
    /// Names with a record first, then those read in this run.
    known: Vec<String>,
}

impl NameSnap {
    const MAX_WRONG: usize = 2;
    const MIN_LENGTH: usize = 5;

    /// `known` most recently seen first. Of two recorded names that differ in a letter or two,
    /// one is a misread kept before this, and the later one stands for both.
    fn new(known: Vec<String>) -> Self {
        let mut snap = Self { known: Vec::new() };
        for name in known {
            if snap.closest(&name).is_none() {
                snap.known.push(name);
            }
        }
        snap
    }

    fn snap(&mut self, name: &mut Option<String>) {
        let Some(read) = name.as_deref() else { return };
        if let Some(known) = self.closest(read) {
            *name = Some(known.clone());
        } else if !read.is_empty() {
            self.known.push(read.to_string());
        }
    }

    fn closest(&self, read: &str) -> Option<&String> {
        let read: Vec<char> = read.to_lowercase().chars().collect();
        let wrong = |known: &String| {
            let known: Vec<char> = known.to_lowercase().chars().collect();
            (known.len() == read.len() && read.len() >= Self::MIN_LENGTH)
                .then(|| known.iter().zip(&read).filter(|(a, b)| a != b).count())
                .filter(|&n| n <= Self::MAX_WRONG)
        };
        // The first known name that fits closest, so a recorded name wins over one read today.
        self.known.iter().filter_map(|k| wrong(k).map(|n| (n, k))).min_by_key(|(n, _)| *n).map(|(_, k)| k)
    }
}

/// How often the character snapshot and the map's estimate are worked out again while on one map.
const FIGHTER_EVERY: Duration = Duration::from_secs(60);
const PRIOR_EVERY: Duration = Duration::from_secs(300);

/// Keeps the tracker supplied with what the estimates know: the character as they are (copied
/// into each stretch of play, so later estimates can compare like with like) and an estimate of
/// the pace on the map they are on, which the time to level starts from.
#[derive(Default)]
struct Estimates {
    fighter_at: Option<Instant>,
    prior_for: Option<(String, String)>,
    prior_at: Option<Instant>,
}

impl Estimates {
    fn follow(&mut self, app: &AppHandle, tracker: &mut SessionTracker, session: &SessionSummary) {
        let Some(name) = session.name.clone() else { return };
        let due = |at: Option<Instant>, every: Duration| at.is_none_or(|t| t.elapsed() >= every);
        if due(self.fighter_at, FIGHTER_EVERY) {
            self.fighter_at = Some(Instant::now());
            let fighter = fighter::fighter(&AppHost::new(app), &name, session.level, &fighter::Choice::default());
            tracker.set_fighter(fighter.and_then(|f| serde_json::to_value(f).ok()));
        }
        let Some(map) = tracker.current_map().map(str::to_string).filter(|_| !session.town) else { return };
        let here = (name.clone(), map.clone());
        if self.prior_for.as_ref() != Some(&here) || due(self.prior_at, PRIOR_EVERY) {
            self.prior_for = Some(here);
            self.prior_at = Some(Instant::now());
            let prior = fighter::prior(&AppHost::new(app), &name, session.level, &map, session.started_at);
            tracker.set_prior(&map, prior);
        }
    }
}

/// Where saved game pictures go: the player's chosen folder, or else `frames` in the app's local
/// data folder. Not Pictures, which OneDrive often syncs, and saved pictures must stay on this PC
/// unless the player moves them.
pub fn frames_folder(app: &AppHandle) -> Result<PathBuf> {
    match app.state::<SettingsState>().get().frames_folder.filter(|f| !f.trim().is_empty()) {
        Some(folder) => Ok(PathBuf::from(folder)),
        None => Ok(app.path().app_local_data_dir()?.join("frames")),
    }
}

/// Saves the last picture read as a PNG in the saved pictures folder, for a bug report: an item
/// check's for a moment after it, or else the last live one.
pub fn save_frame(app: &AppHandle) -> Result<PathBuf> {
    let state = app.state::<LiveState>();
    let item = state.item_picture.lock().expect("live lock").clone();
    let item = item.filter(|(at, _)| at.elapsed() < ITEM_PICTURE_KEPT).map(|(_, frame)| frame);
    let frame = item.or_else(|| state.frame.lock().expect("live lock").clone());
    let frame = frame.ok_or_else(|| Error::Invalid("There is no live picture yet.".into()))?;
    let folder = frames_folder(app)?;
    std::fs::create_dir_all(&folder)?;
    let path = folder.join(format!("leafline-{}.png", file_stamp(SystemTime::now())));
    frame.save(&path)?;
    let _ = app.emit(FRAME_SAVED_EVENT, &path);
    Ok(path)
}

/// What a misread folder's README says.
const MISREAD_README: &str = "Leafline saved this when you pressed This misread.\r\n\r\n\
picture.png is the last picture of the game Leafline read, and read.json is what it read from it, with what \
each part of the game was last read as. Attach both to a bug report (github.com/Crazy3001/Leafline/issues) and \
say what was wrong.\r\n\r\n\
They stay on this PC: Leafline never sends them anywhere. They show your game and your character's name, so look \
at them before you post them.\r\n";

/// "This misread": the last picture read, what Leafline read from it and what each reader has seen,
/// together in a folder of their own in the saved pictures folder, for a bug report. Nothing is sent.
pub fn save_misread(app: &AppHandle) -> Result<PathBuf> {
    let state = app.state::<LiveState>();
    let frame = state.frame.lock().expect("live lock").clone();
    let frame = frame.ok_or_else(|| Error::Invalid("There is no live picture yet. Start live reading with the game in front.".into()))?;
    let folder = frames_folder(app)?.join(format!("misread-{}", file_stamp(SystemTime::now())));
    std::fs::create_dir_all(&folder)?;
    frame.save(folder.join("picture.png"))?;
    let sight = serde_json::to_value(state.sight.lock().expect("live lock").clone()).ok();
    let report = crate::sight::misread_report(
        &app.package_info().version.to_string(),
        unix_seconds(SystemTime::now()),
        readings::current_analysis(app).and_then(|a| serde_json::to_value(a).ok()),
        readings::current_session(app).and_then(|s| serde_json::to_value(s).ok()),
        sight,
    );
    let text = serde_json::to_string_pretty(&report).map_err(|err| Error::Invalid(err.to_string()))?;
    std::fs::write(folder.join("read.json"), text)?;
    std::fs::write(folder.join("README.txt"), MISREAD_README)?;
    let _ = app.emit(FRAME_SAVED_EVENT, &folder);
    Ok(folder)
}

#[tauri::command]
pub fn save_misread_frame(app: AppHandle) -> Result<PathBuf> {
    save_misread(&app)
}

/// Saves the last item check's picture, however long ago it was taken, with what Leafline read from
/// it beside it as a `.json` file, for a bug report about that reading.
pub fn save_item_frame(app: &AppHandle, reading: Option<&str>) -> Result<PathBuf> {
    let frame = app.state::<LiveState>().item_picture.lock().expect("live lock").clone().map(|(_, frame)| frame);
    let frame = frame.ok_or_else(|| Error::Invalid("No item check has been taken yet.".into()))?;
    let folder = frames_folder(app)?;
    std::fs::create_dir_all(&folder)?;
    let path = folder.join(format!("leafline-item-{}.png", file_stamp(SystemTime::now())));
    frame.save(&path)?;
    if let Some(text) = reading {
        std::fs::write(path.with_extension("json"), text)?;
    }
    let _ = app.emit(FRAME_SAVED_EVENT, &path);
    Ok(path)
}

fn read_until_stopped(app: &AppHandle) -> Result<()> {
    let state = app.state::<LiveState>();
    let ocr = Ocr::new()?;
    // Live frames are plentiful, so a meso amount or quest counter must be read the same twice.
    let fresh_tracker =
        |idle_after| SessionTracker::new(TrackerConfig { confirmations: 2, idle_after, ..TrackerConfig::default() });
    let mut tracker = fresh_tracker(app.state::<SettingsState>().get().live.pause_after);
    let mut keeper = Keeper::default();
    *state.sight.lock().expect("live lock") = Sight::new(unix_seconds(SystemTime::now()));
    publish_sight(app);
    // A crash, a closed app or a restart within the break that ends a session carries on with
    // the session that was going, so the rates and the log entry continue.
    let mut resumed = false;
    if let Some((saved, id)) = recovery::load(
        app,
        TrackerConfig { confirmations: 2, idle_after: choice_pause_after(app), ..TrackerConfig::default() },
        BREAK_ENDS_SESSION.as_secs_f64(),
    ) {
        tracker = saved;
        keeper.id = id;
        resumed = true;
    }
    let mut memory = records::CharacterMemory::default();
    let mut titles = QuestTitles::default();
    let mut names = NameSnap::new(app.state::<UserDataState>().with(|db| db.character_names()).unwrap_or_default());
    let mut windows = Watch::default();
    let mut look = LookWatch::default();
    let mut quest_log = QuestLogFollow::default();
    let mut bags = BagFollow::default();
    let mut items = ItemHover::default();
    let mut estimates = Estimates::default();
    let mut twin = None;
    let mut held = false;
    let mut pickups = PickupReader::default();
    // The scale the game draws its interface at, from the last HUD read: the Pickup Log is
    // redrawn on the design grid when it isn't the one its font is read at.
    let mut ui_scale: Option<f32> = None;
    let mut game: Option<(Capturer, GameWindow, bool)> = None;
    // Windows can stop delivering pictures to a capture that was open while the window was
    // minimised, so the status would stay "minimised" after the game came back. A fresh capture
    // is opened once the window is no longer minimised.
    let mut was_minimized = false;
    let mut frames = 0u64;
    // The session's active time when it last grew, and when that was.
    let mut last_active = (if resumed { tracker.summary().active_seconds } else { 0.0 }, Instant::now());
    if resumed {
        readings::publish_session(app, &tracker.summary());
    }
    state.held.store(false, Ordering::SeqCst);

    while !state.stop.load(Ordering::SeqCst) {
        let tick = Instant::now();
        let choice = app.state::<SettingsState>().get().live;
        if !choice.enabled {
            break;
        }
        tracker.set_idle_after(choice.pause_after);
        if !choice.item_tooltips {
            items.forget();
        }
        if !choice.bag_check {
            bags = BagFollow::default();
        }

        if state.reset.swap(false, Ordering::SeqCst) {
            keeper.save(app, &tracker.summary());
            keeper = Keeper::fresh(app);
            tracker = fresh_tracker(choice.pause_after);
            pickups.reset();
            last_active = (0.0, Instant::now());
            held = false;
            state.held.store(false, Ordering::SeqCst);
            readings::publish_session(app, &tracker.summary());
            show_controls(app);
        }
        // A long break, game open or not, ends the session so tonight's play isn't added to
        // this afternoon's.
        if !held && last_active.0 > 0.0 && last_active.1.elapsed() >= BREAK_ENDS_SESSION {
            keeper.save(app, &tracker.summary());
            keeper = Keeper::fresh(app);
            tracker = fresh_tracker(choice.pause_after);
            pickups.reset();
            last_active = (0.0, Instant::now());
            readings::publish_session(app, &tracker.summary());
        }
        let now_held = state.held.load(Ordering::SeqCst);
        if now_held != held {
            held = now_held;
            tracker.set_held(held);
            readings::publish_session(app, &tracker.summary());
        }

        if game.is_none() {
            // Windows sometimes refuses a capture for a moment, so a failure is tried again.
            let found = capture::find(choice.window.as_ref()).and_then(|window| {
                window
                    .map(|window| {
                        let capturer = Capturer::new(&window)?;
                        let hdr = capturer.hdr();
                        Ok((capturer, window, hdr))
                    })
                    .transpose()
            });
            match found {
                Ok(Some(opened)) => game = Some(opened),
                Err(err) => {
                    report::note(format!("Couldn't open the game window: {err}"));
                    set_status(
                        app,
                        LiveStatus::Waiting { reason: format!("Couldn't open the game window, retrying: {err}") },
                    );
                    pause(&state, tick, choice.interval);
                    continue;
                }
                Ok(None) => {
                    let reason = if choice.window.is_some() {
                        "Looking for the game window you picked. Is the game running?"
                    } else {
                        "Looking for the MapleStory window. If the game is running, pick its window in Settings."
                    };
                    set_status(app, LiveStatus::Waiting { reason: reason.into() });
                    pause(&state, tick, choice.interval);
                    continue;
                }
            }
        }
        let (capturer, window, hdr) = game.as_mut().expect("game window found above");

        wait_for_picture(&state);
        let grabbed = match capturer.grab(FRESH_FRAME_WAIT) {
            Ok(grabbed) => grabbed,
            Err(err) => {
                report::note(format!("Couldn't capture the game window: {err}"));
                set_status(app, LiveStatus::Waiting { reason: format!("Couldn't capture the game, retrying: {err}") });
                game = None;
                pause(&state, tick, choice.interval);
                continue;
            }
        };
        if was_minimized && !matches!(grabbed, Grab::Minimized) {
            was_minimized = false;
            game = None;
            continue;
        }
        match grabbed {
            Grab::Frame(frame) => {
                let frame = Arc::new(frame);
                let started = Instant::now();
                let mut analysis = match pipeline::analyze_live(&frame, &ocr) {
                    Ok(analysis) => analysis,
                    Err(err) => {
                        report::note(format!("Couldn't read a picture: {err}"));
                        pause(&state, tick, choice.interval);
                        continue;
                    }
                };
                for part in &analysis.failed {
                    report::note(format!("Couldn't read the {part}"));
                }
                let mut town = false;
                let mut kill_exp = Vec::new();
                if let Some(map) = analysis.map.as_mut()
                    && let Some(atlas) = app.state::<MapDataState>().atlas()
                {
                    let street = map.street.as_deref();
                    let tags = if atlas.has_twin(&map.name, street) {
                        npc::names(&frame, &ocr).unwrap_or_else(|err| {
                            report::note(format!("Couldn't read the NPC names: {err}"));
                            Vec::new()
                        })
                    } else {
                        Vec::new()
                    };
                    let seen: Vec<&str> = tags.iter().map(String::as_str).collect();
                    twin = atlas.twin(&map.name, street, &seen, twin);
                    map.twin = twin;
                    if let Some(known) = atlas.settled(&map.name, map.street.as_deref()) {
                        town = atlas.is_town(known.id);
                        // What a kill here is worth, to count kills and tell quest EXP apart.
                        kill_exp = atlas.card(known.id).map(|c| c.mobs.iter().map(|m| m.exp).collect()).unwrap_or_default();
                        map.name = known.name;
                        map.street = known.street;
                    }
                }
                if let Some(atlas) = app.state::<MapDataState>().atlas() {
                    titles.hold(&atlas, &mut analysis.quest);
                }
                if let Some(hud) = analysis.hud.as_mut() {
                    names.snap(&mut hud.name);
                }
                let read_ms = started.elapsed().as_secs_f32() * 1000.0;
                let mut sample = Sample::from_analysis(unix_seconds(SystemTime::now()), &analysis);
                sample.town = town;
                sample.kill_exp = kill_exp;
                if !held {
                    if tracker.is_other_character(&sample) {
                        keeper.save(app, &tracker.summary());
                        keeper = Keeper::fresh(app);
                        tracker = fresh_tracker(choice.pause_after);
                        last_active = (0.0, Instant::now());
                        pickups.reset();
                    }
                    tracker.push(&sample);
                }
                if let Some(layout) = &analysis.hud_layout {
                    ui_scale = Some(crate::readers::ui::hud_scale(&frame, layout));
                }
                if choice.pickup_log {
                    read_pickups(app, &mut pickups, &mut tracker, &frame, ui_scale, &ocr);
                }
                let session = tracker.summary();
                {
                    let at = unix_seconds(SystemTime::now());
                    let smoothed = ui_scale.is_some_and(|s| (s - s.round()).abs() > 0.02);
                    let mut sight = state.sight.lock().expect("live lock");
                    sight.picture(at, &frame, &analysis, *hdr, smoothed);
                    if !held {
                        sight.session(at, &session, &analysis);
                    }
                }
                if !held {
                    estimates.follow(app, &mut tracker, &session);
                }
                if session.active_seconds > last_active.0 {
                    last_active = (session.active_seconds, Instant::now());
                }
                if choice.item_tooltips {
                    items.look(app, Some(&frame));
                }
                if choice.character_windows {
                    follow_windows(app, &mut windows, &frame, &analysis, &ocr);
                }
                if choice.character_look {
                    follow_look(app, &mut look, &frame, &analysis, &ocr);
                } else {
                    look = LookWatch::default();
                }
                if choice.quest_window {
                    quest_log.follow(app, &frame, &analysis, &ocr);
                }
                if choice.bag_check {
                    bags.follow(app, &frame, &analysis, &session);
                }
                readings::publish_live_analysis(app, analysis);
                readings::publish_session(app, &session);
                if keeper.due() {
                    keeper.save(app, &session);
                }
                keeper.checkpoint(app, &tracker, false);
                if let Err(err) = memory.note(&AppHost::new(app), records::seen_of(&session)) {
                    report::note(format!("Couldn't remember the character: {err}"));
                }

                frames += 1;
                if frames.is_multiple_of(HDR_CHECK_EVERY) {
                    *hdr = capturer.hdr();
                }
                set_status(
                    app,
                    LiveStatus::Reading {
                        window: window.title.clone(),
                        width: frame.width(),
                        height: frame.height(),
                        frames,
                        read_ms,
                        hdr: *hdr,
                        smoothed_ui: ui_scale.is_some_and(|s| (s - s.round()).abs() > 0.02),
                        held,
                    },
                );
                *state.frame.lock().expect("live lock") = Some(frame);
                publish_sight(app);
            }
            // The same picture again proves nothing, so it is not read twice: a misread would
            // otherwise confirm itself. A long quiet spell then pauses the clock like any idle time.
            // A tooltip held over a still scene is still held, though.
            Grab::Unchanged => {
                if choice.item_tooltips {
                    items.look(app, None);
                }
            }
            Grab::Minimized => {
                was_minimized = true;
                set_status(app, LiveStatus::Waiting { reason: "The game is minimised.".into() });
            }
            Grab::Nothing => set_status(app, LiveStatus::Waiting { reason: "Waiting for the game to draw.".into() }),
            Grab::Closed => {
                game = None;
                set_status(app, LiveStatus::Waiting { reason: "The game window closed.".into() });
            }
        }
        match game.as_mut() {
            Some((capturer, _, _)) if choice.pickup_log || choice.item_tooltips => {
                let mut look = |frame: Option<&Arc<Frame>>| {
                    if choice.item_tooltips {
                        items.look(app, frame);
                    }
                    if choice.pickup_log
                        && let Some(frame) = frame
                        && read_pickups(app, &mut pickups, &mut tracker, frame, ui_scale, &ocr)
                    {
                        readings::publish_session(app, &tracker.summary());
                    }
                };
                pause_looking(&state, tick, choice.interval, capturer, &mut look);
            }
            _ => pause(&state, tick, choice.interval),
        }
    }
    keeper.save(app, &tracker.summary());
    keeper.checkpoint(app, &tracker, true);
    Ok(())
}

fn choice_pause_after(app: &AppHandle) -> f64 {
    app.state::<SettingsState>().get().live.pause_after
}

/// Reads the Pickup Log in `frame` into the session: each line once, items named from the item
/// data. Returns whether anything arrived.
fn read_pickups(
    app: &AppHandle,
    reader: &mut PickupReader,
    tracker: &mut SessionTracker,
    frame: &Frame,
    ui_scale: Option<f32>,
    ocr: &Ocr,
) -> bool {
    let read = reader.read(unix_seconds(SystemTime::now()), frame, ui_scale, ocr);
    if read.layout.is_some() {
        tracker.watching_loot();
        see(app, Reader::PickupLog);
    }
    if read.arrivals.is_empty() {
        return false;
    }
    let catalog = app.state::<MapDataState>().catalog();
    let lines: Vec<Pickup> = read.arrivals.iter().map(|a| as_pickup(catalog.as_deref(), a)).collect();
    tracker.push_pickups(&lines);
    true
}

/// A settled log line as the session counts it.
fn as_pickup(catalog: Option<&Catalog>, arrival: &pickup::Arrival) -> Pickup {
    let Some(line) = &arrival.line else { return Pickup::Unread };
    match &line.gain {
        pickup::Gain::Mesos { amount } => Pickup::Mesos(*amount),
        pickup::Gain::Exp { amount } => Pickup::Exp { amount: *amount, party: line.party },
        pickup::Gain::PartyBonus { amount } => Pickup::PartyBonus(*amount),
        pickup::Gain::Item { name, quantity, tab, cut_short } => match catalog.and_then(|c| name_item(c, name, *tab, *cut_short)) {
            Some(item) => Pickup::Item { id: Some(item.id), name: item.name.clone(), count: *quantity },
            None => Pickup::Item { id: None, name: name.clone(), count: *quantity },
        },
    }
}

/// [`pause`], looking at the game every [`PICKUP_EVERY`] meanwhile and handing each new picture
/// to `look`, or `None` when the game drew nothing new since the last.
fn pause_looking(
    state: &LiveState,
    tick: Instant,
    interval: f64,
    capturer: &mut Capturer,
    look: &mut impl FnMut(Option<&Arc<Frame>>),
) {
    let until = tick + Duration::from_secs_f64(interval);
    let mut next = Instant::now() + PICKUP_EVERY;
    while !state.stop.load(Ordering::SeqCst) {
        let now = Instant::now();
        if now >= until {
            break;
        }
        if now >= next {
            next = now + PICKUP_EVERY;
            match capturer.grab(Duration::ZERO) {
                Ok(Grab::Frame(frame)) => look(Some(&Arc::new(frame))),
                Ok(Grab::Unchanged) => look(None),
                _ => {}
            }
            continue;
        }
        std::thread::sleep((until.min(next) - now).min(Duration::from_millis(100)));
    }
}

/// Sleeps until `interval` seconds after `tick`, waking early to stop.
fn pause(state: &LiveState, tick: Instant, interval: f64) {
    let until = tick + Duration::from_secs_f64(interval);
    while !state.stop.load(Ordering::SeqCst) {
        let now = Instant::now();
        if now >= until {
            break;
        }
        std::thread::sleep((until - now).min(Duration::from_millis(100)));
    }
}

fn set_status(app: &AppHandle, status: LiveStatus) {
    let state = app.state::<LiveState>();
    let mut current = state.status.lock().expect("live lock");
    if *current != status {
        *current = status.clone();
        let _ = app.emit(LIVE_EVENT, &status);
    }
}

fn unix_seconds(time: SystemTime) -> f64 {
    time.duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// `2026-10-30_14-05-09`, in UTC, for file names that sort by time.
fn file_stamp(time: SystemTime) -> String {
    let secs = time.duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (days, rest) = (secs / 86_400, secs % 86_400);
    // Days since 1970-01-01 to a civil date (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}_{:02}-{:02}-{:02}", rest / 3600, rest % 3600 / 60, rest % 60)
}

#[tauri::command]
pub fn live_status(app: AppHandle) -> LiveStatus {
    status(&app)
}

#[tauri::command]
pub fn read_character_windows(app: AppHandle) -> Result<()> {
    read_character(&app)
}

#[tauri::command]
pub fn start_live(app: AppHandle) -> Result<()> {
    start(&app)
}

#[tauri::command]
pub fn set_live_held(app: AppHandle, held: bool) {
    set_held(&app, held);
}

#[tauri::command]
pub fn reset_session(app: AppHandle) {
    reset(&app);
}

#[tauri::command]
pub fn stop_live(app: AppHandle) {
    stop(&app);
}

/// Windows that could be the game, for the picker in Settings.
#[tauri::command]
pub fn live_windows() -> Result<Vec<GameWindow>> {
    capture::windows()
}

#[tauri::command]
pub fn save_live_frame(app: AppHandle) -> Result<PathBuf> {
    save_frame(&app)
}

/// Saves the last item check's picture with what was read beside it, for a bug report.
#[tauri::command]
pub fn save_item_picture(app: AppHandle, reading: Option<String>) -> Result<PathBuf> {
    save_item_frame(&app, reading.as_deref())
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant, UNIX_EPOCH};

    use super::{MIN_PICTURE_GAP, NameSnap, file_stamp, gap_left};

    #[test]
    fn a_name_read_a_letter_wrong_is_the_character_it_resembles() {
        let mut names = NameSnap::new(vec!["PebbleThief".into()]);
        let mut snap = |read: &str| {
            let mut name = Some(read.to_string());
            names.snap(&mut name);
            name.unwrap()
        };
        assert_eq!(snap("PebbleThiet"), "PebbleThief", "seen live at 1366x768");
        assert_eq!(snap("pebbleThief"), "PebbleThief");
        assert_eq!(snap("PebbleThief2"), "PebbleThief2", "a longer name is an alt");
        assert_eq!(snap("MossWarden"), "MossWarden");
        assert_eq!(snap("MossWardon"), "MossWarden", "names read this run count too");
        assert_eq!(snap("Pebb"), "Pebb", "too short to tell");
    }

    #[test]
    fn a_misread_name_already_recorded_gives_way_to_the_one_seen_since() {
        let mut names = NameSnap::new(vec!["PebbleThief".into(), "PebbleThiet".into()]);
        let mut name = Some("PebbleThiet".to_string());
        names.snap(&mut name);
        assert_eq!(name.as_deref(), Some("PebbleThief"));
    }

    #[test]
    fn pictures_stay_a_second_apart() {
        let now = Instant::now();
        assert_eq!(gap_left(None, now), Duration::ZERO, "the first picture needn't wait");
        assert_eq!(gap_left(Some(now), now), MIN_PICTURE_GAP);
        assert_eq!(gap_left(Some(now), now + Duration::from_millis(400)), Duration::from_millis(600));
        assert_eq!(gap_left(Some(now), now + Duration::from_secs(5)), Duration::ZERO);
    }

    #[test]
    fn stamps_files_in_utc() {
        assert_eq!(file_stamp(UNIX_EPOCH), "1970-01-01_00-00-00");
        assert_eq!(file_stamp(UNIX_EPOCH + Duration::from_secs(1_793_365_509)), "2026-10-30_13-05-09");
        assert_eq!(file_stamp(UNIX_EPOCH + Duration::from_secs(951_782_400)), "2000-02-29_00-00-00");
    }
}
