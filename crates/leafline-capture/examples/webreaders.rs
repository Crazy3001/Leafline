//! Measures the website's readers against the program's on saved frames: the HUD's name and job,
//! the Quest Helper and item tooltips. The program's readings (Windows OCR) are worked out once
//! and kept in a cache file, since they don't change while the website's readers are worked on.
//! Frames stay on this PC; nothing is written but the cache and standard output.
//!
//!     cargo run --release -p leafline-capture --example webreaders -- <frames.txt> <cache.jsonl> <text-recognition.onnx>
//!
//! `frames.txt` has one picture path per line (anything after a tab is ignored). Prints one JSON
//! line per frame with both readings, then a count per part on standard error.

use std::collections::HashMap;
use std::io::Write;

use leafline_capture::live::LiveReader;
use leafline_core::build::catalog::Catalog;
use leafline_core::mapdata::QuestTitles;
use leafline_core::mapdata::loot::name_tooltip;
use leafline_core::readers::tooltip::split_name;
use leafline_core::mapdata::atlas::Atlas;
use leafline_core::mapdata::osms::{Dataset, Records};
use leafline_core::pipeline::{self, Analysis};
use leafline_core::readers::quest::{Objective, Quest, QuestReading};
use leafline_core::vision::{Ocr, Rect};
use serde_json::{Value, json};

/// The quest titles a reading settles on with the quest data, as the program's live reading
/// shows them: held twice by a fresh holder, so each is confirmed.
fn settled_titles(atlas: &Atlas, quest: &Value) -> Value {
    let Some(quests) = quest["quests"].as_array() else { return Value::Null };
    let quests: Vec<Quest> = quests
        .iter()
        .map(|q| Quest {
            title: q["title"].as_str().unwrap_or_default().to_string(),
            objectives: q["objectives"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|o| Objective {
                    target: o["target"].as_str().unwrap_or_default().to_string(),
                    current: o["current"].as_u64().unwrap_or(0) as u32,
                    required: o["required"].as_u64().unwrap_or(0) as u32,
                    unread: false,
                })
                .collect(),
        })
        .collect();
    let page = quest["page"].as_array().map(|p| (p[0].as_u64().unwrap_or(0) as u32, p[1].as_u64().unwrap_or(0) as u32));
    let reading = QuestReading { quests, page, panel: Rect::new(0.0, 0.0, 1.0, 1.0) };
    let mut holder = QuestTitles::default();
    let mut first = Some(reading.clone());
    holder.hold(atlas, &mut first);
    let mut second = Some(reading);
    holder.hold(atlas, &mut second);
    json!(second.map(|r| r.quests.into_iter().map(|q| q.title).collect::<Vec<_>>()))
}

