//! Measures the website's reads of the game's windows against the program's on saved frames:
//! the Character Stat and Skill windows, the Item Inventory's open tab (sell or keep) and the
//! Quest window. The program's readings (Windows OCR) are worked out once and kept in a cache
//! file. Frames stay on this PC; nothing is written but the cache and standard output.
//!
//!     cargo run --release -p leafline-capture --example webwindows -- scan <frames.txt>
//!     ... -- character <pairs.txt> <cache.jsonl> <text-recognition.onnx> <data folder>
//!     ... -- questlog <pairs.txt> <cache.jsonl> <text-recognition.onnx> <data folder>
//!     ... -- bag <pairs.txt> <data folder> <pictures folder>
//!
//!     ... -- follow <frames.txt> <text-recognition.onnx> <data folder>
//!
//! `follow` feeds the frames to the live reader two seconds apart, as the website's worker does,
//! and prints what the window followers hand over. `scan` lists, without any OCR, which frames show which windows. The others take lines of
//! `website frame<TAB>program frame` (one path alone: the same frame on both sides), so a browser
//! frame of the capture test is read against the program's frame of the same moment. The data
//! folder holds OSMS's `maps.json`, `quests.json`, `items.json`, `monsters.json` and `skills.json`;
//! the pictures folder is a copy of the program's picture cache, which downloads what it lacks.

use std::collections::{BTreeSet, HashMap};
use std::io::Write;

use leafline_capture::live::LiveReader;
use leafline_core::build::catalog::Catalog;
use leafline_core::build::progress::{skill_called, skill_called_clearly};
use leafline_core::mapdata::atlas::Atlas;
use leafline_core::mapdata::bag;
use leafline_core::mapdata::osms::{Dataset, Records};
use leafline_core::mapdata::pictures::{PictureCache, PictureHttp, PictureIndex};
use leafline_core::pipeline;
use leafline_core::readers::character::CharacterReading;
use leafline_core::readers::questlog::{LogItem, QuestLogReading};
use leafline_core::readers::{character, hud, inventory, questlog, ui};
use leafline_core::vision::{Frame, Ocr};
use serde_json::{Value, json};

fn lines(list: &str) -> Vec<(String, String)> {
    std::fs::read_to_string(list)
        .expect("read the list")
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let mut parts = l.split('\t').map(|p| p.trim().to_string());
            let web = parts.next().unwrap_or_default();
            let program = parts.next().filter(|p| !p.is_empty()).unwrap_or_else(|| web.clone());
            (web, program)
        })
        .collect()
}

fn open(path: &str) -> Option<Frame> {
    image::open(path).ok().map(|i| i.to_rgba8())
}

struct Data {
    dataset: Dataset,
    skills: Vec<leafline_core::mapdata::osms::SkillRecord>,
}

fn data(folder: &str) -> Data {
    let mut dataset = Dataset::default();
    let mut skills = Vec::new();
    for file in ["maps.json", "quests.json", "items.json", "monsters.json", "skills.json"] {
        let bytes = std::fs::read(format!("{folder}/{file}")).expect("a data file");
        match Records::parse(file, &bytes).expect("parse") {
            Records::Maps(v) => dataset.maps = v,
            Records::Quests(v) => dataset.quests = v,
            Records::Items(v) => dataset.items = v,
            Records::Monsters(v) => dataset.monsters = v,
            Records::Skills(v) => skills = v,
            _ => {}
        }
    }
    Data { dataset, skills }
}

fn live(model: &str, folder: &str) -> LiveReader {
    let mut live = LiveReader::new(15.0);
    live.set_text_model(std::fs::read(model).expect("model")).expect("load the model");
    for file in ["maps.json", "quests.json", "items.json", "monsters.json"] {
        live.set_data(file, &std::fs::read(format!("{folder}/{file}")).expect("a data file")).expect("load the data");
    }
    live
}

