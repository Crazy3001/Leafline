//! Downloading OSMS files, as its README recommends: read `meta.json`, then fetch only the
//! files whose checksum differs from the cached copy.

use std::io::Read;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use sha2::{Digest, Sha256};

use super::osms::{FILES, Meta, Records, SCHEMA_VERSION};
use super::store::{Download, Store};
use crate::error::{Error, Result};

pub const OSMS_BASE: &str = "https://osmsdataexplorer.com/data/api/v1/";

/// Without a request from the user, OSMS is asked for changes at most this often.
pub const CHECK_INTERVAL: i64 = 24 * 60 * 60;

/// Identifies Leafline to the sites it reads, as the wiki's API etiquette asks.
pub const USER_AGENT: &str = concat!(
    "Leafline/",
    env!("CARGO_PKG_VERSION"),
    " (free, open-source, read-only overlay for MapleStory Classic; fetches on user request)"
);

/// The largest file Leafline will download. OSMS's files are well under 1 MB.
const MAX_BYTES: u64 = 32 * 1024 * 1024;

/// Somewhere to GET from. Tests stand in a fake.
pub trait Source {
    fn get(&self, url: &str) -> Result<Vec<u8>>;
}

/// HTTPS with Leafline's User-Agent, at most one request per `min_interval`.
pub struct Http {
    agent: ureq::Agent,
    min_interval: Duration,
    last: Mutex<Option<Instant>>,
}

impl Http {
    pub fn new(min_interval: Duration) -> Self {
        let agent = ureq::AgentBuilder::new().user_agent(USER_AGENT).timeout(Duration::from_secs(30)).build();
        Self { agent, min_interval, last: Mutex::new(None) }
    }
}

impl Source for Http {
    fn get(&self, url: &str) -> Result<Vec<u8>> {
        {
            // Held across the wait so concurrent callers queue up behind each other.
            let mut last = self.last.lock().expect("HTTP throttle lock");
            if let Some(wait) = last.and_then(|at| self.min_interval.checked_sub(at.elapsed())) {
                std::thread::sleep(wait);
            }
            *last = Some(Instant::now());
        }
        let response = self.agent.get(url).call().map_err(|err| Error::Download(format!("{url}: {err}")))?;
        let mut body = Vec::new();
        response.into_reader().take(MAX_BYTES).read_to_end(&mut body)?;
        Ok(body)
    }
}

/// What a refresh did.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshReport {
    /// `meta.json` was fetched. False when the last check was too recent.
    pub checked: bool,
    /// Files downloaded because they were new or had changed.
    pub downloaded: Vec<String>,
}

