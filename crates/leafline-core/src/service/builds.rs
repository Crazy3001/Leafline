//! Build planner commands and the editor window. Builds live in the player's own records, and
//! nothing about them leaves this PC unless the player copies a code themselves.

use serde::Serialize;

use super::host::{Host, emit};
use super::records::now;
use crate::build::crafting::{CraftingPage, CraftingQuery, ItemCrafting};
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

fn rules(h: &dyn Host) -> Rules {
    h.settings().build_rules
}

fn catalog(h: &dyn Host) -> Result<std::sync::Arc<Catalog>> {
    h.map()
        .catalog()
        .ok_or_else(|| Error::Invalid("Download the map data first: builds use its skills and equipment.".into()))
}

fn planned(h: &dyn Host, build: Build) -> Result<BuildPlan> {
    let expansion = expand(&build, &rules(h), &*catalog(h)?);
    Ok(BuildPlan { build, expansion })
}

/// Items the character's build will wear from `level` on: its own goals and each slot's way up.
/// Empty without a build or the map data.
pub fn planned_gear(h: &dyn Host, character: Option<&str>, level: u32) -> std::collections::HashSet<u32> {
    let mut ids = std::collections::HashSet::new();
    let Some(catalog) = h.map().catalog() else { return ids };
    let Ok(Some(build)) = h.records().with(|db| db.active_build(character)) else { return ids };
    let rules = rules(h);
    let expansion = expand(&build, &rules, &catalog);
    let worn = outfit(&build, &expansion, &rules, &catalog, level.clamp(1, MAX_LEVEL));
    ids.extend(build.plan.gear.iter().copied());
    for path in &worn.paths {
        ids.extend(path.now.iter().chain(&path.steps).map(|step| step.item.id));
    }
    ids
}

