//! Build planner commands and the editor window. Builds live in the player's own records, and
//! nothing about them leaves this PC unless the player copies a code themselves.

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, Url, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_opener::OpenerExt;

use super::mapdata::MapDataState;
use super::records::{UserDataState, now};
use super::settings::SettingsState;
use crate::build::plan::MAX_LEVEL;
use crate::build::{
    Branch, Build, Catalog, Expansion, ItemFit, ItemJob, JobChoice, Rules, Starter, expand, fit, share, starters,
};
use crate::error::{Error, Result};
use crate::mapdata::osms::{ItemRecord, SkillRecord};
use crate::readers::tooltip::Requirements;
use crate::userdata::builds::BuildSummary;

pub const BUILD: &str = "build";

/// Emitted to every window when a build is saved, deleted or chosen.
pub const BUILDS_EVENT: &str = "builds";

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
    let build = tidied(build);
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
        window.show()?;
        window.unminimize()?;
        window.set_focus()?;
        return Ok(());
    }
    WebviewWindowBuilder::new(&app, BUILD, WebviewUrl::App("build".into()))
        .title("Leafline — Build planner")
        .inner_size(960.0, 760.0)
        .min_inner_size(760.0, 520.0)
        .center()
        .background_color(tauri::window::Color(20, 20, 18, 255))
        .build()?;
    Ok(())
}
