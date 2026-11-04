//! The optional update check. When the player turns it on (in setup or Settings), Leafline asks
//! GitHub once at startup for the number of the latest release and says in Settings if it is
//! newer. The request carries Leafline's own version in its user agent and nothing else: nothing
//! read from the game, nothing about the player or their PC. Nothing is downloaded or installed;
//! the notice links to the release page.

use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use super::settings::SettingsState;
use crate::error::{Error, Result};

/// Where releases are published, as `owner/repo`. While the repository is private, GitHub answers
/// 404 to the check, which shows as "No release has been published yet".
pub const RELEASES_REPO: &str = "Crazy3001/leafline";

/// The owner part of [`RELEASES_REPO`] while it is still a placeholder.
const PLACEHOLDER_OWNER: &str = "OWNER";

/// Emitted to every window when the check's state changes.
pub const UPDATE_EVENT: &str = "update";

/// This build's version.
const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// Only Leafline's name and version: GitHub's API refuses requests without a user agent.
const USER_AGENT: &str = concat!("Leafline/", env!("CARGO_PKG_VERSION"));

/// A release's JSON is a few kilobytes; anything far bigger isn't one.
const MAX_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum UpdateStatus {
    /// The player hasn't turned the check on.
    #[default]
    Off,
    Checking,
    /// This is the latest release, or newer.
    UpToDate { current: String, latest: String },
    Available { current: String, latest: String, url: String },
    /// Nothing to compare with: no repository set yet, or no release published there.
    Unavailable { current: String, reason: String },
    /// Offline, or GitHub didn't answer as expected. Tried again at the next start.
    Failed { current: String, message: String },
}

#[derive(Default)]
pub struct UpdateState(Mutex<UpdateStatus>);

impl UpdateState {
    fn get(&self) -> UpdateStatus {
        self.0.lock().expect("update lock").clone()
    }
}

/// The parts of GitHub's release JSON that matter here.
#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
}

/// Starts a check in the background if the player turned it on.
pub fn check_at_startup(app: &AppHandle) {
    if app.state::<SettingsState>().get().update_check {
        start(app);
    }
}

/// Checks in the background and tells every window when it is done.
pub fn start(app: &AppHandle) {
    set(app, UpdateStatus::Checking);
    let app = app.clone();
    std::thread::spawn(move || {
        let status = check();
        // Turned off while the request was out: say nothing about it.
        if app.state::<SettingsState>().get().update_check {
            set(&app, status);
        }
    });
}

/// Forgets the last result, when the player turns the check off.
pub fn turn_off(app: &AppHandle) {
    set(app, UpdateStatus::Off);
}

fn set(app: &AppHandle, status: UpdateStatus) {
    *app.state::<UpdateState>().0.lock().expect("update lock") = status.clone();
    let _ = app.emit(UPDATE_EVENT, &status);
}

/// Asks GitHub for the latest release. Blocks for up to the timeout.
fn check() -> UpdateStatus {
    let current = CURRENT.to_string();
    let Some((owner, _)) = RELEASES_REPO.split_once('/') else {
        return UpdateStatus::Unavailable { current, reason: "No release page is set for this build.".into() };
    };
    if owner == PLACEHOLDER_OWNER {
        return UpdateStatus::Unavailable { current, reason: "This build has no release page to check yet.".into() };
    }
    let url = format!("https://api.github.com/repos/{RELEASES_REPO}/releases/latest");
    let agent = ureq::AgentBuilder::new().user_agent(USER_AGENT).timeout(Duration::from_secs(10)).build();
    let response = agent
        .get(&url)
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .call();
    let body = match response {
        Ok(response) => {
            let mut body = String::new();
            match std::io::Read::read_to_string(&mut std::io::Read::take(response.into_reader(), MAX_BYTES), &mut body) {
                Ok(_) => body,
                Err(err) => return UpdateStatus::Failed { current, message: err.to_string() },
            }
        }
        // A private repository, or one with no release yet, answers 404.
        Err(ureq::Error::Status(404, _)) => {
            return UpdateStatus::Unavailable { current, reason: "No release has been published yet.".into() };
        }
        Err(ureq::Error::Status(code, _)) => {
            return UpdateStatus::Failed { current, message: format!("GitHub answered {code}.") };
        }
        Err(err) => return UpdateStatus::Failed { current, message: format!("Couldn't reach GitHub ({err}).") },
    };
    match compare(&body, CURRENT) {
        Ok(status) => status,
        Err(err) => UpdateStatus::Failed { current, message: err.to_string() },
    }
}

