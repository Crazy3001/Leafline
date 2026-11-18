//! The time, in a way that works in the program and in the browser: std's clock panics on
//! wasm32, so everything asks here (or uses `web_time` directly, which is std on the program).

pub use web_time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Unix seconds now.
pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Unix seconds now, with the fraction.
pub fn now_f64() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}
