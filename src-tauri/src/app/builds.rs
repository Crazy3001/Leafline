//! Build planner commands and the editor window. Builds live in the player's own records, and
//! nothing about them leaves this PC unless the player copies a code themselves.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, Url, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

use super::mapdata::MapDataState;
use super::records::{UserDataState, now};
use super::settings::{self, SettingsState};
use super::{overlay, pins};
use crate::build::plan::{MAX_LEVEL, Piece};
use crate::build::progress::{self, CatchUp};
use crate::build::{
    Branch, Build, Catalog, Expansion, GearFor, GearFound, GearSuggestions, ItemFilters, ItemFit, ItemJob, ItemPage,
    ItemQuery, JobChoice, Outfit, Rules, Starter, Wearer, expand, fit, outfit, share, starters, suggest,
};
use crate::error::{Error, Result};
use crate::mapdata::osms::{ItemRecord, SkillRecord};
use crate::pipeline::CharacterScan;
use crate::readers::tooltip::Requirements;
use crate::userdata::builds::BuildSummary;

pub const BUILD: &str = "build";

/// Emitted to every window when a build is saved, deleted or chosen.
pub const BUILDS_EVENT: &str = "builds";
/// Emitted to every window to bring up an item in the Item tab.
pub const SHOW_ITEM_EVENT: &str = "show-item";

/// A name no longer than a heading can show.
const MAX_NAME: usize = 80;

/// A build with how it plays out.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildPlan {
    build: Build,
    expansion: Expansion,
}

fn rules(app: &AppHandle) -> Rules {
    app.state::<SettingsState>().get().build_rules
}

fn catalog(app: &AppHandle) -> Result<std::sync::Arc<Catalog>> {
    app.state::<MapDataState>()
        .catalog()
        .ok_or_else(|| Error::Invalid("Download the map data first: builds use its skills and equipment.".into()))
}

fn planned(app: &AppHandle, build: Build) -> Result<BuildPlan> {
    let expansion = expand(&build, &rules(app), &*catalog(app)?);
    Ok(BuildPlan { build, expansion })
}

fn changed(app: &AppHandle) -> Result<()> {
    Ok(app.emit(BUILDS_EVENT, ())?)
}

/// Within what the fields can hold, and named, so a list never shows a blank line.
fn tidied(mut build: Build) -> Build {
    build.name = build.name.trim().chars().take(MAX_NAME).collect();
    if build.name.is_empty() {
        build.name = "Unnamed build".into();
    }
    build.character = build.character.map(|c| c.trim().to_string()).filter(|c| !c.is_empty());
    build.guide = build.guide.trim().to_string();
    build.start.level = build.start.level.clamp(1, MAX_LEVEL);
    build.plan.plan_to = build.plan.plan_to.clamp(1, MAX_LEVEL);
    build
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogView {
    jobs: Vec<JobChoice>,
    skills: Vec<SkillRecord>,
}

/// The jobs and skills to choose from, or `None` until the map data is downloaded.
#[tauri::command]
pub fn build_catalog(state: State<MapDataState>) -> Option<CatalogView> {
    let catalog = state.catalog()?;
    Some(CatalogView { jobs: catalog.jobs(), skills: catalog.skills().to_vec() })
}

/// The plans written for Leafline, one for each second job. Empty until the map data is here.
#[tauri::command]
pub fn starter_plans(app: AppHandle) -> Vec<Starter> {
    app.state::<MapDataState>().catalog().map_or_else(Vec::new, |c| starters::all(&c, &rules(&app)))
}

/// Equipment by name, limited to what `job` can use.
#[tauri::command]
pub fn search_gear(state: State<MapDataState>, text: String, job: u32) -> Vec<ItemRecord> {
    state.catalog().map_or_else(Vec::new, |c| c.search_gear(&text, Branch::of_job(job)))
}

/// One page of the item search. Without map data there is nothing to find.
#[tauri::command]
pub fn search_items(
    app: AppHandle,
    state: State<UserDataState>,
    query: ItemQuery,
    level: Option<u32>,
    character: Option<String>,
) -> Result<ItemPage> {
    let Some(catalog) = app.state::<MapDataState>().catalog() else { return Ok(ItemPage::default()) };
    if !query.mine {
        return Ok(catalog.search_items(&query, |_| true));
    }
    let Some(build) = state.with(|db| db.active_build(character.as_deref()))? else { return Ok(ItemPage::default()) };
    let rules = rules(&app);
    let expansion = expand(&build, &rules, &catalog);
    let wearer = Wearer::new(&build, &expansion, &rules, &catalog, level.unwrap_or(build.start.level));
    Ok(catalog.search_items(&query, |item| wearer.can_wear(item)))
}

#[tauri::command]
pub fn item_filters(state: State<MapDataState>) -> ItemFilters {
    state.catalog().map_or_else(ItemFilters::default, |c| c.item_filters())
}

#[tauri::command]
pub fn item_record(state: State<MapDataState>, id: u32) -> Option<ItemRecord> {
    state.catalog()?.item(id).cloned()
}

/// The item a name read from the game most likely means.
#[tauri::command]
pub fn find_item(state: State<MapDataState>, name: String, level: Option<u32>) -> Option<ItemRecord> {
    state.catalog()?.item_like(&name, level).cloned()
}

/// What to bring up in the Item tab: one item, or a search when no item was matched.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ItemLink {
    id: Option<u32>,
    text: Option<String>,
}

