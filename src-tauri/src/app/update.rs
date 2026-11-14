//! The optional update check, and the update itself. When the player turns the check on (in setup
//! or Settings), Leafline asks GitHub once at startup for the latest release and says in Settings
//! if it is newer. The request carries Leafline's own version in its user agent and nothing else:
//! nothing read from the game, nothing about the player or their PC.
//!
//! Only when the player then presses **Download and install** does Leafline download that
//! release's setup program and its `SHA256SUMS.txt`, check the program against its line there,
//! and run it if they match. Leafline closes so the installer can replace it. A program that
//! doesn't match is deleted and never run. The checksum comes from the same release, so it
//! catches a broken or swapped download, not a release published by someone else.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
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
/// The checksum list is a few lines.
const MAX_SUMS_BYTES: u64 = 64 * 1024;
/// The installer is a few megabytes; past this, it isn't one.
const MAX_INSTALLER_BYTES: u64 = 300 * 1024 * 1024;
/// The release's installer: the NSIS setup program, which installs over the old version.
const INSTALLER_SUFFIX: &str = "_x64-setup.exe";
/// The checksum list the release workflow publishes beside the installers.
const SUMS_NAME: &str = "SHA256SUMS.txt";
/// Progress is sent to the windows this often while downloading, in bytes.
const PROGRESS_STEP: u64 = 256 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum UpdateStatus {
    /// The player hasn't turned the check on.
    #[default]
    Off,
    Checking,
    /// This is the latest release, or newer.
    UpToDate { current: String, latest: String },
    Available {
        current: String,
        latest: String,
        url: String,
        /// The release has a setup program and checksums, so it can be downloaded and installed.
        installable: bool,
        /// Why the last download and install didn't go through.
        problem: Option<String>,
    },
    /// The setup program is downloading; `total` when GitHub gave its size.
    Downloading { current: String, latest: String, received: u64, total: Option<u64> },
    /// The setup program matched its checksum and was started; Leafline is closing.
    Installing { latest: String },
    /// Nothing to compare with: no repository set yet, or no release published there.
    Unavailable { current: String, reason: String },
    /// Offline, or GitHub didn't answer as expected. Tried again at the next start.
    Failed { current: String, message: String },
}

#[derive(Default)]
pub struct UpdateState {
    status: Mutex<UpdateStatus>,
    /// Where the newer release's setup program and checksums are, once a check found them.
    installer: Mutex<Option<Installer>>,
}

impl UpdateState {
    fn get(&self) -> UpdateStatus {
        self.status.lock().expect("update lock").clone()
    }
}

/// The parts of GitHub's release JSON that matter here.
#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: Option<u64>,
}

/// A release's setup program and the checksum list that vouches for it.
#[derive(Debug, Clone, PartialEq)]
struct Installer {
    name: String,
    url: String,
    size: Option<u64>,
    sums_url: String,
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
        let (status, installer) = check();
        // Turned off while the request was out: say nothing about it.
        if app.state::<SettingsState>().get().update_check {
            *app.state::<UpdateState>().installer.lock().expect("update lock") = installer;
            set(&app, status);
        }
    });
}

/// Forgets the last result, when the player turns the check off.
pub fn turn_off(app: &AppHandle) {
    *app.state::<UpdateState>().installer.lock().expect("update lock") = None;
    set(app, UpdateStatus::Off);
}

fn set(app: &AppHandle, status: UpdateStatus) {
    *app.state::<UpdateState>().status.lock().expect("update lock") = status.clone();
    let _ = app.emit(UPDATE_EVENT, &status);
}

/// Asks GitHub for the latest release, and where a newer one's installer is. Blocks for up to the
/// timeout.
fn check() -> (UpdateStatus, Option<Installer>) {
    let current = CURRENT.to_string();
    let Some((owner, _)) = RELEASES_REPO.split_once('/') else {
        return (UpdateStatus::Unavailable { current, reason: "No release page is set for this build.".into() }, None);
    };
    if owner == PLACEHOLDER_OWNER {
        return (UpdateStatus::Unavailable { current, reason: "This build has no release page to check yet.".into() }, None);
    }
    let url = format!("https://api.github.com/repos/{RELEASES_REPO}/releases/latest");
    let agent = ureq::AgentBuilder::new().user_agent(USER_AGENT).timeout(Duration::from_secs(10)).build();
    let response = agent
        .get(&url)
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .call();
    let failed = |message: String| (UpdateStatus::Failed { current: CURRENT.to_string(), message }, None);
    let body = match response {
        Ok(response) => {
            let mut body = String::new();
            match response.into_reader().take(MAX_BYTES).read_to_string(&mut body) {
                Ok(_) => body,
                Err(err) => return failed(err.to_string()),
            }
        }
        // A private repository, or one with no release yet, answers 404.
        Err(ureq::Error::Status(404, _)) => {
            return (UpdateStatus::Unavailable { current, reason: "No release has been published yet.".into() }, None);
        }
        Err(ureq::Error::Status(code, _)) => return failed(format!("GitHub answered {code}.")),
        Err(err) => return failed(format!("Couldn't reach GitHub ({err}).")),
    };
    compare(&body, CURRENT).unwrap_or_else(|err| failed(err.to_string()))
}

