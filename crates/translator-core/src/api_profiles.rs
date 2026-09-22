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

    pub fn save(&mut self, path: &Path) -> Result<(), TomlFileError> {
        self.sanitize_in_place();
        save_toml(path, self)
    }

    /// Trim names, drop profiles whose name is empty after trim, and clear fields the provider does not use.
    pub fn sanitize_in_place(&mut self) {
        self.profiles.retain_mut(|p| {
            p.name = p.name.trim().to_string();
            !p.name.is_empty()
        });
        for profile in &mut self.profiles {
            profile.api.blank_inapplicable();
        }
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
        let mut http = sample_http();
        http.cli_path = r"C:\tools\leftover.exe".into();
        http.service_tier = ServiceTier::Priority;
        let mut cli = sample_cli();
        cli.base_url = "https://should-not-stick.example/v1".into();
        cli.api_key = "sk-nope".into();
        cli.http_api = HttpApi::Responses;
        cli.structured_outputs = false;
        cli.stream = false;
        cli.send_reasoning_content = false;
        cli.temperature = Some(0.2);
        let mut file = ApiProfileFile {
            profiles: vec![
                ApiProfile {
                    name: "OpenAI".into(),
                    api: http,
                },
                ApiProfile {
                    name: "Grok".into(),
                    api: cli,
                },
                ApiProfile {
                    name: "Codex".into(),
                    api: ApiConfig {
                        provider: ModelProvider::CodexCli,
                        cli_path: r"C:\tools\codex.exe".into(),
                        model: "gpt-5.6".into(),
                        service_tier: ServiceTier::Priority,
                        base_url: "https://should-not-stick.example/v1".into(),
                        ..ApiConfig::default()
                    },
                },
            ],
        };
        file.save(&path).unwrap();

        let text = fs::read_to_string(&path).unwrap();
        let mut sections = text.split("[[profiles]]").skip(1);
        let http_section = sections.next().expect("http profile");
        let cli_section = sections.next().expect("cli profile");
        let codex_section = sections.next().expect("codex profile");
        assert!(http_section.contains("name = \"OpenAI\""), "{http_section}");
        assert!(http_section.contains("provider = \"openai_compatible\""), "{http_section}");
        assert!(http_section.contains("base_url = \"https://api.openai.com/v1\""), "{http_section}");
        assert!(http_section.contains("api_key = \"sk-test\""), "{http_section}");
        assert!(!http_section.contains("cli_path"), "{http_section}");
        assert!(!http_section.contains("service_tier"), "{http_section}");
        assert!(cli_section.contains("cli_path"), "{cli_section}");
        assert!(cli_section.contains("temperature"), "{cli_section}");
        for key in [
            "base_url",
            "api_key",
            "http_api",
            "structured_outputs",
            "stream",
            "send_reasoning_content",
        ] {
            assert!(!cli_section.contains(key), "{key}");
        }
        assert!(codex_section.contains("service_tier = \"priority\""), "{codex_section}");
        assert!(!codex_section.contains("base_url"), "{codex_section}");
        assert!(!text.contains("[profiles.api]"), "flatten should not nest api:\n{text}");

        let loaded = ApiProfileFile::load_or_empty_at(&path).unwrap();
        assert_eq!(loaded.profiles.len(), 3);
        assert_eq!(loaded.profiles[0].name, "OpenAI");
        assert_eq!(loaded.profiles[0].api, sample_http());
        let mut expected_cli = sample_cli();
        expected_cli.blank_inapplicable();
        expected_cli.temperature = Some(0.2);
        assert_eq!(loaded.profiles[1].name, "Grok");
        assert_eq!(loaded.profiles[1].api, expected_cli);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn load_blanks_legacy_cli_url_and_key() {
        let path = temp_path("legacy");
        let _ = fs::remove_file(&path);
        fs::write(
            &path,
            r#"
[[profiles]]
name = "Grok"
provider = "grok_cli"
base_url = "https://old.example/v1"
api_key = "sk-old"
model = "grok-4.5"
cli_path = "C:\\tools\\grok.exe"
"#,
        )
        .unwrap();

        let loaded = ApiProfileFile::load_or_empty_at(&path).unwrap();
        assert_eq!(loaded.profiles.len(), 1);
        assert!(loaded.profiles[0].api.base_url.is_empty());
        assert!(loaded.profiles[0].api.api_key.is_empty());
        assert_eq!(loaded.profiles[0].api.model, "grok-4.5");
        assert_eq!(loaded.profiles[0].api.cli_path, r"C:\tools\grok.exe");
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
