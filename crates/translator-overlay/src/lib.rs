//! Transparent click-through overlay and an independent translation window.
//!
//! Creates a layered Win32 popup (`WS_EX_LAYERED | WS_EX_TRANSPARENT | …`) that
//! tracks a target window and draws semi-transparent boxes + translations at
//! OCR bounding boxes (mapped from capture-image coordinates). A second,
//! clickable always-on-top reader window shows the same text independently.

mod command;
mod controller;
mod error;
mod gfx;
mod host;
mod picker;
mod reader;

pub use crate::{
    command::{OverlayCommand, OverlayEvent},
    controller::OverlayController,
    error::OverlayError,
    gfx::draw::argb_channels,
};
