//! Downloads from the website's worker. A dedicated worker may make synchronous requests, so the
//! shared download code (`Source::get`) runs unchanged: the worker waits, the page doesn't.

use std::sync::Mutex;

use leafline_core::clock::{Duration, Instant};
use leafline_core::error::{Error, Result};
use leafline_core::mapdata::fetch::{MAX_BYTES, Source, USER_AGENT};
use wasm_bindgen::JsCast;
use web_sys::{XmlHttpRequest, XmlHttpRequestResponseType};

/// At most one request per `min_interval`, like the program's `Http`.
pub struct Xhr {
    min_interval: Duration,
    last: Mutex<Option<Instant>>,
}

impl Xhr {
    pub fn new(min_interval: Duration) -> Self {
        Self { min_interval, last: Mutex::new(None) }
    }
}

const WIKI_API: &str = "https://mapleclassic.wiki/api.php";

/// The wiki's API answers another site's page only when asked with `origin=*` (no cookies).
fn for_browser(url: &str) -> String {
    if url.starts_with(WIKI_API) && !url.contains("origin=") {
        let join = if url.contains('?') { '&' } else { '?' };
        return format!("{url}{join}origin=*");
    }
    url.to_string()
}

fn js_error(url: &str, err: wasm_bindgen::JsValue) -> Error {
    let message = err.as_string().or_else(|| err.dyn_ref::<js_sys::Error>().map(|e| String::from(e.message())));
    Error::Download(format!("{url}: {}", message.unwrap_or_else(|| "the browser refused the request".into())))
}

impl Source for Xhr {
    fn get(&self, url: &str) -> Result<Vec<u8>> {
        {
            let mut last = self.last.lock().expect("download pace lock");
            if let Some(at) = *last {
                // A worker can't sleep; the wait is short and only between downloads.
                while at.elapsed() < self.min_interval {}
            }
            *last = Some(Instant::now());
        }
        let url = for_browser(url);
        let request = XmlHttpRequest::new().map_err(|err| js_error(&url, err))?;
        request.open_with_async("GET", &url, false).map_err(|err| js_error(&url, err))?;
        request.set_response_type(XmlHttpRequestResponseType::Arraybuffer);
        // Browsers don't let pages set User-Agent; the wiki reads this header instead.
        if url.starts_with(WIKI_API) {
            let _ = request.set_request_header("Api-User-Agent", USER_AGENT);
        }
        request.send().map_err(|err| js_error(&url, err))?;
        let status = request.status().map_err(|err| js_error(&url, err))?;
        if !(200..300).contains(&status) {
            return Err(Error::Download(format!("{url}: status {status}")));
        }
        let body = request.response().map_err(|err| js_error(&url, err))?;
        let bytes = js_sys::Uint8Array::new(&body).to_vec();
        if bytes.len() as u64 > MAX_BYTES {
            return Err(Error::Download(format!("{url}: larger than Leafline accepts")));
        }
        Ok(bytes)
    }
}
