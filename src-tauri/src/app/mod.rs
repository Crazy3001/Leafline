//! Tauri wiring: windows, tray, hotkeys and commands.

mod bag;
mod builds;
mod capturetest;
mod cards;
mod commands;
mod damage;
mod desk;
mod fighter;
mod generated;
mod host;
mod hover;
mod layout;
mod live;
mod mapdata;
mod nav;
mod overlay;
mod pictures;
mod pins;
mod questlog;
mod readings;
mod recovery;
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
use nav::NavState;
use overlay::{OVERLAY, OverlayState};
use pins::PinState;
use readings::Readings;
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
        .manage(cards::CardState::default())
        .manage(LiveState::default())
        .manage(capturetest::RecorderState::default())
        .manage(bag::BagState::default())
        .manage(Readings::default())
        .manage(Bindings::default())
        .manage(UpdateState::default())
        .register_asynchronous_uri_scheme_protocol(pictures::SCHEME, pictures::handle)
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let folder = leafline_core::service::files::Folder(dir);
            app.manage(damage::DamageState::load(&folder));
            app.manage(SettingsState::load(&folder));
            app.manage(NavState::load(&folder));
            app.manage(host::DataFolder(folder));
            app.manage(mapdata::load(app.handle())?);
            app.manage(pictures::PictureState::load(app.handle())?);
            app.manage(records::load(app.handle())?);
            mapdata::check_for_updates(app.handle());
            settings::apply(app.handle())?;
            overlay::restore_mode(app.handle())?;
            if let Some(overlay) = app.get_webview_window(OVERLAY) {
                snap::attach(&overlay)?;
            }
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
            live::reader_sight,
            capturetest::open_capture_test,
            capturetest::open_capture_pattern,
            capturetest::capture_pattern,
            capturetest::check_capture_pattern,
            capturetest::start_capture_recording,
            capturetest::stop_capture_recording,
            capturetest::capture_recording_status,
            capturetest::compare_capture_frames,
            bag::bag_look_further,
            bag::last_bag,
            live::start_live,
            live::stop_live,
            live::live_windows,
            live::save_live_frame,
            live::save_item_picture,
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
            cards::pop_out_card,
            cards::show_card,
            cards::close_live_card,
            cards::card_is_live,
            cards::pin_card,
            pins::unpin_panel,
            pins::toggle_pins,
            pins::show_panel,
            generated::damage::player_damage,
            generated::sheet::character_stats,
            generated::sheet::set_buff,
            desk::desk_status,
            desk::desk_take_tab,
            desk::show_desk,
            desk::set_desk_on_top,
            generated::mapdata::map_data_status,
            generated::mapdata::download_map_data,
            generated::mapdata::find_map,
            generated::mapdata::map_card,
            generated::mapdata::search_maps,
            generated::mapdata::map_route,
            generated::mapdata::travel_info,
            generated::mapdata::download_travel,
            mapdata::edit_travel_file,
            generated::mapdata::monster_drops,
            generated::mapdata::quest_guide,
            generated::mapdata::download_drops,
            generated::mapdata::drops_fetched_at,
            generated::mapdata::training_spots,
            generated::mapdata::check_estimates,
            generated::records::known_character,
            generated::records::known_bag,
            generated::records::item_note,
            generated::records::saved_spots,
            generated::records::set_spot_saved,
            generated::records::potion_slots,
            generated::records::loot_prices,
            generated::records::set_potion_slots,
            generated::records::session_potions,
            generated::records::set_session_potion,
            update::update_status,
            update::check_for_update,
            update::install_update,
            generated::records::set_item_note,
            generated::records::map_note,
            generated::records::set_map_note,
            generated::records::drop_log,
            generated::records::log_drop,
            generated::records::delete_drop,
            records::save_image,
            generated::records::session_history,
            generated::records::level_exp,
            generated::builds::item_crafting,
            generated::builds::crafting_recipes,
            generated::records::session_timeline,
            records::show_training,
            generated::records::delete_session,
            generated::records::set_session_note,
            generated::records::rename_map,
            records::export_csv,
            records::export_records,
            generated::typed::typed_session_outcome,
            generated::typed::save_typed_session,
            records::check_records,
            records::list_backups,
            records::check_backup,
            generated::records::records_summary,
            records::import_records,
            generated::builds::build_catalog,
            generated::builds::starter_plans,
            generated::builds::search_gear,
            generated::builds::search_items,
            generated::builds::item_filters,
            generated::builds::item_record,
            generated::builds::find_item,
            builds::show_item,
            generated::builds::set_gear_goal,
            generated::builds::add_piece,
            generated::builds::remove_piece,
            generated::builds::update_piece,
            generated::builds::wear_item,
            generated::builds::build_outfit,
            generated::builds::fill_gear,
            generated::builds::gear_suggestions,
            generated::builds::find_gear,
            live::read_character_windows,
            generated::builds::list_builds,
            generated::builds::build_plan,
            generated::builds::active_build,
            generated::builds::save_build,
            generated::builds::use_build,
            generated::builds::delete_build,
            generated::builds::export_build,
            generated::builds::import_build,
            generated::builds::build_item_fit,
            builds::open_guide,
            builds::show_build,
            settings::get_settings,
            settings::update_settings,
            settings::set_typing,
            settings::show_settings,
            settings::show_setup,
            settings::finish_setup,
            mapdata::open_link,
            generated::mapdata::item_origin,
            generated::mapdata::search_quests,
            generated::mapdata::quest_regions,
            generated::questlog::quest_log,
            generated::questlog::forget_quest_log,
            generated::questlog::quest_region_totals,
            generated::mapdata::quest_sheet,
            mapdata::show_quest,
            generated::mapdata::search_monsters,
            generated::mapdata::monster_filters,
            generated::mapdata::find_monster,
            generated::mapdata::monster_sheet,
            mapdata::show_monster,
            generated::mapdata::browse_maps,
            generated::mapdata::monster_fight,
            generated::mapdata::shop_prices,
            generated::mapdata::map_regions,
            generated::mapdata::map_areas,
            generated::mapdata::search_npcs,
            generated::mapdata::find_npc,
            generated::mapdata::npc_sheet,
            mapdata::show_place,
            pictures::map_picture,
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
