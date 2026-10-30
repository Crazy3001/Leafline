//! Saves window positions and sizes soon after they change. The window-state plugin only writes
//! them on a clean exit, so a crash or a forced close would otherwise lose the layout.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use tauri::AppHandle;
use tauri_plugin_window_state::{AppHandleExt, StateFlags};

/// What is remembered about each window.
pub const SAVED: StateFlags = StateFlags::POSITION.union(StateFlags::SIZE);

/// How long windows must keep still before their places are saved.
const QUIET: Duration = Duration::from_millis(500);

static CHANGES: AtomicU64 = AtomicU64::new(0);
static WAITING: AtomicBool = AtomicBool::new(false);

/// Notes that a window moved or resized. One save follows once they all keep still.
pub fn changed(app: &AppHandle) {
    CHANGES.fetch_add(1, Ordering::SeqCst);
    if WAITING.swap(true, Ordering::SeqCst) {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let mut seen = CHANGES.load(Ordering::SeqCst);
        loop {
            std::thread::sleep(QUIET);
            let now = CHANGES.load(Ordering::SeqCst);
            if now == seen {
                break;
            }
            seen = now;
        }
        WAITING.store(false, Ordering::SeqCst);
        // On the main thread: the plugin holds its lock while asking windows for their places,
        // which from here would deadlock with its handler for the next move.
        let handle = app.clone();
        let _ = app.run_on_main_thread(move || {
            if let Err(err) = handle.save_window_state(SAVED) {
                eprintln!("could not save window positions: {err}");
            }
        });
    });
}
