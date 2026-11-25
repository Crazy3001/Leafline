//! The game's Quest window, as read from live pictures, kept as each character's quest log: the
//! program's live loop and the website's live reading both hand their readings here. Each tab is
//! put together from the views the player scrolls through.
//!
//! The website reads the window with ocrs instead of Windows OCR, and keeps it strictly
//! ([`Strictness::Matched`]): only rows whose titles settle on a quest in the data are kept; a tab
//! is taken as whole (so quests missing from it are marked gone) only when every bar and row in
//! every view read and settled and every bar's count matches the rows under it; region counts are
//! kept only from such a tab; and the detail pane, whose numbers ocrs misreads, isn't kept.

use std::collections::HashMap;

use super::host::{Host, emit};
use super::quests::QUEST_LOG_EVENT;
use crate::clock::now;
use crate::error::Result;
use crate::mapdata::Atlas;
use crate::questlog::Gathered;
use crate::readers::questlog::{LogItem, LogQuest, QuestDetail, QuestLogReading, QuestTab, RowIcon};
use crate::readers::text::squash;
use crate::userdata::questlog::SeenQuest;

/// How much of what was read is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Strictness {
    /// Everything read, as the program keeps it: a title that fits no quest is kept as read.
    #[default]
    AsRead,
    /// Only what settles on the data (the website's).
    Matched,
}

/// The views of each tab seen since the window was opened, for one character.
#[derive(Debug, Default)]
pub struct QuestWindow {
    tabs: HashMap<QuestTab, Gathered>,
    /// Tabs with a row left out because its title didn't settle.
    gaps: HashMap<QuestTab, bool>,
    character: Option<String>,
    strictness: Strictness,
}

impl QuestWindow {
    pub fn new(strictness: Strictness) -> Self {
        Self { strictness, ..Self::default() }
    }

    /// The window was closed: the next look starts each tab afresh.
    pub fn closed(&mut self) {
        self.tabs.clear();
        self.gaps.clear();
    }

    /// Keeps one reading of the window, for the character `name`, and tells the windows.
    pub fn keep(&mut self, h: &dyn Host, name: &str, reading: &QuestLogReading) -> Result<()> {
        if self.character.as_deref() != Some(name) {
            self.closed();
            self.character = Some(name.to_string());
        }
        let atlas = h.map().atlas();
        let atlas = atlas.as_deref();
        let strict = self.strictness == Strictness::Matched;
        let items: Vec<LogItem> = if strict { matched_items(atlas, &reading.items) } else { reading.items.clone() };
        let gap = self.gaps.entry(reading.tab).or_default();
        *gap |= items.len() < reading.items.len() || reading.unread > 0;
        let gap = *gap;
        let gathered = self.tabs.entry(reading.tab).or_default();
        gathered.add(&items, reading.scroll);
        let seen: Vec<SeenQuest> = gathered
            .quests()
            .iter()
            .map(|g| seen_quest(atlas, &g.quest, g.region.as_deref(), strict))
            .filter(|s| !strict || s.quest_id.is_some())
            .collect();
        let mut regions = gathered.regions();
        let whole = if strict {
            let counted = regions.iter().all(|(_, count)| count.is_some_and(|n| n > 0));
            let whole = gathered.whole() && !gap && counted;
            if !whole {
                regions.clear();
            }
            whole
        } else {
            gathered.whole()
        };
        let detail = reading
            .detail
            .as_ref()
            .filter(|_| !strict)
            .and_then(|detail| Some((detail, detail_quest(atlas, reading, &seen, detail)?)));
        let at = now();
        h.records().with(|db| {
            db.record_quest_tab(name, reading.tab, &seen, &regions, whole, at)?;
            if let Some((detail, quest)) = &detail {
                db.record_quest_detail(name, quest, detail, at)?;
            }
            Ok(())
        })?;
        emit(h, QUEST_LOG_EVENT, name);
        Ok(())
    }
}

/// The rows of a strict reading worth keeping: quests whose title settles on one in the data (in
/// the region above them, where that settles), and region bars whose name settles on a region.
fn matched_items(atlas: Option<&Atlas>, items: &[LogItem]) -> Vec<LogItem> {
    let Some(atlas) = atlas else { return Vec::new() };
    let mut region: Option<String> = None;
    items
        .iter()
        .filter_map(|item| match item {
            LogItem::Region { name, count } => {
                region = atlas.quest_region_like(name);
                region.clone().map(|name| LogItem::Region { name, count: *count })
            }
            LogItem::Quest(quest) => {
                let readings: Vec<&str> =
                    std::iter::once(quest.title.as_str()).chain(quest.readings.iter().map(String::as_str)).collect();
                let id = atlas.log_quest(&readings, quest.cut, region.as_deref())?;
                let title = atlas.quest_name(id)?;
                Some(LogItem::Quest(LogQuest { title, readings: Vec::new(), cut: false, ..quest.clone() }))
            }
        })
        .collect()
}

/// A row ready to keep: matched to the data when its title fits a quest there, and then named as
/// the data names it, since the game may have cut it short.
fn seen_quest(atlas: Option<&Atlas>, quest: &LogQuest, region: Option<&str>, strict: bool) -> SeenQuest {
    let readings: Vec<&str> = std::iter::once(quest.title.as_str()).chain(quest.readings.iter().map(String::as_str)).collect();
    let id = atlas.and_then(|a| a.log_quest(&readings, quest.cut, region));
    let data_name = id.and_then(|id| atlas?.quest_name(id));
    let region_like = |r: &str| atlas.and_then(|a| a.quest_region_like(r));
    SeenQuest {
        title: data_name.clone().unwrap_or_else(|| quest.title.clone()),
        cut: quest.cut && data_name.is_none(),
        quest_id: id,
        region: region.and_then(|r| if strict { region_like(r) } else { Some(region_like(r).unwrap_or_else(|| r.to_string())) }),
        icon: quest.icon,
        on_helper: quest.on_helper,
    }
}

/// Which quest the detail pane shows: the picked row's, or else the one whose title its own
/// (often cut short) starts, or else the data's quest of that title.
fn detail_quest(atlas: Option<&Atlas>, reading: &QuestLogReading, seen: &[SeenQuest], detail: &QuestDetail) -> Option<SeenQuest> {
    let picked = reading.items.iter().find_map(|item| match item {
        LogItem::Quest(q) if q.selected => Some(q),
        _ => None,
    });
    if let Some(picked) = picked {
        let key = squash(&picked.title);
        if let Some(found) = seen.iter().find(|s| {
            squash(&s.title) == key
                || s.quest_id.is_some_and(|id| atlas.and_then(|a| a.log_quest(&[picked.title.as_str()], picked.cut, None)) == Some(id))
        }) {
            return Some(found.clone());
        }
    }
    let start = squash(&detail.title);
    let starting: Vec<&SeenQuest> = seen.iter().filter(|s| start.len() >= 6 && squash(&s.title).starts_with(&start)).collect();
    if let [one] = starting[..] {
        return Some(one.clone());
    }
    let id = atlas?.log_quest(&[detail.title.as_str()], detail.cut, None)?;
    Some(SeenQuest {
        title: atlas?.quest_name(id)?,
        cut: false,
        quest_id: Some(id),
        region: None,
        icon: RowIcon::Unknown,
        on_helper: None,
    })
}