/// Brings the cache up to date with OSMS. Unless `force`d, does nothing if the last check was
/// under [`CHECK_INTERVAL`] ago. Every changed file is verified against its checksum and parsed
/// before any of them is stored.
pub fn refresh(store: &mut Store, source: &dyn Source, force: bool, now: i64) -> Result<RefreshReport> {
    let ready = store.status()?.ready;
    if !force && ready && store.checked_at()?.is_some_and(|at| now - at < CHECK_INTERVAL) {
        return Ok(RefreshReport::default());
    }
    let meta: Meta = serde_json::from_slice(&source.get(&format!("{OSMS_BASE}meta.json"))?)
        .map_err(|err| Error::Download(format!("meta.json is not in the expected format: {err}")))?;
    if meta.schema_version != SCHEMA_VERSION {
        return Err(Error::Download(format!(
            "OSMS data is now schema version {}; this version of Leafline reads version {SCHEMA_VERSION}",
            meta.schema_version
        )));
    }
    let have = store.file_hashes()?;
    let mut downloads = Vec::new();
    for name in FILES {
        let file = meta.files.get(name).ok_or_else(|| Error::Download(format!("meta.json does not list {name}")))?;
        if have.get(name).is_some_and(|sha| sha.eq_ignore_ascii_case(&file.sha256)) {
            continue;
        }
        let body = source.get(&format!("{OSMS_BASE}{name}"))?;
        if !sha256_hex(&body).eq_ignore_ascii_case(&file.sha256) {
            return Err(Error::Download(format!(
                "{name} did not match its checksum. OSMS may be mid-update; try again in a few minutes."
            )));
        }
        let records = Records::parse(name, &body)
            .map_err(|err| Error::Download(format!("{name} is not in the expected format: {err}")))?;
        downloads.push(Download { name: name.to_string(), meta: file.clone(), records });
    }
    store.apply(&meta, &downloads, now)?;
    Ok(RefreshReport { checked: true, downloaded: downloads.into_iter().map(|d| d.name).collect() })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;

    use super::*;

    /// Serves fixed bodies by URL and records what was asked for.
    #[derive(Default)]
    pub struct FakeSource {
        pub files: HashMap<String, Vec<u8>>,
        pub requests: RefCell<Vec<String>>,
    }

    impl Source for FakeSource {
        fn get(&self, url: &str) -> Result<Vec<u8>> {
            self.requests.borrow_mut().push(url.to_string());
            self.files.get(url).cloned().ok_or_else(|| Error::Download(format!("{url}: 404")))
        }
    }

    impl FakeSource {
        fn names(&self) -> Vec<String> {
            self.requests.borrow().iter().map(|u| u.trim_start_matches(OSMS_BASE).to_string()).collect()
        }
    }

    // Hand-made records; none of this is real game data.
    const MAPS: &str = r#"[{"id": 1, "name": "Mossbank", "street_name": "Fern Road", "is_town": true,
        "exits": [2], "portals": [{"name": "east00", "dest_map_id": 2, "dest_portal": "west00"}]},
        {"id": 2, "name": "Mossbank Meadow", "street_name": "Fern Road",
        "exits": [1], "portals": [{"name": "west00", "dest_map_id": 1, "dest_portal": "east00"}]}]"#;
    const MONSTERS: &str = r#"[{"id": 10, "name": "Pebble Snail", "level": 2, "hp": 20, "exp": 4, "spawns": [{"map_id": 2, "count": 6}]}]"#;
    const NPCS: &str = r#"[{"id": 50, "name": "Warden Ilse", "maps": [1]}]"#;
    const QUESTS: &str = r#"[{"id": 70, "name": "Shell Sorting", "npc_id": 50, "requirements": [{"type": "mob", "id": 10, "count": 8}]}]"#;
    const SKILLS: &str = r#"[{"id": 80, "name": "Pebble Toss", "job_id": 400, "class_name": "Rogue", "job_tier": "1st Job",
        "max_level": 2, "description": "Throws a pebble.", "required_skills": [{"skill_id": 79, "level": 1}],
        "level_descriptions": ["Damage 1", "Damage 2"]}]"#;
    const ITEMS: &str = r#"[{"id": 90, "name": "Twig Wand", "category": "Equipment", "sub_category": "Weapon",
        "req_classes": ["Magician", "Thief"], "stats": {"reqLevel": 8, "incMAD": 12}}]"#;

    pub fn fake_osms(maps: &str) -> FakeSource {
        let bodies = [
            ("maps.json", maps),
            ("monsters.json", MONSTERS),
            ("npcs.json", NPCS),
            ("quests.json", QUESTS),
            ("skills.json", SKILLS),
            ("items.json", ITEMS),
        ];
        let files: Vec<String> = bodies
            .iter()
            .map(|(name, body)| {
                format!(
                    r#""{name}": {{"sha256": "{}", "bytes": {}, "records": 1, "updated_at": "2026-01-01T00:00:00Z"}}"#,
                    sha256_hex(body.as_bytes()),
                    body.len()
                )
            })
            .collect();
        let meta = format!(
            r#"{{"schema_version": 1, "client_build": "fake", "data_updated_at": "2026-01-01T00:00:00Z", "files": {{{}}}}}"#,
            files.join(",")
        );
        let mut source = FakeSource::default();
        source.files.insert(format!("{OSMS_BASE}meta.json"), meta.into_bytes());
        for (name, body) in bodies {
            source.files.insert(format!("{OSMS_BASE}{name}"), body.as_bytes().to_vec());
        }
        source
    }

    #[test]
    fn first_refresh_downloads_everything() {
        let mut store = Store::open_in_memory().unwrap();
        let source = fake_osms(MAPS);
        let report = refresh(&mut store, &source, false, 1000).unwrap();
        assert_eq!(report.downloaded, FILES);
        assert_eq!(
            source.names(),
            ["meta.json", "maps.json", "monsters.json", "npcs.json", "quests.json", "skills.json", "items.json"]
        );

        let status = store.status().unwrap();
        assert!(status.ready);
        assert_eq!(status.client_build.as_deref(), Some("fake"));
        let data = store.load().unwrap().unwrap();
        assert_eq!(data.maps.len(), 2);
        assert_eq!(data.maps[0].portals[0].dest_map_id, Some(2));
        assert_eq!(data.monsters[0].spawns[0].count, 6);
        assert_eq!(data.quests[0].requirements[0].count, 8);
        assert_eq!(data.skills[0].required_skills[0].skill_id, 79);
        assert_eq!(data.skills[0].level_descriptions, ["Damage 1", "Damage 2"]);
        assert_eq!(data.items[0].classes, ["Magician", "Thief"]);
        assert_eq!(data.items[0].stats[0].value, 12);
    }

    #[test]
    fn checks_at_most_daily_unless_asked() {
        let mut store = Store::open_in_memory().unwrap();
        refresh(&mut store, &fake_osms(MAPS), false, 1000).unwrap();

        let source = fake_osms(MAPS);
        let report = refresh(&mut store, &source, false, 1000 + CHECK_INTERVAL - 1).unwrap();
        assert!(!report.checked);
        assert!(source.names().is_empty());

        // Asked for: only meta.json, since nothing changed.
        let report = refresh(&mut store, &source, true, 2000).unwrap();
        assert!(report.checked && report.downloaded.is_empty());
        assert_eq!(source.names(), ["meta.json"]);
    }

    #[test]
    fn fetches_only_changed_files() {
        let mut store = Store::open_in_memory().unwrap();
        refresh(&mut store, &fake_osms(MAPS), false, 1000).unwrap();

        let renamed = MAPS.replace("Mossbank Meadow", "Mossbank Fields");
        let source = fake_osms(&renamed);
        let report = refresh(&mut store, &source, false, 1000 + CHECK_INTERVAL).unwrap();
        assert_eq!(report.downloaded, ["maps.json"]);
        assert_eq!(source.names(), ["meta.json", "maps.json"]);
        assert_eq!(store.load().unwrap().unwrap().maps[1].name, "Mossbank Fields");
    }

    #[test]
    fn a_bad_checksum_stores_nothing() {
        let mut store = Store::open_in_memory().unwrap();
        let mut source = fake_osms(MAPS);
        source.files.insert(format!("{OSMS_BASE}npcs.json"), br#"[{"id": 1, "name": "Tampered"}]"#.to_vec());
        let err = refresh(&mut store, &source, false, 1000).unwrap_err();
        assert!(err.to_string().contains("npcs.json did not match its checksum"), "{err}");
        let status = store.status().unwrap();
        assert!(!status.ready && status.files.is_empty() && status.checked_at.is_none());
    }

    #[test]
    fn refuses_a_new_schema() {
        let mut store = Store::open_in_memory().unwrap();
        let mut source = fake_osms(MAPS);
        let meta = source.files.get_mut(&format!("{OSMS_BASE}meta.json")).unwrap();
        *meta =
            String::from_utf8(meta.clone()).unwrap().replace(r#""schema_version": 1"#, r#""schema_version": 2"#).into();
        assert!(refresh(&mut store, &source, true, 1000).unwrap_err().to_string().contains("schema version 2"));
    }
}
