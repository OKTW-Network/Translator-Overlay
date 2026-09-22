//! Application configuration loaded from `config.toml` next to the executable.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize, Serializer};
use thiserror::Error;

use crate::{
    paths::{PathError, resolve_under_exe},
    types::ModelTier,
};

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("path error: {0}")]
    Path(#[from] PathError),
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
    /// Load from `path`, or write defaults and return them if the file is absent.
    ///
    /// Invalid fields are skipped (see [`Self::load`]).
    pub fn load_or_create(path: &Path) -> Result<Self, ConfigError> {
        if path.exists() {
            Self::load(path)
        } else {
            let config = Self::default();
            config.save(path)?;
            Ok(config)
        }
    }

    /// Load from `path`.
    ///
    /// Each TOML field is applied independently: a value that fails to parse is
    /// skipped and that field keeps its default. IO errors and TOML syntax
    /// errors still fail the whole load.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = fs::read_to_string(path).map_err(|source| ConfigError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_toml_lenient(&text)
    }

    /// Parse TOML, keeping valid fields and dropping values that do not match the schema.
    fn from_toml_lenient(text: &str) -> Result<Self, ConfigError> {
        let src: toml::Table = toml::from_str(text)?;
        let mut dest = toml::Table::new();
        apply_lenient(&mut dest, &src, "");
        Ok(Self::deserialize(toml::Value::Table(dest)).unwrap_or_default())
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
}

/// Try each key as a whole value; if a table fails, apply its children independently.
fn apply_lenient(root: &mut toml::Table, src: &toml::Table, prefix: &str) {
    for (key, value) in src {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };

        insert_by_path(root, &path, value.clone());
        if AppConfig::deserialize(toml::Value::Table(root.clone())).is_ok() {
            continue;
        }
        remove_by_path(root, &path);

        if let toml::Value::Table(child) = value {
            apply_lenient(root, child, &path);
        }
    }
}

fn insert_by_path(root: &mut toml::Table, path: &str, value: toml::Value) {
    let mut parts = path.split('.');
    let Some(last) = parts.next_back() else {
        return;
    };
    let mut current = root;
    for part in parts {
        if !matches!(current.get(part), Some(toml::Value::Table(_))) {
            current.insert(part.to_string(), toml::Value::Table(toml::Table::new()));
        }
        let Some(toml::Value::Table(next)) = current.get_mut(part) else {
            unreachable!("insert_by_path ensures a table");
        };
        current = next;
    }
    current.insert(last.to_string(), value);
}

fn remove_by_path(root: &mut toml::Table, path: &str) {
    let mut parts = path.split('.');
    let Some(last) = parts.next_back() else {
        return;
    };
    let mut current = root;
    for part in parts {
        let Some(toml::Value::Table(next)) = current.get_mut(part) else {
            return;
        };
        current = next;
    }
    current.remove(last);
}

/// How the translator reaches a model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModelProvider {
    /// OpenAI-compatible HTTP chat completions or Responses API.
    #[default]
    OpenaiCompatible,
    /// Local Grok Build CLI over ACP stdio (`grok agent stdio`).
    GrokCli,
    /// Local OpenCode CLI over ACP stdio (`opencode acp`).
    #[serde(rename = "opencode_cli")]
    OpenCodeCli,
    /// Local Codex CLI over app-server stdio (`codex app-server`).
    CodexCli,
}

/// HTTP wire format when [`ModelProvider::OpenaiCompatible`] is selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum HttpApi {
    #[default]
    ChatCompletions,
    Responses,
}

/// Preferred processing tier when the selected provider supports one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ServiceTier {
    #[default]
    Standard,
    Priority,
}

impl ModelProvider {
    pub fn is_cli(self) -> bool {
        matches!(self, Self::GrokCli | Self::OpenCodeCli | Self::CodexCli)
    }

