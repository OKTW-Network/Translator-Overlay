//! Application configuration loaded from `config.toml` next to the executable.

use std::{
    fmt, fs,
    path::{Path, PathBuf},
};

use serde::{
    Deserialize, Serialize, Serializer,
    de::{self, Deserializer, Visitor},
};
use thiserror::Error;

use crate::{
    paths::{config_path, resolve_under_exe},
    types::ModelTier,
};

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("path error: {0}")]
    Path(#[from] crate::paths::PathError),
    #[error("IO error for {path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("failed to parse config TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("failed to serialize config TOML: {0}")]
    Serialize(#[from] toml::ser::Error),
}

/// Root application configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub api: ApiConfig,
    pub translation: TranslationConfig,
    pub ocr: OcrConfig,
    pub capture: CaptureConfig,
    pub overlay: OverlayConfig,
}

impl AppConfig {
    /// Load from the default path (`{exe_dir}/config.toml`).
    /// Creates a default file when missing.
    pub fn load_or_create_default() -> Result<Self, ConfigError> {
        let path = config_path()?;
        Self::load_or_create(&path)
    }

    /// Load from `path`, or write defaults and return them if the file is absent.
    pub fn load_or_create(path: &Path) -> Result<Self, ConfigError> {
        if path.exists() {
            Self::load(path)
        } else {
            let config = Self::default();
            config.save(path)?;
            Ok(config)
        }
    }

    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = fs::read_to_string(path).map_err(|source| ConfigError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let config: Self = toml::from_str(&text)?;
        Ok(config)
    }

    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| ConfigError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let text = toml::to_string_pretty(self)?;
        fs::write(path, text).map_err(|source| ConfigError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(())
    }

    /// Persist to the default config path next to the executable.
    pub fn save_default_path(&self) -> Result<(), ConfigError> {
        self.save(&config_path()?)
    }
}

/// How the translator reaches a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModelProvider {
    /// OpenAI-compatible HTTP chat completions.
    #[default]
    OpenaiCompatible,
    /// Local Grok Build CLI over ACP stdio (`grok agent stdio`).
    GrokCli,
    /// Local Codex CLI over app-server stdio (`codex app-server`).
    CodexCli,
}

impl ModelProvider {
    pub fn is_cli(self) -> bool {
        matches!(self, Self::GrokCli | Self::CodexCli)
    }

    /// Default executable name when `cli_path` is empty.
    pub fn default_bin(self) -> &'static str {
        match self {
            Self::OpenaiCompatible => "",
            Self::GrokCli => "grok",
            Self::CodexCli => "codex",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::OpenaiCompatible => "OpenAI-compatible",
            Self::GrokCli => "Grok CLI",
            Self::CodexCli => "Codex CLI",
        }
    }
}

/// Locate `cli_path` or the provider default on `PATH`.
pub fn resolve_cli_binary(provider: ModelProvider, cli_path: &str) -> Option<std::path::PathBuf> {
    if !provider.is_cli() {
        return None;
    }
    let trimmed = cli_path.trim();
    if !trimmed.is_empty() {
        let path = std::path::PathBuf::from(trimmed);
        if path.is_file() {
            return Some(path);
        }
        // Bare command names still resolve through PATH; missing absolute paths stay None.
        if path.components().count() == 1 {
            return find_on_path(trimmed);
        }
        return None;
    }
    find_on_path(provider.default_bin())
}

fn find_on_path(name: &str) -> Option<std::path::PathBuf> {
    if name.is_empty() {
        return None;
    }
    let path_var = std::env::var_os("PATH")?;
    let mut names = vec![std::path::PathBuf::from(name)];
    if cfg!(windows) {
        let has_ext = std::path::Path::new(name).extension().is_some_and(|e| !e.is_empty());
        if !has_ext {
            for ext in [".exe", ".cmd", ".bat"] {
                names.push(std::path::PathBuf::from(format!("{name}{ext}")));
            }
        }
    }
    for dir in std::env::split_paths(&path_var) {
        for file_name in &names {
            let candidate = dir.join(file_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

/// OpenAI-compatible API settings.
///
/// Optional sampling parameters use `Option` so they can be omitted from HTTP
/// requests when unset (`skip_serializing_if`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    pub provider: ModelProvider,
    /// Absolute path or bare command. Empty = look up `grok` / `codex` on PATH.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cli_path: String,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    /// HTTP request timeout for chat completions (seconds). 0 = no limit.
    pub request_timeout_secs: u64,
    /// Extra attempts after the first failure (0 = try once only).
    pub max_retries: u32,
    /// Base backoff between retries in milliseconds (doubles each attempt).
    pub retry_backoff_ms: u64,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            provider: ModelProvider::OpenaiCompatible,
            cli_path: String::new(),
            base_url: "https://localhost/v1".to_string(),
            api_key: String::new(),
            model: "gptoss".to_string(),
            temperature: None,
            top_p: None,
            max_tokens: None,
            reasoning_effort: None,
            request_timeout_secs: 60,
            max_retries: 2,
            retry_backoff_ms: 500,
        }
    }
}

/// Body fragment used when calling chat completions.
/// Only fields that are `Some` are included in the JSON payload.
#[derive(Debug, Clone, Serialize)]
pub struct ChatCompletionRequestBody<'a> {
    pub model: &'a str,
    pub messages: &'a [ChatMessage],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<&'a str>,
}

