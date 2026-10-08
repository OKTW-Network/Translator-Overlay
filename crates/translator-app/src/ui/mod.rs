//! windows-reactor control UI — dashboard + settings (card layout).

mod app;
mod cards;
mod chrome;
mod controls;
mod locale;
mod named;
mod pages;
mod preview;
mod shared;
mod xaml;

pub(crate) use crate::ui::chrome::status_text;
pub use crate::ui::{
    app::AppRoot,
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
