//! Control-window pages: dashboard and settings forms.

mod api;
mod dashboard;
mod ocr;
mod overlay;
mod translation;

pub use crate::ui::pages::{api::api_page, dashboard::dashboard_page, ocr::ocr_page, overlay::overlay_page, translation::translation_page};
