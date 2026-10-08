//! Transparent click-through overlay and an independent translation window.
//!
//! The overlay is a layered Win32 popup (`WS_EX_LAYERED | WS_EX_TRANSPARENT | …`)
//! that follows a target window. It draws semi-transparent boxes with translations
//! at the OCR bounding boxes, mapped from capture-image coordinates. A second,
//! clickable, always-on-top reader window shows the same text on its own.
//! A third always-on-top HUD has Start, Pause, and Stop buttons and the pipeline status.

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
