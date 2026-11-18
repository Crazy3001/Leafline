//! The capture test's part B from a terminal: reads the program's and the website's saved frames
//! in a folder with the same readers and prints how far they agree, as the capture test window's
//! Compare button does. The report is also written into the folder.
//!
//! `cargo run --release --example capturecompare -- <folder>`

use std::path::PathBuf;

use leafline_lib::capturetest::agree;
use leafline_lib::vision::Ocr;

fn main() {
    let folder = PathBuf::from(std::env::args().nth(1).expect("usage: capturecompare <folder>"));
    let ocr = Ocr::new().expect("OCR engine");
    let report = agree::compare(&folder, &ocr, |done, total| {
        if done % 20 == 0 || done == total {
            eprintln!("{done} / {total} frames read");
        }
    })
    .expect("compare");
    println!(
        "{} program frames, {} website frames, {} pairs; website frames {}",
        report.program_frames,
        report.website_frames,
        report.pairs,
        match (report.website_offset, report.website_resized) {
            (_, true) => "resized to the program's size".to_string(),
            (Some((0, 0)), _) => "the same size".to_string(),
            (Some((x, y)), _) => format!("cut at {x},{y}"),
            (None, _) => "-".to_string(),
        }
    );
    for r in &report.readers {
        println!(
            "{:<18} {:>7}  agree {:>4}  differ {:>4}  only program {:>4}  only website {:>4}  neither {:>4}{}",
            r.reader,
            r.share.map_or("-".into(), |s| format!("{:.1}%", s * 100.0)),
            r.agree,
            r.differ,
            r.only_program,
            r.only_website,
            r.neither,
            if r.gate { "  (pass bar)" } else { "" }
        );
    }
    let p = &report.pickup;
    println!(
        "Pickup Log         {:>7}  program {} lines, website {}, matched {}",
        p.share.map_or("-".into(), |s| format!("{:.1}%", s * 100.0)),
        p.program_lines,
        p.website_lines,
        p.matched
    );
    println!("\nPass bar (level, EXP, mesos, map name at 99%+): {}", if report.passes { "passes" } else { "not met" });
    std::fs::write(folder.join(agree::REPORT_FILE), serde_json::to_string_pretty(&report).unwrap()).expect("write report");
}
