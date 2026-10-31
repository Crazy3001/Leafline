//! Lists the windows live capture could read, captures one to a PNG, or reads the game window
//! the way live reading does.
//!
//! `cargo run --example capture` lists them.
//! `cargo run --example capture -- "<title text>" out.png` captures the first window whose title
//! contains the text.
//! `cargo run --example capture -- --auto [--frames 3] [--compare image]`
//! finds the game window on its own and runs each picture through the pipeline. `--compare`
//! counts how many pixels differ from a saved screenshot.
//! `cargo run --example capture -- --item` reads the items in one picture, like a price check.

use std::path::Path;
use std::time::{Duration, Instant};

use leafline_lib::capture::{self, Capturer, Grab};
use leafline_lib::pipeline;
use leafline_lib::session::{Sample, SessionTracker, TrackerConfig};
use leafline_lib::vision::{self, Frame, Ocr};

fn main() {
    #[cfg(windows)]
    // SAFETY: called once, before any window is touched. The app gets this from its manifest.
    unsafe {
        use windows::Win32::UI::HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext};
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().is_some_and(|a| a == "--auto") {
        return read_live(&args);
    }
    if args.first().is_some_and(|a| a == "--item") {
        return read_item();
    }

    let windows = capture::windows().expect("list windows");
    let Some(needle) = args.first() else {
        for w in &windows {
            let mark = if w.likely_game { "*" } else { " " };
            println!("{mark} {:>4}x{:<4} {:<24} {}", w.width, w.height, w.class, w.title);
        }
        return;
    };
    let out = args.get(1).map(String::as_str).unwrap_or("capture.png");
    let window = windows.iter().find(|w| w.title.contains(needle.as_str())).expect("no window with that title");

    let started = Instant::now();
    let mut capturer = Capturer::new(window).expect("start capture");
    println!("started in {:?}; HDR: {}", started.elapsed(), capturer.hdr());
    for _ in 0..5 {
        match capturer.grab(Duration::from_millis(500)).expect("grab") {
            Grab::Frame(frame) => {
                println!("frame {}x{}", frame.width(), frame.height());
                frame.save(out).expect("save");
                return;
            }
            Grab::Unchanged => println!("unchanged"),
            Grab::Minimized => println!("minimised"),
            Grab::Nothing => println!("nothing yet"),
            Grab::Closed => return println!("closed"),
        }
    }
}

fn read_item() {
    let window = capture::find(None).expect("list windows").expect("no window looks like the game");
    let started = Instant::now();
    let frame = capture::picture(&window).expect("picture");
    let items = pipeline::read_items(&frame, &Ocr::new().expect("OCR engine")).expect("read items");
    let names: Vec<&String> = items.iter().map(|i| &i.name).collect();
    println!("{}x{} in {:?}: {names:?}", frame.width(), frame.height(), started.elapsed());
}

fn read_live(args: &[String]) {
    let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1));
    let frames: usize = value("--frames").map_or(3, |n| n.parse().expect("frame count"));
    let reference = value("--compare").map(|path| vision::load(Path::new(path)).expect("load comparison image"));

    let window = capture::find(None).expect("list windows").expect("no window looks like the game");
    println!("found {:?} ({}), {}x{}", window.title, window.class, window.width, window.height);
    let mut capturer = Capturer::new(&window).expect("start capture");
    println!("HDR: {}", capturer.hdr());

    let ocr = Ocr::new().expect("OCR engine");
    let mut tracker = SessionTracker::new(TrackerConfig { confirmations: 2, ..TrackerConfig::default() });
    let started = Instant::now();
    for i in 0..frames {
        let frame = match capturer.grab(Duration::from_millis(300)).expect("grab") {
            Grab::Frame(frame) => frame,
            Grab::Unchanged => {
                println!("frame {i}: unchanged");
                continue;
            }
            _ => {
                println!("frame {i}: no picture");
                continue;
            }
        };
        let reading = Instant::now();
        let analysis = pipeline::analyze_live(&frame, &ocr).expect("analyze");
        let read_ms = reading.elapsed().as_millis();
        tracker.push(&Sample::from_analysis(started.elapsed().as_secs_f64(), &analysis));
        let hud = analysis.hud.as_ref();
        println!(
            "frame {i}: {}x{} in {read_ms} ms; level {:?}, exp {:?}, mesos {:?}, map {:?}, quests {}",
            frame.width(),
            frame.height(),
            hud.and_then(|h| h.level),
            hud.and_then(|h| h.exp),
            analysis.mesos.map(|m| m.mesos),
            analysis.map.as_ref().map(|m| m.label()),
            analysis.quest.as_ref().map_or(0, |q| q.quests.len()),
        );
        if let Some(reference) = &reference {
            println!("  {} pixels differ from the saved image", differing(&frame, reference));
        }
    }
    println!("session: {} frames", tracker.summary().frames);
}

fn differing(a: &Frame, b: &Frame) -> String {
    if a.dimensions() != b.dimensions() {
        return format!("all (sizes differ: {:?} and {:?})", a.dimensions(), b.dimensions());
    }
    let count = a.pixels().zip(b.pixels()).filter(|(p, q)| p.0[..3] != q.0[..3]).count();
    count.to_string()
}
