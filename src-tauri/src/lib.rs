//! Leafline: a read-only overlay for MapleStory Classic.
//!
//! It only reads what is on screen (screenshots and OCR). It never touches game memory,
//! files or network traffic, and never sends input to the game.
//!
//! Everything but the windows, tray, hotkeys and live capture loop lives in `leafline-core`,
//! which the website shares; it is re-exported here under the old paths.

mod app;

pub use leafline_core::{
    build, capture, capturetest, clock, error, loot, mapdata, pipeline, questlog, readers, replay, service, session, userdata,
    vision,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app::run();
}