impl ApiConfig {
    pub fn request_body<'a>(&'a self, messages: &'a [ChatMessage]) -> ChatCompletionRequestBody<'a> {
        ChatCompletionRequestBody {
            model: &self.model,
            messages,
            temperature: self.temperature,
            top_p: self.top_p,
            max_tokens: self.max_tokens,
            reasoning_effort: self.reasoning_effort.as_deref(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
        }
    }
}

/// Smallest allowed translation-cache capacity.
pub const TRANSLATION_CACHE_MAX_MIN: usize = 1;
/// Default translation-cache capacity (session LFU).
pub const TRANSLATION_CACHE_MAX_DEFAULT: usize = 128;
/// Hard cap applied when reading config / constructing the cache.
pub const TRANSLATION_CACHE_MAX_CAP: usize = 8192;
/// Translation-page slider lower bound.
pub const TRANSLATION_CACHE_SLIDER_MIN: usize = 1;
/// Translation-page slider upper bound.
pub const TRANSLATION_CACHE_SLIDER_MAX: usize = 8192;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TranslationConfig {
    pub source_lang: String,
    pub target_lang: String,
    pub history_max_items: usize,
    pub conversation_max_turns: usize,
    /// Skip the API for OCR block texts already translated this session.
    pub cache_enabled: bool,
    /// Max unique source strings kept in the in-memory LFU cache.
    pub cache_max_entries: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
}

impl Default for TranslationConfig {
    fn default() -> Self {
        Self {
            source_lang: "auto".to_string(),
            target_lang: "zh-TW".to_string(),
            history_max_items: 8,
            conversation_max_turns: 20,
            cache_enabled: true,
            cache_max_entries: TRANSLATION_CACHE_MAX_DEFAULT,
            system_prompt: None,
        }
    }
}

impl TranslationConfig {
    /// Cache capacity clamped to `[TRANSLATION_CACHE_MAX_MIN, TRANSLATION_CACHE_MAX_CAP]`.
    pub fn cache_max_entries_clamped(&self) -> usize {
        self.cache_max_entries.clamp(TRANSLATION_CACHE_MAX_MIN, TRANSLATION_CACHE_MAX_CAP)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OcrConfig {
    /// PP-OCRv6 tier: tiny | small | medium
    pub model_tier: ModelTier,
    /// Directory for model files (relative to exe dir unless absolute).
    pub models_dir: String,
    pub confidence_threshold: f32,
    /// Page-level: wait this long with unchanged OCR before translating.
    pub stable_duration_ms: u64,
    /// If OCR content keeps changing, force-translate after this many ms anyway.
    /// Covers thrash that never settles (e.g. trailing glyph flicker). 0 = off.
    pub max_unstable_ms: u64,
    /// Drop single ASCII letter/digit OCR hits (e.g. "0", "V", "C").
    pub filter_single_char: bool,
    /// Per-block: text must stay at roughly the same place this long before emit.
    /// Filters icons/animations that OCR misreads as changing text. 0 = disabled.
    pub block_persist_ms: u64,
    /// Forget a track if it is missing from frames longer than this.
    pub block_max_miss_ms: u64,
    /// Multi-line / paragraph merge heuristics (tunable).
    pub line_merge: LineMergeConfig,
}

impl Default for OcrConfig {
    fn default() -> Self {
        Self {
            model_tier: ModelTier::Small,
            models_dir: "models".to_string(),
            confidence_threshold: 0.8,
            stable_duration_ms: 500,
            max_unstable_ms: 1000,
            filter_single_char: true,
            block_persist_ms: 500,
            block_max_miss_ms: 750,
            line_merge: LineMergeConfig::default(),
        }
    }
}

/// Reading order when joining lines inside a merged block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LineMergeOrder {
    /// Rows top-to-bottom; left-to-right within each row.
    TopToBottomLeftToRight,
    /// Columns left-to-right; top-to-bottom within each column.
    #[default]
    LeftToRightTopToBottom,
}

/// Tunable multi-line OCR merge (geometry + join style).
///
/// Distance thresholds are fractions of the **full capture frame** (not line
/// height). Shape comparisons stay box-to-box (`|delta| / larger`). Tune via
/// `config.toml` `[ocr.line_merge]` or the OCR settings UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct LineMergeConfig {
    /// Master switch. `false` keeps every OCR line as its own block.
    pub enabled: bool,
    /// When hand-drawn OCR regions exist, join every line in each region.
    /// Ignored for whole-window OCR (rules still apply).
    pub merge_whole_region: bool,
    /// Join order inside a merged group.
    pub order: LineMergeOrder,
    /// Allowed `|vertical gap|` as a fraction of frame height (paragraph mode).
    #[serde(alias = "max_gap_ratio")]
    pub gap_ratio: f32,
    /// Left- or center-edge delta ≤ this × frame width counts as column-aligned.
    #[serde(alias = "left_align_ratio")]
    pub align_ratio: f32,
    /// Allowed `|h1 − h2| / larger(h)` to treat lines as the same size.
    pub height_delta_ratio: f32,
    /// Horizontal overlap as a fraction of the shorter line width.
    #[serde(alias = "overlap_ratio_min")]
    pub overlap_ratio: f32,
    /// Overlap floor (vs shorter width) when using the align path.
    pub align_overlap_ratio: f32,
    /// Row / column banding as a fraction of frame height / width.
    pub order_band_ratio: f32,
    /// Lower counts as below if `top + height × this ≥` the upper vertical mid.
    pub below_mid_ratio: f32,
    /// When true, do not glue a shorter upper line onto a much wider line below.
    pub reject_short_long: bool,
    /// Allowed `(w_lower − w_upper) / w_lower` when [`Self::reject_short_long`] is on.
    pub width_delta_ratio: f32,
    /// Insert a space between joined lines (`false` concatenates).
    pub join_with_space: bool,
}

impl Default for LineMergeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            merge_whole_region: false,
            order: LineMergeOrder::default(),
            // ~16px on 1080p — below typical UI list pitch, above wrap leading.
            gap_ratio: 0.015,
            align_ratio: 0.012,
            height_delta_ratio: 0.45,
            overlap_ratio: 0.35,
            align_overlap_ratio: 0.10,
            order_band_ratio: 0.012,
            below_mid_ratio: 0.25,
            reject_short_long: true,
            width_delta_ratio: 0.40,
            join_with_space: true,
        }
    }
}

