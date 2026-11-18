use std::path::Path;

use image::RgbaImage;

use crate::error::Result;

/// A single captured or loaded screen image.
pub type Frame = RgbaImage;

/// File extensions the offline test mode can open.
pub const SUPPORTED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "webp"];

pub fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| SUPPORTED_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

pub fn load(path: &Path) -> Result<Frame> {
    Ok(image::open(path)?.to_rgba8())
}
