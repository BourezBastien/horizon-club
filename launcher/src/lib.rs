//! The launcher's core, shared by the GUI.
pub mod backend;
pub mod club;
pub mod config;
pub mod desktop_seams;
pub mod downloads;
pub mod effects;
pub mod first_play;
/// The game's app bundle on macOS: the game shows in the Dock under its own name and icon.
#[cfg(unix)]
pub mod game_app;
pub mod game_log;
pub mod heads;
pub mod i18n;
pub mod identity;
pub mod mojang;
pub mod online;
pub mod pictures;
pub mod process;
pub mod proxy;
pub mod server_list;
pub mod server_status;
pub mod servers;
pub mod sources;
pub mod themes;
pub mod update;
#[cfg(windows)]
mod windows_runtime;
pub mod worlds;
