//! The program's half of the capture test (website plan, stage 2), a developer check reached from
//! Settings in a dev build only: the test pattern window, a check of that window by the program's
//! own capture, saving the game's frames for the side-by-side comparison, and the comparison.
//!
//! Frames go only to the folder the player picks, and the report is written there too.

use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use image::ImageEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};
use serde::Serialize;
use tauri::ipc::Response;
use tauri::{AppHandle, Emitter, Manager, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

use super::settings::{self, SettingsState};
use crate::capture::{self, Capturer, GameWindow, Grab};
use crate::capturetest::agree::{self, Agreement};
use crate::capturetest::measure::{self, Measure};
use crate::capturetest::pattern;
use crate::error::{Error, Result};
use crate::vision::{Frame, Ocr};

/// The window with the test's controls and steps.
pub const TOOLS: &str = "capture-test";
/// The window showing the pattern.
pub const PATTERN: &str = "capture-pattern";
/// Emitted while the comparison runs: frames read so far and the total.
pub const COMPARE_EVENT: &str = "capture-compare";

/// The most frames a second the recorder saves.
const MOST_PER_SECOND: u32 = 4;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecorderStatus {
    pub recording: bool,
    pub folder: Option<String>,
    pub per_second: u32,
    pub window: Option<String>,
    pub saved: u64,
    /// Moments a frame was due and none was saved: the game hadn't drawn one yet, or saving the
    /// last took too long.
    pub missed: u64,
    pub problem: Option<String>,
}

#[derive(Default)]
pub struct RecorderState {
    stop: AtomicBool,
    status: Mutex<RecorderStatus>,
}

impl RecorderState {
    fn update(&self, change: impl FnOnce(&mut RecorderStatus)) {
        change(&mut self.status.lock().expect("recorder lock"));
    }

    fn get(&self) -> RecorderStatus {
        self.status.lock().expect("recorder lock").clone()
    }
}

fn open(app: &AppHandle, label: &str, route: &str, title: &str, size: (f64, f64)) -> Result<tauri::WebviewWindow> {
    if let Some(window) = app.get_webview_window(label) {
        settings::bring_forward(app, &window)?;
        return Ok(window);
    }
    Ok(WebviewWindowBuilder::new(app, label, WebviewUrl::App(route.into()))
        .title(title)
        .inner_size(size.0, size.1)
        .center()
        .background_color(tauri::window::Color(20, 20, 18, 255))
        .additional_browser_args(super::BROWSER_ARGS)
        .build()?)
}

/// Opens the capture test's window.
#[tauri::command]
pub async fn open_capture_test(app: AppHandle) -> Result<()> {
    open(&app, TOOLS, "test/tools", "Leafline — Capture test", (760.0, 820.0)).map(drop)
}

/// Opens the pattern window, its inside exactly the pattern's size in screen pixels.
#[tauri::command]
pub async fn open_capture_pattern(app: AppHandle) -> Result<()> {
    let window = open(&app, PATTERN, "test/pattern", "Leafline capture test pattern", (1024.0, 576.0))?;
    window.set_resizable(false)?;
    window.set_size(PhysicalSize::new(pattern::WIDTH, pattern::HEIGHT))?;
    Ok(())
}

/// The pattern for a window of that size and scale, as RGBA bytes.
#[tauri::command]
pub fn capture_pattern(width: u32, height: u32, scale_milli: u32) -> Response {
    Response::new(pattern::draw(pattern::WindowInfo { width, height, scale_milli }).into_raw())
}

/// The pattern window as the program's own capture sees it, measured: what the window really
/// shows, before any browser touches it.
#[tauri::command]
pub async fn check_capture_pattern(app: AppHandle) -> Result<Measure> {
    let window = app
        .get_webview_window(PATTERN)
        .ok_or_else(|| Error::Invalid("Open the pattern window first.".into()))?;
    let size = window.inner_size()?;
    let target = GameWindow {
        id: window.hwnd()?.0 as isize,
        title: window.title()?,
        class: String::new(),
        width: size.width,
        height: size.height,
        likely_game: false,
    };
    tauri::async_runtime::spawn_blocking(move || {
        let frame = capture::picture(&target)?;
        Ok(measure::measure(&frame, None))
    })
    .await
    .map_err(|err| Error::Invalid(err.to_string()))?
}

/// Starts saving the game window's frames into `folder` as `app-<unix ms>.png`, `per_second`
/// times a second on the clock's whole fractions, as the website's page does, so the two sides'
/// frames are taken at nearly the same moments.
#[tauri::command]
pub fn start_capture_recording(app: AppHandle, folder: String, per_second: u32) -> Result<RecorderStatus> {
    let state = app.state::<RecorderState>();
    if state.get().recording {
        return Ok(state.get());
    }
    let folder = PathBuf::from(folder);
    if !folder.is_dir() {
        return Err(Error::Invalid("Pick a folder that exists.".into()));
    }
    let per_second = per_second.clamp(1, MOST_PER_SECOND);
    let choice = app.state::<SettingsState>().get().live.window;
    let window = capture::find(choice.as_ref())?
        .ok_or_else(|| Error::Invalid("The game window wasn't found. Is the game running?".into()))?;
    state.stop.store(false, Ordering::SeqCst);
    state.update(|s| {
        *s = RecorderStatus {
            recording: true,
            folder: Some(folder.display().to_string()),
            per_second,
            window: Some(window.title.clone()),
            ..RecorderStatus::default()
        }
    });
    let handle = app.clone();
    std::thread::spawn(move || {
        // The capture is opened on the thread that uses it: it can't be handed between threads.
        let result = Capturer::new(&window).and_then(|capturer| record(&handle, capturer, &folder, per_second));
        let state = handle.state::<RecorderState>();
        state.update(|s| {
            s.recording = false;
            s.problem = result.err().map(|err| err.to_string());
        });
    });
    Ok(state.get())
}

fn unix_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

fn save(frame: &Frame, path: &std::path::Path) -> Result<()> {
    let file = BufWriter::new(std::fs::File::create(path)?);
    // Fast rather than small: a 2560 x 1440 frame has to be written well inside a quarter second.
    PngEncoder::new_with_quality(file, CompressionType::Fast, FilterType::Sub).write_image(
        frame.as_raw(),
        frame.width(),
        frame.height(),
        image::ExtendedColorType::Rgba8,
    )?;
    Ok(())
}

fn record(app: &AppHandle, mut capturer: Capturer, folder: &std::path::Path, per_second: u32) -> Result<()> {
    let state = app.state::<RecorderState>();
    let period = u64::from(1000 / per_second);
    let mut last: Option<Frame> = None;
    let mut next = unix_ms().div_ceil(period) * period;
    while !state.stop.load(Ordering::SeqCst) {
        let now = unix_ms();
        if now < next {
            std::thread::sleep(Duration::from_millis((next - now).min(50)));
            continue;
        }
        // Ticks that went by while the last frame was saved.
        let behind = (now - next) / period;
        if behind > 0 {
            state.update(|s| s.missed += behind);
        }
        next = (now / period + 1) * period;
        match capturer.grab(Duration::from_millis(40))? {
            Grab::Frame(frame) => last = Some(frame),
            // Windows sends only changes: the last picture is still what the window shows.
            Grab::Unchanged => {}
            Grab::Closed => return Err(Error::Invalid("The game window closed.".into())),
            Grab::Minimized | Grab::Nothing => {
                state.update(|s| s.missed += 1);
                continue;
            }
        }
        let Some(frame) = &last else {
            state.update(|s| s.missed += 1);
            continue;
        };
        // The moment the picture was in hand, which is what the website's names say too.
        let at = unix_ms();
        save(frame, &folder.join(format!("app-{at}.png")))?;
        state.update(|s| s.saved += 1);
    }
    Ok(())
}

#[tauri::command]
pub fn stop_capture_recording(app: AppHandle) -> RecorderStatus {
    let state = app.state::<RecorderState>();
    state.stop.store(true, Ordering::SeqCst);
    state.get()
}

#[tauri::command]
pub fn capture_recording_status(app: AppHandle) -> RecorderStatus {
    app.state::<RecorderState>().get()
}

/// Reads both sides' frames in `folder` with the program's readers and compares them. Writes the
/// report next to the frames as well.
#[tauri::command]
pub async fn compare_capture_frames(app: AppHandle, folder: String) -> Result<Agreement> {
    tauri::async_runtime::spawn_blocking(move || {
        let folder = PathBuf::from(folder);
        let ocr = Ocr::new()?;
        let report = agree::compare(&folder, &ocr, |done, total| {
            let _ = app.emit(COMPARE_EVENT, (done, total));
        })?;
        let json = serde_json::to_string_pretty(&report).map_err(|err| Error::Invalid(err.to_string()))?;
        std::fs::write(folder.join(agree::REPORT_FILE), json)?;
        Ok(report)
    })
    .await
    .map_err(|err| Error::Invalid(err.to_string()))?
}