impl OcrConfig {
    pub fn models_dir_path(&self) -> Result<PathBuf, crate::paths::PathError> {
        resolve_under_exe(&self.models_dir)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CaptureConfig {
    pub min_interval_ms: u64,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self { min_interval_ms: 250 }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct OverlayConfig {
    /// Draw the click-through overlay on the capture target.
    pub enabled: bool,
    /// Show the independent always-on-top translation window.
    pub reader_enabled: bool,
    /// Translation-window font size in pixels (Segoe UI). Clamped when applied.
    pub reader_font_px: u32,
    /// Text colour including alpha (`0xAARRGGBB` in config.toml).
    #[serde(serialize_with = "serialize_argb_hex", deserialize_with = "deserialize_argb_hex")]
    pub text_color_argb: u32,
    /// Box fill colour including alpha (`0xAARRGGBB` in config.toml).
    #[serde(serialize_with = "serialize_argb_hex", deserialize_with = "deserialize_argb_hex")]
    pub background_color_argb: u32,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            reader_enabled: true,
            reader_font_px: READER_FONT_PX_DEFAULT,
            text_color_argb: 0xFFFF_FFFF,
            background_color_argb: 0xC800_0000,
        }
    }
}

/// Smallest translation-window font (CreateFont cell height).
pub const READER_FONT_PX_MIN: u32 = 8;
/// Largest translation-window font.
pub const READER_FONT_PX_MAX: u32 = 72;
/// Default translation-window font.
pub const READER_FONT_PX_DEFAULT: u32 = 20;

impl OverlayConfig {
    /// Font size used by the translation window (clamped).
    pub fn reader_font_px_clamped(&self) -> i32 {
        self.reader_font_px.clamp(READER_FONT_PX_MIN, READER_FONT_PX_MAX) as i32
    }
}

/// Format ARGB as a readable hex string for TOML (e.g. `"0xC8000000"`).
pub fn format_argb_hex(argb: u32) -> String {
    format!("0x{argb:08X}")
}

/// Parse ARGB from `"0xAARRGGBB"`, `"#AARRGGBB"`, bare hex, or a decimal integer string.
pub fn parse_argb_hex(s: &str) -> Option<u32> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    let hex = t
        .strip_prefix("0x")
        .or_else(|| t.strip_prefix("0X"))
        .or_else(|| t.strip_prefix('#'))
        .unwrap_or(t);
    if hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return u32::from_str_radix(hex, 16).ok();
    }
    // Legacy / accidental decimal string.
    t.parse::<u32>().ok()
}

