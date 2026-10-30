//! Tauri wiring: windows, tray, hotkeys and commands.

mod calibration;
mod commands;
mod launch;
mod mapdata;
mod overlay;
mod pins;
mod readings;
mod records;
mod replay;
mod settings;
mod shortcuts;
mod tray;

use tauri::{Manager, WindowEvent};
use tauri_plugin_window_state::StateFlags;

use calibration::HudFontState;
use launch::LaunchImage;
use mapdata::MapDataState;
use overlay::{OVERLAY, OverlayState, WORKBENCH};
use pins::PinState;
use readings::Readings;
use records::UserDataState;
use replay::ReplayState;
use settings::SettingsState;

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
        .plugin(tauri_plugin_opener::init())
        .plugin(shortcuts::plugin())
        .manage(OverlayState::default())
        .manage(ReplayState::default())
        .manage(Readings::default())
        .manage(LaunchImage::new(&args))
        .setup(|app| {
            app.manage(HudFontState::load(app.handle())?);
            app.manage(MapDataState::load(app.handle())?);
            app.manage(UserDataState::load(app.handle())?);
            mapdata::check_for_updates(app.handle());
            app.manage(SettingsState::load(app.handle())?);
            settings::apply(app.handle())?;
            app.manage(PinState::load(app.handle())?);
            pins::restore(app.handle())?;
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
            replay::replay_session,
            replay::cancel_replay,
            commands::overlay_status,
            commands::set_click_through,
            commands::hide_overlay,
            commands::toggle_overlay,
            commands::show_workbench,
            commands::shortcut_list,
            readings::current_readings,
            pins::pin_status,
            pins::pin_panel,
            pins::unpin_panel,
            pins::toggle_pins,
            launch::take_launch_image,
            mapdata::map_data_status,
            mapdata::download_map_data,
            mapdata::find_map,
            mapdata::map_card,
            mapdata::search_maps,
            mapdata::map_route,
            mapdata::travel_info,
            mapdata::edit_travel_file,
            mapdata::monster_drops,
            mapdata::quest_guide,
            mapdata::look_up_quest_items,
            mapdata::training_spots,
            records::price_log,
            records::record_price,
            records::delete_price,
            records::save_session,
            records::session_history,
            records::delete_session,
            records::export_csv,
            settings::get_settings,
            settings::update_settings,
            settings::show_settings,
            mapdata::open_link,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Leafline");
}
