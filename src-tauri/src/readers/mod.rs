//! One module per piece of game UI. Each locates its element in a frame by appearance,
//! reads it with OCR and parses the text into structured data.

pub mod hud;
pub mod map;
pub mod mesos;
pub mod quest;
mod text;
pub mod tooltip;
