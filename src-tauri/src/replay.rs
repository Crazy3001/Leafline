//! Offline replay: a folder of screenshots run through the pipeline in order and fed to the
//! session tracker, as live capture would be.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::pipeline::{self, Analysis};
use crate::readers::hud::HudFont;
use crate::session::{Sample, SessionSummary, SessionTracker, TrackerConfig};
use crate::vision::{self, Ocr};

/// How frames are put in order, and when each was taken.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase", tag = "by")]
pub enum Order {
    /// By the file's modified time, which is also taken as the moment it shows. Suits
    /// screenshots saved by hand.
    Modified,
    /// By file name, `interval_seconds` apart. Suits frames extracted from a recording, whose
    /// file times say nothing.
    Name { interval_seconds: f64 },
}

impl Order {
    /// Hand-taken screenshots are few and far between, so a single meso or counter reading has
    /// to do; a recording has plenty of frames to agree.
    pub fn tracker_config(self) -> TrackerConfig {
        let confirmations = match self {
            Order::Modified => 1,
            Order::Name { .. } => 2,
        };
        TrackerConfig { confirmations, ..TrackerConfig::default() }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayFrame {
    pub path: PathBuf,
    /// Seconds since the Unix epoch for [`Order::Modified`], since the first frame otherwise.
    pub at: f64,
}

/// The supported images in `folder`, in replay order.
pub fn frames(folder: &Path, order: Order) -> Result<Vec<ReplayFrame>> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(folder)?
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| vision::is_supported(path))
        .collect();
    paths.sort();
    let mut frames = match order {
        Order::Modified => paths
            .into_iter()
            .map(|path| {
                let modified = std::fs::metadata(&path)?.modified()?;
                let at = modified.duration_since(UNIX_EPOCH).map_err(|e| Error::Invalid(e.to_string()))?.as_secs_f64();
                Ok(ReplayFrame { path, at })
            })
            .collect::<Result<Vec<_>>>()?,
        Order::Name { interval_seconds } => {
            if !interval_seconds.is_finite() || interval_seconds <= 0.0 {
                return Err(Error::Invalid("The frame interval must be more than 0 seconds.".into()));
            }
            paths
                .into_iter()
                .enumerate()
                .map(|(i, path)| ReplayFrame { path, at: i as f64 * interval_seconds })
                .collect()
        }
    };
    frames.sort_by(|a, b| a.at.total_cmp(&b.at));
    Ok(frames)
}

/// Analyses each frame and feeds the tracker. `on_frame` sees every step and returns `false`
/// to stop early; the summary so far is returned either way.
pub fn run(
    frames: &[ReplayFrame],
    config: TrackerConfig,
    ocr: &Ocr,
    font: &HudFont,
    mut on_frame: impl FnMut(usize, &ReplayFrame, &Analysis, &SessionSummary) -> bool,
) -> Result<SessionSummary> {
    let mut tracker = SessionTracker::new(config);
    for (i, frame) in frames.iter().enumerate() {
        let analysis = pipeline::analyze(&vision::load(&frame.path)?, ocr, font)?;
        tracker.push(&Sample::from_analysis(frame.at, &analysis));
        if !on_frame(i, frame, &analysis, &tracker.summary()) {
            break;
        }
    }
    Ok(tracker.summary())
}