    /// Default executable name when `cli_path` is empty.
    pub fn default_bin(self) -> &'static str {
        match self {
            Self::OpenaiCompatible => "",
            Self::GrokCli => "grok",
            Self::OpenCodeCli => "opencode",
            Self::CodexCli => "codex",
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
    let append_ext = cfg!(windows) && !std::path::Path::new(name).extension().is_some_and(|e| !e.is_empty());
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
        if append_ext {
            for ext in [".exe", ".cmd", ".bat"] {
                let candidate = dir.join(format!("{name}{ext}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    value == &T::default()
}

/// `ApiConfig` defaults these flags to `true`, which is not [`bool::default`].
fn is_true(value: &bool) -> bool {
    *value
}

/// OpenAI-compatible API settings.
///
/// Optional sampling parameters use `Option` so they can be omitted from HTTP
/// requests when unset (`skip_serializing_if`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    pub provider: ModelProvider,
    /// HTTP endpoint style. Ignored for CLI providers.
    #[serde(skip_serializing_if = "is_default")]
    pub http_api: HttpApi,
    /// Absolute path or bare command. Empty = look up `grok` / `opencode` / `codex` on PATH.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cli_path: String,
    /// Preferred processing tier. Ignored by providers that do not support it.
    #[serde(skip_serializing_if = "is_default")]
    pub service_tier: ServiceTier,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub base_url: String,
    #[serde(skip_serializing_if = "String::is_empty")]
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
    /// Request JSON Schema Structured Outputs on HTTP OpenAI-compatible APIs.
    /// Ignored for CLI providers. Turn off if the endpoint rejects `json_schema`.
    #[serde(skip_serializing_if = "is_true")]
    pub structured_outputs: bool,
    /// Request SSE streaming on HTTP OpenAI-compatible APIs.
    /// Ignored for CLI providers. Turn off if the endpoint rejects `stream`.
    #[serde(skip_serializing_if = "is_true")]
    pub stream: bool,
    /// Replay assistant `reasoning_content` on follow-up Chat Completions turns.
    /// Ignored for Responses APIs and CLI providers. Turn off if the endpoint rejects `reasoning_content`.
    #[serde(skip_serializing_if = "is_true")]
    pub send_reasoning_content: bool,
    /// Idle timeout for HTTP reads / CLI prompts (seconds). 0 = no limit.
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
            http_api: HttpApi::ChatCompletions,
            cli_path: String::new(),
            service_tier: ServiceTier::Standard,
            base_url: "https://localhost/v1".to_string(),
            api_key: String::new(),
            model: "gptoss".to_string(),
            temperature: None,
            top_p: None,
            max_tokens: None,
            reasoning_effort: None,
            structured_outputs: true,
            stream: true,
            send_reasoning_content: true,
            request_timeout_secs: 60,
            max_retries: 2,
            retry_backoff_ms: 500,
        }
    }
}

impl ApiConfig {
    /// Clear fields the selected provider does not store on an API profile.
    ///
    /// `base_url` is emptied, not replaced with the localhost default, so serialization omits it.
    pub(crate) fn blank_inapplicable(&mut self) {
        let defaults = Self::default();
        if self.provider.is_cli() {
            self.http_api = defaults.http_api;
            self.base_url.clear();
            self.api_key.clear();
            self.structured_outputs = defaults.structured_outputs;
            self.stream = defaults.stream;
            self.send_reasoning_content = defaults.send_reasoning_content;
        } else {
            self.cli_path.clear();
        }
        if self.provider != ModelProvider::CodexCli {
            self.service_tier = defaults.service_tier;
        }
    }
}

/// Smallest allowed translation-cache capacity.
pub const TRANSLATION_CACHE_MAX_MIN: usize = 1;
/// Default translation-cache capacity (session LFU).
pub const TRANSLATION_CACHE_MAX_DEFAULT: usize = 128;
/// Hard cap applied when reading config / constructing the cache.
pub const TRANSLATION_CACHE_MAX_CAP: usize = 8192;

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
    /// Down each column, then the next column to the right (column-major).
    TopToBottomLeftToRight,
    /// Across each row left-to-right, then the next row down (row-major).
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
    pub gap_ratio: f32,
    /// Allowed `|horizontal gap|` as a fraction of frame width (paragraph mode).
    pub horizontal_gap_ratio: f32,
    /// Left- or center-edge delta ≤ this × frame width counts as column-aligned.
    /// Top- or center-edge delta ≤ this × frame height counts as row-aligned.
    pub align_ratio: f32,
    /// Allowed `|h1 − h2| / larger(h)` to treat lines as the same size.
    pub height_delta_ratio: f32,
    /// Row / column banding as a fraction of frame height / width.
    pub order_band_ratio: f32,
    /// Lower counts as below if `top + height × this ≥` the upper vertical mid.
    pub below_mid_ratio: f32,
    /// When true, do not glue a shorter upper line onto a much wider line below.
    pub reject_short_long: bool,
    /// When [`Self::reject_short_long`] is on, a shorter upper line stays separate
    /// if `(w_lower − w_upper) / w_lower` is at least this.
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
            // ~29px on 1080p/1920 — side-by-side neighbors in the same row.
            horizontal_gap_ratio: 0.015,
            align_ratio: 0.012,
            height_delta_ratio: 0.45,
            order_band_ratio: 0.012,
            below_mid_ratio: 0.25,
            reject_short_long: true,
            width_delta_ratio: 0.25,
            join_with_space: true,
        }
    }
}

