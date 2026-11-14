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

use image::imageops::FilterType;
use serde::{Deserialize, Serialize};
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

/// A map picture is cut to what is drawn on it and drawn at most this wide for the windows.
const VIEW_WIDTH: u32 = 1200;
/// Room kept around what is drawn and around every marked point, in the picture's pixels.
const VIEW_MARGIN: i32 = 48;
/// Darker than this, or more see-through, counts as the empty space around a map.
const EMPTY_BELOW: u8 = 14;

/// How a map's picture was cut down for the windows: the part kept, in the picture's own pixels
/// (where OSMS's coordinates are), and the size it is drawn at.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MapView {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
    pub out_width: u32,
    pub out_height: u32,
}

impl MapView {
    /// Where a point of the full picture falls on the view, from 0 to 1 across and down, or
    /// `None` when it was cut off.
    pub fn place(&self, x: i32, y: i32) -> Option<(f64, f64)> {
        let fx = f64::from(x - self.left) / f64::from(self.width);
        let fy = f64::from(y - self.top) / f64::from(self.height);
        ((0.0..=1.0).contains(&fx) && (0.0..=1.0).contains(&fy)).then_some((fx, fy))
    }
}

/// Cuts a map's picture to what is drawn on it, keeping every marked point in view, and draws it
/// at most [`VIEW_WIDTH`] wide as a JPEG.
pub fn cut_map(bytes: &[u8], points: &[(i32, i32)]) -> Result<(Vec<u8>, MapView)> {
    let picture = image::load_from_memory(bytes)?.to_rgba8();
    let (full_w, full_h) = (picture.width() as i32, picture.height() as i32);
    let (mut left, mut top, mut right, mut bottom) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for (x, y, pixel) in picture.enumerate_pixels().filter(|(x, y, _)| x % 2 == 0 && y % 2 == 0) {
        let [r, g, b, a] = pixel.0;
        if a > EMPTY_BELOW && r.max(g).max(b) > EMPTY_BELOW {
            let (x, y) = (x as i32, y as i32);
            (left, top, right, bottom) = (left.min(x), top.min(y), right.max(x), bottom.max(y));
        }
    }
    if left > right {
        (left, top, right, bottom) = (0, 0, full_w - 1, full_h - 1);
    }
    for &(x, y) in points.iter().filter(|(x, y)| (0..full_w).contains(x) && (0..full_h).contains(y)) {
        (left, top, right, bottom) = (left.min(x), top.min(y), right.max(x), bottom.max(y));
    }
    let left = (left - VIEW_MARGIN).max(0);
    let top = (top - VIEW_MARGIN).max(0);
    let right = (right + VIEW_MARGIN).min(full_w - 1);
    let bottom = (bottom + VIEW_MARGIN).min(full_h - 1);
    let (width, height) = ((right - left + 1) as u32, (bottom - top + 1) as u32);
    let cut = image::imageops::crop_imm(&picture, left as u32, top as u32, width, height).to_image();
    let (out_width, out_height) = if width > VIEW_WIDTH {
        (VIEW_WIDTH, ((u64::from(height) * u64::from(VIEW_WIDTH)) / u64::from(width)).max(1) as u32)
    } else {
        (width, height)
    };
    let drawn = image::imageops::resize(&cut, out_width, out_height, FilterType::Triangle);
    let rgb = image::DynamicImage::ImageRgba8(drawn).to_rgb8();
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 85).encode_image(&rgb)?;
    Ok((jpeg, MapView { left, top, width, height, out_width, out_height }))
}

