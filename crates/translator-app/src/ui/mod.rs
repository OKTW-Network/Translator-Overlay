//! WinUI 3 control window — dashboard + settings (card layout).
//!
//! - `app`: window, navigation, and the sync loop; `chrome`: title-bar status, Start/Stop, Save bar.
//! - `hud`: the always-on-top control bar window.
//! - `state`: UI state shared by every handler, and the actions that change it.
//! - `widgets`: reusable controls; `pages`: one module per navigation page.

mod app;
mod chrome;
mod hud;
#[cfg(test)]
mod live_tests;
mod locale;
mod pages;
mod state;
mod widgets;

pub use crate::ui::{
    app::{control_hwnd, run},
    locale::{apply_ui_locale, system_locale_name},
};

#[cfg(test)]
mod i18n_tests {
    use rust_i18n::t;

    #[test]
    fn catalog_hit_fallback_and_placeholder() {
        assert_eq!(t!("action.save", locale = "zh-Hant"), "儲存");
        assert_eq!(t!("action.save", locale = "zh-Hans"), "保存");
        assert_eq!(t!("action.save", locale = "en"), "Save");
        assert_eq!(t!("action.save", locale = "fr"), "Save", "missing translations fall back to English");
        assert_eq!(t!("status.waiting_stable", locale = "en", elapsed_ms = 12), "Waiting for stable text (12 ms)");
        assert_eq!(t!("status.waiting_stable", locale = "zh-Hant", elapsed_ms = 12), "等待文字穩定（12 ms）");
    }

    #[test]
    fn pipeline_messages_are_localized() {
        for key in [
            "err.models_loading",
            "err.engine_not_ready",
            "err.overlay_unavailable",
            "err.no_frame",
            "err.models_task_ended",
            "status.downloading_translation",
            "status.loading_translation_model",
            "reader.placeholder",
        ] {
            let en = t!(key, locale = "en");
            assert_ne!(en, key, "{key} is missing from en");
            for locale in ["zh-Hant", "zh-Hans"] {
                assert_ne!(t!(key, locale = locale), en, "{key} is not translated for {locale}");
            }
        }
        assert_eq!(t!("err.ocr", locale = "zh-Hant", error = "x"), "OCR 失敗：x");
        assert_eq!(t!("err.save_config", locale = "zh-Hans", error = "x"), "无法保存设置：x");
    }
}
