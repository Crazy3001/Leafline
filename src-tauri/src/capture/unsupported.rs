//! Placeholder until macOS capture (ScreenCaptureKit) is added.

use std::time::Duration;

use super::{GameWindow, Grab};
use crate::error::{Error, Result};

pub fn windows() -> Result<Vec<GameWindow>> {
    Ok(Vec::new())
}

pub struct Capturer;

impl Capturer {
    pub fn new(_id: isize) -> Result<Self> {
        Err(Error::Invalid("Live capture is not available on this platform yet.".into()))
    }

    pub fn grab(&mut self, _wait: Duration) -> Result<Grab> {
        Ok(Grab::Nothing)
    }

    pub fn hdr(&self) -> bool {
        false
    }
}
