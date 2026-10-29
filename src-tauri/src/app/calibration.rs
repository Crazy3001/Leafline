//! The HUD font learned during calibration, saved in the app's data folder. It is drawn from
//! the user's own screen and never leaves their machine.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{AppHandle, Manager, State};

use crate::error::{Error, Result};
use crate::readers::hud::{self, Calibration, HudFont, HudValues};
use crate::vision;

const FILE_NAME: &str = "hud-font.json";

pub struct HudFontState {
    path: PathBuf,
    font: Mutex<HudFont>,
}

impl HudFontState {
    /// Loads the saved font, or starts empty if there is none yet.
    pub fn load(app: &AppHandle) -> Result<Self> {
        let path = app.path().app_data_dir()?.join(FILE_NAME);
        let font = match std::fs::read_to_string(&path) {
            Ok(json) => {
                serde_json::from_str(&json).map_err(|err| Error::Invalid(format!("{}: {err}", path.display())))?
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => HudFont::default(),
            Err(err) => return Err(err.into()),
        };
        Ok(Self { path, font: Mutex::new(font) })
    }

    pub fn current(&self) -> HudFont {
        self.font.lock().expect("HUD font lock").clone()
    }

    fn save(&self, font: &HudFont) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(font).map_err(|err| Error::Invalid(err.to_string()))?;
        std::fs::write(&self.path, json)?;
        Ok(())
    }
}

/// Learns the HUD font from a screenshot whose HUD values the user has typed in.
#[tauri::command]
pub async fn calibrate_hud(app: AppHandle, path: PathBuf, values: HudValues) -> Result<Calibration> {
    tauri::async_runtime::spawn_blocking(move || learn(&app.state::<HudFontState>(), &path, &values))
        .await
        .map_err(|err| Error::Invalid(err.to_string()))?
}

fn learn(state: &State<HudFontState>, path: &Path, values: &HudValues) -> Result<Calibration> {
    let frame = vision::load(path)?;
    let mut font = state.current();
    let report = hud::calibrate(&frame, &mut font, values)?;
    if !report.learned.is_empty() {
        state.save(&font)?;
        *state.font.lock().expect("HUD font lock") = font;
    }
    Ok(report)
}