/// The program's reading of each frame, from the cache or worked out now with Windows OCR.
fn program(cache: &str, paths: &[String], read: impl Fn(&Frame, &Ocr) -> Value) -> HashMap<String, Value> {
    let mut known: HashMap<String, Value> = std::fs::read_to_string(cache)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter_map(|v| Some((v["path"].as_str()?.to_string(), v["read"].clone())))
        .collect();
    let missing: BTreeSet<&String> = paths.iter().filter(|p| !known.contains_key(*p)).collect();
    if !missing.is_empty() {
        eprintln!("reading {} frames with the program's readers", missing.len());
        let ocr = Ocr::new().expect("Windows OCR");
        let mut out = std::fs::OpenOptions::new().create(true).append(true).open(cache).expect("open the cache");
        for path in missing {
            let value = open(path).map_or(Value::Null, |frame| read(&frame, &ocr));
            writeln!(out, "{}", json!({ "path": path, "read": value })).expect("write the cache");
            known.insert(path.clone(), value);
        }
    }
    known
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["scan", list] => scan(&lines(list)),
        ["character", list, cache, model, folder] => characters(&lines(list), cache, model, folder),
        ["questlog", list, cache, model, folder] => quest_logs(&lines(list), cache, model, folder),
        ["bag", list, folder, pictures] => bags(&lines(list), folder, pictures),
        ["follow", list, model, folder] => follow(&lines(list), model, folder),
        _ => panic!("usage: see the top of webwindows.rs"),
    }
}

fn scan(pairs: &[(String, String)]) {
    let none = Ocr::none();
    for (path, _) in pairs {
        let Some(frame) = open(path) else { continue };
        let analysis = pipeline::analyze_live(&frame, &none).ok();
        let layout = analysis.as_ref().and_then(|a| a.hud_layout);
        let scale = layout.as_ref().map(|l| ui::hud_scale(&frame, l));
        let windows = character::read(&frame, layout.as_ref(), &none).unwrap_or_default();
        let glance = layout.as_ref().and_then(|l| character::glance(&frame, l, None));
        let bag = inventory::read(&frame);
        let log = questlog::glance(&frame, scale).is_some();
        if !(windows.stats.is_some() || windows.detail.is_some() || glance.is_some() || bag.is_some() || log) {
            continue;
        }
        println!(
            "{}",
            json!({
                "path": path, "w": frame.width(), "h": frame.height(), "scale": scale,
                "stats": windows.stats.is_some(), "detail": windows.detail.is_some(), "glance": glance.is_some(),
                "bag": bag.as_ref().map(|b| json!({ "tab": b.tab, "slots": b.slots.len() })),
                "questlog": log,
            })
        );
    }
}

/// Counts of one part: the website's reading the same as the program's, another, none where the
/// program read one, and one where the program read none.
#[derive(Default, Debug)]
struct Tally {
    right: u32,
    wrong: u32,
    none: u32,
    extra: u32,
}

impl Tally {
    fn add(&mut self, program: &Value, web: &Value) -> &'static str {
        let empty = |v: &Value| v.is_null() || v.as_array().is_some_and(|a| a.is_empty());
        match (empty(program), empty(web)) {
            (true, true) => "neither",
            (false, false) if program == web => {
                self.right += 1;
                "right"
            }
            (false, false) => {
                self.wrong += 1;
                "wrong"
            }
            (false, true) => {
                self.none += 1;
                "none"
            }
            (true, false) => {
                self.extra += 1;
                "extra"
            }
        }
    }
}

fn print_tallies(tallies: &[(&str, Tally)]) {
    for (part, t) in tallies {
        eprintln!("{part}: right {}, wrong {}, none {}, website only {}", t.right, t.wrong, t.none, t.extra);
    }
}

/// The skills a reading names, as the build takes them: each row's name settled on a skill of the
/// job's path, with its level.
fn settled_skills(catalog: &Catalog, job: Option<u32>, windows: &Value, clearly: bool) -> Value {
    let (Some(job), Some(rows)) = (job, windows["skills"]["rows"].as_array()) else { return Value::Null };
    let rows: Vec<Value> = rows
        .iter()
        .filter_map(|row| {
            let name = row["name"].as_str()?;
            let (id, _) = if clearly { skill_called_clearly(catalog, name, job)? } else { skill_called(catalog, name, job)? };
            Some(json!([id, row["level"]]))
        })
        .collect();
    json!(rows)
}

