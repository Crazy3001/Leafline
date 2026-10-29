use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not read image: {0}")]
    Image(#[from] image::ImageError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("OCR failed: {0}")]
    Ocr(String),
    #[error("{0}")]
    Invalid(String),
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
    #[error("map data cache: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("download failed: {0}")]
    Download(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Commands return errors to the frontend as plain messages.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
