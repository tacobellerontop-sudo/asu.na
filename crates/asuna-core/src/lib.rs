//! Everything AsuNa knows about launchers, without any GUI code, so it can be
//! unit tested on any OS.

pub mod actions;
pub mod autostart;
pub mod context;
pub mod executables;
pub mod fsx;
pub mod library;
pub mod net;
pub mod providers;
pub mod registry;
pub mod store;
pub mod types;
pub mod vdf;

pub use types::*;
