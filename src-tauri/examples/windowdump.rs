//! Reads the Character Stat and Skill windows in screenshots, to check them by eye.
//!
//! `cargo run --release --example windowdump -- <image>...`

use std::path::Path;
use std::time::Instant;

use leafline_lib::pipeline;
use leafline_lib::readers::{character, hud};
use leafline_lib::vision::{self, Ocr};

fn main() {
    let ocr = Ocr::new().expect("OCR engine");
    for path in std::env::args().skip(1) {
        let frame = vision::load(Path::new(&path)).expect("load image");
        let scan = pipeline::read_character(&frame, &ocr).expect("read");
        println!(
            "{path}\n  {:?} Lv {:?}\n  {:?}\n  {:?}\n  {:?}",
            scan.name, scan.level, scan.windows.stats, scan.windows.detail, scan.windows.skills
        );
        if let Some((layout, reading)) = hud::read(&frame, &ocr).expect("read HUD") {
            let started = Instant::now();
            let glance = character::glance(&frame, &layout, reading.level);
            println!("  glance in {:?}: {glance:?}", started.elapsed());
        }
    }
}
