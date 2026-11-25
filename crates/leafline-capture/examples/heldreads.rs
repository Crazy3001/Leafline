//! Checks the program's held-still reads on saved frames, with the pieces its live loop runs
//! (`src-tauri/src/app/hover.rs` and `bag.rs`): where tooltips stand (`tooltip::name_lines`), the
//! hold (`tooltip::hover::Rest`), the read the Read item hotkey makes (`pipeline::read_items`,
//! Windows OCR), and the inventory tab held still (`inventory::read` with `TabWatch`). Frames stay
//! on this PC; nothing is written but standard output.
//!
//!     cargo run --release -p leafline-capture --example heldreads -- tooltips <program.jsonl> [read]
//!     ... -- live <frames.txt> [read]
//!
//! `tooltips` takes the program's cached readings (lines of `{"path", "read": {"tooltips": [...]}}`)
//! and says, per frame, whether a held hover would start a read: tooltips found where the
//! program read some, none where it read none. With `read`, each frame where one would start is
//! read as the hotkey reads it, and the names compared with the cache. `live` takes frames in the
//! order they were taken (named `app-<unix ms>.png`, or else a second apart) and feeds them to the
//! hold and the tab watch as the live loop does, printing each read and bag check that would
//! start. A gap of more than three seconds between frames starts both afresh.

use std::time::Instant;

use leafline_core::pipeline;
use leafline_core::readers::inventory::{self, TabWatch};
use leafline_core::readers::tooltip::{self, hover::{Rest, same_cards}};
use leafline_core::readers::tooltip::ItemTooltip;
use leafline_core::vision::{Frame, Ocr, Rect};
use serde_json::{Value, json};

fn places(frame: &Frame) -> Vec<Rect> {
    tooltip::name_lines(frame).iter().map(|n| n.bullet).collect()
}

fn names(items: &[ItemTooltip]) -> Vec<String> {
    items.iter().map(|t| t.name.clone()).collect()
}

/// Milliseconds: total and the slowest.
#[derive(Default)]
struct Timer {
    total: f64,
    most: f64,
    count: u32,
}

impl Timer {
    fn time<T>(&mut self, run: impl FnOnce() -> T) -> T {
        let started = Instant::now();
        let out = run();
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        self.total += ms;
        self.most = self.most.max(ms);
        self.count += 1;
        out
    }

    fn line(&self, what: &str) -> String {
        format!("{what}: {} runs, mean {:.1} ms, slowest {:.1} ms", self.count, self.total / f64::from(self.count.max(1)), self.most)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let read = args.iter().any(|a| a == "read");
    match args.first().map(String::as_str) {
        Some("tooltips") => tooltips(&args[1], read),
        Some("live") => live(&args[1], read),
        _ => panic!("usage: heldreads tooltips <program.jsonl> [read] | live <frames.txt> [read]"),
    }
}

fn tooltips(cache: &str, read: bool) {
    let ocr = read.then(|| Ocr::new().expect("Windows OCR"));
    let (mut looking, mut reading) = (Timer::default(), Timer::default());
    // [program read tooltips, it didn't] x [places found, none]
    let mut counts = [[0u32; 2]; 2];
    let [mut same, mut differ] = [0u32; 2];
    for line in std::fs::read_to_string(cache).expect("read the cache").lines() {
        let Ok(entry) = serde_json::from_str::<Value>(line) else { continue };
        let Some(path) = entry["path"].as_str() else { continue };
        let program: Vec<String> =
            entry["read"]["tooltips"].as_array().into_iter().flatten().filter_map(|t| t.as_str().map(str::to_string)).collect();
        let Ok(image) = image::open(path) else { continue };
        let frame = image.to_rgba8();
        let found = looking.time(|| places(&frame));
        counts[usize::from(program.is_empty())][usize::from(found.is_empty())] += 1;
        let mut out = json!({ "path": path, "program": program, "places": found.len() });
        if let Some(ocr) = ocr.as_ref().filter(|_| !found.is_empty()) {
            let items = reading.time(|| pipeline::read_items(&frame, ocr).unwrap_or_default());
            let got = names(&items);
            if got == program {
                same += 1;
            } else {
                differ += 1;
            }
            out["read"] = json!(got);
        }
        if !program.is_empty() || !found.is_empty() {
            println!("{out}");
        }
    }
    let [[hit, missed], [stray, quiet]] = counts;
    eprintln!("program read tooltips: {hit} would start a read, {missed} would not");
    eprintln!("program read none: {stray} would start a read, {quiet} would not");
    eprintln!("{}", looking.line("finding tooltips"));
    if read {
        eprintln!("held reads: {same} name the same as the cache, {differ} differ");
        eprintln!("{}", reading.line("reading them (Windows OCR)"));
    }
}

/// Unix seconds from a name like `app-1795151613031.png`.
fn taken_at(path: &str) -> Option<f64> {
    let name = path.rsplit(['/', '\\']).next()?;
    let ms: f64 = name.strip_prefix("app-")?.strip_suffix(".png")?.parse().ok()?;
    Some(ms / 1000.0)
}

fn live(list: &str, read: bool) {
    let ocr = read.then(|| Ocr::new().expect("Windows OCR"));
    let paths: Vec<String> = std::fs::read_to_string(list)
        .expect("read the list")
        .lines()
        .map(|l| l.split('\t').next().unwrap_or_default().trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    let (mut rest, mut bags) = (Rest::default(), TabWatch::default());
    let mut shown: Vec<ItemTooltip> = Vec::new();
    let (mut looking, mut bagging, mut reading) = (Timer::default(), Timer::default(), Timer::default());
    let [mut held_reads, mut cards, mut checks, mut with_tooltips, mut with_bag] = [0u32; 5];
    let mut last: Option<f64> = None;
    for (i, path) in paths.iter().enumerate() {
        let at = taken_at(path).unwrap_or(i as f64);
        if last.is_some_and(|l| at - l > 3.0) {
            rest = Rest::default();
            bags = TabWatch::default();
        }
        last = Some(at);
        let Ok(image) = image::open(path) else { continue };
        let frame = image.to_rgba8();
        let found = looking.time(|| places(&frame));
        with_tooltips += u32::from(!found.is_empty());
        rest.seen(found, at);
        if rest.due(at) {
            held_reads += 1;
            let mut out = json!({ "path": path, "at": at, "hover": "read" });
            if let Some(ocr) = &ocr {
                let items = reading.time(|| pipeline::read_items(&frame, ocr).unwrap_or_default());
                // As `live::read_hovered`: a card only for items, and only when they differ.
                let changed = !items.is_empty() && !same_cards(&shown, &items);
                out["read"] = json!(names(&items));
                out["card"] = json!(changed);
                if changed {
                    cards += 1;
                    shown = items;
                }
            }
            println!("{out}");
        }
        let seen = bagging.time(|| inventory::read(&frame));
        with_bag += u32::from(seen.as_ref().is_some_and(|i| i.tab.is_some()));
        if let Some(tab) = bags.settled(seen) {
            checks += 1;
            println!("{}", json!({ "path": path, "at": at, "bag": format!("{:?}", tab.tab), "slots": tab.slots.len() }));
            bags.checked(tab);
        }
    }
    eprintln!("{} frames: {with_tooltips} with tooltips, {held_reads} held reads, {cards} new cards", paths.len());
    eprintln!("{with_bag} frames with an inventory tab, {checks} bag checks");
    eprintln!("{}", looking.line("finding tooltips"));
    eprintln!("{}", bagging.line("reading the inventory"));
    if read {
        eprintln!("{}", reading.line("reading held tooltips (Windows OCR)"));
    }
}