fn characters(pairs: &[(String, String)], cache: &str, model: &str, folder: &str) {
    let data = data(folder);
    let catalog = Catalog::new(data.skills.clone(), data.dataset.items.clone());
    let paths: Vec<String> = pairs.iter().map(|(_, p)| p.clone()).collect();
    let known = program(cache, &paths, |frame, ocr| {
        let located = hud::read(frame, ocr).ok().flatten();
        let layout = located.as_ref().map(|(l, _)| *l);
        let job = located.as_ref().and_then(|(_, r)| r.job.clone());
        let windows = character::read(frame, layout.as_ref(), ocr).unwrap_or_default();
        json!({ "job": job, "windows": windows })
    });
    let mut live = live(model, folder);
    let job_id = |name: &str| catalog.jobs().into_iter().find(|j| j.name.eq_ignore_ascii_case(name.trim())).map(|j| j.id);
    let mut tallies: Vec<(&str, Tally)> =
        ["stats", "detail", "skills", "skill points"].into_iter().map(|p| (p, Tally::default())).collect();
    for (web_path, program_path) in pairs {
        let Some(frame) = open(web_path) else { continue };
        let prog = known.get(program_path).cloned().unwrap_or(Value::Null);
        let started = std::time::Instant::now();
        let layout = live.analyze(&frame).and_then(|(a, _)| a.hud_layout);
        let web: CharacterReading = live.read_character_windows(&frame, layout.as_ref()).unwrap_or_default();
        let ms = started.elapsed().as_millis();
        let web = serde_json::to_value(&web).unwrap_or_default();
        let job = prog["job"].as_str().and_then(job_id);
        let pw = &prog["windows"];
        let parts = [
            (pw["stats"].clone(), web["stats"].clone()),
            (pw["detail"].clone(), web["detail"].clone()),
            (settled_skills(&catalog, job, pw, false), settled_skills(&catalog, job, &web, true)),
            (pw["skills"]["skillPoints"].clone(), web["skills"]["skillPoints"].clone()),
        ];
        let verdicts: Vec<&str> = tallies.iter_mut().zip(&parts).map(|((_, t), (p, w))| t.add(p, w)).collect();
        println!("{}", json!({ "web": web_path, "program": program_path, "verdicts": verdicts, "ms": ms, "programRead": pw, "webRead": web }));
    }
    print_tallies(&tallies);
}

/// Each quest row's quest in the data, by position, `null` where its title settles on none; and
/// each region bar's region.
fn settled_log(atlas: &Atlas, reading: &Value) -> (Vec<Value>, Vec<Value>) {
    let Ok(reading) = serde_json::from_value::<QuestLogReading>(reading.clone()) else { return (Vec::new(), Vec::new()) };
    let mut region: Option<String> = None;
    let (mut quests, mut regions) = (Vec::new(), Vec::new());
    for item in &reading.items {
        match item {
            LogItem::Region { name, count } => {
                region = atlas.quest_region_like(name);
                regions.push(json!([region, count]));
            }
            LogItem::Quest(q) => {
                let readings: Vec<&str> = std::iter::once(q.title.as_str()).chain(q.readings.iter().map(String::as_str)).collect();
                quests.push(json!(atlas.log_quest(&readings, q.cut, region.as_deref())));
            }
        }
    }
    (quests, regions)
}

