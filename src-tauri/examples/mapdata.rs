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
//! for each objective; add `--lookup` to download mapleclassic.wiki's drop table first.
//!
//! `--train <level> [--from <map-id>]` lists maps whose monsters average within a few levels of
//! `level`, and that can be reached from the map given.
//!
//! `--hunt <monster> --from <map-id>` lists where the quest guide sends a player for that monster.
//!
//! `--preview <image> <out.json> [--travel]` writes what the overlay would
//! show for a screenshot (analysis, map match, quest guide, training spots), for the dev UI's
//! `window.__LEAFLINE_PREVIEW__`. `--travel` fetches the taxis and return scrolls from
//! mapleclassic.wiki and turns them all on. It holds game data: keep it outside the repo.
//!
//! `--builds` lays out a build for every job, with each skill to its maximum in the data's order,
//! and prints what the planner makes of it.
//!
//! `--gear <level>` prints the gear suggested at that level for each ready-made plan.
//!
//! `--items <text>` prints what the item search finds.
//!
//! `--quests-for <text>` prints what the quest search finds, and the first one's sheet.
//!
//! `--monsters <text> [--from <map-id>]` prints what the monster search finds, and the first one's
//! sheet.
//!
//! `--hold <folder>` reads the Quest Helper of each picture in turn, as live reading does, and
//! prints what each shows raw and once held.

