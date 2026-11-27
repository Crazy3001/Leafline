//! One module per piece of game UI. Each locates its element in a frame by appearance,
//! reads it with OCR and parses the text into structured data.

pub mod character;
pub mod charinfo;
pub mod counts;
mod digits;
pub(crate) mod gamefont;
pub mod hud;
pub mod inventory;
pub mod map;
pub mod mesos;
pub mod npc;
pub mod party;
pub mod pickup;
pub mod quest;
pub(crate) mod questfont;
pub mod questlog;
pub mod quickslot;
pub mod shop;
pub mod text;
pub mod tooltip;
pub mod ui;
