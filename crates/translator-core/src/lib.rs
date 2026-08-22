//! Shared types, configuration, and path helpers for Translator Overlay.

mod config;
mod paths;
mod region_presets;
mod state;
mod types;

pub use crate::{config::*, paths::*, region_presets::*, state::*, types::*};
