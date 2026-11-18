//! Part B: the program's frames and the browser's, saved side by side while the game runs, read
//! by the same readers and compared. Frames are paired by the time each side saved them, within
//! [`PAIRED_WITHIN_MS`]; each reader's agreement is counted over the pairs where either side read
//! something.
//!
//! The program saves `app-<unix ms>.png` and the website `web-<unix ms>.png` into one folder the
//! player picks. Nothing here sends them anywhere; the report is written next to them.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use image::imageops::{self, FilterType};
use serde::Serialize;

use crate::error::{Error, Result};
use crate::pipeline::{self, Analysis};
use crate::readers::pickup::{Gain, PickupReader};
use crate::readers::{hud, ui};
use crate::vision::{self, Frame, Ocr};

/// The most two frames' times may differ to be compared.
pub const PAIRED_WITHIN_MS: u64 = 100;
/// The share of pairs a reader must agree on.
pub const PASS_SHARE: f32 = 0.99;
/// How far apart in time the two sides may count the same Pickup Log line.
const PICKUP_WITHIN: f64 = 1.5;
/// Disagreements kept per reader, for looking at the frames.
const EXAMPLES: usize = 12;

/// The file the report is written to, in the frames' folder.
pub const REPORT_FILE: &str = "leafline-capture-report.json";

/// A saved frame and when it was taken.
#[derive(Debug, Clone, PartialEq)]
pub struct Saved {
    pub path: PathBuf,
    pub at_ms: u64,
}

/// The program's frames and the website's in `folder`, each in time order.
pub fn saved(folder: &Path) -> Result<(Vec<Saved>, Vec<Saved>)> {
    let (mut app, mut web) = (Vec::new(), Vec::new());
    for entry in std::fs::read_dir(folder)? {
        let path = entry?.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        let Some(stem) = name.strip_suffix(".png") else { continue };
        let (list, ms) = match (stem.strip_prefix("app-"), stem.strip_prefix("web-")) {
            (Some(ms), _) => (&mut app, ms),
            (_, Some(ms)) => (&mut web, ms),
            _ => continue,
        };
        if let Ok(at_ms) = ms.parse() {
            list.push(Saved { path, at_ms });
        }
    }
    app.sort_by_key(|s| s.at_ms);
    web.sort_by_key(|s| s.at_ms);
    Ok((app, web))
}

/// Pairs of `(app, web)` indexes no more than `within` ms apart, each frame in one pair at most,
/// the closest first. Both lists are in time order.
pub fn pairs(app: &[Saved], web: &[Saved], within: u64) -> Vec<(usize, usize)> {
    let mut candidates = Vec::new();
    for (w, frame) in web.iter().enumerate() {
        let next = app.partition_point(|a| a.at_ms < frame.at_ms);
        for a in [next.checked_sub(1), Some(next), Some(next + 1)].into_iter().flatten().filter(|&a| a < app.len()) {
            let gap = app[a].at_ms.abs_diff(frame.at_ms);
            if gap <= within {
                candidates.push((gap, a, w));
            }
        }
    }
    candidates.sort_unstable();
    let (mut app_used, mut web_used) = (vec![false; app.len()], vec![false; web.len()]);
    let mut paired = Vec::new();
    for (_, a, w) in candidates {
        if !app_used[a] && !web_used[w] {
            app_used[a] = true;
            web_used[w] = true;
            paired.push((a, w));
        }
    }
    paired.sort_unstable();
    paired
}

/// What each reader made of one frame, as text to compare.
#[derive(Debug, Clone, Default, PartialEq)]
struct Readings {
    values: [Option<String>; READERS.len()],
}

/// The readers compared frame by frame, the four the pass bar needs first. The Pickup Log is
/// compared as a stream instead (see [`PickupScore`]).
/// The Quest Helper is judged on its counters and targets, which the session uses; the titles as
/// OCR reads them are listed apart, since the program matches them to known quests afterwards.
const READERS: [(&str, bool); 8] = [
    ("Level", true),
    ("EXP", true),
    ("Mesos", true),
    ("Map name", true),
    ("Quest Helper", false),
    ("Quest titles (raw)", false),
    ("Item tooltip", false),
    ("Minimap", false),
];

