//! Shared types, configuration, and path helpers for Translator Overlay.

// Feature-only dependency (see build.rs); the import keeps cargo's unused-dependency lint quiet.
use ort_sys as _;

mod api_profiles;
mod config;
mod dictionaries;
mod download;
mod hotkey;
mod http;
mod paths;
mod region_presets;
mod state;
mod toml_file;
mod types;

pub use crate::{
    api_profiles::*,
    config::*,
    dictionaries::*,
    download::*,
    hotkey::{HotkeyAction, HotkeyChord, HotkeyConfig, is_modifier_vk, is_win_vk},
    http::http_client,
    paths::*,
    region_presets::*,
    state::*,
    toml_file::TomlFileError,
    types::*,
};
