//! Runs the analysis pipeline on saved screenshots and prints the readings as JSON.
//!
//! `cargo run --example analyze -- <image or folder> [--overview] [--font hud-font.json] [--npcs]`
//!
//! `--font` loads a HUD font saved by the app's calibration; without it HUD numbers are not read.
//! `--npcs` prints only the NPC name tags, which live reading reads on maps sharing a header.

use std::path::{Path, PathBuf};

use leafline_lib::pipeline;
use leafline_lib::readers::hud::HudFont;
use leafline_lib::readers::npc;
use leafline_lib::vision::{self, Ocr};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let target = PathBuf::from(args.first().expect("usage: analyze <image or folder> [--overview] [--font <file>]"));
    let show_overview = args.iter().any(|a| a == "--overview");
    let font: HudFont = match args.iter().position(|a| a == "--font") {
        Some(i) => {
            serde_json::from_str(&std::fs::read_to_string(&args[i + 1]).expect("read font")).expect("parse font")
        }
        None => HudFont::default(),
    };
    let ocr = Ocr::new().expect("OCR engine");

    for path in images(&target) {
        let frame = vision::load(&path).expect("load image");
        if args.iter().any(|a| a == "--npcs") {
            println!("# {}\n{:?}", path.display(), npc::names(&frame, &ocr).expect("read name tags"));
            continue;
        }
        let mut analysis = pipeline::analyze(&frame, &ocr, &font).expect("analyze");
        if !show_overview {
            analysis.overview.clear();
        }
        println!("# {}\n{}", path.display(), serde_json::to_string_pretty(&analysis).unwrap());
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
