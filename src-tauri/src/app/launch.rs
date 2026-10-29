//! Opening a screenshot passed on the command line ("Open with", or dropped onto the exe).

use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, State};

use super::overlay::{self, WORKBENCH};
use crate::error::Result;
use crate::vision;

/// Emitted to the workbench with a path to open.
pub const OPEN_EVENT: &str = "open-image";

/// An image given at startup, held until the workbench is ready to ask for it.
#[derive(Default)]
pub struct LaunchImage(Mutex<Option<PathBuf>>);

impl LaunchImage {
    pub fn new(args: &[String]) -> Self {
        Self(Mutex::new(image_arg(args)))
    }
}

/// The first argument (after the program itself) that is a supported image file.
pub fn image_arg(args: &[String]) -> Option<PathBuf> {
    args.iter().skip(1).map(PathBuf::from).find(|path| path.is_file() && vision::is_supported(path))
}

/// Shows the workbench and asks it to open `path` (used when a second launch passes one).
pub fn open(app: &AppHandle, path: PathBuf) -> Result<()> {
    overlay::show_workbench(app)?;
    app.emit_to(WORKBENCH, OPEN_EVENT, path)?;
    Ok(())
}

#[tauri::command]
pub fn take_launch_image(state: State<'_, LaunchImage>) -> Option<PathBuf> {
    state.0.lock().ok()?.take()
}
