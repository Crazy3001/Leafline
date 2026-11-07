//! Tauri wiring: windows, tray, hotkeys and commands.

mod builds;
mod commands;
mod desk;
mod layout;
mod live;
mod mapdata;
mod nav;
mod overlay;
mod pins;
mod readings;
mod records;
mod report;
mod settings;
mod shortcuts;
mod snap;
mod tray;
mod update;

use tauri::{Manager, WindowEvent};

/// WebView2 options for every Leafline window. All windows share one browser, which refuses a
/// window asking for different ones, so the overlay's entry in tauri.conf.json carries the same
/// string (a test checks). The first feature list is wry's default, kept. Leafline's windows sit
/// over a game that has focus, and Edge would take them for hidden and stop drawing them until
/// clicked: occlusion detection and background throttling are turned off for that.
pub const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection,CalculateNativeWinOcclusion \
     --disable-background-timer-throttling --disable-renderer-backgrounding --disable-backgrounding-occluded-windows";

use desk::DeskState;
use live::LiveState;
use mapdata::MapDataState;
use nav::NavState;
use overlay::{OVERLAY, OverlayState};
use pins::PinState;
use readings::Readings;
use records::UserDataState;
use settings::SettingsState;
use shortcuts::Bindings;
use update::UpdateState;

pub fn run() {
    tauri::Builder::default()
        // Starting Leafline again while it runs brings the overlay up.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Err(err) = overlay::show(app) {
                shortcuts::log_error(&err);
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::new().with_state_flags(layout::SAVED).build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(shortcuts::plugin())
        .manage(OverlayState::default())
        .manage(LiveState::default())
        .manage(Readings::default())
        .manage(Bindings::default())
        .manage(UpdateState::default())
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
            app.manage(NavState::load(app.handle())?);
            app.manage(PinState::load(app.handle())?);
            pins::restore(app.handle())?;
            app.manage(DeskState::load(app.handle())?);
            desk::restore(app.handle())?;
            tray::create(app)?;
            update::check_at_startup(app.handle());
            // Setup comes first on a new install. After it, live reading starts with Leafline and
            // waits for the game window if the game isn't open.
            let settings = app.state::<SettingsState>().get();
            if !settings.setup_done {
                overlay::fit_first_launch(app.handle())?;
                settings::open_setup(app.handle())?;
            } else if settings.live.enabled
                && let Err(err) = live::start(app.handle())
            {
                shortcuts::log_error(&err);
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // Closing a window hides it; the app keeps running in the tray until Quit.
            WindowEvent::CloseRequested { api, .. } if window.label() == OVERLAY => {
                api.prevent_close();
                let _ = window.hide();
            }
            WindowEvent::CloseRequested { .. } => desk::closed(window),
            WindowEvent::Focused(true) => settings::focused(window),
            WindowEvent::Moved(_) => layout::changed(window.app_handle()),
            WindowEvent::Resized(size) => {
                pins::resized(window, *size);
                layout::changed(window.app_handle());
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            live::live_status,
            live::start_live,
            live::stop_live,
            live::live_windows,
            live::save_live_frame,
            live::set_live_held,
            live::reset_session,
            commands::overlay_status,
            commands::set_click_through,
            commands::set_compact,
            commands::hide_overlay,
            commands::toggle_overlay,
            commands::shortcut_list,
            commands::open_data_folder,
            commands::frames_folder,
            commands::open_frames_folder,
            shortcuts::set_hotkeys_quiet,
            report::diagnostics,
            readings::current_readings,
            pins::pin_status,
            pins::pin_panel,
            pins::unpin_panel,
            pins::toggle_pins,
            pins::show_panel,
            desk::desk_status,
            desk::show_desk,
            desk::set_desk_on_top,
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
            mapdata::drops_fetched_at,
            mapdata::training_spots,
            records::known_mesos,
            records::item_note,
            records::saved_spots,
            records::set_spot_saved,
            update::update_status,
            update::check_for_update,
            records::set_item_note,
            records::map_note,
            records::set_map_note,
            records::drop_log,
            records::log_drop,
            records::delete_drop,
            records::save_image,
            records::session_history,
            records::session_timeline,
            records::show_training,
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
            builds::add_piece,
            builds::remove_piece,
            builds::update_piece,
            builds::wear_item,
            builds::build_outfit,
            builds::fill_gear,
            builds::gear_suggestions,
            builds::find_gear,
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
            settings::set_typing,
            settings::show_settings,
            settings::show_setup,
            settings::finish_setup,
            mapdata::open_link,
            mapdata::item_origin,
            mapdata::search_quests,
            mapdata::quest_regions,
            mapdata::quest_sheet,
            mapdata::show_quest,
            mapdata::search_monsters,
            mapdata::monster_filters,
            mapdata::find_monster,
            mapdata::monster_sheet,
            mapdata::show_monster,
            mapdata::browse_maps,
            mapdata::map_regions,
            mapdata::map_areas,
            mapdata::search_npcs,
            mapdata::find_npc,
            mapdata::npc_sheet,
            mapdata::show_place,
            nav::destination,
            nav::set_destination,
            nav::arrive_at,
            nav::forget_destination,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Leafline");
}

#[cfg(test)]
mod tests {
    use super::BROWSER_ARGS;

    #[test]
    fn the_overlay_asks_for_the_same_browser_as_every_other_window() {
        let config: serde_json::Value = serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
        for window in config["app"]["windows"].as_array().unwrap() {
            assert_eq!(window["additionalBrowserArgs"].as_str(), Some(BROWSER_ARGS), "{}", window["label"]);
        }
    }
}
