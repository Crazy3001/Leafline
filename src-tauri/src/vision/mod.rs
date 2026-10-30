//! Generic image handling and text recognition, independent of the game's UI.

pub mod blobs;
pub mod color;
mod frame;
mod geometry;
pub mod glyphs;
pub mod ocr;
pub mod preprocess;
mod text;

pub use frame::{Frame, SUPPORTED_EXTENSIONS, is_supported, load};
pub use geometry::Rect;
pub use ocr::Ocr;
pub use text::{Line, Word, group_lines};
