//! Runs the website's live reader (`live::LiveReader`) natively over saved frames, to check the
//! website's reading of a shared window against the same readers and tracker on the original
//! pictures. Input: a CSV of `seconds,path` lines (when each frame was read, and the picture);
//! output: one JSON line per read (`level`, `points`, `percent`, `mesos`), then the session summary.
//! Frames stay on this PC; nothing is written but standard output.
//!
//!     cargo run --release -p leafline-capture --example livetrack -- reads.csv [idle_after]

use leafline_capture::live::LiveReader;
use leafline_core::vision::Frame;

fn main() {
    let mut args = std::env::args().skip(1);
    let list = args.next().expect("usage: livetrack <reads.csv> [idle_after]");
    let idle_after: f64 = args.next().map(|a| a.parse().expect("idle_after")).unwrap_or(15.0);
    let text = std::fs::read_to_string(&list).expect("read the list");
    let mut live = LiveReader::new(idle_after);
    // The minimap title, as on the website: LIVETRACK_MODEL (text-recognition.onnx) and
    // LIVETRACK_MAPS (maps.json) when set.
    if let (Ok(model), Ok(maps)) = (std::env::var("LIVETRACK_MODEL"), std::env::var("LIVETRACK_MAPS")) {
        live.set_title_model(std::fs::read(model).expect("model")).expect("load the model");
        live.set_maps(&std::fs::read(maps).expect("maps")).expect("load the maps");
    }
    let mut cache: Option<(String, Frame)> = None;
    for line in text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with("seconds")) {
        let (at, path) = line.split_once(',').expect("seconds,path");
        let at: f64 = at.trim().parse().expect("seconds");
        let path = path.trim().to_string();
        if cache.as_ref().is_none_or(|(p, _)| *p != path) {
            let image = image::open(&path).expect("open the picture").to_rgba8();
            cache = Some((path.clone(), image));
        }
        let frame = &cache.as_ref().expect("cached").1;
        let Some((read, other)) = live.read(frame, at) else {
            println!("{}", serde_json::json!({ "at": at, "path": path, "failed": true }));
            continue;
        };
        let hud = read.analysis.hud.as_ref();
        println!(
            "{}",
            serde_json::json!({
                "at": at,
                "path": path,
                "level": hud.and_then(|h| h.level),
                "points": hud.and_then(|h| h.exp.map(|e| e.points)),
                "percent": hud.and_then(|h| h.exp.map(|e| e.percent).or(h.exp_percent)),
                "mesos": read.analysis.mesos.map(|m| m.mesos),
                "map": read.analysis.map.as_ref().map(|m| m.label()),
                "other": other,
                "ms": read.ms,
                "timings": read.analysis.timings,
            })
        );
    }
    println!("{}", serde_json::json!({ "summary": live.summary() }));
}