fn quest_logs(pairs: &[(String, String)], cache: &str, model: &str, folder: &str) {
    let data = data(folder);
    let atlas = Atlas::new(data.dataset.clone());
    let paths: Vec<String> = pairs.iter().map(|(_, p)| p.clone()).collect();
    let known = program(cache, &paths, |frame, ocr| {
        let located = hud::read(frame, ocr).ok().flatten();
        let scale = located.as_ref().map(|(l, _)| ui::hud_scale(frame, l));
        json!(questlog::read(frame, scale, ocr).ok().flatten())
    });
    let mut live = live(model, folder);
    // Per row: the website's row settles on the program's quest (right), another (wrong), none
    // where the program's settles (none), or one where the program's doesn't (checked by eye).
    let (mut rows, mut bars, mut details) = (Tally::default(), Tally::default(), Tally::default());
    for (web_path, program_path) in pairs {
        let Some(frame) = open(web_path) else { continue };
        let prog = known.get(program_path).cloned().unwrap_or(Value::Null);
        let started = std::time::Instant::now();
        let analysis = live.analyze(&frame).map(|(a, _)| a);
        let scale = analysis.as_ref().and_then(|a| a.hud_layout.as_ref()).map(|l| ui::hud_scale(&frame, l));
        let web = json!(live.read_quest_window(&frame, scale));
        let ms = started.elapsed().as_millis();
        let (pq, pr) = settled_log(&atlas, &prog);
        let (wq, wr) = settled_log(&atlas, &web);
        // Per quest the website keeps (a row settled on the data): one the program's reading
        // settles on too (right) or not (wrong, unless checked by eye); per quest the program's
        // settles on that the website's doesn't keep (none).
        let mut row_verdicts = Vec::new();
        let ids = |v: &[Value]| v.iter().filter_map(Value::as_u64).collect::<BTreeSet<u64>>();
        let (p_ids, w_ids) = (ids(&pq), ids(&wq));
        for id in &w_ids {
            row_verdicts.push(if p_ids.contains(id) { rows.right += 1; "right" } else { rows.wrong += 1; "wrong" });
        }
        let missed = p_ids.difference(&w_ids).count();
        rows.none += missed as u32;
        row_verdicts.extend(std::iter::repeat_n("none", missed));
        let unread = web["unread"].as_u64().unwrap_or(0);
        let mut bar_verdicts = Vec::new();
        if pr.len() == wr.len() {
            for (p, w) in pr.iter().zip(&wr) {
                let settled = |v: &Value| if v[0].is_null() { Value::Null } else { v.clone() };
                bar_verdicts.push(bars.add(&settled(p), &settled(w)));
            }
        }
        let detail = |v: &Value| {
            let d = &v["detail"];
            if d.is_null() { Value::Null } else { json!([atlas.log_quest(&[d["title"].as_str().unwrap_or("")], d["cut"].as_bool().unwrap_or(false), None), d["level"], d["counters"], d["rewards"], d["rewardExp"], d["rewardMesos"], d["talkTo"]]) }
        };
        let detail_verdict = details.add(&detail(&prog), &detail(&web));
        println!(
            "{}",
            json!({ "web": web_path, "program": program_path, "ms": ms, "unread": unread, "rows": row_verdicts, "bars": bar_verdicts, "detail": detail_verdict,
                    "programQuests": pq, "webQuests": wq, "programRead": prog, "webRead": web })
        );
    }
    print_tallies(&[("quest rows", rows), ("region bars", bars), ("detail pane", details)]);
}

