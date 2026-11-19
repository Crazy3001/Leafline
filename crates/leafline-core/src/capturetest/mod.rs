//! The capture test (website plan, stage 2): whether the website may read the game from a window
//! shared to Chrome or Edge. A go/no-go check, not a player feature.
//!
//! - [`pattern`] and [`measure`], part A: a test pattern the program shows in a window of its
//!   own, and what a capture did to it, measured on the website against the same pattern drawn
//!   again.
//! - [`agree`], part B: the program's and the website's frames of the real game, saved side by
//!   side to a folder the player picks, read by the same readers and compared.
//! - [`readers`], part C: the readers the website could run today, timed frame by frame while
//!   the page sits behind the game.
//!
//! Frames stay on the PC: nothing here sends anything anywhere.

pub mod agree;
pub mod measure;
pub mod pattern;
pub mod readers;
