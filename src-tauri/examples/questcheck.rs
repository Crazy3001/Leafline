//! Scores the Quest Helper reader against hand-checked rows, frame by frame. For measuring a
//! reader change on pictures kept on this PC; the pictures and the list stay here.
//!
//! `cargo run --release --example questcheck -- <expected.json> [--show]`
//!
//! The list maps each picture's path to `{"quests": [{"title", "objectives": [[current,
//! required, target]]}]}` and or `{"map": {"street", "name"}}`, read by eye. A title or objective is right only when it is exactly as
//! drawn; one left out counts as missed, one wrong as wrong (a wrong reading is worse than none).

use std::path::Path;

use leafline_lib::pipeline;
use leafline_lib::vision::{self, Ocr};
use serde_json::{Map, Value};

#[derive(Default)]
struct Tally {
    right: usize,
    wrong: usize,
    missed: usize,
}

impl Tally {
    fn add(&mut self, want: &Value, got: Option<&Value>) -> &'static str {
        match got {
            Some(got) if got == want => {
                self.right += 1;
                "ok"
            }
            Some(_) => {
                self.wrong += 1;
                "WRONG"
            }
            None => {
                self.missed += 1;
                "missed"
            }
        }
    }

    fn line(&self, what: &str) -> String {
        let total = self.right + self.wrong + self.missed;
        format!("{what:<12} {:>4} right, {:>3} wrong, {:>3} missed of {total}", self.right, self.wrong, self.missed)
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let list = args.first().expect("usage: questcheck <expected.json> [--show]");
    let show = args.iter().any(|a| a == "--show");
    let expected: Map<String, Value> =
        serde_json::from_str(&std::fs::read_to_string(list).expect("read the list")).expect("parse the list");
    let ocr = Ocr::new().expect("OCR engine");
    let (mut titles, mut counters, mut targets, mut maps) = (Tally::default(), Tally::default(), Tally::default(), Tally::default());
    for (file, want) in &expected {
        let Ok(frame) = vision::load(Path::new(file)) else {
            eprintln!("can't load {file}");
            continue;
        };
        let got = serde_json::to_value(pipeline::analyze(&frame, &ocr).expect("analyze")).unwrap();
        if let Some(map) = want.get("map") {
            for field in ["street", "name"] {
                let verdict = maps.add(&map[field], Some(&got["map"][field]));
                if show && verdict != "ok" {
                    println!("{file}: map {field} {verdict}: want {} got {}", map[field], got["map"][field]);
                }
            }
        }
        let Some(want_quests) = want.get("quests") else { continue };
        if args.iter().any(|a| a == "--dump") {
            println!("{file}: {}", serde_json::to_string(&got["quest"]["quests"]).unwrap());
        }
        let got_quests = got["quest"]["quests"].as_array().cloned().unwrap_or_default();
        // Quests are matched in order; a quest the reader lost shifts nothing, since each is
        // looked up by its place among those with the same first objective or title.
        for (i, quest) in want_quests.as_array().unwrap().iter().enumerate() {
            let got_quest = got_quests.get(i);
            let title = &quest["title"];
            if title.as_str().is_some_and(|t| !t.is_empty()) {
                let verdict = titles.add(title, got_quest.map(|q| &q["title"]));
                if show && verdict != "ok" {
                    println!("{file}: title {verdict}: want {title} got {:?}", got_quest.map(|q| &q["title"]));
                }
            }
            let got_objectives = got_quest.and_then(|q| q["objectives"].as_array()).cloned().unwrap_or_default();
            for (j, objective) in quest["objectives"].as_array().unwrap().iter().enumerate() {
                let got = got_objectives.get(j);
                let counter = Value::from(vec![objective[0].clone(), objective[1].clone()]);
                let got_counter = got.map(|o| Value::from(vec![o["current"].clone(), o["required"].clone()]));
                let verdict = counters.add(&counter, got_counter.as_ref());
                if show && verdict != "ok" {
                    println!("{file}: counter {verdict}: want {counter} got {got_counter:?}");
                }
                let verdict = targets.add(&objective[2], got.map(|o| &o["target"]));
                if show && verdict != "ok" {
                    println!("{file}: target {verdict}: want {} got {:?}", objective[2], got.map(|o| &o["target"]));
                }
            }
        }
    }
    for (tally, what) in [(titles, "titles"), (counters, "counters"), (targets, "targets"), (maps, "map lines")] {
        if tally.right + tally.wrong + tally.missed > 0 {
            println!("{}", tally.line(what));
        }
    }
}