/// Brings an item up in the Item tab, from whichever window asked. A pinned Item panel shows it
/// by itself; otherwise the overlay has to.
#[tauri::command]
pub fn show_item(app: AppHandle, id: Option<u32>, text: Option<String>) -> Result<()> {
    if !pins::shows_item(&app) {
        overlay::show_full(&app)?;
    }
    Ok(app.emit(SHOW_ITEM_EVENT, ItemLink { id, text })?)
}

/// Puts on a piece of equipment, as an item check read it, in the build for `character`. It takes
/// the place of what was in its slot, as it would in game.
#[tauri::command]
pub fn add_piece(app: AppHandle, state: State<UserDataState>, piece: Piece, character: Option<String>) -> Result<()> {
    wear(&app, &state, piece, character)
}

/// Puts on a clean copy of an item from the data, for gear the player names rather than checks.
#[tauri::command]
pub fn wear_item(app: AppHandle, state: State<UserDataState>, item: u32, character: Option<String>) -> Result<()> {
    let catalog = catalog(&app)?;
    let Some(record) = catalog.item(item) else {
        return Err(Error::Invalid("Leafline doesn't know that item.".into()));
    };
    wear(&app, &state, Piece::of_item(record), character)
}

fn wear(app: &AppHandle, state: &UserDataState, piece: Piece, character: Option<String>) -> Result<()> {
    let catalog = catalog(app)?;
    let Some(mut build) = state.with(|db| db.active_build(character.as_deref()))? else {
        return Err(Error::Invalid("Plan a build first, in the Character tab.".into()));
    };
    let pieces = &mut build.start.pieces;
    // A tooltip read twice is the same piece, not a second copy.
    if pieces.iter().any(|p| same_piece(p, &piece)) {
        return Ok(());
    }
    let slot = piece.slot(&catalog);
    pieces.retain(|p| !Piece::clashes(&p.slot(&catalog), &slot));
    let id = pieces.iter().map(|p| p.id).max().unwrap_or(0) + 1;
    pieces.push(Piece { id, ..piece });
    state.with(|db| db.save_build(&build, now()))?;
    changed(app)
}

/// Puts right a piece the item check misread: its name, item, stats, upgrades or slots.
#[tauri::command]
pub fn update_piece(app: AppHandle, state: State<UserDataState>, piece: Piece, character: Option<String>) -> Result<()> {
    let Some(mut build) = state.with(|db| db.active_build(character.as_deref()))? else { return Ok(()) };
    let Some(old) = build.start.pieces.iter_mut().find(|p| p.id == piece.id) else {
        return Err(Error::Invalid("That piece is no longer in the build.".into()));
    };
    *old = piece;
    state.with(|db| db.save_build(&build, now()))?;
    changed(&app)
}

