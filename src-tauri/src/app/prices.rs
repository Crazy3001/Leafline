//! Price log commands. The records live in the user's own database, never sent anywhere.

use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager, State};

use crate::error::{Error, Result};
use crate::userdata::UserData;
use crate::userdata::prices::{NewPrice, PriceLog};

const FILE_NAME: &str = "leafline.sqlite3";

/// The user's records, or why they couldn't be opened. A damaged file must not stop the app;
/// the commands report it instead.
pub struct UserDataState(Mutex<std::result::Result<UserData, String>>);

impl UserDataState {
    pub fn load(app: &AppHandle) -> Result<Self> {
        let path = app.path().app_data_dir()?.join(FILE_NAME);
        let opened = UserData::open(&path).map_err(|err| {
            eprintln!("user data unreadable: {err}");
            format!("Your saved records ({}) could not be opened: {err}", path.display())
        });
        Ok(Self(Mutex::new(opened)))
    }

    pub fn with<T>(&self, f: impl FnOnce(&mut UserData) -> Result<T>) -> Result<T> {
        let mut guard: MutexGuard<_> = self.0.lock().expect("user data lock");
        match guard.as_mut() {
            Ok(db) => f(db),
            Err(message) => Err(Error::Invalid(message.clone())),
        }
    }
}

/// What has been logged for an item, summarised for a copy with these upgrades and slots.
#[tauri::command]
pub fn price_log(
    state: State<UserDataState>,
    item: String,
    upgrades: Option<u32>,
    slots: Option<u32>,
) -> Result<PriceLog> {
    state.with(|db| db.price_log(&item, upgrades, slots))
}

/// Records a price and returns the item's updated log.
#[tauri::command]
pub fn record_price(state: State<UserDataState>, price: NewPrice) -> Result<PriceLog> {
    state.with(|db| {
        db.record_price(&price, now())?;
        db.price_log(&price.item, price.upgrades, price.slots)
    })
}

#[tauri::command]
pub fn delete_price(state: State<UserDataState>, id: i64) -> Result<()> {
    state.with(|db| db.delete_price(id))
}

pub fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}
