//! Map, monster, NPC and quest facts for the map details card and the lookups, and skill and
//! equipment facts for the build planner.
//!
//! The facts come from OSMS Data Explorer's public JSON (CC BY 4.0), which is extracted from the
//! Classic client. Leafline downloads it only after the user asks, keeps it in a local SQLite
//! cache and credits it wherever it is shown; none of it ships in the repo or the installer.
//! Image fields and map coordinates are never read. Drops come separately from
//! mapleclassic.wiki, where players log them.

pub mod atlas;
pub mod bestiary;
pub mod drops;
pub mod fetch;
pub mod guide;
pub mod osms;
pub mod places;
pub mod questbook;
pub mod store;
pub mod training;
pub mod travel;

pub use atlas::{Atlas, MapCard, MapMatch, MapRef, Route};
pub use bestiary::{MonsterFilters, MonsterPage, MonsterQuery, MonsterRow, MonsterSheet};
pub use drops::{ItemSources, MonsterDrops};
pub use fetch::RefreshReport;
pub use guide::{ItemOrigin, QuestGuide, QuestQuery, QuestTitles};
pub use places::{MapPage, MapQuery, MapRow, NpcPage, NpcQuery, NpcRow, NpcSheet};
pub use questbook::{QuestPage, QuestRow, QuestSearch, QuestSheet};
pub use store::{DataStatus, Store};
pub use training::{MeasuredRate, TrainingSpot};
pub use travel::{Travel, TravelChoice, TravelFile};

/// The credit OSMS asks for, shown with any data from it.
pub const OSMS_CREDIT: &str = "Data from OSMS Data Explorer (https://osmsdataexplorer.com), CC BY 4.0";
