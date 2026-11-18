//! Leafline's website: the program's shared code run in the browser, in a dedicated Web Worker.
//!
//! The page calls [`call`] with the same command names and camelCase arguments the program's
//! windows pass to Tauri, and gets back the value and any events (the program's Tauri events),
//! which the page passes on to every open tab. The player's records, the map data cache and the
//! small settings files are SQLite files in the browser's private storage for this site (OPFS),
//! on this computer only. Nothing the player records is sent anywhere.

pub mod reply;

#[cfg(target_arch = "wasm32")]
mod files;
#[cfg(target_arch = "wasm32")]
mod generated;
#[cfg(target_arch = "wasm32")]
mod web;
#[cfg(target_arch = "wasm32")]
mod xhr;

#[cfg(target_arch = "wasm32")]
pub use web::*;
