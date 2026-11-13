//! Game pictures: item, monster and NPC icons and map pictures, as OSMS Data Explorer publishes
//! them (allowed since 2026-11-13).
//!
//! A picture is downloaded the first time a window shows it, never in bulk, and kept for good in
//! a folder of the app's local (never synced) data, named after a hash of its address. Only
//! OSMS's own host is asked; requests are plain GETs with a bare `Leafline/<version>` User-Agent
//! and nothing about the player, at most [`MAX_DOWNLOADS`] at a time. A download that fails is
//! not tried again for a while. No picture ever goes in the repo or the release: tests use
//! hand-made fake bytes.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};
use tauri::Url;

use super::fetch::Source;
use super::osms::Dataset;
use crate::error::{Error, Result};

/// The one host pictures may come from.
pub const PICTURE_HOST: &str = "osmsdataexplorer.com";

/// Downloads running at once. More wait their turn.
pub const MAX_DOWNLOADS: usize = 4;

/// A map picture is about 1 MB; anything far bigger is not a picture Leafline wants.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

/// How long a failed download waits before it is tried again, doubling with each failure in a
/// row up to [`LONGEST_WAIT`].
const FIRST_WAIT: Duration = Duration::from_secs(60);
const LONGEST_WAIT: Duration = Duration::from_secs(6 * 60 * 60);

/// What a picture shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PictureKind {
    Item,
    Mob,
    Npc,
    /// A whole map, drawn.
    Map,
    Minimap,
    /// The emblem of the area a map is in.
    Mark,
}

impl PictureKind {
    fn parse(text: &str) -> Option<Self> {
        Some(match text {
            "item" => Self::Item,
            "mob" => Self::Mob,
            "npc" => Self::Npc,
            "map" => Self::Map,
            "minimap" => Self::Minimap,
            "mark" => Self::Mark,
            _ => return None,
        })
    }
}

/// The address of every picture the data names, by what it shows, and the ids of names, for
/// the links that know only a name.
#[derive(Debug, Default)]
pub struct PictureIndex {
    urls: HashMap<(PictureKind, u32), String>,
    names: HashMap<(PictureKind, String), u32>,
}

fn name_key(name: &str) -> String {
    name.trim().to_lowercase()
}

impl PictureIndex {
    pub fn new(data: &Dataset) -> Self {
        let mut index = Self::default();
        let mut add = |kind: PictureKind, id: u32, name: Option<&str>, url: &Option<String>| {
            let Some(url) = url.as_deref().filter(|u| allowed(u)) else { return };
            index.urls.insert((kind, id), url.to_string());
            // The first record of a name with a picture stands for the name.
            if let Some(name) = name {
                index.names.entry((kind, name_key(name))).or_insert(id);
            }
        };
        for item in &data.items {
            add(PictureKind::Item, item.id, Some(&item.name), &item.icon_url);
        }
        for mob in &data.monsters {
            add(PictureKind::Mob, mob.id, Some(&mob.name), &mob.icon_url);
        }
        for npc in &data.npcs {
            add(PictureKind::Npc, npc.id, Some(&npc.name), &npc.icon_url);
        }
        for map in &data.maps {
            let name = Some(map.name.as_str());
            add(PictureKind::Map, map.id, name, &map.render_url);
            add(PictureKind::Minimap, map.id, name, &map.minimap_url);
            add(PictureKind::Mark, map.id, name, &map.map_mark_url);
        }
        index
    }

    /// The address a request's path names: `/item/123`, or `/item/name/Red%20Snail` for a name.
    pub fn resolve(&self, path: &str) -> Option<&str> {
        let mut parts = path.trim_start_matches('/').splitn(3, '/');
        let kind = PictureKind::parse(parts.next()?)?;
        let id = match (parts.next()?, parts.next()) {
            ("name", Some(name)) => {
                let name = percent_decode(name)?;
                *self.names.get(&(kind, name_key(&name)))?
            }
            (id, None) => id.parse().ok()?,
            _ => return None,
        };
        self.urls.get(&(kind, id)).map(String::as_str)
    }
}

/// `%20` and the like, as a link's path carries a name.
fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = text.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Whether a picture may be downloaded from `url`: HTTPS on OSMS's host and its standard port,
/// with no user name or password.
pub fn allowed(url: &str) -> bool {
    Url::parse(url).is_ok_and(|u| {
        u.scheme() == "https"
            && u.host_str() == Some(PICTURE_HOST)
            && u.port().is_none()
            && u.username().is_empty()
            && u.password().is_none()
    })
}

/// The file a picture is kept in: a hash of its address, so any address makes a safe name.
pub fn file_name(url: &str) -> String {
    let hash = Sha256::digest(url.as_bytes());
    let hex: String = hash.iter().take(16).map(|b| format!("{b:02x}")).collect();
    format!("{hex}.img")
}

/// The picture format `bytes` hold, by their first bytes, or `None` if they aren't a picture.
pub fn sniff(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Some("image/webp")
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else {
        None
    }
}