/// Takes off a piece of equipment in the build for `character`. Builds from before worn gear
/// was kept apart made its item a goal too, so that goal goes with it.
#[tauri::command]
pub fn remove_piece(app: AppHandle, state: State<UserDataState>, id: i64, character: Option<String>) -> Result<()> {
    let Some(mut build) = state.with(|db| db.active_build(character.as_deref()))? else { return Ok(()) };
    let item = build.start.pieces.iter().find(|p| p.id == id).and_then(|p| p.item);
    build.start.pieces.retain(|p| p.id != id);
    if let Some(item) = item
        && !build.start.pieces.iter().any(|p| p.item == Some(item))
    {
        build.plan.gear.retain(|g| *g != item);
    }
    state.with(|db| db.save_build(&build, now()))?;
    changed(&app)
}

/// Everything read off the tooltip matches; only the id differs.
fn same_piece(a: &Piece, b: &Piece) -> bool {
    Piece { id: 0, ..a.clone() } == Piece { id: 0, ..b.clone() }
}

/// Adds an item to the gear goals of the build for `character`, or takes it off.
#[tauri::command]
pub fn set_gear_goal(
    app: AppHandle,
    state: State<UserDataState>,
    item: u32,
    wanted: bool,
    character: Option<String>,
) -> Result<()> {
    let Some(mut build) = state.with(|db| db.active_build(character.as_deref()))? else {
        return Err(Error::Invalid("Plan a build first, in the Character tab.".into()));
    };
    build.plan.gear.retain(|id| *id != item);
    if wanted {
        build.plan.gear.push(item);
    }
    state.with(|db| db.save_build(&build, now()))?;
    changed(&app)
}

/// The build's gear at `level`, slot by slot: worn, next and suggested. `None` without the
/// build or the map data.
#[tauri::command]
pub fn build_outfit(app: AppHandle, state: State<UserDataState>, id: i64, level: u32) -> Result<Option<Outfit>> {
    let Some(catalog) = app.state::<MapDataState>().catalog() else { return Ok(None) };
    let Some(build) = state.with(|db| db.build(id))? else { return Ok(None) };
    let rules = rules(&app);
    let expansion = expand(&build, &rules, &catalog);
    Ok(Some(outfit(&build, &expansion, &rules, &catalog, level)))
}

/// Fills each slot with no goals with an upgrade path from the data, starting at `level`, and
/// returns how many goals were added. A build that doesn't say who its gear is for takes the
/// gender of the gear worn, so the path holds nothing the character can't put on.
#[tauri::command]
pub fn fill_gear(app: AppHandle, state: State<UserDataState>, id: i64, level: u32) -> Result<usize> {
    let catalog = catalog(&app)?;
    let Some(mut build) = state.with(|db| db.build(id))? else {
        return Err(Error::Invalid("That build is gone.".into()));
    };
    if build.start.gender.is_none() {
        build.start.gender = outfit::worn_gender(&build, &catalog);
    }
    let rules = rules(&app);
    let expansion = expand(&build, &rules, &catalog);
    let mut added = 0;
    for (_, path) in outfit::fill_paths(&build, &expansion, &rules, &catalog, level) {
        for item in path {
            if !build.plan.gear.contains(&item) {
                build.plan.gear.push(item);
                added += 1;
            }
        }
    }
    state.with(|db| db.save_build(&build, now()))?;
    changed(&app)?;
    Ok(added)
}

/// How reading the character's windows went.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterRead {
    /// Which windows were read.
    stats: bool,
    skills: bool,
    /// The build that was brought up to date.
    build: Option<String>,
    result: CatchUp,
    /// Why nothing was updated.
    problem: Option<String>,
    /// It came from a live picture, not from the player asking.
    automatic: bool,
}

