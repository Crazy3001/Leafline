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

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use leafline_lib::mapdata::fetch::{self, Http};
use leafline_lib::mapdata::{Atlas, MapCard, Store, drops};
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
