//! The tray icon. The overlay has no taskbar entry, so this is how it is reached when hidden
//! or locked.

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{App, Manager};

use super::overlay::OverlayState;
use super::shortcuts::{Action, log_error};

const QUIT: &str = "quit";
const SETTINGS: &str = "settings";
const DESK: &str = "desk";

pub fn create(app: &App) -> tauri::Result<()> {
    let item = |action: Action, text: &str| MenuItem::with_id(app, action_id(action), text, true, None::<&str>);
    let click_through = CheckMenuItem::with_id(
        app,
        action_id(Action::ToggleClickThrough),
        "Click-through",
        true,
        app.state::<OverlayState>().click_through(),
        None::<&str>,
    )?;
    let menu = Menu::with_items(
        app,
        &[
            &item(Action::ToggleOverlay, "Show / hide overlay")?,
            &click_through,
            &item(Action::TogglePins, "Show / hide pinned panels")?,
            &MenuItem::with_id(app, DESK, "Desk view", true, None::<&str>)?,
            &item(Action::ToggleLive, "Start / stop live reading")?,
            &MenuItem::with_id(app, SETTINGS, "Settings…", true, None::<&str>)?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, QUIT, "Quit Leafline", true, None::<&str>)?,
        ],
    )?;
    if Action::ShowWorkbench.available() {
        menu.insert(&item(Action::ShowWorkbench, "Open workbench")?, 5)?;
    }
    let _ = app.state::<OverlayState>().click_through_item.set(click_through);

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("bundle has an icon"))
        .tooltip("Leafline")
        .menu(&menu)
        .on_menu_event(|app, event| {
            if event.id() == QUIT {
                app.exit(0);
            } else if event.id() == DESK {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(err) = super::desk::show_desk(app).await {
                        log_error(&err);
                    }
                });
            } else if event.id() == SETTINGS {
                if let Err(err) = super::settings::open(app) {
                    log_error(&err);
                }
            } else if let Some(action) = Action::ALL.into_iter().find(|a| event.id() == action_id(*a).as_str())
                && let Err(err) = action.run(app)
            {
                log_error(&err);
            }
        })
        .build(app)?;
    Ok(())
}

fn action_id(action: Action) -> String {
    format!("{action:?}")
}
