//! Named API connection profiles stored next to the executable.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::{
    config::ApiConfig,
    toml_file::{TomlFileError, load_toml_or_empty, save_toml},
};

/// One named copy of [`ApiConfig`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiProfile {
    pub name: String,
    #[serde(flatten)]
    pub api: ApiConfig,
}

/// Root of `api-profiles.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ApiProfileFile {
    pub profiles: Vec<ApiProfile>,
}

impl ApiProfileFile {
    pub fn load_or_empty_at(path: &Path) -> Result<Self, TomlFileError> {
        let mut file: Self = load_toml_or_empty(path)?;
        file.sanitize_in_place();
        Ok(file)
    }

    pub fn save(&self, path: &Path) -> Result<(), TomlFileError> {
        save_toml(path, self)
    }

    /// Trim names and drop profiles whose name is empty after trim.
    pub fn sanitize_in_place(&mut self) {
        self.profiles.retain_mut(|p| {
            p.name = p.name.trim().to_string();
            !p.name.is_empty()
        });
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::*;
    use crate::config::{HttpApi, ModelProvider, ServiceTier};

    fn temp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("translator_overlay_api_profiles_{name}_{nanos}.toml"))
    }

    fn sample_http() -> ApiConfig {
        ApiConfig {
            provider: ModelProvider::OpenaiCompatible,
            http_api: HttpApi::Responses,
            base_url: "https://api.openai.com/v1".into(),
            api_key: "sk-test".into(),
            model: "gpt-4o-mini".into(),
            temperature: Some(0.4),
            max_tokens: Some(1024),
            ..ApiConfig::default()
        }
    }

    fn sample_cli() -> ApiConfig {
        ApiConfig {
            provider: ModelProvider::GrokCli,
            cli_path: r"C:\tools\grok.exe".into(),
            model: "grok-4.5".into(),
            service_tier: ServiceTier::Standard,
            reasoning_effort: Some("high".into()),
            ..ApiConfig::default()
        }
    }

    #[test]
    fn flatten_roundtrip_http_and_cli() {
        let path = temp_path("roundtrip");
        let _ = fs::remove_file(&path);
        let file = ApiProfileFile {
            profiles: vec![
                ApiProfile {
                    name: "OpenAI".into(),
                    api: sample_http(),
                },
                ApiProfile {
                    name: "Grok".into(),
                    api: sample_cli(),
                },
            ],
        };
        file.save(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("name = \"OpenAI\""), "got:\n{text}");
        assert!(text.contains("provider = \"openai_compatible\""), "got:\n{text}");
        assert!(text.contains("api_key = \"sk-test\""), "got:\n{text}");
        assert!(text.contains("cli_path"), "got:\n{text}");
        assert!(!text.contains("[profiles.api]"), "flatten should not nest api:\n{text}");

        let loaded = ApiProfileFile::load_or_empty_at(&path).unwrap();
        assert_eq!(loaded.profiles.len(), 2);
        assert_eq!(loaded.profiles[0].name, "OpenAI");
        assert_eq!(loaded.profiles[0].api, sample_http());
        assert_eq!(loaded.profiles[1].name, "Grok");
        assert_eq!(loaded.profiles[1].api, sample_cli());
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn missing_file_is_empty_and_does_not_create() {
        let path = temp_path("missing");
        let _ = fs::remove_file(&path);
        let loaded = ApiProfileFile::load_or_empty_at(&path).unwrap();
        assert!(loaded.profiles.is_empty());
        assert!(!path.exists());
    }

    #[test]
    fn sanitize_drops_empty_names() {
        let mut file = ApiProfileFile {
            profiles: vec![
                ApiProfile {
                    name: "  ".into(),
                    api: sample_http(),
                },
                ApiProfile {
                    name: "  keep  ".into(),
                    api: sample_cli(),
                },
            ],
        };
        file.sanitize_in_place();
        assert_eq!(file.profiles.len(), 1);
        assert_eq!(file.profiles[0].name, "keep");
    }
}
