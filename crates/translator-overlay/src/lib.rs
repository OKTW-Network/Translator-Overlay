//! Transparent click-through overlay and an independent translation window.
//!
//! Creates a layered Win32 popup (`WS_EX_LAYERED | WS_EX_TRANSPARENT | …`) that
//! tracks a target window and draws semi-transparent boxes + translations at
//! OCR bounding boxes (mapped from capture-image coordinates). A second,
//! clickable always-on-top reader window shows the same text independently.
//! A third always-on-top HUD has start / pause / stop and pipeline status.

mod command;
mod controller;
mod error;
mod gfx;
mod host;
mod hud;
mod picker;
mod reader;

pub use crate::{
    command::{HudPrimary, HudSnapshot, OverlayCommand, OverlayEvent},
    controller::OverlayController,
    error::OverlayError,
};
