//! The tray icon. The overlay has no taskbar entry, so this is how it is reached when hidden
//! or locked.

use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{App, Manager};

use super::overlay::OverlayState;
use super::shortcuts::{Action, log_error};

const QUIT: &str = "quit";

pub fn create(app: &App) -> tauri::Result<()> {
    let item = |action: Action, text: &str| MenuItem::with_id(app, action_id(action), text, true, Some(action.keys()));
    let click_through = CheckMenuItem::with_id(
        app,
        action_id(Action::ToggleClickThrough),
        "Click-through",
        true,
        false,
        Some(Action::ToggleClickThrough.keys()),
    )?;
    let menu = Menu::with_items(
        app,
        &[
            &item(Action::ToggleOverlay, "Show / hide overlay")?,
            &click_through,
            &item(Action::TogglePins, "Show / hide pinned panels")?,
            &item(Action::ShowWorkbench, "Open workbench")?,
            &PredefinedMenuItem::separator(app)?,
            &MenuItem::with_id(app, QUIT, "Quit Leafline", true, None::<&str>)?,
        ],
    )?;
    let _ = app.state::<OverlayState>().click_through_item.set(click_through);

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().cloned().expect("bundle has an icon"))
        .tooltip("Leafline")
        .menu(&menu)
        .on_menu_event(|app, event| {
            if event.id() == QUIT {
                app.exit(0);
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
