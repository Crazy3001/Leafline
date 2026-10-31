//! The build planner: a player's ordered goals for a job, laid out level by level.
//!
//! Nothing here models the game's damage, accuracy or HP formulas. It only counts the AP and SP
//! a character earns and where the plan puts them, by rules the player can correct. Everything
//! is information the player acts on by hand.

pub mod catalog;
pub mod fit;
pub mod plan;
pub mod rules;
pub mod share;

pub use catalog::{Catalog, JobChoice};
pub use fit::{ItemFit, ItemJob};
pub use plan::{Build, Expansion, Plan, SkillLevel, Start, Stat, StatGoal, Stats, expand};
pub use rules::{Branch, Rules};
