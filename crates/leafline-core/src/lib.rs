//! What Leafline's Windows program and its website share: map data and routes, the build
//! planner, the player's records, the readers and the session tracker.
//!
//! It only reads what is on screen (screenshots and OCR). It never touches game memory, files or
//! network traffic, and never sends input to the game.

pub mod build;
pub mod capture;
pub mod clock;
pub mod error;
pub mod loot;
pub mod mapdata;
pub mod pipeline;
pub mod questlog;
pub mod readers;
pub mod replay;
pub mod service;
pub mod session;
pub mod userdata;
pub mod vision;
