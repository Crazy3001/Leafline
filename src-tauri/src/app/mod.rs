//! Tauri wiring: windows, tray, hotkeys and commands.

mod calibration;
mod commands;
mod launch;
mod overlay;
mod shortcuts;
mod tray;

use tauri::{Manager, WindowEvent};
use tauri_plugin_window_state::StateFlags;

use calibration::HudFontState;
use launch::LaunchImage;
use overlay::{OVERLAY, OverlayState, WORKBENCH};

pub fn run() {
    let args: Vec<String> = std::env::args().collect();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            let result = match launch::image_arg(&args) {
                Some(path) => launch::open(app, path),
                None => overlay::show_workbench(app),
            };
            if let Err(err) = result {
                shortcuts::log_error(&err);
            }
        }))
        .plugin(
            tauri_plugin_window_state::Builder::new().with_state_flags(StateFlags::POSITION | StateFlags::SIZE).build(),
        )
        .plugin(tauri_plugin_dialog::init())
        .plugin(shortcuts::plugin())
        .manage(OverlayState::default())
        .manage(LaunchImage::new(&args))
        .setup(|app| {
            app.manage(HudFontState::load(app.handle())?);
            tray::create(app)?;
            overlay::fit_workbench(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing a window hides it; the app keeps running in the tray until Quit.
            if let WindowEvent::CloseRequested { api, .. } = event
                && matches!(window.label(), OVERLAY | WORKBENCH)
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::analyze_image,
            calibration::calibrate_hud,
            commands::list_images,
            commands::read_image,
            commands::overlay_status,
            commands::set_click_through,
            commands::hide_overlay,
            commands::toggle_overlay,
            commands::show_workbench,
            commands::shortcut_list,
            launch::take_launch_image,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Leafline");
}
