//! Shared types, configuration, and path helpers for Translator Overlay.

mod api_profiles;
mod config;
mod hotkey;
mod paths;
mod region_presets;
mod state;
mod toml_file;
mod types;

pub use crate::{
    api_profiles::*,
    config::*,
    hotkey::{HotkeyAction, HotkeyChord, HotkeyConfig, is_modifier_vk, is_win_vk},
    paths::*,
    region_presets::*,
    state::*,
    toml_file::TomlFileError,
    types::*,
};