fn readings(analysis: &Analysis) -> Readings {
    let hud = analysis.hud.as_ref();
    let exp = hud.and_then(|h| {
        h.exp.map(|e| format!("{} ({:.2}%)", e.points, e.percent)).or(h.exp_percent.map(|p| format!("{p:.2}%")))
    });
    let quests = analysis.quest.as_ref().filter(|q| !q.quests.is_empty());
    let objectives = quests.map(|q| {
        q.quests
            .iter()
            .flat_map(|quest| quest.objectives.iter().map(|o| format!("{}/{} {}", o.current, o.required, o.target)))
            .collect::<Vec<_>>()
            .join(" | ")
    });
    let titles = quests.map(|q| q.quests.iter().map(|quest| quest.title.as_str()).collect::<Vec<_>>().join(" | "));
    let mut names: Vec<String> = analysis.tooltips.iter().map(|t| t.name.clone()).collect();
    names.sort();
    let minimap = analysis.map.as_ref().map(|m| {
        let round = |v: f32| (v / 2.0).round() as i32 * 2;
        format!("{} at {},{}", if m.collapsed { "collapsed" } else { "expanded" }, round(m.rect.x), round(m.rect.y))
    });
    Readings {
        values: [
            hud.and_then(|h| h.level).map(|l| l.to_string()),
            exp,
            analysis.mesos.map(|m| m.mesos.to_string()),
            analysis.map.as_ref().map(|m| m.label()),
            objectives,
            titles,
            (!names.is_empty()).then(|| names.join(" | ")),
            minimap,
        ],
    }
}

/// One reader's agreement over the pairs.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaderScore {
    pub reader: &'static str,
    /// Part of the pass bar (level, EXP, mesos, map name).
    pub gate: bool,
    pub agree: u32,
    pub differ: u32,
    pub only_program: u32,
    pub only_website: u32,
    /// Pairs where neither side read anything; left out of the share.
    pub neither: u32,
    /// Agreeing pairs over pairs where either side read something.
    pub share: Option<f32>,
    pub passes: Option<bool>,
    pub examples: Vec<Difference>,
}

/// A pair the two sides read differently.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Difference {
    pub program_frame: String,
    pub website_frame: String,
    pub program: Option<String>,
    pub website: Option<String>,
}

