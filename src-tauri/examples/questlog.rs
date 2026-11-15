//! Reads the Quest window in saved screenshots and prints what was read as JSON.
//!
//! `cargo run --example questlog -- <image or folder> [--quests <OSMS quests.json>]`
//!
//! With `--quests`, each row also shows the quest id it matches in that file.

use std::path::{Path, PathBuf};

use leafline_lib::mapdata::Atlas;
use leafline_lib::mapdata::osms::{Dataset, Records};
use leafline_lib::readers::questlog::{self, LogItem};
use leafline_lib::vision::{self, Ocr};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let target = PathBuf::from(args.first().expect("usage: questlog <image or folder> [--quests <file>]"));
    let atlas = args.iter().position(|a| a == "--quests").map(|i| {
        let body = std::fs::read(&args[i + 1]).expect("read quests file");
        let Records::Quests(quests) = Records::parse("quests.json", &body).expect("parse quests") else { unreachable!() };
        Atlas::new(Dataset { quests, ..Dataset::default() })
    });
    let ocr = Ocr::new().expect("OCR engine");
    for path in images(&target) {
        let frame = vision::load(&path).expect("load image");
        let started = std::time::Instant::now();
        let reading = questlog::read(&frame, None, &ocr).expect("read");
        let ms = started.elapsed().as_millis();
        println!("# {} ({ms} ms)\n{}", path.display(), serde_json::to_string_pretty(&reading).unwrap());
        let (Some(atlas), Some(reading)) = (&atlas, &reading) else { continue };
        let mut region = None;
        for item in &reading.items {
            match item {
                LogItem::Region { name, .. } => region = Some(name.clone()),
                LogItem::Quest(q) => {
                    let readings: Vec<&str> =
                        std::iter::once(q.title.as_str()).chain(q.readings.iter().map(String::as_str)).collect();
                    println!("  {:?} -> {:?}", q.title, atlas.log_quest(&readings, q.cut, region.as_deref()));
                }
            }
        }
    }
}

fn images(target: &Path) -> Vec<PathBuf> {
    if target.is_file() {
        return vec![target.to_path_buf()];
    }
    let mut paths: Vec<PathBuf> = std::fs::read_dir(target)
        .expect("read folder")
        .filter_map(|e| Some(e.ok()?.path()))
        .filter(|p| vision::is_supported(p))
        .collect();
    paths.sort();
    paths
}