/// What a picture shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PictureKind {
    Item,
    Mob,
    Npc,
    Skill,
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
            "skill" => Self::Skill,
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
        for skill in &data.skills {
            add(PictureKind::Skill, skill.id, Some(&skill.name), &skill.icon_url);
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

    fn view_paths(&self, url: &str) -> (PathBuf, PathBuf) {
        let stem = self.dir.join(file_name(url));
        (stem.with_extension("view.jpg"), stem.with_extension("view.json"))
    }

    /// A map's picture cut down for the windows (see [`cut_map`]), made once and kept. The full
    /// picture is let go once it is cut, since only the cut one is shown.
    pub fn map_view(&self, url: &str, points: &[(i32, i32)]) -> Result<MapView> {
        let (picture, about) = self.view_paths(url);
        if picture.exists()
            && let Some(view) = std::fs::read(&about).ok().and_then(|json| serde_json::from_slice(&json).ok())
        {
            return Ok(view);
        }
        let full = self.get(url)?;
        let (jpeg, view) = cut_map(&full.bytes, points)?;
        let partial = picture.with_extension("part");
        std::fs::write(&partial, &jpeg)?;
        std::fs::rename(&partial, &picture)?;
        std::fs::write(&about, serde_json::to_vec(&view).map_err(|err| Error::Invalid(err.to_string()))?)?;
        let _ = std::fs::remove_file(self.path(url));
        Ok(view)
    }

    /// The cut-down picture [`map_view`](Self::map_view) made.
    pub fn map_view_picture(&self, url: &str) -> Option<Picture> {
        Self::kept(&self.view_paths(url).0)
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

    use super::super::osms::{ItemRecord, MapRecord, NpcRecord, SkillRecord};
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

    /// A hand-made picture: black, with a lit block from (60, 30) to (99, 49).
    fn fake_map() -> Vec<u8> {
        let mut picture = image::RgbaImage::from_pixel(400, 200, image::Rgba([0, 0, 0, 255]));
        for x in 60..100 {
            for y in 30..50 {
                picture.put_pixel(x, y, image::Rgba([200, 180, 90, 255]));
            }
        }
        let mut png = Vec::new();
        picture.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
        png
    }

    #[test]
    fn cuts_a_map_to_what_is_drawn_and_every_point() {
        let (jpeg, view) = cut_map(&fake_map(), &[]).unwrap();
        assert_eq!(sniff(&jpeg), Some("image/jpeg"));
        assert_eq!((view.left, view.top), (60 - VIEW_MARGIN, 0), "the margin, held inside the picture");
        assert_eq!(view.left + view.width as i32 - 1, 98 + VIEW_MARGIN);
        assert_eq!((view.out_width, view.out_height), (view.width, view.height), "small enough to keep its size");
        // A point out in the dark keeps its part of the picture; one off the picture is ignored.
        let (_, view) = cut_map(&fake_map(), &[(300, 150), (900, 900)]).unwrap();
        assert_eq!(view.left + view.width as i32 - 1, 300 + VIEW_MARGIN);
        assert_eq!(view.top + view.height as i32 - 1, 150 + VIEW_MARGIN);
        let (fx, fy) = view.place(300, 150).unwrap();
        assert!(fx > 0.8 && fy > 0.7);
        assert_eq!(view.place(0, 0), None, "cut off");
    }

    #[test]
    fn keeps_a_cut_map_and_lets_the_full_picture_go() {
        let png = fake_map();
        let (cache, calls, dir) = cache(Some(&png), Duration::ZERO);
        let url = "https://osmsdataexplorer.com/data/maps/fake.webp";
        let view = cache.map_view(url, &[]).unwrap();
        assert_eq!(cache.map_view(url, &[]).unwrap(), view);
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(!dir.join(file_name(url)).exists(), "only the cut picture is kept");
        assert_eq!(cache.map_view_picture(url).unwrap().mime, "image/jpeg");
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
            skills: vec![SkillRecord {
                icon_url: Some("https://osmsdataexplorer.com/data/current/images/skills/0001000.png".into()),
                ..crate::build::catalog::tests::skill(1000, "Pebble Toss", 0, 3, &[])
            }],
            ..Dataset::default()
        };
        let index = PictureIndex::new(&data);
        assert_eq!(index.resolve("/skill/1000"), Some("https://osmsdataexplorer.com/data/current/images/skills/0001000.png"));
        assert_eq!(index.resolve("/skill/name/pebble%20toss"), index.resolve("/skill/1000"));
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
