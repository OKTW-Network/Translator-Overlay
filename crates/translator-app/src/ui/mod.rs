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
    use std::collections::{BTreeMap, BTreeSet};

    use rust_i18n::t;

    use crate::ui::locale::UI_LANGUAGES;

    fn collect_locale_entries(raw: &str) -> BTreeMap<String, String> {
        let table: toml::Table = toml::from_str(raw).expect("valid locale TOML");
        let mut out = BTreeMap::new();
        for (k, v) in &table {
            if k == "_version" {
                assert!(v.is_integer(), "_version must be an integer, got {v:?}");
                continue;
            }
            flatten_toml(k, v, &mut out);
        }
        out
    }

    fn flatten_toml(prefix: &str, value: &toml::Value, out: &mut BTreeMap<String, String>) {
        match value {
            toml::Value::String(s) => {
                assert!(!s.trim().is_empty(), "empty translation for key {prefix}");
                out.insert(prefix.to_string(), s.clone());
            }
            toml::Value::Table(map) => {
                for (k, v) in map {
                    flatten_toml(&format!("{prefix}.{k}"), v, out);
                }
            }
            other => panic!("unexpected non-string value at {prefix}: {other:?}"),
        }
    }

    fn placeholders(s: &str) -> BTreeSet<String> {
        let mut set = BTreeSet::new();
        let mut rest = s;
        while let Some(idx) = rest.find("%{") {
            let after = &rest[idx + 2..];
            let end = after.find('}').expect("unclosed %{var} placeholder");
            set.insert(after[..end].to_string());
            rest = &after[end + 1..];
        }
        set
    }

    #[test]
    fn locales_have_complete_matching_keys_and_placeholders() {
        let available = rust_i18n::available_locales!();
        for (lang, _) in UI_LANGUAGES {
            assert!(available.iter().any(|l| l.as_ref() == lang.as_str()), "missing locale {lang:?} in rust_i18n");
        }

        let en = collect_locale_entries(include_str!("../../locales/en.toml"));
        let zh_hans = collect_locale_entries(include_str!("../../locales/zh-Hans.toml"));
        let zh_hant = collect_locale_entries(include_str!("../../locales/zh-Hant.toml"));

        let en_keys: BTreeSet<&str> = en.keys().map(String::as_str).collect();
        let hans_keys: BTreeSet<&str> = zh_hans.keys().map(String::as_str).collect();
        let hant_keys: BTreeSet<&str> = zh_hant.keys().map(String::as_str).collect();

        assert_eq!(hans_keys, hant_keys, "zh-Hans and zh-Hant key sets differ");
        let orphan_hans: Vec<_> = hans_keys.difference(&en_keys).collect();
        assert!(orphan_hans.is_empty(), "orphan keys absent from en.toml: {orphan_hans:?}");

        const INTENTIONALLY_ENGLISH: &[&str] = &[
            "api.base_url",
            "api.builtin_gpu",
            "api.chat_completions",
            "api.reasoning_placeholder",
            "api.responses",
            "api.temperature",
            "api.top_k",
            "api.top_p",
            "dash.ocr",
            "nav.api",
            "nav.ocr",
            "nav.overlay",
            "ocr.cpu",
            "ocr.det_side_1280",
            "ocr.det_side_1920",
            "ocr.det_side_960",
            "ocr.directml",
            "ocr.medium",
            "ocr.small",
            "ocr.tiny",
            "ocr.v5_mobile",
            "ocr.v5_server",
            "ocr.webgpu",
            "tr.source_placeholder",
        ];
        let expected_en_only: BTreeSet<&str> = INTENTIONALLY_ENGLISH.iter().copied().collect();
        let actual_en_only: BTreeSet<&str> = en_keys.difference(&hans_keys).copied().collect();
        assert_eq!(actual_en_only, expected_en_only, "unexpected untranslated keys missing from zh-Hans/zh-Hant");

        for (key, en_val) in &en {
            let expected = placeholders(en_val);
            if let Some(val) = zh_hans.get(key) {
                assert_eq!(placeholders(val), expected, "placeholder mismatch in zh-Hans for {key}");
            }
            if let Some(val) = zh_hant.get(key) {
                assert_eq!(placeholders(val), expected, "placeholder mismatch in zh-Hant for {key}");
            }
        }
    }

    #[test]
    fn catalog_hit_fallback_and_placeholder() {
        assert_eq!(t!("action.save", locale = "zh-Hant"), "儲存");
        assert_eq!(t!("action.save", locale = "zh-Hans"), "保存");
        assert_eq!(t!("action.save", locale = "en"), "Save");
        assert_eq!(t!("action.save", locale = "fr"), "Save", "missing translations fall back to English");
        assert_eq!(t!("status.waiting_stable", locale = "en", elapsed_ms = 12), "Waiting for stable text (12 ms)");
        assert_eq!(t!("status.waiting_stable", locale = "zh-Hant", elapsed_ms = 12), "等待文字穩定（12 ms）");
        assert_eq!(t!("err.ocr", locale = "zh-Hant", error = "x"), "OCR 失敗：x");
        assert_eq!(t!("err.save_config", locale = "zh-Hans", error = "x"), "无法保存设置：x");
    }
}
