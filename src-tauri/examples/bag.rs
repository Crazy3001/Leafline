//! Reads the Item Inventory in saved screenshots and names what each slot holds, against item
//! pictures kept in a folder as `<item id>.png` and the OSMS item list. A developer check of the
//! sell-or-keep reader; nothing here ships, and the pictures and frames stay off the repo.
//!
//! `cargo run --release --example bag -- <image or folder> <pictures folder> <items.json>`

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use leafline_lib::mapdata::bag;
use leafline_lib::readers::inventory::{self, Tab};
use leafline_lib::vision;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let usage = "usage: bag <image or folder> <pictures folder> <items.json>";
    let (target, folder, list) = (
        PathBuf::from(args.first().expect(usage)),
        PathBuf::from(args.get(1).expect(usage)),
        PathBuf::from(args.get(2).expect(usage)),
    );
    let items: Vec<serde_json::Value> = serde_json::from_slice(&std::fs::read(list).expect("item list")).expect("item list");
    let about: HashMap<u32, (String, String, String)> = items
        .iter()
        .filter_map(|i| {
            let id = i["id"].as_u64()? as u32;
            let text = |key: &str| i[key].as_str().unwrap_or_default().to_string();
            Some((id, (text("name"), text("category"), text("sub_category"))))
        })
        .collect();
    let pictures: Vec<(u32, image::RgbaImage)> = std::fs::read_dir(&folder)
        .expect("pictures folder")
        .filter_map(|e| {
            let path = e.ok()?.path();
            let id: u32 = path.file_stem()?.to_str()?.parse().ok()?;
            Some((id, image::open(&path).ok()?.to_rgba8()))
        })
        .collect();
    println!("{} pictures", pictures.len());

    for path in images(&target) {
        let frame = vision::load(&path).expect("load image");
        let started = std::time::Instant::now();
        let Some(bag) = inventory::read(&frame) else {
            println!("# {}: no inventory", path.display());
            continue;
        };
        let read_ms = started.elapsed().as_secs_f32() * 1000.0;
        let tab = bag.tab;
        let fits = |category: &str, sub: &str| match tab {
            Some(Tab::Equip) => category == "Equipment",
            Some(Tab::Use) => category == "Consumable",
            Some(Tab::Setup) => category == "Setup",
            Some(Tab::Etc) => category == "Etc",
            _ => sub.is_empty() && false,
        };
        let candidates: Vec<(u32, &image::RgbaImage)> = pictures
            .iter()
            .filter(|(id, _)| about.get(id).is_some_and(|(_, c, s)| fits(c, s)))
            .map(|(id, p)| (*id, p))
            .collect();
        let started = std::time::Instant::now();
        println!("# {} — tab {:?}, {} slots, {} candidates, read in {read_ms:.1} ms", path.display(), tab, bag.slots.len(), candidates.len());
        for slot in &bag.slots {
            let named = bag::name(&slot.picture, &candidates);
            let text = match &named {
                Some(n) => {
                    let names: Vec<&str> = n.ids.iter().filter_map(|id| about.get(id).map(|a| a.0.as_str())).collect();
                    format!("{} ({:.1})", names.join(" | "), n.score)
                }
                None => "?".to_string(),
            };
            println!("  slot {:2} count {:>5}  {}", slot.index, slot.count.map(|c| c.to_string()).unwrap_or_default(), text);
        }
        println!("  named in {:.0} ms", started.elapsed().as_secs_f32() * 1000.0);
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
