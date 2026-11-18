//! Where the small JSON files (settings, Go here, the character's read damage, travel) live.
//! The program keeps them in its data folder; the website keeps them in its own browser storage.

use std::path::{Path, PathBuf};

use crate::error::{Error, Result};

/// Named files kept between runs.
pub trait Files: Send + Sync {
    /// The file's bytes, or `None` when it doesn't exist or can't be read.
    fn read(&self, name: &str) -> Option<Vec<u8>>;
    fn write(&self, name: &str, bytes: &[u8]) -> Result<()>;
    /// Where the file is on disk, for the program's "open in editor" and Settings. `None` on the web.
    fn path(&self, name: &str) -> Option<PathBuf>;
}

/// Reads a JSON file, or `None` when missing or unreadable.
pub fn read_json<T: serde::de::DeserializeOwned>(files: &dyn Files, name: &str) -> Option<T> {
    files.read(name).and_then(|bytes| serde_json::from_slice(&bytes).ok())
}

/// Writes a value as pretty JSON.
pub fn write_json<T: serde::Serialize>(files: &dyn Files, name: &str, value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value).map_err(|err| Error::Invalid(err.to_string()))?;
    files.write(name, json.as_bytes())
}

/// A folder on disk: the program's app data folder.
pub struct Folder(pub PathBuf);

impl Folder {
    pub fn dir(&self) -> &Path {
        &self.0
    }
}

impl Files for Folder {
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        std::fs::read(self.0.join(name)).ok()
    }

    fn write(&self, name: &str, bytes: &[u8]) -> Result<()> {
        std::fs::create_dir_all(&self.0)?;
        std::fs::write(self.0.join(name), bytes)?;
        Ok(())
    }

    fn path(&self, name: &str) -> Option<PathBuf> {
        Some(self.0.join(name))
    }
}

/// Files held in memory: tests, and the website before its storage opens.
#[derive(Default)]
pub struct Memory(std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>);

impl Files for Memory {
    fn read(&self, name: &str) -> Option<Vec<u8>> {
        self.0.lock().expect("memory files lock").get(name).cloned()
    }

    fn write(&self, name: &str, bytes: &[u8]) -> Result<()> {
        self.0.lock().expect("memory files lock").insert(name.to_string(), bytes.to_vec());
        Ok(())
    }

    fn path(&self, _name: &str) -> Option<PathBuf> {
        None
    }
}