/// What a release JSON means for a build of version `current`.
fn compare(json: &str, current: &str) -> Result<UpdateStatus> {
    let release: Release =
        serde_json::from_str(json).map_err(|_| Error::Invalid("GitHub's answer wasn't a release.".into()))?;
    let latest = release.tag_name.trim().trim_start_matches(['v', 'V']).to_string();
    let (Some(new), Some(old)) = (version(&latest), version(current)) else {
        return Err(Error::Invalid(format!("Couldn't read the version \"{}\".", release.tag_name)));
    };
    let current = current.to_string();
    Ok(if new > old {
        UpdateStatus::Available { current, latest, url: release.html_url }
    } else {
        UpdateStatus::UpToDate { current, latest }
    })
}

/// `1.2.3` as numbers, with a pre-release (`1.2.3-beta.1`) sorting before the release itself.
fn version(text: &str) -> Option<(u64, u64, u64, bool)> {
    let (core, pre) = match text.split_once('-') {
        Some((core, pre)) => (core, !pre.is_empty()),
        None => (text, false),
    };
    let core = core.split('+').next()?;
    let mut parts = core.split('.').map(|p| p.parse::<u64>().ok());
    let major = parts.next()??;
    let minor = parts.next().unwrap_or(Some(0))?;
    let patch = parts.next().unwrap_or(Some(0))?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch, !pre))
}

#[tauri::command]
pub fn update_status(state: State<UpdateState>) -> UpdateStatus {
    state.get()
}

/// Checks now, from the button in Settings. Only while the check is turned on.
#[tauri::command]
pub fn check_for_update(app: AppHandle) -> Result<UpdateStatus> {
    if !app.state::<SettingsState>().get().update_check {
        return Err(Error::Invalid("Turn on the update check first.".into()));
    }
    start(&app);
    Ok(app.state::<UpdateState>().get())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str) -> String {
        format!(r#"{{"tag_name":"{tag}","html_url":"https://github.com/o/r/releases/tag/{tag}","assets":[]}}"#)
    }

    #[test]
    fn a_newer_release_is_offered_and_an_older_or_equal_one_is_not() {
        assert!(matches!(compare(&release("v0.2.0"), "0.1.0").unwrap(), UpdateStatus::Available { latest, .. } if latest == "0.2.0"));
        assert!(matches!(compare(&release("0.1.0"), "0.1.0").unwrap(), UpdateStatus::UpToDate { .. }));
        assert!(matches!(compare(&release("v0.0.9"), "0.1.0").unwrap(), UpdateStatus::UpToDate { .. }));
        assert!(matches!(compare(&release("v0.10.0"), "0.9.0").unwrap(), UpdateStatus::Available { .. }));
    }

    #[test]
    fn pre_releases_sort_before_their_release() {
        assert!(version("1.0.0-beta.1") < version("1.0.0"));
        assert!(version("1.0.0-beta.1") > version("0.9.9"));
        assert_eq!(version("1.2"), Some((1, 2, 0, true)));
        assert_eq!(version("1.2.3+build.7"), Some((1, 2, 3, true)));
        assert_eq!(version("one"), None);
        assert_eq!(version("1.2.3.4"), None);
    }

    #[test]
    fn odd_answers_are_errors_not_offers() {
        assert!(compare("{}", "0.1.0").is_err());
        assert!(compare(&release("latest"), "0.1.0").is_err());
    }

    #[test]
    fn the_placeholder_repository_is_never_asked() {
        // Only while it is a placeholder: a real repository would mean a request from the tests.
        if RELEASES_REPO.starts_with(PLACEHOLDER_OWNER) {
            assert!(matches!(check(), UpdateStatus::Unavailable { .. }));
        }
    }
}