impl CharacterRead {
    pub fn failed(problem: String) -> Self {
        Self { problem: Some(problem), ..Self::default() }
    }

    /// Whether the build moved.
    pub fn changed(&self) -> bool {
        self.problem.is_none() && !self.result.changes.is_empty()
    }

    pub fn on_its_own(self) -> Self {
        Self { automatic: true, ..self }
    }
}

/// Moves the character's build to what its windows show, so the plan carries on from there. A
/// build tied to another character is left alone.
pub fn catch_up(app: &AppHandle, scan: CharacterScan) -> Result<CharacterRead> {
    let (stats, skills) = (scan.windows.stats.is_some(), scan.windows.skills.is_some());
    let done = |problem: &str| CharacterRead { stats, skills, problem: Some(problem.into()), ..CharacterRead::default() };
    if !stats && !skills {
        return Ok(done("Open your Character Stat or Skill window so all of it shows, then try again."));
    }
    let Some(catalog) = app.state::<MapDataState>().catalog() else {
        return Ok(done("Download the map data first: builds use its skills and equipment."));
    };
    let state = app.state::<UserDataState>();
    let Some(mut build) = state.with(|db| db.active_build(scan.name.as_deref()))? else {
        return Ok(done("Plan a build first, in the Character tab."));
    };
    if let (Some(owner), Some(name)) = (&build.character, &scan.name)
        && !owner.eq_ignore_ascii_case(name)
    {
        return Ok(done(&format!("This build is for {owner}, not {name}.")));
    }
    build.character = build.character.or(scan.name);
    let result = progress::catch_up(&mut build.start, scan.level, &scan.windows, build.plan.job, &rules(app), &catalog);
    state.with(|db| db.save_build(&build, now()))?;
    changed(app)?;
    Ok(CharacterRead { stats, skills, build: Some(build.name), result, ..CharacterRead::default() })
}

/// Gear to look for at `level` on this build. `None` without the build or the map data.
#[tauri::command]
pub fn gear_suggestions(app: AppHandle, state: State<UserDataState>, id: i64, level: u32) -> Result<Option<GearSuggestions>> {
    let Some(catalog) = app.state::<MapDataState>().catalog() else { return Ok(None) };
    let Some(build) = state.with(|db| db.build(id))? else { return Ok(None) };
    let rules = rules(&app);
    let expansion = expand(&build, &rules, &catalog);
    Ok(Some(suggest::suggest(&build, &expansion, &rules, &catalog, level.clamp(1, MAX_LEVEL))))
}

/// Gear for a character. `seen_job` is the job the HUD shows, for a character with no build.
/// `None` until the map data is downloaded.
#[tauri::command]
pub fn find_gear(
    app: AppHandle,
    state: State<UserDataState>,
    who: GearFor,
    character: Option<String>,
    seen_job: Option<String>,
) -> Result<Option<GearFound>> {
    let Some(catalog) = app.state::<MapDataState>().catalog() else { return Ok(None) };
    let own = state.with(|db| db.active_build(character.as_deref()))?;
    let seen = seen_job.and_then(|name| catalog.jobs().into_iter().find(|j| j.name.eq_ignore_ascii_case(name.trim())));
    Ok(Some(suggest::find_gear(&who, own, seen.map(|j| j.id), &rules(&app), &catalog)))
}

#[tauri::command]
pub fn list_builds(state: State<UserDataState>) -> Result<Vec<BuildSummary>> {
    state.with(|db| db.builds())
}

#[tauri::command]
pub fn build_plan(app: AppHandle, state: State<UserDataState>, id: i64) -> Result<Option<BuildPlan>> {
    state.with(|db| db.build(id))?.map(|build| planned(&app, build)).transpose()
}