/// The parts compared, as JSON, from either side's analysis.
fn parts(analysis: &Analysis) -> Value {
    let hud = analysis.hud.as_ref();
    json!({
        "name": hud.and_then(|h| h.name.clone()),
        "job": hud.and_then(|h| h.job.clone()),
        "quest": analysis.quest.as_ref().map(|q| json!({ "page": q.page, "quests": q.quests })),
        "tooltips": analysis.tooltips.iter().map(|t| t.name.clone()).collect::<Vec<_>>(),
    })
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [list, cache, model] = args.as_slice() else {
        panic!("usage: webreaders <frames.txt> <cache.jsonl> <text-recognition.onnx>")
    };
    let paths: Vec<String> = std::fs::read_to_string(list)
        .expect("read the list")
        .lines()
        .map(|l| l.split('\t').next().unwrap_or_default().trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    let mut program: HashMap<String, Value> = std::fs::read_to_string(cache)
        .unwrap_or_default()
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
        .filter_map(|v| Some((v["path"].as_str()?.to_string(), v["read"].clone())))
        .collect();
    let missing: Vec<&String> = paths.iter().filter(|p| !program.contains_key(*p)).collect();
    if !missing.is_empty() {
        eprintln!("reading {} frames with the program's readers", missing.len());
        let ocr = Ocr::new().expect("Windows OCR");
        let mut out = std::fs::OpenOptions::new().create(true).append(true).open(cache).expect("open the cache");
        for (i, path) in missing.into_iter().enumerate() {
            let read = match image::open(path) {
                Ok(image) => pipeline::analyze(&image.to_rgba8(), &ocr).map(|a| parts(&a)).unwrap_or(Value::Null),
                Err(_) => Value::Null,
            };
            writeln!(out, "{}", json!({ "path": path, "read": read })).expect("write the cache");
            program.insert(path.clone(), read);
            if i % 50 == 49 {
                eprintln!("  {}", i + 1);
            }
        }
    }

    let mut live = LiveReader::new(15.0);
    live.set_text_model(std::fs::read(model).expect("model")).expect("load the model");
    // The reference data, for settling quest titles: WEBREADERS_DATA, a folder of OSMS files.
    let mut dataset = Dataset::default();
    if let Ok(folder) = std::env::var("WEBREADERS_DATA") {
        for file in ["maps.json", "quests.json", "items.json", "monsters.json"] {
            let bytes = std::fs::read(format!("{folder}/{file}")).expect("a data file");
            live.set_data(file, &bytes).expect("load the data");
            match Records::parse(file, &bytes).expect("parse") {
                Records::Maps(v) => dataset.maps = v,
                Records::Quests(v) => dataset.quests = v,
                Records::Items(v) => dataset.items = v,
                Records::Monsters(v) => dataset.monsters = v,
                _ => {}
            }
        }
    }
    let catalog = Catalog::new(Vec::new(), dataset.items.clone());
    let atlas = Atlas::new(dataset);
    // Item tooltips alone, per tooltip: WEBREADERS_TOOLTIPS.
    if std::env::var("WEBREADERS_TOOLTIPS").is_ok() {
        return tooltips(&paths, &program, &mut live, &catalog);
    }
    // field -> (agree, differ, program only, website only)
    let mut counts: HashMap<&str, [u32; 4]> = HashMap::new();
    for path in &paths {
        let Ok(image) = image::open(path) else { continue };
        let frame = image.to_rgba8();
        let Some((mut analysis, _)) = live.analyze(&frame) else { continue };
        analysis.tooltips = live.read_items(&frame);
        let mut web = parts(&analysis);
        let mut prog = program.get(path).cloned().unwrap_or(Value::Null);
        if !prog.is_null() {
            prog["titles"] = settled_titles(&atlas, &prog["quest"]);
        }
        web["titles"] = settled_titles(&atlas, &web["quest"]);
        for field in ["name", "job", "quest", "titles", "tooltips"] {
            let (p, w) = (&prog[field], &web[field]);
            let empty = |v: &Value| v.is_null() || v.as_array().is_some_and(|a| a.is_empty());
            let slot = match (empty(p), empty(w)) {
                (true, true) => continue,
                (false, false) if p == w => 0,
                (false, false) => 1,
                (false, true) => 2,
                (true, false) => 3,
            };
            counts.entry(field).or_default()[slot] += 1;
        }
        println!("{}", json!({ "path": path, "program": prog, "web": web }));
    }
    for field in ["name", "job", "quest", "titles", "tooltips"] {
        let [agree, differ, program_only, web_only] = counts.get(field).copied().unwrap_or_default();
        eprintln!("{field}: agree {agree}, differ {differ}, program only {program_only}, website only {web_only}");
    }
}

/// The item each of the program's tooltip names settles on, by the same rule as the website's.
fn program_items(catalog: &Catalog, read: &Value) -> Vec<String> {
    read["tooltips"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|t| name_tooltip(catalog, &split_name(t.as_str()?).0).map(|i| i.name.clone()))
        .collect()
}

/// Item tooltips only: per frame, whether every item the website names is one the program's
/// reading settles on (right), any is not (wrong), or it names none where the program named one.
fn tooltips(paths: &[String], program: &HashMap<String, Value>, live: &mut LiveReader, catalog: &Catalog) {
    let [mut right, mut wrong, mut none, mut unsettled, mut extra] = [0u32; 5];
    for path in paths {
        let Some(prog) = program.get(path) else { continue };
        let raw = prog["tooltips"].as_array().map_or(0, |a| a.len());
        let Ok(image) = image::open(path) else { continue };
        let frame = image.to_rgba8();
        let names = live.read_item_names(&frame);
        let items = live.read_items(&frame);
        let web: Vec<String> = items.iter().map(|t| t.name.clone()).collect();
        let cards: Vec<Value> = items
            .iter()
            .map(|t| json!({ "name": t.name, "upgrades": t.upgrades, "stats": t.stats.iter().map(|s| json!([s.label, s.value, s.base, s.bonus])).collect::<Vec<_>>(), "left": t.remaining_enhancements }))
            .collect();
        let truth = program_items(catalog, prog);
        let verdict = if web.is_empty() {
            if truth.is_empty() { if raw > 0 { unsettled += 1; "unsettled" } else { "neither" } } else { none += 1; "none" }
        } else if truth.is_empty() {
            extra += 1;
            "extra"
        } else if web.iter().all(|w| truth.contains(w)) {
            right += 1;
            "right"
        } else {
            wrong += 1;
            "wrong"
        };
        let read: Vec<Value> = names
            .iter()
            .map(|n| json!({ "lines": n.lines, "amber": n.drawn.amber, "unit": n.drawn.unit, "at": [n.drawn.bullet.x, n.drawn.bullet.y] }))
            .collect();
        println!("{}", json!({ "path": path, "verdict": verdict, "program": prog["tooltips"], "truth": truth, "web": web, "cards": cards, "read": read }));
    }
    eprintln!("tooltips: right {right}, wrong {wrong}, none {none}; program unsettled {unsettled}, website only {extra}");
}
