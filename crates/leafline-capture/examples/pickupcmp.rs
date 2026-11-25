//! Reads the Pickup Log from a folder of pictures taken in order twice, as the program reads it
//! (its game-font reader with Windows OCR for lines it can't finish) and as the website must
//! (no OCR), and prints where the two differ. Pictures stay on this PC; nothing is written.
//!
//!     cargo run --release -p leafline-capture --example pickupcmp -- <folder> [--prefix app-] [--size 2560x1440] [--ui-scale 1.875] [--items items.json]
//!
//! Pictures are taken in name order, the number in each name its time in milliseconds. A
//! picture smaller than `--size` is a crop of the window's bottom right, placed there on black.
//! With `--items` (OSMS `items.json`) items are compared by the item each names, as the session
//! counts them: a glyph the font doesn't know stands as a gap that the item's name fills.
//!
//! Measured 2026-11-20 on the alt run's saved frames (a picture a second): on the
//! program's pictures the website named 38 of 39 lines right, the program 36 (its OCR made three
//! unnameable); on the browser's pictures the website 39 of 39. A Default Ratio recording at four
//! a second: 17 of 17 alike.

use std::path::{Path, PathBuf};

use image::Rgba;
use leafline_core::build::catalog::Catalog;
use leafline_core::mapdata::loot::name_item;
use leafline_core::mapdata::osms::Records;
use leafline_core::readers::pickup::{Gain, PickupReader};
use leafline_core::readers::{hud, ui};
use leafline_core::vision::{Frame, Ocr};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let folder = PathBuf::from(args.first().expect("usage: pickupcmp <folder> [--prefix p] [--size WxH] [--ui-scale n]"));
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let prefix = value("--prefix").unwrap_or_default();
    let size = value("--size").map(|v| {
        let (w, h) = v.split_once('x').expect("--size WxH");
        (w.parse::<u32>().unwrap(), h.parse::<u32>().unwrap())
    });
    let fixed_scale: Option<f32> = value("--ui-scale").map(|v| v.parse().expect("--ui-scale n"));
    // `--items items.json` names items as the session would, from the item data.
    let catalog = value("--items").map(|path| {
        let Records::Items(items) = Records::parse("items.json", &std::fs::read(path).expect("items")).expect("parse") else { unreachable!() };
        Catalog::new(Vec::new(), items)
    });
    let named = |gain: Gain| match (gain, catalog.as_ref()) {
        (Gain::Item { name, quantity, tab, cut_short }, Some(catalog)) => match name_item(catalog, &name, tab, cut_short) {
            Some(item) => Gain::Item { name: item.name.clone(), quantity, tab: None, cut_short: false },
            None => Gain::Item { name: format!("unnamed {name}"), quantity, tab: None, cut_short: false },
        },
        (gain, _) => gain,
    };
    let windows = Ocr::new().expect("Windows OCR");
    let none = Ocr::none();
    let (mut program, mut website) = (PickupReader::default(), PickupReader::default());
    let mut program_lines: Vec<(f64, Gain)> = Vec::new();
    let mut website_lines: Vec<(f64, Gain)> = Vec::new();
    let mut ui_scale = fixed_scale;
    for path in pictures(&folder, &prefix) {
        let at = millis(&path) as f64 / 1000.0;
        let Ok(image) = image::open(&path) else { continue };
        let mut frame = image.to_rgba8();
        if let Some((w, h)) = size
            && frame.dimensions() != (w, h)
        {
            frame = placed(&frame, w, h);
        }
        if fixed_scale.is_none()
            && let Some(layout) = hud::locate(&frame)
        {
            ui_scale = Some(ui::hud_scale(&frame, &layout));
        }
        let read = program.read(at, &frame, ui_scale, &windows);
        program_lines.extend(read.arrivals.into_iter().filter_map(|a| Some((a.at, a.line?.gain))));
        let read = website.read(at, &frame, ui_scale, &none);
        website_lines.extend(read.arrivals.into_iter().filter_map(|a| Some((a.at, a.line?.gain))));
    }
    program_lines.extend(program.finish().into_iter().filter_map(|a| Some((a.at, a.line?.gain))));
    website_lines.extend(website.finish().into_iter().filter_map(|a| Some((a.at, a.line?.gain))));

    let program_lines: Vec<(f64, Gain)> = program_lines.into_iter().map(|(at, g)| (at, named(g))).collect();
    let website_lines: Vec<(f64, Gain)> = website_lines.into_iter().map(|(at, g)| (at, named(g))).collect();
    let mut used = vec![false; website_lines.len()];
    let mut matched = 0;
    for (at, gain) in &program_lines {
        let found = website_lines.iter().enumerate().position(|(i, (w_at, w_gain))| !used[i] && w_gain == gain && (w_at - at).abs() <= 0.6);
        match found {
            Some(i) => {
                used[i] = true;
                matched += 1;
            }
            None => println!("program only {at:8.3} {gain:?}"),
        }
    }
    for (i, (at, gain)) in website_lines.iter().enumerate() {
        if !used[i] {
            println!("website only {at:8.3} {gain:?}");
        }
    }
    println!("{}: program {} lines, website {}, alike {matched}", folder.display(), program_lines.len(), website_lines.len());
}

/// `crop` at the bottom right of a black picture `width` by `height`.
fn placed(crop: &Frame, width: u32, height: u32) -> Frame {
    let mut frame = Frame::from_pixel(width, height, Rgba([0, 0, 0, 255]));
    image::imageops::replace(&mut frame, crop, i64::from(width - crop.width()), i64::from(height - crop.height()));
    frame
}

fn millis(path: &Path) -> u64 {
    let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default();
    name.chars().filter(char::is_ascii_digit).collect::<String>().parse().unwrap_or(0)
}

fn pictures(folder: &Path, prefix: &str) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(folder)
        .expect("read folder")
        .filter_map(|e| Some(e.ok()?.path()))
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")))
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(prefix)))
        .collect();
    paths.sort();
    paths
}
