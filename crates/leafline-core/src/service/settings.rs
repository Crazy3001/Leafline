//! The player's settings, shared by every window of the program and every tab of the website.
//! Settings only one of them uses (hotkeys, live reading, the overlay's scale) are kept by both,
//! so a records file or a later program install finds them as they were.

use std::collections::BTreeMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::files::{Files, read_json, write_json};
use crate::build::Rules;
use crate::capture::WindowChoice;
use crate::error::Result;
use crate::mapdata::TravelChoice;

pub const FILE_NAME: &str = "settings.json";

/// Emitted to every window when the settings change.
pub const SETTINGS_EVENT: &str = "settings";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    /// How opaque the overlay and pinned panels are, 0.4 to 1.
    pub opacity: f64,
    /// How large their text and controls are drawn, 0.6 to 2.
    pub scale: f64,
    /// Keep the overlay and pinned panels above the game.
    pub always_on_top: bool,
    /// Clicking the overlay or a pinned panel leaves the game focused, so play goes on without
    /// clicking back into it. Typing in a box still takes focus while the box is in use.
    pub keep_game_focus: bool,
    pub compact_shape: CompactShape,
    pub hotkeys: HotkeyChoice,
    /// Taxis and return scrolls in routes. Off until the player turns it on.
    pub travel: TravelChoice,
    pub live: LiveChoice,
    /// How points are earned, as the build planner counts them. The player can correct these.
    pub build_rules: Rules,
    /// Where saved game pictures go. None means the app's local folder, which never syncs.
    pub frames_folder: Option<String>,
    /// The first-launch setup has been finished. Until then it opens at every start.
    pub setup_done: bool,
    /// Ask GitHub for the latest version number at startup. Off until the player turns it on, in
    /// setup or Settings; sends nothing about the game or the player (see `update.rs`).
    pub update_check: bool,
    /// Show item, monster, NPC and map pictures, downloaded from OSMS the first time each is
    /// shown. Off, nothing is downloaded and names stand alone.
    pub show_pictures: bool,
    /// The look every window is drawn in.
    pub theme: Theme,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            scale: 1.0,
            always_on_top: true,
            keep_game_focus: true,
            compact_shape: CompactShape::default(),
            hotkeys: HotkeyChoice::default(),
            travel: TravelChoice::default(),
            live: LiveChoice::default(),
            build_rules: Rules::default(),
            frames_folder: None,
            setup_done: false,
            update_check: false,
            show_pictures: true,
            theme: Theme::default(),
        }
    }
}

/// The four looks. One layout; a look changes colours, fonts, corners and outlines only.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    /// Dark graphite and amber.
    #[default]
    EmberDark,
    /// Ember on warm stone, with the header and tab bar kept dark.
    EmberLight,
    /// Light and playful: cream, ink outlines, rounded letters.
    SproutLight,
    /// The same stickers on plum.
    SproutDark,
}

/// How the compact HUD was laid out. Since 2026-11-05 it has one layout and this is no longer
/// offered; it stays so settings files that name a shape still load.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompactShape {
    /// A narrow card, with the quest counters under the numbers.
    #[default]
    Card,
    /// Longer sideways, with a row of four numbers.
    Wide,
    /// One thin strip.
    Bar,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HotkeyChoice {
    pub enabled: bool,
    /// Keys the player changed, by action. Empty turns that hotkey off.
    pub keys: BTreeMap<String, String>,
}

impl Default for HotkeyChoice {
    fn default() -> Self {
        Self { enabled: true, keys: BTreeMap::new() }
    }
}

/// Live reading of the game window. On from the start (decided 2026-11-04); setup says in one line
/// that Leafline is unofficial, and the player can turn it off.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LiveChoice {
    pub enabled: bool,
    /// Seconds between readings, 1 to 10.
    pub interval: f64,
    /// Seconds without EXP before the clock pauses, 5 to 300.
    pub pause_after: f64,
    /// The window the player picked, for when Leafline can't tell which one is the game.
    pub window: Option<WindowChoice>,
    /// The Pickup Log is read between readings, for the loot picked up.
    pub pickup_log: bool,
    /// The website reads an item tooltip the mouse rests on and shows its card (the program reads
    /// them on its hotkey instead).
    pub item_tooltips: bool,
    /// The website updates the build from the Character Stat and Skill windows while they're
    /// open (the program follows them always, and reads them on its hotkey).
    pub character_windows: bool,
    /// The website checks the inventory tab that's open, for sell or keep (the program, on its
    /// hotkey).
    pub bag_check: bool,
    /// The website keeps the quest log from the Quest window while it's open.
    pub quest_window: bool,
}

impl Default for LiveChoice {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: 2.0,
            pause_after: 15.0,
            window: None,
            pickup_log: true,
            item_tooltips: true,
            character_windows: true,
            bag_check: true,
            quest_window: true,
        }
    }
}

impl Settings {
    pub fn clamped(self) -> Self {
        let finite = |v: f64, fallback: f64| if v.is_finite() { v } else { fallback };
        Self {
            opacity: finite(self.opacity, 1.0).clamp(0.4, 1.0),
            scale: finite(self.scale, 1.0).clamp(0.6, 2.0),
            live: LiveChoice {
                interval: finite(self.live.interval, 2.0).clamp(1.0, 10.0),
                pause_after: finite(self.live.pause_after, 15.0).clamp(5.0, 300.0),
                ..self.live
            },
            build_rules: self.build_rules.clamped(),
            ..self
        }
    }
}

pub struct SettingsState {
    current: Mutex<Settings>,
}

impl SettingsState {
    /// Loads the saved settings. A missing or unreadable file means the defaults.
    pub fn load(files: &dyn Files) -> Self {
        let current = read_json::<Settings>(files, FILE_NAME).unwrap_or_default().clamped();
        Self { current: Mutex::new(current) }
    }

    pub fn get(&self) -> Settings {
        self.current.lock().expect("settings lock").clone()
    }

    /// Keeps and saves new settings, within their limits, and returns the ones before and after.
    pub fn replace(&self, files: &dyn Files, settings: Settings) -> Result<(Settings, Settings)> {
        let settings = settings.clamped();
        let old = std::mem::replace(&mut *self.current.lock().expect("settings lock"), settings.clone());
        write_json(files, FILE_NAME, &settings)?;
        Ok((old, settings))
    }
}
