//! Scores the readers against hand-checked values for a folder of screenshots.
//!
//! `cargo run --release --example check -- [folder]` (default `../test-screenshots`)
//!
//! The folder needs an `expected.json` mapping each file name to what the screenshot really
//! shows; fields left out are not scored. Prints every mismatch and a total.

use std::path::PathBuf;

use leafline_lib::pipeline;
use leafline_lib::vision::{self, Ocr};
use serde_json::{Map, Value};

fn main() {
    let folder = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "../test-screenshots".into()));
    let expected: Map<String, Value> =
        serde_json::from_str(&std::fs::read_to_string(folder.join("expected.json")).expect("read expected.json"))
            .expect("parse expected.json");
    let ocr = Ocr::new().expect("OCR engine");

    let mut score = Score::default();
    for (file, want) in expected.iter().filter(|(k, _)| !k.starts_with('_')) {
        let frame = vision::load(&folder.join(file)).expect("load image");
        let got = serde_json::to_value(pipeline::analyze(&frame, &ocr).expect("analyze")).unwrap();
        let before = score.clone();
        let mut check = |what: String, want: &Value, got: &Value| score.check(file, what, want, got);

        if let Some(hud) = want.get("hud") {
            check_hud(&mut check, hud, &got["hud"]);
        }
        if let Some(tips) = want.get("tooltips").and_then(Value::as_array) {
            let got_tips = got["tooltips"].as_array().cloned().unwrap_or_default();
            check("tooltip count".into(), &tips.len().into(), &got_tips.len().into());
            for (i, tip) in tips.iter().enumerate() {
                check_tooltip(&mut check, i, tip, got_tips.get(i).unwrap_or(&Value::Null));
            }
        }
        if let Some(quests) = want.get("quests").and_then(Value::as_array) {
            let got_quests = got["quest"]["quests"].as_array().cloned().unwrap_or_default();
            check("quest count".into(), &quests.len().into(), &got_quests.len().into());
            for (i, quest) in quests.iter().enumerate() {
                check_quest(&mut check, i, quest, got_quests.get(i).unwrap_or(&Value::Null));
            }
        }
        if let Some(mesos) = want.get("mesos") {
            check("mesos".into(), mesos, &got["mesos"]["mesos"]);
        }
        if let Some(map) = want.get("map") {
            for field in ["street", "name"] {
                check(format!("map {field}"), &map[field], &got["map"][field]);
            }
        }
        println!("{file}: {}/{}", score.passed - before.passed, score.total - before.total);
    }
    println!("\n{} of {} checks pass ({:.0}%)", score.passed, score.total, score.percent());
}

fn check_hud(check: &mut impl FnMut(String, &Value, &Value), want: &Value, got: &Value) {
    if want.is_null() || got.is_null() {
        check("hud found".into(), &want.is_null().into(), &got.is_null().into());
        return;
    }
    for field in ["level", "job", "name"] {
        if let Some(w) = want.get(field) {
            check(format!("hud {field}"), w, &got[field]);
        }
    }
    for gauge in ["hp", "mp"] {
        if let Some(w) = want.get(gauge) {
            check(
                format!("hud {gauge}"),
                w,
                &Value::from(vec![got[gauge]["current"].clone(), got[gauge]["max"].clone()]),
            );
        }
    }
    if let Some(w) = want.get("exp") {
        check("hud exp points".into(), &w[0], &got["exp"]["points"]);
        let close = (w[1].as_f64().unwrap() - got["exp"]["percent"].as_f64().unwrap_or(-1.0)).abs() < 0.005;
        check("hud exp percent".into(), &w[1], if close { &w[1] } else { &got["exp"]["percent"] });
    }
}

fn check_tooltip(check: &mut impl FnMut(String, &Value, &Value), i: usize, want: &Value, got: &Value) {
    for field in ["name", "upgrades", "itemType", "remainingEnhancements"] {
        if let Some(w) = want.get(field) {
            check(format!("tooltip {i} {field}"), w, &got[field]);
        }
    }
    if let Some(reqs) = want.get("requirements").and_then(Value::as_object) {
        for (stat, w) in reqs {
            check(format!("tooltip {i} REQ {stat}"), w, &got["requirements"][stat]);
        }
    }
    if let Some(w) = want.get("eligible") {
        let eligible: Vec<Value> = got["jobs"]
            .as_array()
            .map(|jobs| jobs.iter().filter(|j| j["eligible"] == true).map(|j| j["job"].clone()).collect())
            .unwrap_or_default();
        check(format!("tooltip {i} jobs"), w, &eligible.into());
    }
    if let Some(stats) = want.get("stats").and_then(Value::as_object) {
        let got_stats = got["stats"].as_array().cloned().unwrap_or_default();
        for (label, w) in stats {
            let value =
                got_stats.iter().find(|s| s["label"] == label.as_str()).map_or(Value::Null, |s| s["value"].clone());
            check(format!("tooltip {i} stat {label}"), w, &value);
        }
    }
}

fn check_quest(check: &mut impl FnMut(String, &Value, &Value), i: usize, want: &Value, got: &Value) {
    check(format!("quest {i} title"), &want["title"], &got["title"]);
    let got_objectives = got["objectives"].as_array().cloned().unwrap_or_default();
    for (j, w) in want["objectives"].as_array().into_iter().flatten().enumerate() {
        let o = got_objectives.get(j).unwrap_or(&Value::Null);
        let read = Value::from(vec![o["current"].clone(), o["required"].clone(), o["target"].clone()]);
        check(format!("quest {i} objective {j}"), w, &read);
    }
}

#[derive(Default, Clone)]
struct Score {
    passed: usize,
    total: usize,
}

impl Score {
    fn check(&mut self, file: &str, what: String, want: &Value, got: &Value) {
        self.total += 1;
        if want == got {
            self.passed += 1;
        } else {
            println!("  ✗ {file} {what}: want {want}, got {got}");
        }
    }

    fn percent(&self) -> f64 {
        100.0 * self.passed as f64 / self.total.max(1) as f64
    }
}
