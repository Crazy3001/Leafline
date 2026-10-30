//! Live reading: the game window captured every few seconds and run through the same pipeline
//! and session tracker as the workbench's replays. It stays off until the player turns it on in
//! Settings, and only ever looks at the window; see [`crate::capture`].

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use super::calibration::HudFontState;
use super::readings;
use super::replay;
use super::settings::{self, SettingsState};
use crate::capture::{self, Capturer, GameWindow, Grab};
use crate::error::{Error, Result};
use crate::pipeline;
use crate::session::{Sample, SessionTracker, TrackerConfig};
use crate::vision::{Frame, Ocr};

/// Emitted to every window when the live status changes.
pub const LIVE_EVENT: &str = "live";
/// Emitted with the path when a live picture is saved.
pub const FRAME_SAVED_EVENT: &str = "frame-saved";

/// How long a grab waits for the game to draw a fresh picture.
const FRESH_FRAME_WAIT: Duration = Duration::from_millis(300);
/// How often HDR is checked again, in readings, in case the game moves to another display.
const HDR_CHECK_EVERY: u64 = 30;

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
    },
    /// Stopped by an error.
    Failed { message: String },
}

#[derive(Default)]
pub struct LiveState {
    running: AtomicBool,
    stop: AtomicBool,
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
    let mut tracker = SessionTracker::new(TrackerConfig { confirmations: 2, ..TrackerConfig::default() });
    let mut game: Option<(Capturer, GameWindow, bool)> = None;
    let mut frames = 0u64;

    while !state.stop.load(Ordering::SeqCst) {
        let tick = Instant::now();
        let choice = app.state::<SettingsState>().get().live;
        if !choice.enabled {
            break;
        }

        if game.is_none() {
            match capture::find(choice.window.as_ref())? {
                Some(window) => {
                    let capturer = Capturer::new(&window)?;
                    let hdr = capturer.hdr();
                    game = Some((capturer, window, hdr));
                }
                None => {
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

        match capturer.grab(FRESH_FRAME_WAIT)? {
            Grab::Frame(frame) => {
                let font = app.state::<HudFontState>().current();
                let started = Instant::now();
                let analysis = pipeline::analyze(&frame, &ocr, &font)?;
                let read_ms = started.elapsed().as_secs_f32() * 1000.0;
                tracker.push(&Sample::from_analysis(unix_seconds(SystemTime::now()), &analysis));
                readings::publish_analysis(app, &analysis);
                readings::publish_session(app, &tracker.summary());

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
    app.state::<LiveState>().status.lock().expect("live lock").clone()
}

#[tauri::command]
pub fn start_live(app: AppHandle) -> Result<()> {
    start(&app)
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
