//! Replays a folder of screenshots as a session and prints what the tracker made of it.
//!
//! `cargo run --release --example replay -- <folder> [--every <seconds>]`
//!
//! Frames are ordered by file modified time, or by name `--every` seconds apart (for frames
//! extracted from a recording).

use std::path::PathBuf;

use leafline_lib::replay::{self, Order};
use leafline_lib::vision::Ocr;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let folder = PathBuf::from(args.first().expect("usage: replay <folder> [--every <seconds>]"));
    let value = |flag: &str| args.iter().position(|a| a == flag).map(|i| args[i + 1].clone());
    let order = match value("--every") {
        Some(seconds) => Order::Name { interval_seconds: seconds.parse().expect("--every takes seconds") },
        None => Order::Modified,
    };
    let ocr = Ocr::new().expect("OCR engine");
    let frames = replay::frames(&folder, order).expect("list frames");
    let start = frames.first().map_or(0.0, |f| f.at);

    let summary = replay::run(&frames, order.tracker_config(), &ocr, |_, frame, analysis, _| {
        let hud = analysis.hud.as_ref();
        println!(
            "{:>7.0} s  {:<14} LV {:<4} EXP {:<20} mesos {:<8} {}",
            frame.at - start,
            frame.path.file_name().unwrap_or_default().to_string_lossy(),
            hud.and_then(|h| h.level).map_or("-".into(), |l| l.to_string()),
            hud.and_then(|h| h.exp).map_or("-".into(), |e| format!("{} ({:.2}%)", e.points, e.percent)),
            analysis.mesos.map_or("-".into(), |m| m.mesos.to_string()),
            analysis.map.as_ref().map_or("-".into(), |m| m.label()),
        );
        true
    })
    .expect("replay");
    println!("\n{}", serde_json::to_string_pretty(&summary).unwrap());
}
