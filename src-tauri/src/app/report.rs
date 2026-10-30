//! What the player can copy into a bug report: how Leafline is doing, with what went wrong
//! lately. Nothing leaves this PC unless the player pastes it somewhere, and it holds no
//! character name, chat or file paths.

use std::collections::VecDeque;
use std::fmt::Write;
use std::sync::Mutex;

use tauri::{AppHandle, Manager};

use super::live::{self, LiveStatus};
use super::readings;
use super::settings::SettingsState;

/// How many recent problems are kept.
const KEPT: usize = 40;

static RECENT: Mutex<VecDeque<String>> = Mutex::new(VecDeque::new());

/// Remembers a problem for the next report, once if it repeats.
pub fn note(problem: impl Into<String>) {
    let problem = problem.into();
    eprintln!("leafline: {problem}");
    let mut recent = RECENT.lock().expect("report lock");
    if recent.back() != Some(&problem) {
        recent.push_back(problem);
    }
    while recent.len() > KEPT {
        recent.pop_front();
    }
}

#[tauri::command]
pub fn diagnostics(app: AppHandle) -> String {
    let settings = app.state::<SettingsState>().get();
    let mut text = String::new();
    let _ = writeln!(text, "Leafline {} on {}", app.package_info().version, std::env::consts::OS);
    match live::status(&app) {
        LiveStatus::Off => text.push_str("Live reading: off\n"),
        LiveStatus::Waiting { reason } => {
            let _ = writeln!(text, "Live reading: waiting ({reason})");
        }
        LiveStatus::Failed { message } => {
            let _ = writeln!(text, "Live reading: stopped ({message})");
        }
        LiveStatus::Reading { width, height, frames, read_ms, hdr, hud_taught, held, logging, .. } => {
            let _ = writeln!(
                text,
                "Live reading: {width}x{height}, {frames} readings, last {read_ms:.0} ms, HDR {hdr}, HUD taught {hud_taught}, paused {held}, logging {logging}",
            );
        }
    }
    let _ = writeln!(text, "Reading every {} s, pause after {} s", settings.live.interval, settings.live.pause_after);
    if let Some(analysis) = readings::current_analysis(&app) {
        let hud = analysis.hud.as_ref();
        let _ = writeln!(text, "Last picture: {}x{}", analysis.width, analysis.height);
        let _ = writeln!(
            text,
            "Read: HUD {}, level {}, HP {}, MP {}, EXP {}, mesos {}, map {}, Quest Helper {}, meso boxes {}, shop window {}, party {}",
            hud.is_some(),
            hud.is_some_and(|h| h.level.is_some()),
            hud.is_some_and(|h| h.hp.is_some()),
            hud.is_some_and(|h| h.mp.is_some()),
            hud.is_some_and(|h| h.exp.is_some() || h.exp_percent.is_some()),
            analysis.mesos.is_some(),
            analysis.map.is_some(),
            analysis.quest.is_some(),
            analysis.meso_boxes,
            analysis.shop_open,
            analysis.party_size,
        );
        let timings: Vec<String> = analysis.timings.iter().map(|t| format!("{} {:.0} ms", t.stage, t.ms)).collect();
        let _ = writeln!(text, "Timings: {}", timings.join(", "));
        if !analysis.failed.is_empty() {
            let _ = writeln!(text, "Failed: {}", analysis.failed.join(", "));
        }
    }
    if let Some(session) = readings::current_session(&app) {
        let _ = writeln!(
            text,
            "Session: {} frames, {:.0} s active of {:.0} s, dropped {} EXP and {} meso readings",
            session.frames,
            session.active_seconds,
            session.elapsed_seconds,
            session.rejected.exp,
            session.rejected.mesos,
        );
    }
    let recent = RECENT.lock().expect("report lock");
    if !recent.is_empty() {
        text.push_str("Recent problems:\n");
        for problem in recent.iter() {
            let _ = writeln!(text, "- {problem}");
        }
    }
    text
}
