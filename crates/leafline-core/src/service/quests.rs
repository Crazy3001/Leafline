//! Each character's quest log, as the game's Quest window last showed it, and the quest totals.

use super::host::{Host, emit};
use crate::error::Result;
use crate::mapdata::questlog::RegionTotal;
use crate::userdata::questlog::QuestLog;

/// Emitted to every window when a character's quest log changes, with the character's name.
pub const QUEST_LOG_EVENT: &str = "quest-log";

/// A character's quest log as last seen, or the last one read's.
pub fn quest_log(h: &dyn Host, name: Option<String>) -> Result<QuestLog> {
    h.records().with(|db| db.quest_log(name.as_deref()))
}

/// Forgets a character's quest log; the next look at the Quest window starts it again.
pub fn forget_quest_log(h: &dyn Host, name: String) -> Result<()> {
    h.records().with(|db| db.forget_quest_log(&name))?;
    emit(h, QUEST_LOG_EVENT, &name);
    Ok(())
}

/// How many quests the data files under each region.
pub fn quest_region_totals(h: &dyn Host) -> Vec<RegionTotal> {
    h.map().atlas().map(|atlas| atlas.quest_region_totals()).unwrap_or_default()
}