/// A picture ready to hand to a window.
#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    pub bytes: Vec<u8>,
    pub mime: &'static str,
}

/// When a download last failed and how long to wait before the next try.
#[derive(Debug, Clone, Copy)]
struct Failure {
    at: Instant,
    wait: Duration,
}

#[derive(Default)]
struct Running {
    count: usize,
    /// Addresses being downloaded, so a second window asking for one waits for the first.
    urls: HashSet<String>,
}

/// The pictures kept on this PC, and the downloads of the rest.
pub struct PictureCache<S: Source> {
    dir: PathBuf,
    source: S,
    running: Mutex<Running>,
    turn: Condvar,
    failures: Mutex<HashMap<String, Failure>>,
}

impl<S: Source> PictureCache<S> {
    pub fn new(dir: PathBuf, source: S) -> Self {
        Self { dir, source, running: Mutex::default(), turn: Condvar::new(), failures: Mutex::default() }
    }

    fn path(&self, url: &str) -> PathBuf {
        self.dir.join(file_name(url))
    }

    fn kept(path: &Path) -> Option<Picture> {
        let bytes = std::fs::read(path).ok()?;
        Some(Picture { mime: sniff(&bytes)?, bytes })
    }

    fn waiting(&self, url: &str) -> bool {
        let failures = self.failures.lock().expect("picture failures lock");
        failures.get(url).is_some_and(|f| f.at.elapsed() < f.wait)
    }

    /// The picture at `url`: from this PC if kept, else downloaded and kept.
    pub fn get(&self, url: &str) -> Result<Picture> {
        if !allowed(url) {
            return Err(Error::Invalid(format!("pictures come only from {PICTURE_HOST}")));
        }
        let path = self.path(url);
        {
            let mut running = self.running.lock().expect("picture downloads lock");
            loop {
                if let Some(picture) = Self::kept(&path) {
                    return Ok(picture);
                }
                if self.waiting(url) {
                    return Err(Error::Download(format!("{url}: failed recently, will try again later")));
                }
                if !running.urls.contains(url) && running.count < MAX_DOWNLOADS {
                    running.count += 1;
                    running.urls.insert(url.to_string());
                    break;
                }
                running = self.turn.wait(running).expect("picture downloads lock");
            }
        }
        let result = self.download(url, &path);
        {
            let mut failures = self.failures.lock().expect("picture failures lock");
            match &result {
                Ok(_) => {
                    failures.remove(url);
                }
                Err(_) => {
                    let wait = failures.get(url).map_or(FIRST_WAIT, |f| (f.wait * 2).min(LONGEST_WAIT));
                    failures.insert(url.to_string(), Failure { at: Instant::now(), wait });
                }
            }
        }
        let mut running = self.running.lock().expect("picture downloads lock");
        running.count -= 1;
        running.urls.remove(url);
        self.turn.notify_all();
        result
    }

    fn download(&self, url: &str, path: &Path) -> Result<Picture> {
        let bytes = self.source.get(url)?;
        if bytes.len() as u64 > MAX_BYTES {
            return Err(Error::Download(format!("{url}: too large for a picture")));
        }
        let mime = sniff(&bytes).ok_or_else(|| Error::Download(format!("{url}: not a picture")))?;
        std::fs::create_dir_all(&self.dir)?;
        // Written aside and then moved, so a half-written file is never taken for a picture.
        let partial = path.with_extension("part");
        std::fs::write(&partial, &bytes)?;
        std::fs::rename(&partial, path)?;
        Ok(Picture { bytes, mime })
    }
}

/// Plain HTTPS GETs with a bare `Leafline/<version>` User-Agent. Redirects are not followed, so
/// a request can't be sent on to another host.
pub struct PictureHttp {
    agent: ureq::Agent,
}

impl Default for PictureHttp {
    fn default() -> Self {
        let agent = ureq::AgentBuilder::new()
            .user_agent(concat!("Leafline/", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(20))
            .redirects(0)
            .build();
        Self { agent }
    }
}

impl Source for PictureHttp {
    fn get(&self, url: &str) -> Result<Vec<u8>> {
        let response = self.agent.get(url).call().map_err(|err| Error::Download(format!("{url}: {err}")))?;
        let mut body = Vec::new();
        response.into_reader().take(MAX_BYTES + 1).read_to_end(&mut body)?;
        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::super::osms::{ItemRecord, MapRecord, NpcRecord};
    use super::*;

    /// Hand-made bytes that only start like a PNG: no real picture is used in tests.
    const FAKE_PNG: &[u8] = b"\x89PNG\r\n\x1a\nnot really a picture";
    const ICON: &str = "https://osmsdataexplorer.com/data/current/images/items/1.png";

    /// Answers every request with `body`, counting them, after `delay`.
    struct Fake {
        body: Option<Vec<u8>>,
        calls: Arc<AtomicUsize>,
        delay: Duration,
    }

    impl Source for Fake {
        fn get(&self, _url: &str) -> Result<Vec<u8>> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(self.delay);
            self.body.clone().ok_or_else(|| Error::Download("offline".into()))
        }
    }

