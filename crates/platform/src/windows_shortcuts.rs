//! Native hotkeys use non-printing keys so AltGr characters remain available.
mod config;
pub use config::{Controller, Snapshot, start};
