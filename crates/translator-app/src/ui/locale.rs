//! Control-window language. The first run follows the system locale, and later runs use the saved choice.

use translator_core::UiLanguage;
use windows::Win32::Globalization::GetUserDefaultLocaleName;

/// Languages shown in the language combo, in native names.
pub const UI_LANGUAGES: [(UiLanguage, &str); 3] = [
    (UiLanguage::En, "English"),
    (UiLanguage::ZhHant, "繁體中文"),
    (UiLanguage::ZhHans, "简体中文"),
];

pub fn apply_ui_locale(language: Option<UiLanguage>) {
    if let Some(language) = language {
        rust_i18n::set_locale(language.as_str());
    }
}

/// `GetUserDefaultLocaleName`, or empty when the call fails.
pub fn system_locale_name() -> String {
    // LOCALE_NAME_MAX_LENGTH
    let mut buf = [0u16; 85];
    let len = unsafe { GetUserDefaultLocaleName(&mut buf) };
    if len <= 1 {
        return String::new();
    }
    String::from_utf16_lossy(&buf[..(len as usize) - 1])
}
