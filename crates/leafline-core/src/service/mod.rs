//! The bodies of the commands the program's windows and the website both call, as plain
//! functions over Leafline's state. The program wraps them in Tauri commands; the website's
//! worker calls them by the same command names.

pub mod files;
