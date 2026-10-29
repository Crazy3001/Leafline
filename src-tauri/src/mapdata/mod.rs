//! Map, monster, NPC and quest facts for the map details card.
//!
//! The facts come from OSMS Data Explorer's public JSON (CC BY 4.0), which is extracted from the
//! Classic client. Leafline downloads it only after the user asks, keeps it in a local SQLite
//! cache and credits it wherever it is shown; none of it ships in the repo or the installer.
//! Image fields and map coordinates are never read. Drops come separately from
//! mapleclassic.wiki, where players log them. See `docs/map-data-sources.md`.

pub mod atlas;
pub mod drops;
pub mod fetch;
pub mod osms;
pub mod store;

pub use atlas::{Atlas, MapCard, MapMatch, MapRef, Route};
pub use drops::MonsterDrops;
pub use fetch::RefreshReport;
pub use store::{DataStatus, Store};

/// The credit OSMS asks for, shown with any data from it.
pub const OSMS_CREDIT: &str = "Data from OSMS Data Explorer (https://osmsdataexplorer.com), CC BY 4.0";