fn changed(h: &dyn Host) -> Result<()> {
    emit(h, BUILDS_EVENT, &());
    Ok(())
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
pub fn build_catalog(h: &dyn Host) -> Option<CatalogView> {
    let catalog = h.map().catalog()?;
    Some(CatalogView { jobs: catalog.jobs(), skills: catalog.skills().to_vec() })
}

/// The plans written for Leafline, one for each second job. Empty until the map data is here.
pub fn starter_plans(h: &dyn Host) -> Vec<Starter> {
    h.map().catalog().map_or_else(Vec::new, |c| starters::all(&c, &rules(h)))
}

/// Equipment by name, limited to what `job` can use.
pub fn search_gear(h: &dyn Host, text: String, job: u32) -> Vec<ItemRecord> {
    h.map().catalog().map_or_else(Vec::new, |c| c.search_gear(&text, Branch::of_job(job)))
}

/// The recipes that make an item and the ones that use it. None without map data.
pub fn item_crafting(h: &dyn Host, id: u32) -> Option<ItemCrafting> {
    h.map().catalog().map(|catalog| catalog.crafting_of(id))
}

/// The Crafting section's recipes. Empty without map data.
pub fn crafting_recipes(h: &dyn Host, query: CraftingQuery) -> CraftingPage {
    h.map().catalog().map(|catalog| catalog.crafting(&query)).unwrap_or_default()
}

/// One page of the item search. Without map data there is nothing to find.
pub fn search_items(h: &dyn Host,
    query: ItemQuery,
    level: Option<u32>,
    character: Option<String>,
) -> Result<ItemPage> {
    let Some(catalog) = h.map().catalog() else { return Ok(ItemPage::default()) };
    if !query.mine {
        return Ok(catalog.search_items(&query, |_| true));
    }
    let Some(build) = h.records().with(|db| db.active_build(character.as_deref()))? else { return Ok(ItemPage::default()) };
    let rules = rules(h);
    let expansion = expand(&build, &rules, &catalog);
    let wearer = Wearer::new(&build, &expansion, &rules, &catalog, level.unwrap_or(build.start.level));
    Ok(catalog.search_items(&query, |item| wearer.can_wear(item)))
}

pub fn item_filters(h: &dyn Host) -> ItemFilters {
    h.map().catalog().map_or_else(ItemFilters::default, |c| c.item_filters())
}

pub fn item_record(h: &dyn Host, id: u32) -> Option<ItemRecord> {
    h.map().catalog()?.item(id).cloned()
}

/// The item a name read from the game most likely means.
pub fn find_item(h: &dyn Host, name: String, level: Option<u32>) -> Option<ItemRecord> {
    h.map().catalog()?.item_like(&name, level).cloned()
}

/// Puts on a piece of equipment, as an item check read it, in the build for `character`. It takes
/// the place of what was in its slot, as it would in game.
pub fn add_piece(h: &dyn Host, piece: Piece, character: Option<String>) -> Result<()> {
    wear(h, piece, character)
}

/// Puts on a clean copy of an item from the data, for gear the player names rather than checks.
pub fn wear_item(h: &dyn Host, item: u32, character: Option<String>) -> Result<()> {
    let catalog = catalog(h)?;
    let Some(record) = catalog.item(item) else {
        return Err(Error::Invalid("Leafline doesn't know that item.".into()));
    };
    wear(h, Piece::of_item(record), character)
}

fn wear(h: &dyn Host, piece: Piece, character: Option<String>) -> Result<()> {
    let catalog = catalog(h)?;
    let Some(mut build) = h.records().with(|db| db.active_build(character.as_deref()))? else {
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
    h.records().with(|db| db.save_build(&build, now()))?;
    changed(h)
}

/// Puts right a piece the item check misread: its name, item, stats, upgrades or slots.
pub fn update_piece(h: &dyn Host, piece: Piece, character: Option<String>) -> Result<()> {
    let Some(mut build) = h.records().with(|db| db.active_build(character.as_deref()))? else { return Ok(()) };
    let Some(old) = build.start.pieces.iter_mut().find(|p| p.id == piece.id) else {
        return Err(Error::Invalid("That piece is no longer in the build.".into()));
    };
    *old = piece;
    h.records().with(|db| db.save_build(&build, now()))?;
    changed(h)
}

/// Takes off a piece of equipment in the build for `character`. Builds from before worn gear
/// was kept apart made its item a goal too, so that goal goes with it.
pub fn remove_piece(h: &dyn Host, id: i64, character: Option<String>) -> Result<()> {
    let Some(mut build) = h.records().with(|db| db.active_build(character.as_deref()))? else { return Ok(()) };
    let item = build.start.pieces.iter().find(|p| p.id == id).and_then(|p| p.item);
    build.start.pieces.retain(|p| p.id != id);
    if let Some(item) = item
        && !build.start.pieces.iter().any(|p| p.item == Some(item))
    {
        build.plan.gear.retain(|g| *g != item);
    }
    h.records().with(|db| db.save_build(&build, now()))?;
    changed(h)
}

/// Everything read off the tooltip matches; only the id differs.
fn same_piece(a: &Piece, b: &Piece) -> bool {
    Piece { id: 0, ..a.clone() } == Piece { id: 0, ..b.clone() }
}

/// Adds an item to the gear goals of the build for `character`, or takes it off.
pub fn set_gear_goal(h: &dyn Host,
    item: u32,
    wanted: bool,
    character: Option<String>,
) -> Result<()> {
    let Some(mut build) = h.records().with(|db| db.active_build(character.as_deref()))? else {
        return Err(Error::Invalid("Plan a build first, in the Character tab.".into()));
    };
    build.plan.gear.retain(|id| *id != item);
    if wanted {
        build.plan.gear.push(item);
    }
    h.records().with(|db| db.save_build(&build, now()))?;
    changed(h)
}

/// The build's gear at `level`, slot by slot: worn, next and suggested. `None` without the
/// build or the map data.
pub fn build_outfit(h: &dyn Host, id: i64, level: u32) -> Result<Option<Outfit>> {
    let Some(catalog) = h.map().catalog() else { return Ok(None) };
    let Some(build) = h.records().with(|db| db.build(id))? else { return Ok(None) };
    let rules = rules(h);
    let expansion = expand(&build, &rules, &catalog);
    Ok(Some(outfit(&build, &expansion, &rules, &catalog, level)))
}

/// Fills each slot with no goals with an upgrade path from the data, starting at `level`, and
/// returns how many goals were added. A build that doesn't say who its gear is for takes the
/// gender of the gear worn, so the path holds nothing the character can't put on.
pub fn fill_gear(h: &dyn Host, id: i64, level: u32) -> Result<usize> {
    let catalog = catalog(h)?;
    let Some(mut build) = h.records().with(|db| db.build(id))? else {
        return Err(Error::Invalid("That build is gone.".into()));
    };
    if build.start.gender.is_none() {
        build.start.gender = outfit::worn_gender(&build, &catalog);
    }
    let rules = rules(h);
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
    h.records().with(|db| db.save_build(&build, now()))?;
    changed(h)?;
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
pub fn catch_up(h: &dyn Host, scan: CharacterScan) -> Result<CharacterRead> {
    super::damage::note(h, &scan);
    let (stats, skills) = (scan.windows.stats.is_some(), scan.windows.skills.is_some());
    let done = |problem: &str| CharacterRead { stats, skills, problem: Some(problem.into()), ..CharacterRead::default() };
    if !stats && !skills {
        return Ok(done("Open your Character Stat or Skill window so all of it shows, then try again."));
    }
    let Some(catalog) = h.map().catalog() else {
        return Ok(done("Download the map data first: builds use its skills and equipment."));
    };
    let Some(mut build) = h.records().with(|db| db.active_build(scan.name.as_deref()))? else {
        return Ok(done("Plan a build first, in the Character tab."));
    };
    if let (Some(owner), Some(name)) = (&build.character, &scan.name)
        && !owner.eq_ignore_ascii_case(name)
    {
        return Ok(done(&format!("This build is for {owner}, not {name}.")));
    }
    build.character = build.character.or(scan.name);
    let result = progress::catch_up(&mut build.start, scan.level, &scan.windows, build.plan.job, &rules(h), &catalog);
    h.records().with(|db| db.save_build(&build, now()))?;
    changed(h)?;
    Ok(CharacterRead { stats, skills, build: Some(build.name), result, ..CharacterRead::default() })
}

/// Gear to look for at `level` on this build. `None` without the build or the map data.
pub fn gear_suggestions(h: &dyn Host, id: i64, level: u32) -> Result<Option<GearSuggestions>> {
    let Some(catalog) = h.map().catalog() else { return Ok(None) };
    let Some(build) = h.records().with(|db| db.build(id))? else { return Ok(None) };
    let rules = rules(h);
    let expansion = expand(&build, &rules, &catalog);
    Ok(Some(suggest::suggest(&build, &expansion, &rules, &catalog, level.clamp(1, MAX_LEVEL))))
}

/// Gear for a character. `seen_job` is the job the HUD shows, for a character with no build.
/// `None` until the map data is downloaded.
pub fn find_gear(h: &dyn Host,
    who: GearFor,
    character: Option<String>,
    seen_job: Option<String>,
) -> Result<Option<GearFound>> {
    let Some(catalog) = h.map().catalog() else { return Ok(None) };
    let own = h.records().with(|db| db.active_build(character.as_deref()))?;
    let seen = seen_job.and_then(|name| catalog.jobs().into_iter().find(|j| j.name.eq_ignore_ascii_case(name.trim())));
    Ok(Some(suggest::find_gear(&who, own, seen.map(|j| j.id), &rules(h), &catalog)))
}

pub fn list_builds(h: &dyn Host) -> Result<Vec<BuildSummary>> {
    h.records().with(|db| db.builds())
}

pub fn build_plan(h: &dyn Host, id: i64) -> Result<Option<BuildPlan>> {
    h.records().with(|db| db.build(id))?.map(|build| planned(h, build)).transpose()
}

/// The build to show for `character`. `None` with no builds, or before the map data is here.
pub fn active_build(h: &dyn Host, character: Option<String>) -> Result<Option<BuildPlan>> {
    if h.map().catalog().is_none() {
        return Ok(None);
    }
    h.records().with(|db| db.active_build(character.as_deref()))?.map(|build| planned(h, build)).transpose()
}

/// Saves a build, new or changed, and returns how it plays out.
pub fn save_build(h: &dyn Host, build: Build) -> Result<BuildPlan> {
    let mut build = tidied(build);
    // Pieces change only by adding or removing one, so an editor's older copy can't undo either.
    if build.id != 0
        && let Some(stored) = h.records().with(|db| db.build(build.id))?
    {
        build.start.pieces = stored.start.pieces;
    }
    let id = h.records().with(|db| db.save_build(&build, now()))?;
    changed(h)?;
    planned(h, Build { id, ..build })
}

pub fn use_build(h: &dyn Host, id: i64) -> Result<()> {
    h.records().with(|db| db.use_build(id, now()))?;
    changed(h)
}

pub fn delete_build(h: &dyn Host, id: i64) -> Result<()> {
    h.records().with(|db| db.delete_build(id))?;
    changed(h)
}

/// The build's plan as a line of text, for the player to pass on themselves.
pub fn export_build(h: &dyn Host, id: i64) -> Result<String> {
    let build = h.records().with(|db| db.build(id))?.ok_or_else(|| Error::Invalid("That build no longer exists.".into()))?;
    share::export(&build.name, &build.plan)
}

/// Adds a build from a code. Its starting point is blank, for the player to fill in.
pub fn import_build(h: &dyn Host, code: String, character: Option<String>) -> Result<BuildPlan> {
    let (name, plan) = share::import(&code)?;
    save_build(h, Build { name, character, plan, ..Build::default() })
}

/// How a hovered item suits the build for `character`. `None` with no build to judge it by.
pub fn build_item_fit(h: &dyn Host,
    requirements: Requirements,
    jobs: Vec<ItemJob>,
    item_type: Option<String>,
    level: Option<u32>,
    character: Option<String>,
) -> Result<Option<ItemFit>> {
    let Some(catalog) = h.map().catalog() else { return Ok(None) };
    let Some(build) = h.records().with(|db| db.active_build(character.as_deref()))? else { return Ok(None) };
    let expansion = expand(&build, &rules(h), &catalog);
    Ok(fit::check(&requirements, &jobs, item_type.as_deref(), &expansion, Branch::of_job(build.plan.job), level))
}
