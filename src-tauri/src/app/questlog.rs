//! The game's Quest window, read from live pictures while the player has it open, kept as each
//! character's quest log. Nothing is read with the window shut, and nothing read leaves the PC.

use std::collections::HashMap;

use tauri::{AppHandle, Emitter, Manager, State};

use super::mapdata::MapDataState;
use super::records::{self, UserDataState};
use super::report;
use crate::error::Result;
use crate::mapdata::Atlas;
use crate::mapdata::questlog::RegionTotal;
use crate::pipeline::Analysis;
use crate::questlog::Gathered;
use crate::readers::questlog::{self, LogItem, LogQuest, QuestDetail, QuestLogReading, QuestTab, Watch};
use crate::readers::text::squash;
use crate::readers::ui;
use crate::userdata::questlog::{QuestLog, SeenQuest};
use crate::vision::{Frame, Ocr};

/// Emitted to every window when a character's quest log changes, with the character's name.
pub const QUEST_LOG_EVENT: &str = "quest-log";

/// Pictures in a row without the window before it counts as closed, and the next opening as a
/// fresh look. One missed picture doesn't close it.
const CLOSED_AFTER: u32 = 3;

/// Follows the Quest window across live pictures: reads it once a change has settled, and puts
/// each tab together from the views the player scrolls through.
#[derive(Default)]
pub struct QuestLogFollow {
    watch: Watch,
    tabs: HashMap<QuestTab, Gathered>,
    character: Option<String>,
    closed: u32,
}

impl QuestLogFollow {
    pub fn follow(&mut self, app: &AppHandle, frame: &Frame, analysis: &Analysis, ocr: &Ocr) {
        let scale = analysis.hud_layout.as_ref().map(|layout| ui::hud_scale(frame, layout));
        let glance = questlog::glance(frame, scale);
        if glance.is_none() {
            self.closed += 1;
            if self.closed >= CLOSED_AFTER {
                self.tabs.clear();
            }
        } else {
            self.closed = 0;
        }
        if !self.watch.settled(glance) {
            return;
        }
        // The log is kept per character, so the name has to be known.
        let Some(name) = analysis.hud.as_ref().and_then(|hud| hud.name.clone()) else { return };
        if self.character.as_deref() != Some(name.as_str()) {
            self.tabs.clear();
            self.character = Some(name.clone());
        }
        match questlog::read(frame, scale, ocr) {
            Ok(Some(reading)) => self.keep(app, &name, &reading),
            Ok(None) => {}
            Err(err) => report::note(format!("Couldn't read the Quest window: {err}")),
        }
    }

    fn keep(&mut self, app: &AppHandle, name: &str, reading: &QuestLogReading) {
        let atlas = app.state::<MapDataState>().atlas();
        let atlas = atlas.as_deref();
        let gathered = self.tabs.entry(reading.tab).or_default();
        gathered.add(&reading.items, reading.scroll);
        let seen: Vec<SeenQuest> =
            gathered.quests().iter().map(|g| seen_quest(atlas, &g.quest, g.region.as_deref())).collect();
        let whole = gathered.whole();
        let regions = gathered.regions();
        let detail = reading.detail.as_ref().and_then(|detail| Some((detail, detail_quest(atlas, reading, &seen, detail)?)));
        let now = records::now();
        let saved = app.state::<UserDataState>().with(|db| {
            db.record_quest_tab(name, reading.tab, &seen, &regions, whole, now)?;
            if let Some((detail, quest)) = &detail {
                db.record_quest_detail(name, quest, detail, now)?;
            }
            Ok(())
        });
        match saved {
            Ok(()) => {
                let _ = app.emit(QUEST_LOG_EVENT, name);
            }
            Err(err) => report::note(format!("Couldn't keep the quest log: {err}")),
        }
    }
}

/// A row ready to keep: matched to the data when its title fits a quest there, and then named as
/// the data names it, since the game may have cut it short.
fn seen_quest(atlas: Option<&Atlas>, quest: &LogQuest, region: Option<&str>) -> SeenQuest {
    let readings: Vec<&str> = std::iter::once(quest.title.as_str()).chain(quest.readings.iter().map(String::as_str)).collect();
    let id = atlas.and_then(|a| a.log_quest(&readings, quest.cut, region));
    let data_name = id.and_then(|id| atlas?.quest_name(id));
    SeenQuest {
        title: data_name.clone().unwrap_or_else(|| quest.title.clone()),
        cut: quest.cut && data_name.is_none(),
        quest_id: id,
        region: region.map(|r| atlas.and_then(|a| a.quest_region_like(r)).unwrap_or_else(|| r.to_string())),
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
        if let Some(found) = seen.iter().find(|s| squash(&s.title) == key || s.quest_id.is_some_and(|id| {
            atlas.and_then(|a| a.log_quest(&[picked.title.as_str()], picked.cut, None)) == Some(id)
        })) {
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
        icon: questlog::RowIcon::Unknown,
        on_helper: None,
    })
}

/// A character's quest log as last seen, or the last one read's.
#[tauri::command]
pub fn quest_log(state: State<UserDataState>, name: Option<String>) -> Result<QuestLog> {
    state.with(|db| db.quest_log(name.as_deref()))
}

/// Forgets a character's quest log; the next look at the Quest window starts it again.
#[tauri::command]
pub fn forget_quest_log(app: AppHandle, name: String) -> Result<()> {
    app.state::<UserDataState>().with(|db| db.forget_quest_log(&name))?;
    let _ = app.emit(QUEST_LOG_EVENT, &name);
    Ok(())
}

/// How many quests the data files under each region.
#[tauri::command]
pub fn quest_region_totals(state: State<MapDataState>) -> Vec<RegionTotal> {
    state.atlas().map(|atlas| atlas.quest_region_totals()).unwrap_or_default()
}
