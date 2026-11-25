//! The website's game readers as WebAssembly: live reading of a shared game window (`live`), and
//! the capture test pages' pattern, measuring and timed readers. Frames come in as RGBA bytes from
//! the page's worker and never leave it.

use leafline_core::capturetest::{measure, pattern};
use leafline_core::vision::Frame;

pub mod engine;
pub mod identity;
pub mod items;
pub mod live;
pub mod ocr;
pub mod title;
pub mod words;

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
    use leafline_core::capturetest::readers;

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

    /// The website's live reading: readers and the session tracker (see `live::LiveReader`).
    #[wasm_bindgen]
    pub struct Live(super::live::LiveReader);

    #[wasm_bindgen]
    impl Live {
        /// A new session, or the one in `saved` (a snapshot's JSON) when it can be carried on.
        #[wasm_bindgen(constructor)]
        pub fn new(idle_after: f64, saved: Option<String>) -> Live {
            let restored = saved
                .and_then(|text| serde_json::from_str(&text).ok())
                .and_then(|value| super::live::LiveReader::restore(idle_after, value));
            Live(restored.unwrap_or_else(|| super::live::LiveReader::new(idle_after)))
        }

        /// Reads a frame taken at `at` Unix seconds, as JSON: `{read, other}`, or `null` when the
        /// bytes don't make a frame.
        pub fn read(&mut self, rgba: &[u8], width: u32, height: u32, at: f64) -> String {
            let Some(frame) = frame(rgba, width, height) else { return "null".into() };
            match self.0.read(&frame, at) {
                Some((read, other)) => serde_json::json!({ "read": read, "other": other }).to_string(),
                None => "null".into(),
            }
        }

        /// Looks at the Pickup Log alone in a frame taken at `at` Unix seconds: the session's
        /// JSON when a line arrived, else an empty string.
        pub fn look(&mut self, rgba: &[u8], width: u32, height: u32, at: f64) -> String {
            let Some(frame) = frame(rgba, width, height) else { return String::new() };
            if self.0.look(&frame, at) { serde_json::to_string(&self.0.summary()).unwrap_or_default() } else { String::new() }
        }

        pub fn summary(&self) -> String {
            serde_json::to_string(&self.0.summary()).unwrap_or_default()
        }

        /// The player's switch: count the Pickup Log.
        pub fn set_pickup_log(&mut self, on: bool) {
            self.0.set_pickup_log(on);
        }

        /// The player's switch: read a tooltip the mouse rests on.
        pub fn set_item_tooltips(&mut self, on: bool) {
            self.0.set_item_tooltips(on);
        }

        /// The player's switches for the windows read when open: the Character Stat and Skill
        /// windows, the inventory tab (sell or keep), the Quest window.
        pub fn set_windows(&mut self, character: bool, bag: bool, quest_log: bool) {
            self.0.set_windows(character, bag, quest_log);
        }

        /// What the game's windows gave since this was last asked, as JSON (see
        /// `live::WindowReads`); else an empty string.
        pub fn take_windows(&mut self) -> String {
            self.0.take_windows().map(|reads| serde_json::to_string(&reads).unwrap_or_default()).unwrap_or_default()
        }

        /// The item tooltips last read, as JSON, if they changed since this was last asked; else
        /// an empty string.
        pub fn take_items(&mut self) -> String {
            self.0.take_items().map(|items| serde_json::to_string(&items).unwrap_or_default()).unwrap_or_default()
        }

        /// ocrs's text recognition model, for the minimap title and the HUD's name and job.
        pub fn set_text_model(&mut self, recognition: Vec<u8>) -> Result<(), String> {
            self.0.set_text_model(recognition)
        }

        /// A reference data file by its name (`maps.json`, `quests.json`, `items.json`,
        /// `monsters.json`).
        pub fn set_data(&mut self, file: &str, json: &[u8]) -> Result<(), String> {
            self.0.set_data(file, json)
        }

        pub fn snapshot(&self) -> String {
            self.0.snapshot().to_string()
        }

        pub fn set_held(&mut self, held: bool) {
            self.0.set_held(held);
        }

        pub fn set_idle_after(&mut self, seconds: f64) {
            self.0.set_idle_after(seconds);
        }

        pub fn reset(&mut self) {
            self.0.reset();
        }
    }
}
