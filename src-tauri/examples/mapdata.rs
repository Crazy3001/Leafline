//! Downloads map data and looks maps up, to check the map card against real data without the
//! app. The cache file holds data extracted from the game client: keep it outside the repo.
//!
//! `cargo run --release --example mapdata -- <cache.sqlite3> [--download] [--find "<street> : <name>"]
//!     [--route <from-id> <to-id>] [--drops <map-id>] [--screens <folder>] [--json <out.json>]`
//!
//! `--json` writes the `--find` match as the app would send it to the map card.
//!
//! `--screens` reads the minimap of every screenshot in the folder and shows which map each
//! reading matched.
//!
//! `--quests <image>` reads the Quest Helper and minimap of one screenshot and prints where to go
//! for each objective; add `--lookup` to ask mapleclassic.wiki who drops the quest items.
//!
//! `--train <level>` lists maps whose monsters average within a few levels of `level`.
//!
//! `--preview <image> <out.json> [--font <hud-font.json>]` writes what the overlay would show for a
//! screenshot (analysis, map match, quest guide, training spots), for the dev UI's
//! `window.__LEAFLINE_PREVIEW__`. It holds game data: keep it outside the repo.

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use leafline_lib::mapdata::fetch::{self, Http};
use leafline_lib::mapdata::{Atlas, MapCard, QuestQuery, Store, drops};
use leafline_lib::pipeline;
use leafline_lib::readers::hud::HudFont;
use leafline_lib::readers::map;
use leafline_lib::vision::{self, Ocr};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = "usage: mapdata <cache.sqlite3> [--download] [--find <label>] [--route <from> <to>] [--drops <map>] [--screens <folder>]";
    let mut store = Store::open(Path::new(args.first().expect(usage))).expect("open cache");
    let value = |flag: &str, n: usize| args.iter().position(|a| a == flag).map(|i| args[i + n].clone());
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;

    if args.iter().any(|a| a == "--download") {
        let report = fetch::refresh(&mut store, &Http::new(Duration::from_millis(250)), true, now).expect("download");
        println!("downloaded: {:?}", report.downloaded);
    }
    let status = store.status().expect("status");
    println!("client build {:?}, data from {:?}", status.client_build, status.data_updated_at);
    let atlas = Atlas::new(store.load().expect("load cache").expect("no map data yet: run with --download"));
    println!("{} maps\n", atlas.map_count());

    if let Some(label) = value("--find", 1) {
        let (street, name) = label.split_once(" : ").map_or((None, label.as_str()), |(s, n)| (Some(s), n));
        match atlas.find(name, street) {
            Some(found) => {
                println!("matched {:.2}, alternatives {:?}", found.score, found.alternatives);
                print_card(&found.card);
                if let Some(out) = value("--json", 1) {
                    std::fs::write(out, serde_json::to_string_pretty(&found).expect("serialise")).expect("write json");
                }
            }
            None => println!("no match for {label:?}"),
        }
    }
    if let (Some(from), Some(to)) = (value("--route", 1), value("--route", 2)) {
        match atlas.route(from.parse().expect("map id"), to.parse().expect("map id")) {
            Some(route) => {
                for (i, step) in route.steps.iter().enumerate() {
                    let portal = step.portal.as_deref().unwrap_or("-");
                    println!("{:>2}. {} --{:?} ({portal})--> {}", i + 1, step.from.name, step.direction, step.to.name);
                }
            }
            None => println!("no route"),
        }
    }
    if let Some(id) = value("--drops", 1) {
        let card = atlas.card(id.parse().expect("map id")).expect("unknown map");
        let names: Vec<String> = card.mobs.iter().map(|m| m.name.clone()).collect();
        let wiki = Http::new(Duration::from_secs(1));
        for entry in drops::drops_for(&mut store, &wiki, &names, now).expect("drops") {
            println!("{} ({:?} kills, v{:?})", entry.monster, entry.sample_kills, entry.game_version);
            for d in entry.drops {
                println!("    {:>6}  {}", d.rate.map_or("?".into(), |r| format!("{r:.1}%")), d.item);
            }
        }
    }
    if let Some(image) = value("--quests", 1) {
        let frame = vision::load(Path::new(&image)).expect("load screenshot");
        let analysis =
            pipeline::analyze(&frame, &Ocr::new().expect("OCR engine"), &HudFont::default()).expect("analyze");
        let quests: Vec<QuestQuery> = analysis
            .quest
            .iter()
            .flat_map(|q| &q.quests)
            .map(|q| QuestQuery {
                title: q.title.clone(),
                objectives: q.objectives.iter().map(|o| o.target.clone()).collect(),
            })
            .collect();
        let from = analysis.map.as_ref().and_then(|m| atlas.find(&m.name, m.street.as_deref())).map(|m| m.card.map);
        println!("on {:?}", from.as_ref().map(|m| &m.name));
        let items: Vec<String> = quests.iter().flat_map(|q| q.objectives.clone()).collect();
        let sources = if args.iter().any(|a| a == "--lookup") {
            drops::sources_for(&mut store, &Http::new(Duration::from_secs(1)), &items, now).expect("look up items")
        } else {
            let keys: Vec<String> = items.iter().map(|i| drops::item_key(i)).collect();
            store.cached_item_sources(&keys, now - drops::MAX_AGE).expect("cached items").0
        };
        for guide in atlas.quest_guide(&quests, from.map(|m| m.id), &sources) {
            println!("{} -> {:?}", guide.title, guide.quest.as_ref().map(|q| (&q.quest.name, &q.giver, &q.giver_maps)));
            for o in &guide.objectives {
                println!("    {:?} {:?} item={:?} looked_up={}", o.kind, o.target, o.item, o.looked_up);
                for h in &o.hunts {
                    let rate = h.drop.as_ref().and_then(|d| d.rate);
                    println!(
                        "        hunt {} Lv {} (known {}, rate {rate:?})",
                        h.monster.name, h.monster.level, h.monster.known
                    );
                    for g in &h.maps {
                        println!("            {} x{} hops {:?}", g.map.name, g.count, g.hops);
                    }
                }
            }
        }
    }
    if let Some(level) = value("--train", 1) {
        let level: f64 = level.parse().expect("level");
        let mut spots = atlas.training_spots(None, &[]);
        spots.retain(|s| s.reachable && (s.average_level - level).abs() <= 5.0);
        spots.sort_by_key(|s| std::cmp::Reverse(s.exp_per_clear));
        for s in spots {
            println!(
                "{:<36} Lv {:>2}-{:<2} avg {:>4.1}  x{:<3} {:>6} EXP/clear  {:>5.1} EXP/1k HP",
                s.map.name,
                s.level_range.0,
                s.level_range.1,
                s.average_level,
                s.mob_total,
                s.exp_per_clear,
                s.exp_per_clear as f64 * 1000.0 / s.hp_per_clear.max(1) as f64
            );
        }
    }
    if let (Some(image), Some(out)) = (value("--preview", 1), value("--preview", 2)) {
        let font: HudFont = value("--font", 1)
            .map(|f| serde_json::from_str(&std::fs::read_to_string(f).expect("read font")).expect("parse font"))
            .unwrap_or_default();
        let frame = vision::load(Path::new(&image)).expect("load screenshot");
        let mut analysis = pipeline::analyze(&frame, &Ocr::new().expect("OCR engine"), &font).expect("analyze");
        analysis.overview.clear();
        let found = analysis.map.as_ref().and_then(|m| atlas.find(&m.name, m.street.as_deref()));
        let from = found.as_ref().map(|m| m.card.map.id);
        let quests: Vec<QuestQuery> = analysis
            .quest
            .iter()
            .flat_map(|q| &q.quests)
            .map(|q| QuestQuery {
                title: q.title.clone(),
                objectives: q.objectives.iter().map(|o| o.target.clone()).collect(),
            })
            .collect();
        let keys: Vec<String> = quests.iter().flat_map(|q| &q.objectives).map(|o| drops::item_key(o)).collect();
        let sources = store.cached_item_sources(&keys, now - drops::MAX_AGE).expect("cached items").0;
        let preview = serde_json::json!({
            "analysis": analysis,
            "map": found,
            "guide": { "from": found.as_ref().map(|m| &m.card.map), "quests": atlas.quest_guide(&quests, from, &sources) },
            "spots": { "from": found.as_ref().map(|m| &m.card.map), "spots": atlas.training_spots(from, &[]) },
        });
        std::fs::write(&out, serde_json::to_string(&preview).expect("serialise")).expect("write preview");
        println!("wrote {out}");
    }
    if let Some(folder) = value("--screens", 1) {
        let ocr = Ocr::new().expect("OCR engine");
        let mut paths: Vec<_> =
            std::fs::read_dir(folder).expect("read folder").filter_map(|e| Some(e.ok()?.path())).collect();
        paths.retain(|p| vision::is_supported(p));
        paths.sort();
        for path in paths {
            let frame = vision::load(&path).expect("load screenshot");
            let file = path.file_name().unwrap().to_string_lossy();
            match map::read(&frame, &ocr).expect("read minimap") {
                Some(reading) => match atlas.find(&reading.name, reading.street.as_deref()) {
                    Some(found) => {
                        println!("{file:<14} {:<48} -> {} ({:.2})", reading.label(), found.card.map.name, found.score)
                    }
                    None => println!("{file:<14} {:<48} -> no match", reading.label()),
                },
                None => println!("{file:<14} minimap not found"),
            }
        }
    }
}

fn print_card(card: &MapCard) {
    println!(
        "{} [{}] {:?}, town: {}, levels {:?}",
        card.map.name, card.map.id, card.map.street, card.is_town, card.level_range
    );
    for m in &card.mobs {
        println!("    mob  Lv {:>3} {:<24} HP {:>6} EXP {:>5} x{}", m.level, m.name, m.hp, m.exp, m.count);
    }
    println!("    {} monsters, {} EXP per clear", card.mob_total, card.exp_per_clear);
    for n in &card.npcs {
        println!("    npc  {} {:?}", n.name, n.quests.iter().map(|q| &q.name).collect::<Vec<_>>());
    }
    for q in &card.quests {
        println!(
            "    quest {} (from {:?}) {:?}",
            q.quest.name,
            q.giver,
            q.targets.iter().map(|t| (&t.name, t.count)).collect::<Vec<_>>()
        );
    }
    for e in &card.exits {
        println!("    exit {:?} {} {:?}{}", e.direction, e.to.name, e.portals, if e.hidden { " hidden" } else { "" });
    }
}
