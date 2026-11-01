//! Tauri wiring: windows, tray, hotkeys and commands.

mod builds;
mod commands;
mod launch;
mod layout;
mod live;
mod mapdata;
mod overlay;
mod pins;
mod readings;
mod records;
mod replay;
mod report;
mod settings;
mod shortcuts;
mod snap;
mod tray;

use tauri::{Manager, WindowEvent};

use launch::LaunchImage;
use live::LiveState;
use mapdata::MapDataState;
use overlay::{OVERLAY, OverlayState, WORKBENCH};
use pins::PinState;
use readings::Readings;
use records::UserDataState;
use replay::ReplayState;
use settings::SettingsState;
use shortcuts::Bindings;

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
        .plugin(tauri_plugin_window_state::Builder::new().with_state_flags(layout::SAVED).build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(shortcuts::plugin())
        .manage(OverlayState::default())
        .manage(ReplayState::default())
        .manage(LiveState::default())
        .manage(Readings::default())
        .manage(Bindings::default())
        .manage(LaunchImage::new(&args))
        .setup(|app| {
            app.manage(MapDataState::load(app.handle())?);
            app.manage(UserDataState::load(app.handle())?);
            mapdata::check_for_updates(app.handle());
            app.manage(SettingsState::load(app.handle())?);
            settings::apply(app.handle())?;
            overlay::restore_mode(app.handle())?;
            if let Some(overlay) = app.get_webview_window(OVERLAY) {
                snap::attach(&overlay)?;
            }
            app.manage(PinState::load(app.handle())?);
            pins::restore(app.handle())?;
            tray::create(app)?;
            overlay::create_workbench(app.handle())?;
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // Closing a window hides it; the app keeps running in the tray until Quit.
            WindowEvent::CloseRequested { api, .. } if matches!(window.label(), OVERLAY | WORKBENCH) => {
                api.prevent_close();
                let _ = window.hide();
            }
            WindowEvent::Moved(_) => layout::changed(window.app_handle()),
            WindowEvent::Resized(size) => {
                pins::resized(window, *size);
                layout::changed(window.app_handle());
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            commands::analyze_image,
            commands::list_images,
            commands::read_image,
            replay::replay_session,
            replay::cancel_replay,
            live::live_status,
            live::start_live,
            live::stop_live,
            live::live_windows,
            live::save_live_frame,
            live::set_live_held,
            live::reset_session,
            live::set_live_logging,
            commands::overlay_status,
            commands::set_click_through,
            commands::set_compact,
            commands::hide_overlay,
            commands::toggle_overlay,
            commands::show_workbench,
            commands::shortcut_list,
            commands::open_data_folder,
            report::diagnostics,
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
            mapdata::download_travel,
            mapdata::edit_travel_file,
            mapdata::monster_drops,
            mapdata::quest_guide,
            mapdata::download_drops,
            mapdata::training_spots,
            records::price_log,
            records::record_price,
            records::delete_price,
            records::save_session,
            records::session_history,
            records::delete_session,
            records::set_session_note,
            records::rename_map,
            records::export_csv,
            builds::build_catalog,
            builds::starter_plans,
            builds::search_gear,
            builds::search_items,
            builds::item_filters,
            builds::item_record,
            builds::find_item,
            builds::show_item,
            builds::set_gear_goal,
            builds::gear_suggestions,
            live::read_character_windows,
            builds::list_builds,
            builds::build_plan,
            builds::active_build,
            builds::save_build,
            builds::use_build,
            builds::delete_build,
            builds::export_build,
            builds::import_build,
            builds::build_item_fit,
            builds::open_guide,
            builds::show_build,
            settings::get_settings,
            settings::update_settings,
            settings::show_settings,
            mapdata::open_link,
            mapdata::item_origin,
            mapdata::search_quests,
            mapdata::quest_regions,
            mapdata::quest_sheet,
            mapdata::show_quest,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Leafline");
}