/// What a release JSON means for a build of version `current`, and where a newer one's installer is.
fn compare(json: &str, current: &str) -> Result<(UpdateStatus, Option<Installer>)> {
    let release: Release =
        serde_json::from_str(json).map_err(|_| Error::Invalid("GitHub's answer wasn't a release.".into()))?;
    let latest = release.tag_name.trim().trim_start_matches(['v', 'V']).to_string();
    let (Some(new), Some(old)) = (version(&latest), version(current)) else {
        return Err(Error::Invalid(format!("Couldn't read the version \"{}\".", release.tag_name)));
    };
    let current = current.to_string();
    if new <= old {
        return Ok((UpdateStatus::UpToDate { current, latest }, None));
    }
    let exe = release.assets.iter().find(|a| a.name.ends_with(INSTALLER_SUFFIX));
    let sums = release.assets.iter().find(|a| a.name == SUMS_NAME);
    let installer = exe.zip(sums).map(|(exe, sums)| Installer {
        name: exe.name.clone(),
        url: exe.browser_download_url.clone(),
        size: exe.size,
        sums_url: sums.browser_download_url.clone(),
    });
    let status = UpdateStatus::Available {
        current,
        latest,
        url: release.html_url,
        installable: installer.is_some(),
        problem: None,
    };
    Ok((status, installer))
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

/// The SHA-256 a checksum list gives for `name`, in lower case: lines of `<hex>  <file name>`.
fn sum_for(list: &str, name: &str) -> Option<String> {
    list.lines().find_map(|line| {
        let (hash, file) = line.trim().split_once(char::is_whitespace)?;
        let file = file.trim().trim_start_matches('*');
        let valid = hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit());
        (file == name && valid).then(|| hash.to_ascii_lowercase())
    })
}

/// Copies `from` into `to` while hashing it, up to `limit` bytes, calling `progress` as it goes.
/// Returns the SHA-256 in lower-case hex.
fn copy_hashing(from: &mut dyn Read, to: &mut dyn Write, limit: u64, mut progress: impl FnMut(u64)) -> Result<String> {
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 64 * 1024];
    let (mut received, mut told) = (0u64, 0u64);
    loop {
        let n = from.read(&mut buf).map_err(|e| Error::Download(format!("The download broke off ({e}).")))?;
        if n == 0 {
            break;
        }
        received += n as u64;
        if received > limit {
            return Err(Error::Download("The download is far bigger than an installer.".into()));
        }
        hasher.update(&buf[..n]);
        to.write_all(&buf[..n])?;
        if received - told >= PROGRESS_STEP {
            told = received;
            progress(received);
        }
    }
    progress(received);
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn download(agent: &ureq::Agent, url: &str) -> Result<ureq::Response> {
    agent.get(url).call().map_err(|err| match err {
        ureq::Error::Status(code, _) => Error::Download(format!("GitHub answered {code}.")),
        err => Error::Download(format!("Couldn't reach GitHub ({err}).")),
    })
}

