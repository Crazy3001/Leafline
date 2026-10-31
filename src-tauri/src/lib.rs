//! Leafline: a read-only overlay for MapleStory Classic.
//!
//! It only reads what is on screen (screenshots and OCR). It never touches game memory,
//! files or network traffic, and never sends input to the game.

mod app;
pub mod build;
pub mod capture;
pub mod error;
pub mod mapdata;
pub mod pipeline;
pub mod readers;
pub mod replay;
pub mod session;
pub mod userdata;
pub mod vision;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    app::run();
}