/// The Pickup Log, read as each side's stream of frames.
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickupScore {
    pub program_lines: usize,
    pub website_lines: usize,
    /// Lines both counted, alike, within a second and a half of each other.
    pub matched: usize,
    pub share: Option<f32>,
    pub passes: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Agreement {
    pub program_frames: usize,
    pub website_frames: usize,
    pub pairs: usize,
    /// How the website's frames were fitted to the program's: cut by this offset (a title bar or
    /// border the browser kept), or resized.
    pub website_offset: Option<(u32, u32)>,
    pub website_resized: bool,
    pub readers: Vec<ReaderScore>,
    pub pickup: PickupScore,
    /// Every reader of the pass bar agrees on [`PASS_SHARE`] or more of its pairs.
    pub passes: bool,
}

/// How the browser's frames line up with the program's: the offset in the browser's frame that
/// matches best, when it is at least as big. `None` means it must be resized.
fn alignment(app: &Frame, web: &Frame) -> Option<(u32, u32)> {
    let (aw, ah) = app.dimensions();
    let (ww, wh) = web.dimensions();
    if ww < aw || wh < ah {
        return None;
    }
    if (ww, wh) == (aw, ah) {
        return Some((0, 0));
    }
    let luma = |f: &Frame, x: u32, y: u32| i32::from(vision::preprocess::luminance(f.get_pixel(x, y)));
    let mut best: Option<(i64, (u32, u32))> = None;
    for dy in 0..=(wh - ah).min(200) {
        for dx in 0..=(ww - aw).min(200) {
            let mut sum = 0i64;
            for y in (0..ah).step_by(16) {
                for x in (0..aw).step_by(16) {
                    sum += i64::from((luma(app, x, y) - luma(web, x + dx, y + dy)).abs());
                }
                if best.is_some_and(|(least, _)| sum >= least) {
                    break;
                }
            }
            if best.is_none_or(|(least, _)| sum < least) {
                best = Some((sum, (dx, dy)));
            }
        }
    }
    best.map(|(_, at)| at)
}

/// A browser frame brought to `size`: cut at `offset`, or resized when it can't be cut.
fn fitted(web: Frame, size: (u32, u32), offset: Option<(u32, u32)>) -> Frame {
    match offset {
        _ if web.dimensions() == size => web,
        Some((x, y)) => imageops::crop_imm(&web, x, y, size.0, size.1).to_image(),
        None => imageops::resize(&web, size.0, size.1, FilterType::Triangle),
    }
}

/// One side read: the paired frames' readings by index, and the Pickup Log's lines with their times.
type Side = (HashMap<usize, Readings>, Vec<(f64, Gain)>);

/// Reads one side's frames in order: each paired frame in full, every frame's Pickup Log.
/// Returns the paired frames' readings by index and the Pickup Log's lines.
fn read_side(
    frames: &[Saved],
    paired: &[bool],
    ocr: &Ocr,
    mut load: impl FnMut(&Saved) -> Result<Frame>,
    mut done: impl FnMut(),
) -> Result<Side> {
    let mut readings = HashMap::new();
    let mut pickup = PickupReader::default();
    let mut lines = Vec::new();
    let mut ui_scale = None;
    for (i, saved) in frames.iter().enumerate() {
        let frame = load(saved)?;
        if paired[i] {
            let analysis = pipeline::analyze(&frame, ocr)?;
            if let Some(layout) = &analysis.hud_layout {
                ui_scale = Some(ui::hud_scale(&frame, layout));
            }
            readings.insert(i, self::readings(&analysis));
        } else if let Some(layout) = hud::locate(&frame) {
            ui_scale = Some(ui::hud_scale(&frame, &layout));
        }
        let read = pickup.read(saved.at_ms as f64 / 1000.0, &frame, ui_scale, ocr);
        lines.extend(read.arrivals.into_iter().filter_map(|a| Some((a.at, a.line?.gain))));
        done();
    }
    lines.extend(pickup.finish().into_iter().filter_map(|a| Some((a.at, a.line?.gain))));
    Ok((readings, lines))
}

fn name(saved: &Saved) -> String {
    saved.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// Compares the frames in `folder`. `progress` hears frames read so far and the total.
pub fn compare(folder: &Path, ocr: &Ocr, mut progress: impl FnMut(usize, usize)) -> Result<Agreement> {
    let (app, web) = saved(folder)?;
    if app.is_empty() || web.is_empty() {
        return Err(Error::Invalid(format!(
            "The folder needs both the program's frames (app-….png) and the website's (web-….png); it has {} and {}.",
            app.len(),
            web.len()
        )));
    }
    let paired = pairs(&app, &web, PAIRED_WITHIN_MS);
    if paired.is_empty() {
        return Err(Error::Invalid(
            "No program frame was saved within 0.1 s of a website frame. Were both saving at the same time?".into(),
        ));
    }
    let (mut app_paired, mut web_paired) = (vec![false; app.len()], vec![false; web.len()]);
    for &(a, w) in &paired {
        app_paired[a] = true;
        web_paired[w] = true;
    }

    // How the browser's frames fit the program's, from the first pair.
    let size = vision::load(&app[paired[0].0].path)?.dimensions();
    let first_web = vision::load(&web[paired[0].1].path)?;
    let app_first = vision::load(&app[paired[0].0].path)?;
    let offset = alignment(&app_first, &first_web);
    let resized = offset.is_none();
    let mut offsets: HashMap<(u32, u32), Option<(u32, u32)>> = HashMap::from([(first_web.dimensions(), offset)]);

    let total = app.len() + web.len();
    let mut count = 0;
    let (app_read, app_lines) =
        read_side(&app, &app_paired, ocr, |s| vision::load(&s.path), || {
            count += 1;
            progress(count, total);
        })?;
    let (web_read, web_lines) = read_side(
        &web,
        &web_paired,
        ocr,
        |s| {
            let frame = vision::load(&s.path)?;
            // A window resized during the test: lined up again against the first program frame,
            // which shows the same interface even if the scene moved on.
            let at = *offsets.entry(frame.dimensions()).or_insert_with(|| alignment(&app_first, &frame));
            Ok(fitted(frame, size, at))
        },
        || {
            count += 1;
            progress(count, total);
        },
    )?;

    let readers = READERS
        .iter()
        .enumerate()
        .map(|(r, &(reader, gate))| {
            let mut score = ReaderScore {
                reader,
                gate,
                agree: 0,
                differ: 0,
                only_program: 0,
                only_website: 0,
                neither: 0,
                share: None,
                passes: None,
                examples: Vec::new(),
            };
            for &(a, w) in &paired {
                let program = app_read.get(&a).and_then(|x| x.values[r].clone());
                let website = web_read.get(&w).and_then(|x| x.values[r].clone());
                match (&program, &website) {
                    (None, None) => score.neither += 1,
                    (Some(p), Some(s)) if p == s => score.agree += 1,
                    _ => {
                        match (&program, &website) {
                            (Some(_), Some(_)) => score.differ += 1,
                            (Some(_), None) => score.only_program += 1,
                            _ => score.only_website += 1,
                        }
                        if score.examples.len() < EXAMPLES {
                            score.examples.push(Difference {
                                program_frame: name(&app[a]),
                                website_frame: name(&web[w]),
                                program,
                                website,
                            });
                        }
                    }
                }
            }
            let counted = score.agree + score.differ + score.only_program + score.only_website;
            score.share = (counted > 0).then(|| score.agree as f32 / counted as f32);
            score.passes = score.share.map(|s| s >= PASS_SHARE);
            score
        })
        .collect::<Vec<_>>();

    let mut used = vec![false; web_lines.len()];
    let matched = app_lines
        .iter()
        .filter(|(at, gain)| {
            let found = web_lines
                .iter()
                .enumerate()
                .position(|(i, (w_at, w_gain))| !used[i] && w_gain == gain && (w_at - at).abs() <= PICKUP_WITHIN);
            if let Some(i) = found {
                used[i] = true;
            }
            found.is_some()
        })
        .count();
    let most = app_lines.len().max(web_lines.len());
    let share = (most > 0).then(|| matched as f32 / most as f32);
    let pickup = PickupScore {
        program_lines: app_lines.len(),
        website_lines: web_lines.len(),
        matched,
        share,
        passes: share.map(|s| s >= PASS_SHARE),
    };

    let passes = readers.iter().filter(|r| r.gate).all(|r| r.passes == Some(true));
    Ok(Agreement {
        program_frames: app.len(),
        website_frames: web.len(),
        pairs: paired.len(),
        website_offset: offset,
        website_resized: resized,
        readers,
        pickup,
        passes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(ms: &[u64]) -> Vec<Saved> {
        ms.iter().map(|&at_ms| Saved { path: PathBuf::from(format!("{at_ms}.png")), at_ms }).collect()
    }

    #[test]
    fn pairs_the_closest_frames_once_each_within_the_limit() {
        let app = at(&[1000, 1500, 2000, 3000]);
        let web = at(&[1010, 1490, 1520, 2300, 3099]);
        // 1490 is closer to 1500 than 1520 is, so 1520 goes unpaired; 2300 is too far from 2000.
        assert_eq!(pairs(&app, &web, 100), vec![(0, 0), (1, 1), (3, 4)]);
        assert!(pairs(&app, &at(&[5000]), 100).is_empty());
    }

    #[test]
    fn finds_both_sides_frames_by_name() {
        let folder = std::env::temp_dir().join(format!("leafline-agree-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        for name in ["app-2000.png", "app-1000.png", "web-1005.png", "web-x.png", "notes.txt", "leafline-1.png"] {
            std::fs::write(folder.join(name), b"").unwrap();
        }
        let (app, web) = saved(&folder).unwrap();
        std::fs::remove_dir_all(&folder).unwrap();
        assert_eq!(app.iter().map(|s| s.at_ms).collect::<Vec<_>>(), vec![1000, 2000]);
        assert_eq!(web.iter().map(|s| s.at_ms).collect::<Vec<_>>(), vec![1005]);
    }

    #[test]
    fn lines_up_a_browser_frame_that_kept_the_title_bar() {
        let app = Frame::from_fn(320, 200, |x, y| image::Rgba([(x * 7 % 256) as u8, (y * 3 % 256) as u8, ((x ^ y) % 256) as u8, 255]));
        let web = Frame::from_fn(326, 231, |x, y| {
            if x >= 3 && y >= 31 && x - 3 < 320 && y - 31 < 200 { *app.get_pixel(x - 3, y - 31) } else { image::Rgba([240, 240, 240, 255]) }
        });
        assert_eq!(alignment(&app, &web), Some((3, 31)));
        assert_eq!(fitted(web, (320, 200), Some((3, 31))), app);
        assert_eq!(alignment(&app, &Frame::new(100, 100)), None, "a smaller frame must be resized");
    }
}
