//! Named OCR region presets stored next to the executable.

use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    paths::{PathError, region_presets_path},
    types::NormRect,
};

#[derive(Debug, Error)]
pub enum RegionPresetsError {
    #[error("path error: {0}")]
    Path(#[from] PathError),
    #[error("IO error for {path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("failed to parse region presets TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("failed to serialize region presets TOML: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("{0}")]
    Invalid(String),
}

/// One named set of OCR boxes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionPreset {
    pub name: String,
    pub regions: Vec<NormRect>,
}

/// Root of `region-presets.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RegionPresetFile {
    pub presets: Vec<RegionPreset>,
}

impl RegionPresetFile {
    /// Load from the default path. Missing file → empty list (does not create).
    pub fn load_or_empty() -> Result<Self, RegionPresetsError> {
        let path = region_presets_path()?;
        Self::load_or_empty_at(&path)
    }

    pub fn load_or_empty_at(path: &Path) -> Result<Self, RegionPresetsError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(path).map_err(|source| RegionPresetsError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let mut file: Self = toml::from_str(&text)?;
        file.sanitize_in_place();
        Ok(file)
    }

    pub fn save(&self, path: &Path) -> Result<(), RegionPresetsError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| RegionPresetsError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        let text = toml::to_string_pretty(self)?;
        fs::write(path, text).map_err(|source| RegionPresetsError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(())
    }

    pub fn save_default(&self) -> Result<(), RegionPresetsError> {
        let path = region_presets_path()?;
        self.save(&path)
    }

    /// Drop invalid rects; drop presets that end up with no regions or empty names.
    pub fn sanitize_in_place(&mut self) {
        self.presets.retain_mut(|p| {
            p.name = p.name.trim().to_string();
            p.regions = sanitize_regions(&p.regions);
            !p.name.is_empty() && !p.regions.is_empty()
        });
    }

    /// Insert or replace by exact name. `name` and `regions` must already be validated.
    pub fn upsert(&mut self, name: String, regions: Vec<NormRect>) {
        if let Some(i) = self.presets.iter().position(|p| p.name == name) {
            self.presets[i].regions = regions;
        } else {
            self.presets.push(RegionPreset { name, regions });
        }
    }
}

/// Sanitize and require a non-empty trimmed name plus at least one valid rect.
pub fn validate_preset(name: &str, regions: &[NormRect]) -> Result<(String, Vec<NormRect>), RegionPresetsError> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(RegionPresetsError::Invalid("Preset name is required.".into()));
    }
    let regions = sanitize_regions(regions);
    if regions.is_empty() {
        return Err(RegionPresetsError::Invalid("A preset needs at least one OCR region.".into()));
    }
    Ok((name, regions))
}

pub fn sanitize_regions(regions: &[NormRect]) -> Vec<NormRect> {
    regions.iter().copied().filter_map(NormRect::sanitize).collect()
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        std::env::temp_dir().join(format!("translator_overlay_presets_{name}_{nanos}.toml"))
    }

    fn sample_regions() -> Vec<NormRect> {
        vec![NormRect::new(0.02, 0.80, 0.40, 0.18), NormRect::new(0.70, 0.05, 0.28, 0.12)]
    }

    #[test]
    fn roundtrip_named_preset() {
        let path = temp_path("roundtrip");
        let _ = fs::remove_file(&path);
        let mut file = RegionPresetFile::default();
        file.upsert("HUD".into(), sample_regions());
        file.save(&path).unwrap();
        let loaded = RegionPresetFile::load_or_empty_at(&path).unwrap();
        assert_eq!(loaded.presets.len(), 1);
        assert_eq!(loaded.presets[0].name, "HUD");
        assert_eq!(loaded.presets[0].regions.len(), 2);
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn missing_file_is_empty_and_does_not_create() {
        let path = temp_path("missing");
        let _ = fs::remove_file(&path);
        let loaded = RegionPresetFile::load_or_empty_at(&path).unwrap();
        assert!(loaded.presets.is_empty());
        assert!(!path.exists());
    }

    #[test]
    fn case_sensitive_upsert() {
        let mut file = RegionPresetFile::default();
        file.upsert("HUD".into(), sample_regions());
        file.upsert("hud".into(), vec![NormRect::new(0.1, 0.1, 0.2, 0.2)]);
        assert_eq!(file.presets.len(), 2);
        file.upsert("HUD".into(), vec![NormRect::new(0.5, 0.5, 0.2, 0.2)]);
        assert_eq!(file.presets.len(), 2);
        assert_eq!(file.presets[0].regions.len(), 1);
        assert!((file.presets[0].regions[0].x - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn sanitize_and_validate() {
        let bad = [NormRect::new(0.0, 0.0, 0.001, 0.5), NormRect::new(0.1, 0.2, 0.3, 0.4)];
        let cleaned = sanitize_regions(&bad);
        assert_eq!(cleaned.len(), 1);
        assert!(validate_preset("  ", &cleaned).is_err());
        assert!(validate_preset("ok", &[]).is_err());
        let (name, regions) = validate_preset("  ok  ", &bad).unwrap();
        assert_eq!(name, "ok");
        assert_eq!(regions.len(), 1);
    }
}