/// The build to show for `character`. `None` with no builds, or before the map data is here.
#[tauri::command]
pub fn active_build(app: AppHandle, state: State<UserDataState>, character: Option<String>) -> Result<Option<BuildPlan>> {
    if app.state::<MapDataState>().catalog().is_none() {
        return Ok(None);
    }
    state.with(|db| db.active_build(character.as_deref()))?.map(|build| planned(&app, build)).transpose()
}

/// Saves a build, new or changed, and returns how it plays out.
#[tauri::command]
pub fn save_build(app: AppHandle, state: State<UserDataState>, build: Build) -> Result<BuildPlan> {
    let mut build = tidied(build);
    // Pieces change only by adding or removing one, so an editor's older copy can't undo either.
    if build.id != 0
        && let Some(stored) = state.with(|db| db.build(build.id))?
    {
        build.start.pieces = stored.start.pieces;
    }
    let id = state.with(|db| db.save_build(&build, now()))?;
    changed(&app)?;
    planned(&app, Build { id, ..build })
}

#[tauri::command]
pub fn use_build(app: AppHandle, state: State<UserDataState>, id: i64) -> Result<()> {
    state.with(|db| db.use_build(id, now()))?;
    changed(&app)
}

#[tauri::command]
pub fn delete_build(app: AppHandle, state: State<UserDataState>, id: i64) -> Result<()> {
    state.with(|db| db.delete_build(id))?;
    changed(&app)
}

/// The build's plan as a line of text, for the player to pass on themselves.
#[tauri::command]
pub fn export_build(state: State<UserDataState>, id: i64) -> Result<String> {
    let build = state.with(|db| db.build(id))?.ok_or_else(|| Error::Invalid("That build no longer exists.".into()))?;
    share::export(&build.name, &build.plan)
}

/// Adds a build from a code. Its starting point is blank, for the player to fill in.
#[tauri::command]
pub fn import_build(app: AppHandle, state: State<UserDataState>, code: String, character: Option<String>) -> Result<BuildPlan> {
    let (name, plan) = share::import(&code)?;
    save_build(app, state, Build { name, character, plan, ..Build::default() })
}

/// How a hovered item suits the build for `character`. `None` with no build to judge it by.
#[tauri::command]
pub fn build_item_fit(
    app: AppHandle,
    state: State<UserDataState>,
    requirements: Requirements,
    jobs: Vec<ItemJob>,
    item_type: Option<String>,
    level: Option<u32>,
    character: Option<String>,
) -> Result<Option<ItemFit>> {
    let Some(catalog) = app.state::<MapDataState>().catalog() else { return Ok(None) };
    let Some(build) = state.with(|db| db.active_build(character.as_deref()))? else { return Ok(None) };
    let expansion = expand(&build, &rules(&app), &catalog);
    Ok(fit::check(&requirements, &jobs, item_type.as_deref(), &expansion, Branch::of_job(build.plan.job), level))
}

/// Opens the guide link the player typed for a build. It is theirs, so any web address will do.
#[tauri::command]
pub fn open_guide(app: AppHandle, url: String) -> Result<()> {
    let parsed = Url::parse(url.trim()).map_err(|err| Error::Invalid(format!("That isn't a web address: {err}")))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(Error::Invalid("Guide links must be web addresses.".into()));
    }
    app.opener().open_url(parsed.as_str(), None::<&str>).map_err(|err| Error::Invalid(err.to_string()))
}

/// Opens the build editor, or brings it forward.
#[tauri::command]
pub async fn show_build(app: AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window(BUILD) {
        return settings::bring_forward(&app, &window);
    }
    let keep = settings::keeps_game_focus(&app);
    WebviewWindowBuilder::new(&app, BUILD, WebviewUrl::App("build".into()))
        .title("Leafline — Build planner")
        .inner_size(960.0, 760.0)
        .min_inner_size(760.0, 520.0)
        .center()
        .focusable(!keep)
        .focused(!keep)
        .background_color(tauri::window::Color(20, 20, 18, 255))
        .additional_browser_args(super::BROWSER_ARGS)
        .on_page_load(settings::raise_when_loaded)
        .build()?;
    Ok(())
}
