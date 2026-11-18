//! `Windows.Media.Ocr`, built into Windows 10 and 11.

use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::DataWriter;

use crate::error::{Error, Result};
use crate::vision::{Frame, Rect, Word};

/// Used when the engine cannot report its own limit.
const FALLBACK_MAX_DIMENSION: u32 = 2600;

pub struct Engine {
    inner: OcrEngine,
}

impl Engine {
    /// Creates an English recogniser. The game client is English-only for now.
    pub fn new() -> Result<Self> {
        let english = OcrEngine::AvailableRecognizerLanguages()?
            .into_iter()
            .find(|lang| lang.LanguageTag().is_ok_and(|tag| tag.to_string().starts_with("en")))
            .ok_or_else(|| {
                Error::Ocr(
                    "no English OCR language is installed (Settings → Time & language → Language → add English)".into(),
                )
            })?;
        Ok(Self { inner: OcrEngine::TryCreateFromLanguage(&english)? })
    }

    pub fn max_dimension() -> u32 {
        OcrEngine::MaxImageDimension().unwrap_or(FALLBACK_MAX_DIMENSION)
    }

    pub fn recognize(&self, image: &Frame) -> Result<Vec<Word>> {
        let mut bgra = image.as_raw().clone();
        for px in bgra.as_chunks_mut::<4>().0 {
            px.swap(0, 2);
        }
        let writer = DataWriter::new()?;
        writer.WriteBytes(&bgra)?;
        let bitmap = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
            &writer.DetachBuffer()?,
            BitmapPixelFormat::Bgra8,
            image.width() as i32,
            image.height() as i32,
            BitmapAlphaMode::Ignore,
        )?;

        let result = self.inner.RecognizeAsync(&bitmap)?.join()?;
        let mut words = Vec::new();
        for line in result.Lines()? {
            for word in line.Words()? {
                let r = word.BoundingRect()?;
                words.push(Word { text: word.Text()?.to_string(), rect: Rect::new(r.X, r.Y, r.Width, r.Height) });
            }
        }
        Ok(words)
    }
}

impl From<windows::core::Error> for Error {
    fn from(err: windows::core::Error) -> Self {
        Error::Ocr(err.message())
    }
}
