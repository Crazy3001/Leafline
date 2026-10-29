//! Text recognition. Each platform provides an `Engine` with `new`, `max_dimension` and
//! `recognize`; this wrapper adds the shared behaviour on top.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use self::windows as platform;

#[cfg(not(windows))]
mod unsupported;
#[cfg(not(windows))]
use self::unsupported as platform;

use super::{Frame, Rect, Word, preprocess};
use crate::error::Result;

pub struct Ocr {
    engine: platform::Engine,
}

impl Ocr {
    pub fn new() -> Result<Self> {
        Ok(Self { engine: platform::Engine::new()? })
    }

    /// The largest width or height the engine accepts; larger images are shrunk to fit.
    pub fn max_dimension() -> u32 {
        platform::Engine::max_dimension()
    }

    /// Recognises words in `image`, with rectangles in `image` pixels.
    pub fn words(&self, image: &Frame) -> Result<Vec<Word>> {
        let limit = Self::max_dimension() as f32;
        let longest = image.width().max(image.height()) as f32;
        if longest <= limit {
            return self.engine.recognize(image);
        }
        let factor = limit / longest;
        let words = self.engine.recognize(&preprocess::upscale(image, factor))?;
        Ok(words.into_iter().map(|word| Word { rect: word.rect.scaled(1.0 / factor), ..word }).collect())
    }

    /// Recognises words inside `region` of `frame` after `prepare` (upscaling, keying,
    /// padding, ...) has been applied. `scale` is the factor `prepare` enlarged the crop by;
    /// any margin it added is detected from the size. Rectangles are returned in `frame` pixels.
    pub fn words_in(
        &self,
        frame: &Frame,
        region: Rect,
        scale: f32,
        prepare: impl Fn(&Frame) -> Frame,
    ) -> Result<Vec<Word>> {
        let (x, y, _, _) = region.to_pixels(frame.width(), frame.height());
        let crop = preprocess::crop(frame, region);
        let prepared = prepare(&crop);
        let margin_x = (prepared.width() as f32 - crop.width() as f32 * scale) / 2.0;
        let margin_y = (prepared.height() as f32 - crop.height() as f32 * scale) / 2.0;
        let words = self.words(&prepared)?;
        Ok(words
            .into_iter()
            .map(|word| {
                let rect =
                    word.rect.translated(-margin_x, -margin_y).scaled(1.0 / scale).translated(x as f32, y as f32);
                Word { rect, ..word }
            })
            .collect())
    }
}
