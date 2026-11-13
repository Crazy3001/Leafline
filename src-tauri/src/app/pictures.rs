//! The `pic` URI scheme: windows ask for a game picture by what it shows (`pic://localhost/item/123`,
//! or `http://pic.localhost/...` on Windows), and get it from the picture cache, downloaded from
//! OSMS the first time. Anything missing, refused or turned off answers 404, and the window shows
//! the name alone.

use tauri::http::{Request, Response, StatusCode, header};
use tauri::{AppHandle, Manager, UriSchemeContext, UriSchemeResponder, Wry};

use super::mapdata::MapDataState;
use super::settings::SettingsState;
use crate::error::Result;
use crate::mapdata::pictures::{PictureCache, PictureHttp};

/// The scheme the windows use. Listed in the content security policy in tauri.conf.json.
pub const SCHEME: &str = "pic";

/// The folder pictures are kept in, under the app's local data: never a synced folder.
const FOLDER: &str = "pictures";

pub struct PictureState {
    cache: PictureCache<PictureHttp>,
}

impl PictureState {
    pub fn load(app: &AppHandle) -> Result<Self> {
        let dir = app.path().app_local_data_dir()?.join(FOLDER);
        Ok(Self { cache: PictureCache::new(dir, PictureHttp::default()) })
    }
}

/// Answers a window's request off the main thread: a download can take a while.
pub fn handle(context: UriSchemeContext<'_, Wry>, request: Request<Vec<u8>>, responder: UriSchemeResponder) {
    let app = context.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || responder.respond(respond(&app, request.uri().path())));
}

fn respond(app: &AppHandle, path: &str) -> Response<Vec<u8>> {
    let missing = || Response::builder().status(StatusCode::NOT_FOUND).body(Vec::new()).expect("empty response");
    // A window can ask before setup has finished; it gets the names alone until then.
    let (Some(settings), Some(data), Some(pictures)) =
        (app.try_state::<SettingsState>(), app.try_state::<MapDataState>(), app.try_state::<PictureState>())
    else {
        return missing();
    };
    if !settings.get().show_pictures {
        return missing();
    }
    let index = data.pictures();
    let Some(url) = index.resolve(path) else { return missing() };
    match pictures.cache.get(url) {
        Ok(picture) => Response::builder()
            .header(header::CONTENT_TYPE, picture.mime)
            .header(header::CACHE_CONTROL, "max-age=3600")
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
            .body(picture.bytes)
            .unwrap_or_else(|_| missing()),
        Err(err) => {
            eprintln!("picture {path}: {err}");
            missing()
        }
    }
}
