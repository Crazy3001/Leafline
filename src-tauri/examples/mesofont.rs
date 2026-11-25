//! Collects the meso amount's digit shapes from saved frames, labelled by what the program's own
//! reader (with OCR) reads there, to build and check the table the website reads mesos with.
//! Frames are only read; nothing is kept but the letter shapes printed.
//!
//! `cargo run --release --example mesofont -- <image or folder>... [--json <file>]`
//!
//! With `--shapes` it instead prints what the website reads (the live pipeline without OCR).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use leafline_lib::pipeline;
use leafline_lib::readers::mesos::{self, Ink};
use leafline_lib::vision::{self, Ocr};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().position(|a| a == "--json").map(|i| PathBuf::from(&args[i + 1]));
    let targets: Vec<&String> =
        args.iter().enumerate().filter(|(i, a)| !a.starts_with("--") && (*i == 0 || args[i - 1] != "--json")).map(|(_, a)| a).collect();
    let shapes_only = args.iter().any(|a| a == "--shapes");
    let ocr = if shapes_only { Ocr::none() } else { Ocr::new().expect("OCR engine") };
    // (ink, char) -> glyph rows -> how many times
    let mut seen: BTreeMap<(String, char), BTreeMap<Vec<String>, usize>> = BTreeMap::new();
    let mut frames = 0;
    let mut labelled = 0;
    for path in targets.iter().flat_map(|t| images(Path::new(t))) {
        let frame = vision::load(&path).expect("load image");
        let boxes = mesos::locate(&frame);
        if boxes.is_empty() {
            continue;
        }
        frames += 1;
        if shapes_only {
            let analysis = pipeline::analyze_live(&frame, &Ocr::none()).expect("analyze");
            let read = analysis.mesos.map_or("-".to_string(), |m| grouped(m.mesos));
            let gaps: Vec<String> =
                mesos::right_gaps(&frame, Ink::CRISP).iter().map(|g| g.map_or("-".into(), |g| format!("{g:.2}"))).collect();
            let whole = mesos::whole_boxes(&frame);
            let ratios: Vec<String> =
                boxes.iter().zip(whole).map(|(b, w)| format!("{:.1}x{:.0}{}@{:.0},{:.0},{:.0},{:.0}", b.w / b.h, b.h, if w { "" } else { " CUT" }, b.x, b.y, b.w, b.h)).collect();
            println!("{}  {} box(es), shapes {}  gaps {}  boxes {}", path.display(), boxes.len(), read, gaps.join(" "), ratios.join(" "));
            continue;
        }
        let analysis = pipeline::analyze(&frame, &ocr).expect("analyze");
        let Some(reading) = analysis.mesos else {
            println!("{}  {} box(es), not read", path.display(), boxes.len());
            continue;
        };
        let text = grouped(reading.mesos);
        let mut used = Vec::new();
        for (name, ink) in [("crisp", Ink::CRISP), ("smoothed", Ink::SMOOTHED)] {
            for glyphs in mesos::box_glyphs(&frame, ink) {
                if glyphs.len() != text.chars().count() {
                    continue;
                }
                for (c, mask) in text.chars().zip(glyphs) {
                    *seen.entry((name.to_string(), c)).or_default().entry(mask.into()).or_default() += 1;
                }
                used.push(name);
            }
        }
        if !used.is_empty() {
            labelled += 1;
        }
        println!("{}  {} box(es), read {}  labelled by {:?}", path.display(), boxes.len(), text, used);
    }
    println!("\n{frames} frames with a meso box, {labelled} labelled\n");
    for ((ink, c), shapes) in &seen {
        println!("{ink} '{c}': {} shape(s), seen {:?}", shapes.len(), shapes.values().collect::<Vec<_>>());
        for (rows, n) in shapes {
            println!("  x{n} {}x{}", rows.first().map_or(0, |r| r.len()), rows.len());
            for r in rows {
                println!("    {r}");
            }
        }
    }
    if let Some(file) = json {
        let out: Vec<_> = seen
            .iter()
            .flat_map(|((ink, c), shapes)| shapes.iter().map(move |(rows, n)| (ink.clone(), c.to_string(), rows.clone(), *n)))
            .collect();
        std::fs::write(file, serde_json::to_string(&out).unwrap()).expect("write json");
    }
}

/// `36644` as the game draws it, `36,644`.
fn grouped(amount: u64) -> String {
    let digits = amount.to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
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
