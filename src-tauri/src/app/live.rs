//! Live reading: the game window captured every few seconds and run through the pipeline and
//! the session tracker. It stays off until the player turns it on in Settings, and only ever looks at the window; see [`crate::capture`]. Each session saves itself
//! as it goes, so a crash or a closed window costs half a minute of it at most.
//!
//! Item tooltips are not part of this. An item check is one picture on the player's hotkey. The
//! character's stat and skill windows are read from these same pictures while they are open, and
//! from one picture on a hotkey otherwise.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use super::mapdata::MapDataState;
use super::records::{self, HISTORY_EVENT, MESOS_EVENT, UserDataState};
use super::builds;
use super::report;
use super::settings::{self, SettingsState};
use super::{overlay, pins, readings};
use crate::capture::{self, Capturer, GameWindow, Grab};
use crate::error::{Error, Result};
use crate::mapdata::QuestTitles;
use crate::pipeline;
use crate::readers::character::{self, Watch};
use crate::readers::npc;
use crate::readers::tooltip::ItemTooltip;
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
/// A meso amount is remembered for its character only while it is on screen, at most this long
/// after it was last read.
const MESOS_FRESH: f64 = 5.0;
/// How often an unchanged amount still on screen refreshes when it was last seen.
const MESOS_SEEN_EVERY: Duration = Duration::from_secs(60);

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

/// One picture of the game, for something the player asked to read.
fn one_picture(app: &AppHandle) -> Result<Frame> {
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
            builds::catch_up(&handle, scan)
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
        builds::catch_up(app, pipeline::CharacterScan { name, level: hud.and_then(|h| h.level), windows })
    });
    match read {
        Ok(read) if read.changed() => {
            let _ = app.emit(CHARACTER_READ_EVENT, &read.on_its_own());
        }
        Ok(_) => {}
        Err(err) => report::note(format!("Couldn't read the character's windows: {err}")),
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
}

impl Keeper {
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

/// Remembers each character's mesos between runs, once the tracker has confirmed both the name
/// and the amount.
#[derive(Default)]
struct MesoMemory {
    saved: Option<(String, u64)>,
    saved_at: Option<Instant>,
}

impl MesoMemory {
    fn note(&mut self, app: &AppHandle, session: &SessionSummary) {
        let (Some(name), Some(mesos)) = (&session.name, session.mesos) else { return };
        if session.mesos_age.is_none_or(|age| age > MESOS_FRESH) {
            return;
        }
        let same = self.saved.as_ref().is_some_and(|(n, m)| n == name && *m == mesos);
        if same && self.saved_at.is_some_and(|at| at.elapsed() < MESOS_SEEN_EVERY) {
            return;
        }
        match app.state::<UserDataState>().with(|db| db.save_mesos(name, mesos, records::now())) {
            Ok(()) => {
                self.saved = Some((name.clone(), mesos));
                self.saved_at = Some(Instant::now());
                let _ = app.emit(MESOS_EVENT, ());
            }
            Err(err) => report::note(format!("Couldn't remember the mesos: {err}")),
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
    let mut memory = MesoMemory::default();
    let mut titles = QuestTitles::default();
    let mut names = NameSnap::new(app.state::<UserDataState>().with(|db| db.character_names()).unwrap_or_default());
    let mut windows = Watch::default();
    let mut twin = None;
    let mut held = false;
    let mut game: Option<(Capturer, GameWindow, bool)> = None;
    // Windows can stop delivering pictures to a capture that was open while the window was
    // minimised, so the status would stay "minimised" after the game came back. A fresh capture
    // is opened once the window is no longer minimised.
    let mut was_minimized = false;
    let mut frames = 0u64;
    // The session's active time when it last grew, and when that was.
    let mut last_active = (0.0f64, Instant::now());
    state.held.store(false, Ordering::SeqCst);

    while !state.stop.load(Ordering::SeqCst) {
        let tick = Instant::now();
        let choice = app.state::<SettingsState>().get().live;
        if !choice.enabled {
            break;
        }
        tracker.set_idle_after(choice.pause_after);

        if state.reset.swap(false, Ordering::SeqCst) {
            keeper.save(app, &tracker.summary());
            keeper = Keeper::default();
            tracker = fresh_tracker(choice.pause_after);
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
            keeper = Keeper::default();
            tracker = fresh_tracker(choice.pause_after);
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
                if !held {
                    if tracker.is_other_character(&sample) {
                        keeper.save(app, &tracker.summary());
                        keeper = Keeper::default();
                        tracker = fresh_tracker(choice.pause_after);
                        last_active = (0.0, Instant::now());
                    }
                    tracker.push(&sample);
                }
                let session = tracker.summary();
                if session.active_seconds > last_active.0 {
                    last_active = (session.active_seconds, Instant::now());
                }
                follow_windows(app, &mut windows, &frame, &analysis, &ocr);
                readings::publish_live_analysis(app, analysis);
                readings::publish_session(app, &session);
                if keeper.due() {
                    keeper.save(app, &session);
                }
                memory.note(app, &session);

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
                        held,
                    },
                );
                *state.frame.lock().expect("live lock") = Some(Arc::new(frame));
            }
            // The same picture again proves nothing, so it is not read twice: a misread would
            // otherwise confirm itself. A long quiet spell then pauses the clock like any idle time.
            Grab::Unchanged => {}
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
        pause(&state, tick, choice.interval);
    }
    keeper.save(app, &tracker.summary());
    Ok(())
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