/// Downloads the checksum list and the installer into `folder`, and keeps the installer only if
/// it matches its line in the list.
fn fetch_installer(installer: &Installer, folder: &Path, progress: impl FnMut(u64)) -> Result<PathBuf> {
    let agent = ureq::AgentBuilder::new().user_agent(USER_AGENT).timeout_connect(Duration::from_secs(10)).build();
    let mut sums = String::new();
    download(&agent, &installer.sums_url)?
        .into_reader()
        .take(MAX_SUMS_BYTES)
        .read_to_string(&mut sums)
        .map_err(|e| Error::Download(format!("The checksum list broke off ({e}).")))?;
    let expected = sum_for(&sums, &installer.name)
        .ok_or_else(|| Error::Download(format!("The release's {SUMS_NAME} has no line for {}.", installer.name)))?;

    std::fs::create_dir_all(folder)?;
    let path = folder.join(&installer.name);
    let result = (|| {
        let mut file = std::fs::File::create(&path)?;
        let mut body = download(&agent, &installer.url)?.into_reader();
        let hash = copy_hashing(&mut body, &mut file, MAX_INSTALLER_BYTES, progress)?;
        file.sync_all()?;
        if hash != expected {
            return Err(Error::Download(
                "The download didn't match the release's SHA-256, so it was deleted and not run.".into(),
            ));
        }
        Ok(())
    })();
    match result {
        Ok(()) => Ok(path),
        Err(err) => {
            let _ = std::fs::remove_file(&path);
            Err(err)
        }
    }
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

/// Downloads the newer release's setup program, checks it against the release's checksums, starts
/// it and closes Leafline. Only when the player asks, from Settings, after a check found it.
#[tauri::command]
pub fn install_update(app: AppHandle) -> Result<()> {
    if !app.state::<SettingsState>().get().update_check {
        return Err(Error::Invalid("Turn on the update check first.".into()));
    }
    let state = app.state::<UpdateState>();
    let Some(installer) = state.installer.lock().expect("update lock").clone() else {
        return Err(Error::Invalid("There is no update to install. Check again first.".into()));
    };
    let UpdateStatus::Available { current, latest, url, installable, .. } = state.get() else {
        return Err(Error::Invalid("The update is already on its way.".into()));
    };
    // Kept in the app's local folder, which isn't synced, and cleared each time.
    let folder = app.path().app_local_data_dir()?.join("updates");
    let downloading = {
        let (current, latest, total) = (current.clone(), latest.clone(), installer.size);
        move |received| UpdateStatus::Downloading { current: current.clone(), latest: latest.clone(), received, total }
    };
    set(&app, downloading(0));
    std::thread::spawn(move || {
        let _ = std::fs::remove_dir_all(&folder);
        let fetched = fetch_installer(&installer, &folder, |received| set(&app, downloading(received)));
        let started = fetched.and_then(|path| {
            std::process::Command::new(&path)
                .spawn()
                .map(|_| ())
                .map_err(|e| Error::Invalid(format!("The installer didn't start ({e}).")))
        });
        match started {
            Ok(()) => {
                set(&app, UpdateStatus::Installing { latest: latest.clone() });
                // A moment for the windows to say so, then out of the installer's way.
                std::thread::sleep(Duration::from_millis(1500));
                app.exit(0);
            }
            Err(err) => set(
                &app,
                UpdateStatus::Available {
                    current: current.clone(),
                    latest: latest.clone(),
                    url,
                    installable,
                    problem: Some(err.to_string()),
                },
            ),
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str) -> String {
        format!(r#"{{"tag_name":"{tag}","html_url":"https://github.com/o/r/releases/tag/{tag}","assets":[]}}"#)
    }

    fn status(json: &str, current: &str) -> UpdateStatus {
        compare(json, current).unwrap().0
    }

    #[test]
    fn a_newer_release_is_offered_and_an_older_or_equal_one_is_not() {
        assert!(matches!(status(&release("v0.2.0"), "0.1.0"), UpdateStatus::Available { latest, .. } if latest == "0.2.0"));
        assert!(matches!(status(&release("0.1.0"), "0.1.0"), UpdateStatus::UpToDate { .. }));
        assert!(matches!(status(&release("v0.0.9"), "0.1.0"), UpdateStatus::UpToDate { .. }));
        assert!(matches!(status(&release("v0.10.0"), "0.9.0"), UpdateStatus::Available { .. }));
    }

    #[test]
    fn a_release_with_a_setup_program_and_checksums_can_be_installed() {
        let asset = |name: &str| format!(r#"{{"name":"{name}","browser_download_url":"https://x/{name}","size":5}}"#);
        let json = |assets: &[String]| format!(r#"{{"tag_name":"v0.4.0","html_url":"https://x/r","assets":[{}]}}"#, assets.join(","));
        let full = json(&[asset("Leafline_0.4.0_x64_en-US.msi"), asset("Leafline_0.4.0_x64-setup.exe"), asset("SHA256SUMS.txt")]);
        let (found, installer) = compare(&full, "0.3.0").unwrap();
        assert!(matches!(found, UpdateStatus::Available { installable: true, .. }));
        let installer = installer.unwrap();
        assert_eq!((installer.name.as_str(), installer.size), ("Leafline_0.4.0_x64-setup.exe", Some(5)));
        assert_eq!(installer.sums_url, "https://x/SHA256SUMS.txt");
        let (found, installer) = compare(&json(&[asset("Leafline_0.4.0_x64-setup.exe")]), "0.3.0").unwrap();
        assert!(matches!(found, UpdateStatus::Available { installable: false, .. }), "no checksums, no install");
        assert!(installer.is_none());
    }

    #[test]
    fn the_checksum_list_gives_each_files_hash() {
        let a = "a".repeat(64);
        let list = format!("{a}  Leafline_0.4.0_x64-setup.exe\n{}  Leafline_0.4.0_x64_en-US.msi\n", "B".repeat(64));
        assert_eq!(sum_for(&list, "Leafline_0.4.0_x64-setup.exe"), Some(a));
        assert_eq!(sum_for(&list, "Leafline_0.4.0_x64_en-US.msi"), Some("b".repeat(64)));
        assert_eq!(sum_for(&list, "other.exe"), None);
        assert_eq!(sum_for("short  Leafline.exe", "Leafline.exe"), None, "not a SHA-256");
    }

    #[test]
    fn a_download_is_hashed_as_it_is_written_and_capped() {
        let mut out = Vec::new();
        let hash = copy_hashing(&mut &b"abc"[..], &mut out, 10, |_| {}).unwrap();
        assert_eq!(hash, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(out, b"abc");
        assert!(copy_hashing(&mut &[0u8; 20][..], &mut Vec::new(), 10, |_| {}).is_err(), "past the cap");
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
            assert!(matches!(check().0, UpdateStatus::Unavailable { .. }));
        }
    }
}
