//! Placeholder until Apple Vision (macOS) and the Tesseract fallback are added.

use crate::error::{Error, Result};
use crate::vision::{Frame, Word};

pub struct Engine;

impl Engine {
    pub fn new() -> Result<Self> {
        Err(Error::Ocr("OCR is not available on this platform yet".into()))
    }

    pub fn max_dimension() -> u32 {
        u32::MAX
    }

    pub fn recognize(&self, _image: &Frame) -> Result<Vec<Word>> {
        Ok(Vec::new())
    }
}
