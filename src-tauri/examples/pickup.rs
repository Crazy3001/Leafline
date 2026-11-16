//! Runs the Pickup Log reader over a folder of pictures taken in order and prints each line it
//! counts. For checking the reader against a recording on this PC; the pictures stay here.
//!
//! `cargo run --example pickup -- <folder> [--every <n>] [--size <width>x<height>] [--rows]`
//!
//! Pictures are taken in name order, and the number in each name is its time in milliseconds
//! (`br-001265.png`). A picture smaller than `--size` is a crop of the window's bottom right and
//! is placed there on a black window of that size. `--every 2` reads every second picture, as a
//! slower rate would. `--rows` prints each picture's rows as well.

use std::path::{Path, PathBuf};

use image::Rgba;
use leafline_lib::readers::pickup::{PickupReader, look};
use leafline_lib::vision::{self, Frame, Ocr};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let folder = PathBuf::from(args.first().expect("usage: pickup <folder> [--every <n>] [--size WxH] [--rows]"));
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let every: usize = value("--every").map_or(1, |v| v.parse().expect("--every n"));
    let size = value("--size").map(|v| {
        let (w, h) = v.split_once('x').expect("--size WxH");
        (w.parse::<u32>().unwrap(), h.parse::<u32>().unwrap())
    });
    let show_rows = args.iter().any(|a| a == "--rows");
    let ocr = Ocr::new().expect("OCR engine");
    let mut reader = PickupReader::default();
    let mut timings: Vec<(f64, bool)> = Vec::new();
    // `--items <map data file>` names items as Leafline would, from a copy of its map data.
    let catalog = value("--items").map(|path| {
        let data = leafline_lib::mapdata::store::Store::open(std::path::Path::new(&path)).expect("open map data").load().expect("load").expect("downloaded");
        leafline_lib::build::catalog::Catalog::new(Vec::new(), data.items)
    });
    let named = |gain: &leafline_lib::readers::pickup::Gain| match (gain, catalog.as_ref()) {
        (leafline_lib::readers::pickup::Gain::Item { name, tab, cut_short, .. }, Some(catalog)) => {
            format!("  -> {:?}", leafline_lib::mapdata::loot::name_item(catalog, name, *tab, *cut_short).map(|i| i.name.clone()))
        }
        _ => String::new(),
    };
    let prefix = value("--prefix").unwrap_or_default();
    for path in pictures(&folder, &prefix).into_iter().step_by(every) {
        let at = millis(&path) as f64 / 1000.0;
        let mut frame = vision::load(&path).expect("load picture");
        if let Some((w, h)) = size
            && frame.dimensions() != (w, h)
        {
            frame = placed(&frame, w, h);
        }
        // `--ui-scale 1.875` for a recording at UI Size Default Ratio at 1440p.
        let ui_scale: Option<f32> = value("--ui-scale").map(|v| v.parse().expect("--ui-scale n"));
        let started = std::time::Instant::now();
        let read = reader.read(at, &frame, ui_scale, &ocr);
        let ms = started.elapsed().as_secs_f64() * 1000.0;
        timings.push((ms, read.layout.is_some()));
        if let Some(view) = leafline_lib::readers::pickup::view(&frame, ui_scale) {
            frame = view;
        }
        if args.iter().any(|a| a == "--layout") {
            println!("{at:7.3}  layout {:?}", read.layout);
        }
        if show_rows && let Some(layout) = read.layout {
            let rows = look::rows(&frame, &layout);
            let seen: Vec<String> =
                rows.iter().map(|r| r.as_ref().map_or("  -  ".into(), |r| format!("{:.2}{}", r.brightness, colour(r)))).collect();
            println!("{at:7.3}  {}", seen.join(" "));
        }
        if let (true, Some(layout)) = (args.iter().any(|a| a == "--ocr"), read.layout) {
            for (slot, row) in look::rows(&frame, &layout).iter().enumerate() {
                let Some(row) = row else { continue };
                let zoom: u32 = value("--zoom").map_or(4, |z| z.parse().expect("--zoom n"));
                let Some(picture) = look::line_picture(&frame, &layout, slot, zoom) else { continue };
                let mut words = ocr.words(&picture).unwrap_or_default();
                words.sort_by(|a, b| a.rect.x.total_cmp(&b.rect.x));
                let text: Vec<&str> = words.iter().map(|w| w.text.as_str()).collect();
                let parsed = leafline_lib::readers::pickup::parse::parse(&text.join(" "));
                let shapes = look::line_ink(&frame, &layout, slot).as_deref().and_then(leafline_lib::readers::pickup::figure::figure);
                let font = look::line_ink(&frame, &layout, slot).as_deref().and_then(leafline_lib::readers::pickup::font::read_line);
                println!("{at:7.3}  [{slot}] {:.2} {} {:?} => {:?} | digits {shapes:?} | font {font:?}", row.brightness, if parsed.is_some() { "OK" } else { "--" }, text.join(" "), parsed.map(|p| p.gain));
                if let Some(dir) = value("--dump") {
                    let _ = picture.save(format!("{dir}/{:06}-{slot}.png", (at * 1000.0) as u64));
                }
            }
        }
        for arrival in read.arrivals {
            let to = arrival.line.as_ref().map(|l| named(&l.gain)).unwrap_or_default();
            println!("{:7.3}  + {:?}{to}", arrival.at, arrival.line);
        }
    }
    if args.iter().any(|a| a == "--time") {
        let mut found: Vec<f64> = timings.iter().filter(|t| t.1).map(|t| t.0).collect();
        let mut searching: Vec<f64> = timings.iter().filter(|t| !t.1).map(|t| t.0).collect();
        let show = |name: &str, v: &mut Vec<f64>| {
            if v.is_empty() {
                return;
            }
            v.sort_by(f64::total_cmp);
            let mean = v.iter().sum::<f64>() / v.len() as f64;
            println!("time {name}: {} looks, mean {mean:.1} ms, p95 {:.1} ms, max {:.1} ms", v.len(), v[v.len() * 95 / 100], v[v.len() - 1]);
        };
        show("log found", &mut found);
        show("searching", &mut searching);
    }
    for arrival in reader.finish() {
        let to = arrival.line.as_ref().map(|l| named(&l.gain)).unwrap_or_default();
        println!("{:7.3}  + {:?}{to}", arrival.at, arrival.line);
    }
}

fn colour(row: &leafline_lib::readers::pickup::Row) -> &'static str {
    match row.colour {
        leafline_lib::readers::pickup::Colour::White => "w",
        leafline_lib::readers::pickup::Colour::Yellow => "y",
        leafline_lib::readers::pickup::Colour::Other => "?",
    }
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
        .filter(|p| vision::is_supported(p))
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(prefix)))
        .collect();
    paths.sort();
    paths
}