fn serialize_argb_hex<S>(value: &u32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&format_argb_hex(*value))
}

fn deserialize_argb_hex<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: Deserializer<'de>,
{
    struct ArgbVisitor;

    impl<'de> Visitor<'de> for ArgbVisitor {
        type Value = u32;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("ARGB color as \"0xAARRGGBB\" hex string or integer")
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<u32, E> {
            parse_argb_hex(v).ok_or_else(|| E::custom(format!("invalid ARGB color: {v:?}")))
        }

        fn visit_string<E: de::Error>(self, v: String) -> Result<u32, E> {
            self.visit_str(&v)
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<u32, E> {
            u32::try_from(v).map_err(|_| E::custom(format!("ARGB color out of range: {v}")))
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<u32, E> {
            u32::try_from(v).map_err(|_| E::custom(format!("ARGB color out of range: {v}")))
        }
    }

    deserializer.deserialize_any(ArgbVisitor)
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temp_config_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("translator_overlay_{name}_{nanos}.toml"))
    }

    #[test]
    fn default_roundtrip_toml() {
        let config = AppConfig::default();
        let text = toml::to_string_pretty(&config).unwrap();
        assert!(text.contains("text_color_argb = \"0xFFFFFFFF\""), "expected hex string in TOML, got:\n{text}");
        assert!(text.contains("background_color_argb = \"0xC8000000\""), "expected hex string in TOML, got:\n{text}");
        let parsed: AppConfig = toml::from_str(&text).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn overlay_colors_accept_legacy_decimal_and_hex_forms() {
        let text = r#"
[overlay]
text_color_argb = 4294967295
background_color_argb = "C8000000"
"#;
        let config: AppConfig = toml::from_str(text).unwrap();
        assert_eq!(config.overlay.text_color_argb, 0xFFFF_FFFF);
        assert_eq!(config.overlay.background_color_argb, 0xC800_0000);

        let hashed = r##"
[overlay]
text_color_argb = "#AABBCCDD"
background_color_argb = "0xc8000000"
"##;
        let config: AppConfig = toml::from_str(hashed).unwrap();
        assert_eq!(config.overlay.text_color_argb, 0xAABB_CCDD);
        assert_eq!(config.overlay.background_color_argb, 0xC800_0000);
    }

    #[test]
    fn load_or_create_writes_default() {
        let path = temp_config_path("create");
        let _ = fs::remove_file(&path);
        let config = AppConfig::load_or_create(&path).unwrap();
        assert!(path.exists());
        assert_eq!(config.translation.target_lang, "zh-TW");
        assert_eq!(config.ocr.model_tier, ModelTier::Small);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn provider_defaults_to_openai_compatible() {
        let text = r#"
[api]
model = "my-model"
"#;
        let config: AppConfig = toml::from_str(text).unwrap();
        assert_eq!(config.api.provider, ModelProvider::OpenaiCompatible);
        assert!(config.api.cli_path.is_empty());
        assert!(!config.api.provider.is_cli());
    }

    #[test]
    fn provider_cli_roundtrip() {
        let mut config = AppConfig::default();
        config.api.provider = ModelProvider::GrokCli;
        config.api.cli_path = r"C:\tools\grok.exe".into();
        config.api.model = "grok-4.5".into();
        let text = toml::to_string_pretty(&config).unwrap();
        assert!(text.contains("provider = \"grok_cli\""), "got:\n{text}");
        assert!(text.contains("cli_path"), "got:\n{text}");
        let parsed: AppConfig = toml::from_str(&text).unwrap();
        assert_eq!(parsed.api.provider, ModelProvider::GrokCli);
        assert_eq!(parsed.api.cli_path, r"C:\tools\grok.exe");
    }

    #[test]
    fn resolve_cli_missing_path_is_none() {
        assert!(resolve_cli_binary(ModelProvider::OpenaiCompatible, "").is_none());
        assert!(resolve_cli_binary(ModelProvider::GrokCli, r"C:\definitely-missing\grok.exe").is_none());
        assert!(resolve_cli_binary(ModelProvider::CodexCli, r"Z:\no-such-codex.exe").is_none());
    }

    #[test]
    fn optional_api_params_omitted_from_request_json() {
        let api = ApiConfig::default();
        let messages = [ChatMessage::user("hi")];
        let body = api.request_body(&messages);
        let json = serde_json::to_value(&body).unwrap();
        let obj = json.as_object().unwrap();
        assert!(obj.contains_key("model"));
        assert!(obj.contains_key("messages"));
        assert!(!obj.contains_key("temperature"));
        assert!(!obj.contains_key("top_p"));
        assert!(!obj.contains_key("max_tokens"));
        assert!(!obj.contains_key("reasoning_effort"));
    }

    #[test]
    fn optional_api_params_included_when_set() {
        let api = ApiConfig {
            temperature: Some(0.2),
            reasoning_effort: Some("medium".to_string()),
            ..ApiConfig::default()
        };
        let messages = [ChatMessage::user("hi")];
        let body = api.request_body(&messages);
        let json = serde_json::to_value(&body).unwrap();
        let temp = json["temperature"].as_f64().unwrap();
        assert!((temp - 0.2).abs() < 1e-5);
        assert_eq!(json["reasoning_effort"], "medium");
        assert!(json.get("top_p").is_none());
    }

    #[test]
    fn line_merge_legacy_keys_ignored_and_order_parses() {
        let text = r#"
[ocr.line_merge]
enabled = true
gap_slack = 9.9
list_min_peers = 99
keep_speaker_separate = true
order = "top_to_bottom_left_to_right"
merge_whole_region = true
left_align_ratio = 0.05
"#;
        let config: AppConfig = toml::from_str(text).unwrap();
        assert!(config.ocr.line_merge.enabled);
        assert!(config.ocr.line_merge.merge_whole_region);
        assert_eq!(config.ocr.line_merge.order, LineMergeOrder::TopToBottomLeftToRight);
        assert!((config.ocr.line_merge.gap_ratio - 0.015).abs() < 1e-6);
        assert!((config.ocr.line_merge.align_ratio - 0.05).abs() < 1e-6);
        assert!(config.ocr.line_merge.join_with_space);
        assert!(config.ocr.line_merge.reject_short_long);
    }

    #[test]
    fn line_merge_max_gap_ratio_alias_fills_gap_ratio() {
        let text = r#"
[ocr.line_merge]
max_gap_ratio = 0.02
overlap_ratio_min = 0.4
"#;
        let config: AppConfig = toml::from_str(text).unwrap();
        assert!((config.ocr.line_merge.gap_ratio - 0.02).abs() < 1e-6);
        assert!((config.ocr.line_merge.overlap_ratio - 0.4).abs() < 1e-6);
    }

    #[test]
    fn line_merge_omitted_order_defaults_left_to_right() {
        let text = r#"
[ocr.line_merge]
enabled = true
"#;
        let config: AppConfig = toml::from_str(text).unwrap();
        assert_eq!(config.ocr.line_merge.order, LineMergeOrder::LeftToRightTopToBottom);
    }

    #[test]
    fn line_merge_defaults_land_on_ocr_slider_ticks() {
        // Same integer-micro-unit snap as `quantize_to_step` (not `min + n×0.1`).
        const SCALE: f64 = 1_000_000.0;
        fn snapped(pct: f64, min: f64, max: f64, step: f64) -> f64 {
            let v = pct.clamp(min, max);
            if !(step.is_finite() && step > 0.0) {
                return v;
            }
            let min_i = (min * SCALE).round() as i64;
            let step_i = (step * SCALE).round() as i64;
            if step_i == 0 {
                return v;
            }
            let n = ((v - min) / step).round() as i64;
            let out = (min_i.saturating_add(n.saturating_mul(step_i))) as f64 / SCALE;
            out.clamp(min, max)
        }
        let c = LineMergeConfig::default();
        let on_tick = |pct: f64, min: f64, max: f64, step: f64, want: f64| {
            let got = snapped(pct, min, max, step);
            assert!((got - want).abs() < 1e-9, "got={got} want={want} (pct={pct})");
        };
        on_tick(f64::from(c.gap_ratio) * 100.0, 0.0, 8.0, 0.1, 1.5);
        on_tick(f64::from(c.height_delta_ratio) * 100.0, 0.0, 90.0, 1.0, 45.0);
        on_tick(f64::from(c.overlap_ratio) * 100.0, 0.0, 100.0, 1.0, 35.0);
        on_tick(f64::from(c.align_ratio) * 100.0, 0.0, 5.0, 0.1, 1.2);
        on_tick(f64::from(c.align_overlap_ratio) * 100.0, 0.0, 50.0, 1.0, 10.0);
        on_tick(f64::from(c.order_band_ratio) * 100.0, 0.1, 5.0, 0.1, 1.2);
        on_tick(f64::from(c.below_mid_ratio) * 100.0, 0.0, 50.0, 1.0, 25.0);
        on_tick(f64::from(c.width_delta_ratio) * 100.0, 0.0, 90.0, 1.0, 40.0);
    }

    #[test]
    fn partial_toml_uses_defaults() {
        let text = r#"
[api]
model = "my-model"

[translation]
target_lang = "ja"
"#;
        let config: AppConfig = toml::from_str(text).unwrap();
        assert_eq!(config.api.model, "my-model");
        assert_eq!(config.translation.target_lang, "ja");
        assert_eq!(config.ocr.model_tier, ModelTier::Small);
        assert!(config.overlay.enabled);
        assert!(config.overlay.reader_enabled);
    }

    #[test]
    fn overlay_display_defaults_on() {
        let overlay = OverlayConfig::default();
        assert!(overlay.enabled);
        assert!(overlay.reader_enabled);
        assert_eq!(overlay.reader_font_px, READER_FONT_PX_DEFAULT);
        assert_eq!(overlay.reader_font_px_clamped(), READER_FONT_PX_DEFAULT as i32);
    }

    #[test]
    fn overlay_reader_font_clamps() {
        let mut overlay = OverlayConfig {
            reader_font_px: 0,
            ..OverlayConfig::default()
        };
        assert_eq!(overlay.reader_font_px_clamped(), READER_FONT_PX_MIN as i32);
        overlay.reader_font_px = 200;
        assert_eq!(overlay.reader_font_px_clamped(), READER_FONT_PX_MAX as i32);
    }

    #[test]
    fn translation_cache_defaults_and_clamp() {
        let text = r#"
[translation]
target_lang = "ja"
"#;
        let config: AppConfig = toml::from_str(text).unwrap();
        assert!(config.translation.cache_enabled);
        assert_eq!(config.translation.cache_max_entries, TRANSLATION_CACHE_MAX_DEFAULT);
        assert_eq!(config.translation.cache_max_entries_clamped(), TRANSLATION_CACHE_MAX_DEFAULT);

        let mut cfg = TranslationConfig {
            cache_max_entries: 0,
            ..TranslationConfig::default()
        };
        assert_eq!(cfg.cache_max_entries_clamped(), TRANSLATION_CACHE_MAX_MIN);
        cfg.cache_max_entries = 1_000_000;
        assert_eq!(cfg.cache_max_entries_clamped(), TRANSLATION_CACHE_MAX_CAP);
    }

    #[test]
    fn overlay_display_flags_roundtrip_and_partial() {
        let text = r#"
[overlay]
enabled = false
reader_enabled = false
"#;
        let config: AppConfig = toml::from_str(text).unwrap();
        assert!(!config.overlay.enabled);
        assert!(!config.overlay.reader_enabled);

        let partial = r#"
[overlay]
text_color_argb = "0xFFFFFFFF"
"#;
        let config: AppConfig = toml::from_str(partial).unwrap();
        assert!(config.overlay.enabled);
        assert!(config.overlay.reader_enabled);
        assert_eq!(config.overlay.reader_font_px, READER_FONT_PX_DEFAULT);
    }
}