fn bags(pairs: &[(String, String)], folder: &str, pictures: &str) {
    let data = data(folder);
    let catalog = Catalog::new(data.skills.clone(), data.dataset.items.clone());
    let index = PictureIndex::new(&data.dataset);
    let cache = PictureCache::new(pictures.into(), PictureHttp::default());
    let mut images: HashMap<u32, image::RgbaImage> = HashMap::new();
    let none = Ocr::none();
    let mut tally = Tally::default();
    for (web_path, program_path) in pairs {
        let (Some(web_frame), Some(program_frame)) = (open(web_path), open(program_path)) else { continue };
        // WEBWINDOWS_I420: the website's frame made from the program's, as a browser's video would
        // give it back (BT.601 limited range, colour for each square of four pixels).
        let web_frame = match std::env::var("WEBWINDOWS_I420").ok().as_deref() {
            Some("nearest") => i420(&program_frame, false),
            Some(_) => i420(&program_frame, true),
            None => web_frame,
        };
        let started = std::time::Instant::now();
        let web_bag = inventory::read_video(&web_frame);
        let find_ms = started.elapsed().as_millis();
        let program_bag = inventory::read(&program_frame);
        let Some(tab) = program_bag.as_ref().and_then(|b| b.tab) else {
            println!("{}", json!({ "web": web_path, "program": program_path, "programTab": null, "webTab": web_bag.as_ref().map(|b| b.tab) }));
            continue;
        };
        // Candidates: every item of the tab whose picture is in the folder, plus what a character
        // of this level likely carries (downloaded when missing).
        let level = pipeline::analyze_live(&program_frame, &none).ok().and_then(|a| a.hud).and_then(|h| h.level);
        let atlas = Atlas::new(data.dataset.clone());
        let url = |id: u32| index.resolve(&format!("/item/{id}")).map(str::to_string);
        let mut ids: BTreeSet<u32> = bag::likely(&atlas, &catalog, &[], level, tab, &[]).into_iter().collect();
        ids.extend(bag::every(&catalog, tab).into_iter().filter(|&id| url(id).is_some_and(|u| cache.has(&u))));
        for &id in &ids {
            if images.contains_key(&id) {
                continue;
            }
            let Some(u) = url(id) else { continue };
            if let Ok(picture) = cache.get(&u)
                && let Ok(image) = image::load_from_memory(&picture.bytes)
            {
                images.insert(id, image.to_rgba8());
            }
        }
        let candidates: Vec<(u32, &image::RgbaImage)> = ids.iter().filter_map(|id| Some((*id, images.get(id)?))).collect();
        // The program names by `name`; the website, which always reads a video, by `name_video`.
        let named = |b: &Option<inventory::Inventory>, video: bool| -> HashMap<u32, Value> {
            b.iter()
                .flat_map(|b| &b.slots)
                .map(|s| {
                    let n = if s.count_unsure { None } else if video { bag::name_video(&s.picture, &candidates) } else { bag::name(&s.picture, &candidates) };
                    (s.index, n.map_or(Value::Null, |n| json!([n.ids, s.count])))
                })
                .collect()
        };
        if std::env::var("WEBWINDOWS_SCORES").is_ok() {
            // Per slot the program names: how the right item and the best other one score on the
            // website's frame, by each way of comparing.
            let web_slots: HashMap<u32, &inventory::BagSlot> = web_bag.iter().flat_map(|b| &b.slots).map(|s| (s.index, s)).collect();
            for slot in program_bag.iter().flat_map(|b| &b.slots) {
                let Some(truth) = bag::name(&slot.picture, &candidates) else { continue };
                let Some(web_slot) = web_slots.get(&slot.index) else {
                    println!("{}", json!({ "web": web_path, "slot": slot.index, "missing": true }));
                    continue;
                };
                let windows = (program_bag.as_ref().map(|b| b.window), web_bag.as_ref().map(|b| b.window));
                let mut out = json!({ "web": web_path, "slot": slot.index, "truth": truth.ids, "count": [slot.count, web_slot.count], "windows": format!("{windows:?}") });
                for (key, metric) in [("exact", bag::Metric::Exact), ("luma", bag::Metric::Luma), ("smooth", bag::Metric::Smooth)] {
                    let scored = bag::scores(&web_slot.picture, &candidates, metric);
                    let right = scored.iter().find(|(_, id)| truth.ids.contains(id)).map(|s| s.0);
                    let other = scored.iter().find(|(_, id)| !truth.ids.contains(id)).copied();
                    out[key] = json!({ "right": right, "other": other });
                }
                println!("{out}");
            }
            continue;
        }
        let (p, w) = (named(&program_bag, false), named(&web_bag, true));
        let mut verdicts = Vec::new();
        for (slot, prog) in &p {
            let web = w.get(slot).cloned().unwrap_or(Value::Null);
            verdicts.push(json!([slot, tally.add(prog, &web), prog, web]));
        }
        println!(
            "{}",
            json!({ "web": web_path, "program": program_path, "tab": tab, "webTab": web_bag.as_ref().map(|b| b.tab), "findMs": find_ms,
                    "candidates": candidates.len(), "slots": verdicts })
        );
    }
    print_tallies(&[("bag slots", tally)]);
}

