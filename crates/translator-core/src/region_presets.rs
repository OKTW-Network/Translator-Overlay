//! Named OCR region presets stored next to the executable.

use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    toml_file::{TomlFileError, load_toml_or_empty, write_toml_str},
    types::NormRect,
};

#[derive(Debug, Error)]
pub enum RegionPresetsError {
    #[error(transparent)]
    Toml(#[from] TomlFileError),
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
    pub fn load_or_empty_at(path: &Path) -> Result<Self, RegionPresetsError> {
        let mut file: Self = load_toml_or_empty(path)?;
        file.sanitize_in_place();
        Ok(file)
    }

    pub fn save(&self, path: &Path) -> Result<(), RegionPresetsError> {
        let mut doc = toml_edit::ser::to_document(self).map_err(TomlFileError::from)?;
        // `to_string` keeps every preset on one line. A standard table puts `name` and `regions` on their own lines.
        if let Some(slot) = doc.get_mut("presets") {
            let tables = slot.as_array().filter(|array| !array.is_empty()).map(|array| {
                array
                    .iter()
                    .filter_map(toml_edit::Value::as_inline_table)
                    .map(|preset| preset.clone().into_table())
                    .collect()
            });
            if let Some(tables) = tables {
                *slot = toml_edit::Item::ArrayOfTables(tables);
            }
        }
        Ok(write_toml_str(path, &doc.to_string())?)
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
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

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
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("[[presets]]\n"), "{text}");
        assert!(text.contains("name = \"HUD\"\n"), "{text}");
        assert!(text.contains("x = 0.02"), "{text}");
        assert!(!text.contains("[[presets.regions]]"), "{text}");
        let loaded = RegionPresetFile::load_or_empty_at(&path).unwrap();
        assert_eq!(loaded.presets.len(), 1);
        assert_eq!(loaded.presets[0].name, "HUD");
        assert_eq!(loaded.presets[0].regions, sanitize_regions(&sample_regions()));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn legacy_array_of_tables_still_loads() {
        let path = temp_path("legacy");
        let _ = fs::remove_file(&path);
        fs::write(
            &path,
            r#"
[[presets]]
name = "HUD"

[[presets.regions]]
x = 0.02
y = 0.8
width = 0.4
height = 0.18

[[presets.regions]]
x = 0.7
y = 0.05
width = 0.28
height = 0.12
"#,
        )
        .unwrap();
        let loaded = RegionPresetFile::load_or_empty_at(&path).unwrap();
        assert_eq!(loaded.presets.len(), 1);
        assert_eq!(loaded.presets[0].name, "HUD");
        assert_eq!(loaded.presets[0].regions.len(), 2);
        let region = loaded.presets[0].regions[1];
        assert!((region.x - 0.7).abs() < 1e-5, "{region:?}");
        assert!((region.y - 0.05).abs() < 1e-5, "{region:?}");
        assert!((region.width - 0.28).abs() < 1e-5, "{region:?}");
        assert!((region.height - 0.12).abs() < 1e-5, "{region:?}");
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn empty_save_writes_empty_array() {
        let path = temp_path("empty");
        let _ = fs::remove_file(&path);
        RegionPresetFile::default().save(&path).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "presets = []\n");
        let loaded = RegionPresetFile::load_or_empty_at(&path).unwrap();
        assert!(loaded.presets.is_empty());
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