use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use leafline_lib::build::{
    Build, Catalog, ItemQuery, Plan, Rules, SkillLevel, Start, Stat, StatGoal, Stats, expand, starters, suggest,
};
use leafline_lib::mapdata::fetch::{self, Http};
use leafline_lib::mapdata::{
    Atlas, MapCard, MonsterQuery, QuestQuery, QuestSearch, QuestTitles, Store, Travel, TravelChoice, TravelFile, drops,
};
use leafline_lib::pipeline;
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

    if args.iter().any(|a| a == "--builds") {
        let data = store.load().expect("load cache").expect("no map data yet");
        print_builds(Catalog::new(data.skills, data.items));
    }

    if let Some(folder) = value("--hold", 1) {
        let ocr = Ocr::new().expect("OCR engine");
        let mut titles = QuestTitles::default();
        let mut paths: Vec<_> =
            std::fs::read_dir(folder).expect("read folder").filter_map(|e| Some(e.ok()?.path())).collect();
        paths.retain(|p| vision::is_supported(p));
        paths.sort();
        for path in paths {
            let frame = vision::load(&path).expect("load screenshot");
            let mut quest = pipeline::analyze_live(&frame, &ocr).expect("analyze").quest;
            println!("{}", path.file_name().unwrap().to_string_lossy());
            println!("    raw  {:?}", quest.as_ref().map(|q| &q.quests));
            titles.hold(&atlas, &mut quest);
            println!("    held {:?}", quest.as_ref().map(|q| &q.quests));
        }
    }
    if let Some(text) = value("--quests-for", 1) {
        let page = atlas.search_quests(&QuestSearch { text, ..QuestSearch::default() });
        println!("{} found", page.total);
        for quest in page.quests.iter().take(12) {
            println!("{:<7} {:<44} Lv {:?} {:?} {:?} EXP {}", quest.id, quest.name, quest.level_min, quest.giver, quest.region, quest.exp);
        }
        if let Some(first) = page.quests.first() {
            let data = store.load().expect("load cache").expect("no map data yet");
            let catalog = Catalog::new(data.skills, data.items);
            let name = |id: u32| catalog.item(id).map(|i| i.name.clone());
            println!("{:#?}", atlas.quest_sheet(first.id, name, &[], None, &Travel::none()).map(|s| (s.story, s.after, s.next, s.rewards)));
        }
    }
    if let Some(text) = value("--monsters", 1) {
        let dropped = store.dropped_pairs().expect("drop table");
        let page = atlas.search_monsters(&MonsterQuery { text, ..MonsterQuery::default() }, &dropped);
        println!("{} found, closest: {}", page.total, page.closest);
        for monster in page.monsters.iter().take(20) {
            println!("{:<8} {:<26} Lv {:>3} HP {:>7} EXP {:>5} {:?}", monster.id, monster.name, monster.level, monster.hp, monster.exp, monster.note);
        }
        if let Some(first) = page.monsters.first() {
            let from = value("--from", 1).map(|id| id.parse().expect("map id"));
            let found = store.drops(std::slice::from_ref(&first.name)).expect("drop table").pop();
            println!("{:#?}", atlas.monster_sheet(first.id, found, from, &[], &Travel::none()));
        }
    }
    if let Some(level) = value("--gear", 1) {
        let data = store.load().expect("load cache").expect("no map data yet");
        print_gear(Catalog::new(data.skills, data.items), level.parse().expect("level"));
    }
    if let Some(text) = value("--items", 1) {
        let data = store.load().expect("load cache").expect("no map data yet");
        let catalog = Catalog::new(data.skills, data.items);
        let page = catalog.search_items(&ItemQuery { text, ..ItemQuery::default() }, |_| true);
        println!("{} found, closest: {}", page.total, page.closest);
        for item in page.items {
            println!("{:<8} {:<40} {:<8} Lv {:>3} {:?}", item.id, item.name, item.slot, item.req_level, item.scroll);
        }
    }

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
        drops::refresh(&mut store, &Http::new(Duration::from_secs(1)), now).expect("download drops");
        for entry in store.drops(&names).expect("drops") {
            println!("{} ({:?} kills, v{:?})", entry.monster, entry.sample_kills, entry.game_version);
            for d in entry.drops {
                println!("    {:>6}  {}", d.rate.map_or("?".into(), |r| format!("{r:.1}%")), d.item);
            }
        }
    }
    if let Some(image) = value("--quests", 1) {
        let frame = vision::load(Path::new(&image)).expect("load screenshot");
        let analysis = pipeline::analyze(&frame, &Ocr::new().expect("OCR engine")).expect("analyze");
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
        if args.iter().any(|a| a == "--lookup") {
            drops::refresh(&mut store, &Http::new(Duration::from_secs(1)), now).expect("download drops");
        }
        let sources = drops::sources_for(&store, &items).expect("look up items");
        for guide in atlas.quest_guide(&quests, from.map(|m| m.id), &sources, &[], &Travel::none()) {
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
        let from = value("--from", 1).map(|id| id.parse().expect("map id"));
        let mut spots = atlas.training_spots(from, &[], &Travel::none(), None, &Default::default());
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
    if let Some(monster) = value("--hunt", 1) {
        let from = value("--from", 1).map(|id| id.parse().expect("map id"));
        let query = QuestQuery { title: String::new(), objectives: vec![monster] };
        let guide = atlas.quest_guide(&[query], from, &[], &[], &Travel::none());
        for hunt in guide.iter().flat_map(|g| &g.objectives).flat_map(|o| &o.hunts) {
            for ground in &hunt.maps {
                println!(
                    "{:<20} {:<36} x{:<3} {:?} portals",
                    hunt.monster.name, ground.map.name, ground.count, ground.hops
                );
            }
        }
    }
    if let (Some(image), Some(out)) = (value("--preview", 1), value("--preview", 2)) {
        let frame = vision::load(Path::new(&image)).expect("load screenshot");
        let mut analysis = pipeline::analyze(&frame, &Ocr::new().expect("OCR engine")).expect("analyze");
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
        let objectives: Vec<String> = quests.iter().flat_map(|q| q.objectives.clone()).collect();
        let sources = drops::sources_for(&store, &objectives).expect("look up items");
        let travel = if args.iter().any(|a| a == "--travel") {
            let all = TravelChoice { enabled: true, cabs: true, vip_cabs: true, scrolls: true };
            let file = TravelFile::fetch(&Http::new(Duration::from_secs(1)), now).expect("fetch travel from the wiki");
            atlas.travel(&file, all).0
        } else {
            Travel::none()
        };
        let guide = atlas.quest_guide(&quests, from, &sources, &[], &travel);
        // Routes to every map the guide names, so the dev UI can show them.
        let routes: Vec<_> = from
            .map(|start| {
                let mut ids: Vec<u32> = guide
                    .iter()
                    .flat_map(|q| &q.objectives)
                    .flat_map(|o| &o.hunts)
                    .flat_map(|h| &h.maps)
                    .map(|g| g.map.id)
                    .collect();
                ids.sort_unstable();
                ids.dedup();
                ids.into_iter().filter_map(|to| atlas.route_with(start, to, &travel)).collect()
            })
            .unwrap_or_default();
        let preview = serde_json::json!({
            "analysis": analysis,
            "map": found,
            "routes": routes,
            "guide": { "from": found.as_ref().map(|m| &m.card.map), "quests": guide },
            "spots": { "from": found.as_ref().map(|m| &m.card.map), "spots": atlas.training_spots(from, &[], &travel, None, &Default::default()) },
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

fn print_builds(catalog: Catalog) {
    println!("{} skills, {} jobs", catalog.skills().len(), catalog.jobs().len());
    println!("gear: {} match \"hat\"\n", catalog.search_gear("hat", None).len());
    // Each ready-made plan on a Lv 1 character. Anything printed under one needs fixing.
    for starter in starters::all(&catalog, &Rules::default()) {
        let build = Build {
            start: Start { level: 1, stats: Some(Stats { str: 4, dex: 5, int: 4, luk: 12 }), ..Start::default() },
            plan: starter.plan.clone(),
            ..Build::default()
        };
        let plan = expand(&build, &Rules::default(), &catalog);
        let last = plan.steps.last().expect("a plan has levels");
        let weapons = plan.gear.iter().map(|g| format!("{} {:?}", g.item.name, g.level)).collect::<Vec<_>>().join(", ");
        println!(
            "{:<24} Lv 70: STR {} DEX {} INT {} LUK {}, {} AP and {} SP left",
            starter.name, last.stats.str, last.stats.dex, last.stats.int, last.stats.luk, last.ap_left, last.sp_left
        );
        println!("      {weapons}");
        for missing in &starter.missing {
            println!("      MISSING: {missing}");
        }
        for problem in &plan.problems {
            println!("      {:?}: {}", problem.severity, problem.message);
        }
    }
    println!();
    for job in catalog.jobs() {
        let path = [job.parent.unwrap_or(job.id), job.id];
        let skills: Vec<SkillLevel> = catalog
            .skills()
            .iter()
            .filter(|s| path.contains(&s.job_id))
            .map(|s| SkillLevel { skill_id: s.id, level: s.max_level })
            .collect();
        let build = Build {
            start: Start { level: 10, stats: Some(Stats { str: 4, dex: 25, int: 4, luk: 12 }), ..Start::default() },
            plan: Plan {
                job: job.id,
                skills,
                stats: vec![StatGoal { stat: job.primary.unwrap_or(Stat::Luk), target: None, from_level: None }],
                plan_to: 120,
                ..Plan::default()
            },
            ..Build::default()
        };
        let plan = expand(&build, &Rules::default(), &catalog);
        let last_sp = plan.steps.iter().rev().find(|s| !s.sp.is_empty()).map(|s| s.level);
        println!("{:>3} {:<14} tier {} last SP spent at {last_sp:?}", job.id, job.name, job.tier);
        for problem in &plan.problems {
            println!("      {:?}: {}", problem.severity, problem.message);
        }
    }
}

fn print_gear(catalog: Catalog, level: u32) {
    let rules = Rules::default();
    for starter in starters::all(&catalog, &rules) {
        let build = Build {
            start: Start { level: 1, stats: Some(Stats { str: 4, dex: 5, int: 4, luk: 12 }), ..Start::default() },
            plan: starter.plan.clone(),
            ..Build::default()
        };
        let plan = expand(&build, &rules, &catalog);
        let picks = suggest::suggest(&build, &plan, &rules, &catalog, level);
        println!("{} at Lv {level}, by {:?}", starter.name, picks.ranked_by);
        for slot in picks.slots {
            let items: Vec<String> = slot
                .picks
                .iter()
                .map(|p| {
                    let stats: Vec<String> = p.item.stats.iter().map(|s| format!("{} {}", s.label, s.value)).collect();
                    format!("{} (Lv {}; {})", p.item.name, p.item.req_level, stats.join(", "))
                })
                .collect();
            println!("    {:<8} {}", slot.slot, items.join(" | "));
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
        let hidden = if e.hidden { " hidden" } else { "" };
        println!("    exit {:?} {:?} {:?} {} {:?}{hidden}", e.side, e.height, e.direction, e.to.name, e.portals);
    }
}
