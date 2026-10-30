//! Live reading: the game window captured every few seconds and run through the same pipeline
//! and session tracker as the workbench's replays. It stays off until the player turns it on in
//! Settings, and only ever looks at the window; see [`crate::capture`]. Each session saves itself
//! as it goes, so a crash or a closed window costs half a minute of it at most.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use super::calibration::HudFontState;
use super::mapdata::MapDataState;
use super::readings;
use super::records::{self, HISTORY_EVENT, UserDataState};
use super::replay;
use super::report;
use super::settings::{self, SettingsState};
use crate::capture::{self, Capturer, GameWindow, Grab};
use crate::error::{Error, Result};
use crate::pipeline;
use crate::session::{Sample, SessionSummary, SessionTracker, TrackerConfig};
use crate::vision::{Frame, Ocr};

/// Emitted to every window when the live status changes.
pub const LIVE_EVENT: &str = "live";
/// Emitted with the path when a live picture is saved.
pub const FRAME_SAVED_EVENT: &str = "frame-saved";

/// How long a grab waits for the game to draw a fresh picture.
const FRESH_FRAME_WAIT: Duration = Duration::from_millis(300);
/// How often HDR is checked again, in readings, in case the game moves to another display.
const HDR_CHECK_EVERY: u64 = 30;
/// Active seconds before a session is worth keeping, and how often it is saved from then on.
const SAVE_AFTER_ACTIVE: f64 = 60.0;
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
        /// Whether the HUD font has been taught; without it EXP, HP and MP aren't read.
        hud_taught: bool,
        /// Tracking is paused by hand.
        held: bool,
        /// This run goes to the log.
        logging: bool,
    },
    /// Stopped by an error.
    Failed { message: String },
}

#[derive(Default)]
pub struct LiveState {
    running: AtomicBool,
    stop: AtomicBool,
    /// Tracking is paused by hand; pictures are still read.
    held: AtomicBool,
    /// A new session was asked for.
    reset: AtomicBool,
    /// This run stays out of the log. It ends with the session.
    log_off: AtomicBool,
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
    if replay::running(app) {
        return Err(Error::Invalid("A replay is running in the workbench. Stop it first.".into()));
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

/// Keeps this run in or out of the log.
pub fn set_logging(app: &AppHandle, logging: bool) {
    app.state::<LiveState>().log_off.store(!logging, Ordering::SeqCst);
    show_controls(app);
}

pub fn toggle_logging(app: &AppHandle) {
    set_logging(app, app.state::<LiveState>().log_off.load(Ordering::SeqCst));
}

/// Shows a change of the controls right away, without waiting for a new picture.
fn show_controls(app: &AppHandle) {
    let state = app.state::<LiveState>();
    let mut current = state.status.lock().expect("live lock");
    if let LiveStatus::Reading { held, logging, .. } = &mut *current {
        *held = state.held.load(Ordering::SeqCst);
        *logging = !state.log_off.load(Ordering::SeqCst);
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
        if session.active_seconds < SAVE_AFTER_ACTIVE || app.state::<LiveState>().log_off.load(Ordering::SeqCst) {
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

/// Saves the last picture read as a PNG in Pictures\Leafline, for the workbench or a bug report.
pub fn save_frame(app: &AppHandle) -> Result<PathBuf> {
    let frame = app.state::<LiveState>().frame.lock().expect("live lock").clone();
    let frame = frame.ok_or_else(|| Error::Invalid("There is no live picture yet.".into()))?;
    let folder = app.path().picture_dir()?.join("Leafline");
    std::fs::create_dir_all(&folder)?;
    let path = folder.join(format!("leafline-{}.png", file_stamp(SystemTime::now())));
    frame.save(&path)?;
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
    let mut held = false;
    let mut game: Option<(Capturer, GameWindow, bool)> = None;
    let mut frames = 0u64;
    state.held.store(false, Ordering::SeqCst);
    state.log_off.store(false, Ordering::SeqCst);

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
            state.log_off.store(false, Ordering::SeqCst);
            held = false;
            state.held.store(false, Ordering::SeqCst);
            readings::publish_session(app, &tracker.summary());
            show_controls(app);
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
        match grabbed {
            Grab::Frame(frame) => {
                let font = app.state::<HudFontState>().current();
                let started = Instant::now();
                let mut analysis = match pipeline::analyze(&frame, &ocr, &font) {
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
                    && let Some(known) = atlas.settled(&map.name, map.street.as_deref())
                {
                    town = atlas.is_town(known.id);
                    map.name = known.name;
                    map.street = known.street;
                }
                let read_ms = started.elapsed().as_secs_f32() * 1000.0;
                let mut sample = Sample::from_analysis(unix_seconds(SystemTime::now()), &analysis);
                sample.town = town;
                if !held {
                    if tracker.is_other_character(&sample) {
                        keeper.save(app, &tracker.summary());
                        keeper = Keeper::default();
                        tracker = fresh_tracker(choice.pause_after);
                    }
                    tracker.push(&sample);
                }
                let session = tracker.summary();
                readings::publish_analysis(app, &analysis);
                readings::publish_session(app, &session);
                if keeper.due() {
                    keeper.save(app, &session);
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
                        hud_taught: !font.values.is_empty(),
                        held,
                        logging: !state.log_off.load(Ordering::SeqCst),
                    },
                );
                *state.frame.lock().expect("live lock") = Some(Arc::new(frame));
            }
            // The same picture again proves nothing, so it is not read twice: a misread would
            // otherwise confirm itself. A long quiet spell then pauses the clock like any idle time.
            Grab::Unchanged => {}
            Grab::Minimized => set_status(app, LiveStatus::Waiting { reason: "The game is minimised.".into() }),
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
pub fn set_live_logging(app: AppHandle, logging: bool) {
    set_logging(&app, logging);
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

#[cfg(test)]
mod tests {
    use std::time::{Duration, UNIX_EPOCH};

    use super::file_stamp;

    #[test]
    fn stamps_files_in_utc() {
        assert_eq!(file_stamp(UNIX_EPOCH), "1970-01-01_00-00-00");
        assert_eq!(file_stamp(UNIX_EPOCH + Duration::from_secs(1_793_365_509)), "2026-10-30_13-05-09");
        assert_eq!(file_stamp(UNIX_EPOCH + Duration::from_secs(951_782_400)), "2000-02-29_00-00-00");
    }
}
