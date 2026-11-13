//! The `pic` URI scheme: windows ask for a game picture by what it shows (`pic://localhost/item/123`,
//! or `http://pic.localhost/...` on Windows), and get it from the picture cache, downloaded from
//! OSMS the first time. Anything missing, refused or turned off answers 404, and the window shows
//! the name alone.

use serde::Serialize;
use tauri::http::{Request, Response, StatusCode, header};
use tauri::{AppHandle, Manager, UriSchemeContext, UriSchemeResponder, Wry};

use super::mapdata::MapDataState;
use super::settings::SettingsState;
use crate::error::{Error, Result};
use crate::mapdata::atlas::ViewPoint;
use crate::mapdata::pictures::{MapView, PictureCache, PictureHttp};

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

/// A map's picture as the windows show it, cut to what is drawn, with its markers.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapPicture {
    pub width: u32,
    pub height: u32,
    pub points: Vec<ViewPoint>,
}

/// Cuts a map's picture down (downloading it the first time) and says where its points fall.
/// The picture itself comes through the scheme at `/mapview/<id>`.
fn map_view(app: &AppHandle, id: u32) -> Result<Option<(MapView, String)>> {
    let (Some(settings), Some(data), Some(pictures)) =
        (app.try_state::<SettingsState>(), app.try_state::<MapDataState>(), app.try_state::<PictureState>())
    else {
        return Ok(None);
    };
    if !settings.get().show_pictures {
        return Ok(None);
    }
    let Some(url) = data.pictures().resolve(&format!("/map/{id}")).map(str::to_string) else { return Ok(None) };
    let points: Vec<(i32, i32)> = data.map_points(id)?.iter().map(|p| (p.x, p.y)).collect();
    Ok(Some((pictures.cache.map_view(&url, &points)?, url)))
}

#[tauri::command]
pub async fn map_picture(app: AppHandle, id: u32) -> Result<Option<MapPicture>> {
    tauri::async_runtime::spawn_blocking(move || {
        let Some((view, _)) = map_view(&app, id)? else { return Ok(None) };
        let data = app.state::<MapDataState>();
        let points = match data.atlas() {
            Some(atlas) => atlas.view_points(id, &data.map_points(id)?, &view),
            None => Vec::new(),
        };
        Ok(Some(MapPicture { width: view.out_width, height: view.out_height, points }))
    })
    .await
    .map_err(|err| Error::Invalid(err.to_string()))?
}

/// Answers a window's request off the main thread: a download can take a while.
pub fn handle(context: UriSchemeContext<'_, Wry>, request: Request<Vec<u8>>, responder: UriSchemeResponder) {
    let app = context.app_handle().clone();
    tauri::async_runtime::spawn_blocking(move || responder.respond(respond(&app, request.uri().path())));
}

fn respond(app: &AppHandle, path: &str) -> Response<Vec<u8>> {
    let missing = || Response::builder().status(StatusCode::NOT_FOUND).body(Vec::new()).expect("empty response");
    // A map cut down for the windows, made on first ask.
    if let Some(id) = path.strip_prefix("/mapview/").and_then(|id| id.parse::<u32>().ok()) {
        let picture = match map_view(app, id) {
            Ok(Some((_, url))) => app.state::<PictureState>().cache.map_view_picture(&url),
            Ok(None) => None,
            Err(err) => {
                eprintln!("map picture {id}: {err}");
                None
            }
        };
        return match picture {
            Some(picture) => Response::builder()
                .header(header::CONTENT_TYPE, picture.mime)
                .header(header::CACHE_CONTROL, "max-age=3600")
                .body(picture.bytes)
                .unwrap_or_else(|_| missing()),
            None => missing(),
        };
    }
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