impl OcrConfig {
    pub fn models_dir_path(&self) -> Result<PathBuf, PathError> {
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
    /// Text colour including alpha (`#AARRGGBB` in config.toml).
    #[serde(serialize_with = "serialize_argb_hex", deserialize_with = "deserialize_argb_hex")]
    pub text_color_argb: u32,
    /// Box fill colour including alpha (`#AARRGGBB` in config.toml).
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

/// Format ARGB as `"#AARRGGBB"`.
pub fn format_argb_hex(argb: u32) -> String {
    format!("#{argb:08X}")
}

/// Parse `"#AARRGGBB"` (optional surrounding space; hex digits case-insensitive).
pub fn parse_argb_hex(s: &str) -> Option<u32> {
    let hex = s.trim().strip_prefix('#')?;
    if hex.len() != 8 {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

fn serialize_argb_hex<S>(value: &u32, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&format_argb_hex(*value))
}

fn deserialize_argb_hex<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    parse_argb_hex(&s).ok_or_else(|| serde::de::Error::custom(format!("invalid ARGB color: {s:?}")))
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
        assert!(text.contains("text_color_argb = \"#FFFFFFFF\""), "expected hex string in TOML, got:\n{text}");
        assert!(text.contains("background_color_argb = \"#C8000000\""), "expected hex string in TOML, got:\n{text}");
        let parsed: AppConfig = toml::from_str(&text).unwrap();
        assert_eq!(config, parsed);
    }

    #[test]
    fn parse_argb_hex_hash_argb() {
        assert_eq!(parse_argb_hex("#AABBCCDD"), Some(0xAABB_CCDD));
        assert_eq!(parse_argb_hex("#aabbccdd"), Some(0xAABB_CCDD));
        assert_eq!(parse_argb_hex("  #C8000000  "), Some(0xC800_0000));
        assert_eq!(parse_argb_hex("0xC8000000"), None);
        assert_eq!(parse_argb_hex("#AABBCC"), None);
        assert_eq!(parse_argb_hex("FFFFFFFF"), None);
        assert_eq!(parse_argb_hex("#FFFF"), None);
        assert_eq!(parse_argb_hex("#"), None);
    }

    #[test]
    fn overlay_colors_accept_hash_argb_and_reject_legacy_forms() {
        let hashed = r##"
[overlay]
text_color_argb = "#AABBCCDD"
background_color_argb = "#c8000000"
"##;
        let config: AppConfig = toml::from_str(hashed).unwrap();
        assert_eq!(config.overlay.text_color_argb, 0xAABB_CCDD);
        assert_eq!(config.overlay.background_color_argb, 0xC800_0000);

        for bad in [
            "[overlay]\ntext_color_argb = \"C8000000\"\n",
            "[overlay]\ntext_color_argb = 4294967295\n",
            "[overlay]\ntext_color_argb = \"0xC8000000\"\n",
            "[overlay]\ntext_color_argb = \"#AABBCC\"\n",
        ] {
            assert!(toml::from_str::<AppConfig>(bad).is_err(), "{bad}");
        }
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
        assert_eq!(config.api.http_api, HttpApi::ChatCompletions);
        assert!(config.api.cli_path.is_empty());
        assert!(!config.api.provider.is_cli());
    }

    #[test]
    fn http_api_responses_roundtrip() {
        let mut config = AppConfig::default();
        config.api.http_api = HttpApi::Responses;
        let text = toml::to_string_pretty(&config).unwrap();
        assert!(text.contains("http_api = \"responses\""), "got:\n{text}");
        let parsed: AppConfig = toml::from_str(&text).unwrap();
        assert_eq!(parsed.api.http_api, HttpApi::Responses);
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
    fn provider_opencode_cli_roundtrip() {
        let mut config = AppConfig::default();
        config.api.provider = ModelProvider::OpenCodeCli;
        config.api.cli_path = r"C:\tools\opencode.exe".into();
        config.api.model = "opencode/gpt-5".into();
        let text = toml::to_string_pretty(&config).unwrap();
        assert!(text.contains("provider = \"opencode_cli\""), "got:\n{text}");
        let parsed: AppConfig = toml::from_str(&text).unwrap();
        assert_eq!(parsed.api.provider, ModelProvider::OpenCodeCli);
        assert_eq!(parsed.api.cli_path, r"C:\tools\opencode.exe");
        assert_eq!(parsed.api.model, "opencode/gpt-5");
        assert!(parsed.api.provider.is_cli());
    }

    #[test]
    fn priority_service_tier_roundtrip() {
        let mut config = AppConfig::default();
        config.api.provider = ModelProvider::CodexCli;
        config.api.service_tier = ServiceTier::Priority;

        let text = toml::to_string(&config).unwrap();
        assert!(text.contains("service_tier = \"priority\""), "got:\n{text}");

        let parsed: AppConfig = toml::from_str(&text).unwrap();
        assert_eq!(parsed.api.service_tier, ServiceTier::Priority);
    }

    #[test]
    fn resolve_cli_missing_path_is_none() {
        assert!(resolve_cli_binary(ModelProvider::OpenaiCompatible, "").is_none());
        assert!(resolve_cli_binary(ModelProvider::GrokCli, r"C:\definitely-missing\grok.exe").is_none());
        assert!(resolve_cli_binary(ModelProvider::CodexCli, r"Z:\no-such-codex.exe").is_none());
        assert!(resolve_cli_binary(ModelProvider::OpenCodeCli, r"Z:\no-such-opencode.exe").is_none());
    }

    #[test]
    fn structured_outputs_defaults_true_including_missing_toml() {
        assert!(ApiConfig::default().structured_outputs);

        let config: AppConfig = toml::from_str("[api]\nmodel = \"gpt-4o-mini\"\n").unwrap();
        assert!(config.api.structured_outputs);

        let mut config = AppConfig::default();
        config.api.structured_outputs = false;
        let text = toml::to_string_pretty(&config).unwrap();
        assert!(text.contains("structured_outputs = false"), "got:\n{text}");
        let parsed: AppConfig = toml::from_str(&text).unwrap();
        assert!(!parsed.api.structured_outputs);
    }

    #[test]
    fn stream_defaults_true_including_missing_toml() {
        assert!(ApiConfig::default().stream);

        let config: AppConfig = toml::from_str("[api]\nmodel = \"gpt-4o-mini\"\n").unwrap();
        assert!(config.api.stream);

        let mut config = AppConfig::default();
        config.api.stream = false;
        let text = toml::to_string_pretty(&config).unwrap();
        assert!(text.contains("stream = false"), "got:\n{text}");
        let parsed: AppConfig = toml::from_str(&text).unwrap();
        assert!(!parsed.api.stream);
    }

    #[test]
    fn send_reasoning_content_defaults_true_including_missing_toml() {
        assert!(ApiConfig::default().send_reasoning_content);

        let config: AppConfig = toml::from_str("[api]\nmodel = \"gpt-4o-mini\"\n").unwrap();
        assert!(config.api.send_reasoning_content);

        let mut config = AppConfig::default();
        config.api.send_reasoning_content = false;
        let text = toml::to_string_pretty(&config).unwrap();
        assert!(text.contains("send_reasoning_content = false"), "got:\n{text}");
        let parsed: AppConfig = toml::from_str(&text).unwrap();
        assert!(!parsed.api.send_reasoning_content);
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
        assert!((config.ocr.line_merge.align_ratio - LineMergeConfig::default().align_ratio).abs() < 1e-6);
        assert!(config.ocr.line_merge.join_with_space);
        assert!(config.ocr.line_merge.reject_short_long);
    }

    #[test]
    fn line_merge_max_gap_ratio_legacy_key_ignored() {
        let text = r#"
[ocr.line_merge]
max_gap_ratio = 0.02
"#;
        let config: AppConfig = toml::from_str(text).unwrap();
        assert!((config.ocr.line_merge.gap_ratio - LineMergeConfig::default().gap_ratio).abs() < 1e-6);
    }

    #[test]
    fn line_merge_legacy_overlap_keys_ignored() {
        let text = r#"
[ocr.line_merge]
overlap_ratio = 0.9
overlap_ratio_min = 0.9
align_overlap_ratio = 0.9
"#;
        let config: AppConfig = toml::from_str(text).unwrap();
        assert!(config.ocr.line_merge.enabled);
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

        let partial = r##"
[overlay]
text_color_argb = "#FFFFFFFF"
"##;
        let config: AppConfig = toml::from_str(partial).unwrap();
        assert!(config.overlay.enabled);
        assert!(config.overlay.reader_enabled);
        assert_eq!(config.overlay.reader_font_px, READER_FONT_PX_DEFAULT);
    }

    #[test]
    fn invalid_fields_are_dropped_not_whole_config() {
        let text = r##"
[api]
model = "keep-me"
provider = "not_a_real_provider"

[overlay]
enabled = false
text_color_argb = "FFFFFFFF"
background_color_argb = "#C8000000"

[translation]
target_lang = "ja"
history_max_items = "nope"
"##;
        let config = AppConfig::from_toml_lenient(text).unwrap();
        assert_eq!(config.api.model, "keep-me");
        assert_eq!(config.api.provider, ModelProvider::OpenaiCompatible);
        assert!(!config.overlay.enabled);
        assert_eq!(config.overlay.text_color_argb, OverlayConfig::default().text_color_argb);
        assert_eq!(config.overlay.background_color_argb, 0xC800_0000);
        assert_eq!(config.translation.target_lang, "ja");
        assert_eq!(config.translation.history_max_items, TranslationConfig::default().history_max_items);
    }

    #[test]
    fn nested_invalid_field_keeps_siblings() {
        let text = r#"
[ocr.line_merge]
enabled = false
gap_ratio = "bad"
join_with_space = false
"#;
        let config = AppConfig::from_toml_lenient(text).unwrap();
        assert!(!config.ocr.line_merge.enabled);
        assert!(!config.ocr.line_merge.join_with_space);
        assert!((config.ocr.line_merge.gap_ratio - LineMergeConfig::default().gap_ratio).abs() < 1e-6);
    }

    #[test]
    fn syntax_error_still_fails_lenient_parse() {
        assert!(AppConfig::from_toml_lenient("[[[not valid").is_err());
    }

    #[test]
    fn load_skips_invalid_field_and_keeps_the_rest() {
        let path = temp_config_path("lenient");
        let _ = fs::remove_file(&path);
        fs::write(&path, "[api]\nmodel = \"keep-me\"\nprovider = \"nope\"\n").unwrap();
        let config = AppConfig::load(&path).unwrap();
        assert_eq!(config.api.model, "keep-me");
        assert_eq!(config.api.provider, ModelProvider::OpenaiCompatible);
        let _ = fs::remove_file(&path);
    }
}
