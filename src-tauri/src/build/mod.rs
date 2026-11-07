//! The build planner: a player's ordered goals for a job, laid out level by level.
//!
//! Nothing here models the game's damage, accuracy or HP formulas. It only counts the AP and SP
//! a character earns and where the plan puts them, by rules the player can correct. Everything
//! is information the player acts on by hand.

pub mod catalog;
pub mod fit;
pub mod outfit;
pub mod plan;
pub mod progress;
pub mod rules;
pub mod share;
pub mod starters;
pub mod suggest;

pub use catalog::{Catalog, ItemFilters, ItemPage, ItemQuery, JobChoice};
pub use fit::{ItemFit, ItemJob};
pub use outfit::{Outfit, outfit};
pub use plan::{Build, Expansion, Plan, SkillLevel, Start, Stat, StatGoal, Stats, expand};
pub use rules::{Branch, Rules};
pub use starters::Starter;
pub use suggest::{GearFor, GearFound, GearSuggestions, Wearer};
