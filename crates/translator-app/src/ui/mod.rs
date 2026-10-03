//! windows-reactor control UI — dashboard + settings (card layout).

mod app;
mod cards;
mod chrome;
mod controls;
mod locale;
mod pages;
mod preview;
mod shared;
mod xaml;

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
        assert_eq!(t!("only_en", locale = "zh-Hant"), "English only");
        assert_eq!(t!("status.waiting_stable", locale = "en", elapsed_ms = 12), "Waiting for stable text (12 ms)");
        assert_eq!(t!("status.waiting_stable", locale = "zh-Hant", elapsed_ms = 12), "等待文字穩定（12 ms）");
    }
}
