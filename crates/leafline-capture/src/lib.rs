//! The capture test page's WebAssembly (website plan, stage 2): the test pattern drawn again, a
//! delivered frame measured against it, and the readers the website could run, timed. Frames
//! come in as RGBA bytes from the page's worker and never leave it.

use leafline_core::capturetest::{measure, pattern, readers};
use leafline_core::vision::Frame;

/// The pattern for a window `width` x `height` screen pixels at `scale_milli` thousandths, as
/// RGBA bytes, [`pattern::WIDTH`] x [`pattern::HEIGHT`].
pub fn pattern_rgba(width: u32, height: u32, scale_milli: u32) -> Vec<u8> {
    pattern::draw(pattern::WindowInfo { width, height, scale_milli }).into_raw()
}

fn frame(rgba: &[u8], width: u32, height: u32) -> Option<Frame> {
    Frame::from_raw(width, height, rgba.to_vec())
}

/// The frame measured against the pattern, as JSON. `hint` is where the pattern was last found.
pub fn measure_json(rgba: &[u8], width: u32, height: u32, hint: Option<(u32, u32)>) -> String {
    match frame(rgba, width, height) {
        Some(frame) => serde_json::to_string(&measure::measure(&frame, hint)).unwrap_or_default(),
        None => r#"{"problem":"The frame's bytes don't match its size."}"#.into(),
    }
}

#[cfg(target_arch = "wasm32")]
mod web {
    use wasm_bindgen::prelude::*;

    use super::*;

    #[wasm_bindgen(start)]
    pub fn start() {
        console_error_panic_hook::set_once();
    }

    #[wasm_bindgen]
    pub fn pattern_size() -> Vec<u32> {
        vec![pattern::WIDTH, pattern::HEIGHT]
    }

    #[wasm_bindgen]
    pub fn draw_pattern(width: u32, height: u32, scale_milli: u32) -> Vec<u8> {
        pattern_rgba(width, height, scale_milli)
    }

    /// `hint_x` below 0 means no hint.
    #[wasm_bindgen]
    pub fn measure(rgba: &[u8], width: u32, height: u32, hint_x: i32, hint_y: i32) -> String {
        let hint = (hint_x >= 0 && hint_y >= 0).then_some((hint_x as u32, hint_y as u32));
        measure_json(rgba, width, height, hint)
    }

    /// The website's readers with what they remember between frames (the Pickup Log's lines).
    #[wasm_bindgen]
    pub struct Readers(readers::Readers);

    #[wasm_bindgen]
    impl Readers {
        #[wasm_bindgen(constructor)]
        pub fn new() -> Readers {
            Readers(readers::Readers::default())
        }

        /// Reads a frame taken at `at` seconds, as JSON (see `capturetest::readers::Read`).
        pub fn read(&mut self, rgba: &[u8], width: u32, height: u32, at: f64, full_every: f64) -> String {
            match frame(rgba, width, height) {
                Some(frame) => serde_json::to_string(&self.0.read(&frame, at, full_every)).unwrap_or_default(),
                None => "null".into(),
            }
        }
    }

    impl Default for Readers {
        fn default() -> Self {
            Self::new()
        }
    }
}