    fn cache(body: Option<&[u8]>, delay: Duration) -> (PictureCache<Fake>, Arc<AtomicUsize>, PathBuf) {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir()
            .join(format!("leafline-pictures-test-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::SeqCst)));
        let _ = std::fs::remove_dir_all(&dir);
        let calls = Arc::new(AtomicUsize::new(0));
        let fake = Fake { body: body.map(<[u8]>::to_vec), calls: calls.clone(), delay };
        (PictureCache::new(dir.clone(), fake), calls, dir)
    }

    #[test]
    fn only_osms_pictures_over_https_are_allowed() {
        assert!(allowed(ICON));
        assert!(!allowed("http://osmsdataexplorer.com/a.png"));
        assert!(!allowed("https://osmsdataexplorer.com.example.invalid/a.png"));
        assert!(!allowed("https://example.invalid/a.png"));
        assert!(!allowed("https://user@osmsdataexplorer.com/a.png"));
        assert!(!allowed("https://osmsdataexplorer.com:8443/a.png"));
        assert!(!allowed("not a url"));
    }

    #[test]
    fn tells_pictures_from_other_bytes() {
        assert_eq!(sniff(FAKE_PNG), Some("image/png"));
        assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));
        assert_eq!(sniff(b"GIF89a.."), Some("image/gif"));
        assert_eq!(sniff(b"<html>nope</html>"), None);
    }

    #[test]
    fn names_files_by_a_hash_of_the_address() {
        let name = file_name(ICON);
        assert_eq!(name.len(), 36);
        assert!(name.chars().all(|c| c.is_ascii_hexdigit() || c == '.' || c == 'i' || c == 'm' || c == 'g'));
        assert_ne!(name, file_name("https://osmsdataexplorer.com/data/current/images/items/2.png"));
    }

    #[test]
    fn downloads_once_and_keeps_the_picture() {
        let (cache, calls, dir) = cache(Some(FAKE_PNG), Duration::ZERO);
        assert_eq!(cache.get(ICON).unwrap(), Picture { bytes: FAKE_PNG.to_vec(), mime: "image/png" });
        assert_eq!(cache.get(ICON).unwrap().bytes, FAKE_PNG);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(dir.join(file_name(ICON)).exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn refuses_other_hosts_without_asking_them() {
        let (cache, calls, dir) = cache(Some(FAKE_PNG), Duration::ZERO);
        assert!(cache.get("https://example.invalid/a.png").is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn waits_after_a_failure_before_trying_again() {
        let (cache, calls, dir) = cache(None, Duration::ZERO);
        assert!(cache.get(ICON).is_err());
        assert!(cache.get(ICON).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1, "the second ask waits instead of downloading");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn keeps_nothing_that_is_not_a_picture() {
        let (cache, _, dir) = cache(Some(b"<html>error page</html>"), Duration::ZERO);
        assert!(cache.get(ICON).is_err());
        assert!(!dir.join(file_name(ICON)).exists());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn windows_asking_at_once_share_one_download() {
        let (cache, calls, dir) = cache(Some(FAKE_PNG), Duration::from_millis(50));
        let cache = Arc::new(cache);
        let threads: Vec<_> = (0..6)
            .map(|_| {
                let cache = cache.clone();
                std::thread::spawn(move || cache.get(ICON).map(|p| p.bytes))
            })
            .collect();
        for thread in threads {
            assert_eq!(thread.join().unwrap().unwrap(), FAKE_PNG);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn finds_addresses_by_id_or_name() {
        let item = |id, name: &str, url: Option<&str>| ItemRecord {
            icon_url: url.map(str::to_string),
            ..crate::build::catalog::tests::item(id, name, 0, &[])
        };
        let data = Dataset {
            items: vec![item(1, "Red Snail Shell", Some(ICON)), item(2, "Plain Cap", None)],
            npcs: vec![NpcRecord {
                id: 7,
                name: "Warden Ilse".into(),
                maps: vec![],
                icon_url: Some("https://example.invalid/npc.png".into()),
            }],
            maps: vec![MapRecord {
                render_url: Some("https://osmsdataexplorer.com/data/maps/abc.webp".into()),
                ..crate::mapdata::atlas::tests::map(5, "Mossbank", "Fern Road", vec![])
            }],
            ..Dataset::default()
        };
        let index = PictureIndex::new(&data);
        assert_eq!(index.resolve("/item/1"), Some(ICON));
        assert_eq!(index.resolve("/item/name/red%20snail%20shell"), Some(ICON));
        assert_eq!(index.resolve("/item/2"), None, "no picture in the data");
        assert_eq!(index.resolve("/npc/7"), None, "only OSMS addresses are kept");
        assert_eq!(index.resolve("/map/5"), Some("https://osmsdataexplorer.com/data/maps/abc.webp"));
        assert_eq!(index.resolve("/minimap/5"), None);
        assert_eq!(index.resolve("/hat/1"), None);
        assert_eq!(index.resolve("/item/1/extra"), None);
    }
}
