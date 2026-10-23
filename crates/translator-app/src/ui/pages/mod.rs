//! Control-window pages: dashboard and settings forms.

mod api;
// Public inside `ui` so the live tests can reach the preview.
pub mod dashboard;
mod hotkeys;
mod ocr;
mod overlay;
mod translation;

pub use crate::ui::pages::{
    api::api_page,
    dashboard::{Dashboard, dashboard_page},
    hotkeys::hotkeys_page,
    ocr::ocr_page,
    overlay::overlay_page,
    translation::translation_page,
};
