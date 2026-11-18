//! The bodies of the commands the program's windows and the website both call, as plain
//! functions over Leafline's state ([`host::Host`]). The program wraps each in a Tauri command;
//! the website's worker calls them by the same command names.

pub mod builds;
pub mod damage;
pub mod fighter;
pub mod files;
pub mod host;
pub mod mapdata;
pub mod nav;
pub mod quests;
pub mod records;
pub mod settings;
pub mod sheet;