/// `frame` through a 4:2:0 video encoding and back, BT.601 limited range: brightness for each
/// pixel, colour averaged over each square of four, spread back nearest or blended.
fn i420(frame: &Frame, blend: bool) -> Frame {
    let (w, h) = frame.dimensions();
    let px = |x: u32, y: u32| {
        let p = frame.get_pixel(x.min(w - 1), y.min(h - 1)).0;
        [f32::from(p[0]), f32::from(p[1]), f32::from(p[2])]
    };
    let luma = |[r, g, b]: [f32; 3]| (16.0 + (65.481 * r + 128.553 * g + 24.966 * b) / 255.0).round();
    let (cw, ch) = (w.div_ceil(2), h.div_ceil(2));
    let mut cb = vec![0.0f32; (cw * ch) as usize];
    let mut cr = vec![0.0f32; (cw * ch) as usize];
    for cy in 0..ch {
        for cx in 0..cw {
            let (mut u, mut v) = (0.0, 0.0);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let [r, g, b] = px(cx * 2 + dx, cy * 2 + dy);
                u += 128.0 + (-37.797 * r - 74.203 * g + 112.0 * b) / 255.0;
                v += 128.0 + (112.0 * r - 93.786 * g - 18.214 * b) / 255.0;
            }
            cb[(cy * cw + cx) as usize] = (u / 4.0).round();
            cr[(cy * cw + cx) as usize] = (v / 4.0).round();
        }
    }
    let chroma = |plane: &[f32], x: u32, y: u32| -> f32 {
        if !blend {
            return plane[((y / 2).min(ch - 1) * cw + (x / 2).min(cw - 1)) as usize];
        }
        // Centred between the samples, as a bilinear upscale does.
        let fx = (x as f32 - 0.5) / 2.0;
        let fy = (y as f32 - 0.5) / 2.0;
        let (x0, y0) = (fx.floor().max(0.0) as u32, fy.floor().max(0.0) as u32);
        let (x1, y1) = ((x0 + 1).min(cw - 1), (y0 + 1).min(ch - 1));
        let (tx, ty) = ((fx - x0 as f32).clamp(0.0, 1.0), (fy - y0 as f32).clamp(0.0, 1.0));
        let at = |x: u32, y: u32| plane[(y * cw + x) as usize];
        let top = at(x0, y0) * (1.0 - tx) + at(x1, y0) * tx;
        let bottom = at(x0, y1) * (1.0 - tx) + at(x1, y1) * tx;
        top * (1.0 - ty) + bottom * ty
    };
    Frame::from_fn(w, h, |x, y| {
        let yy = (luma(px(x, y)) - 16.0) * 255.0 / 219.0;
        let (u, v) = ((chroma(&cb, x, y) - 128.0) * 255.0 / 224.0, (chroma(&cr, x, y) - 128.0) * 255.0 / 224.0);
        let r = yy + 1.402 * v;
        let g = yy - 0.344_136 * u - 0.714_136 * v;
        let b = yy + 1.772 * u;
        let c = |v: f32| v.round().clamp(0.0, 255.0) as u8;
        image::Rgba([c(r), c(g), c(b), 255])
    })
}

fn follow(pairs: &[(String, String)], model: &str, folder: &str) {
    let mut live = live(model, folder);
    for (i, (path, _)) in pairs.iter().enumerate() {
        let Some(frame) = open(path) else { continue };
        let started = std::time::Instant::now();
        live.read(&frame, 1_000.0 + 2.0 * i as f64);
        let ms = started.elapsed().as_millis();
        let reads = live.take_windows();
        let what = reads.as_ref().map(|r| {
            json!({
                "character": r.character.as_ref().map(|c| json!({ "name": c.name, "stats": c.windows.stats.is_some(), "skills": c.windows.skills.as_ref().map(|s| s.rows.len()) })),
                "bag": r.bag.as_ref().map(|b| json!({ "tab": b.tab, "slots": b.slots.len() })),
                "questLog": r.quest_log.as_ref().map(|q| json!({ "tab": q.reading.tab, "rows": q.reading.items.len(), "unread": q.reading.unread, "fresh": q.fresh })),
            })
        });
        println!("{}", json!({ "frame": path.rsplit(['/', '\\']).next(), "ms": ms, "reads": what }));
    }
}
