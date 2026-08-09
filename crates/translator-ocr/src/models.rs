//! PP-OCRv6 ONNX registry names for `oar-ocr` (GreatV/oar-ocr).
//!
//! File names match the oar-ocr model registry. Missing files are fetched by
//! oar-ocr `auto-download` into `$OAR_HOME` (app sets this to `models_dir`).

use std::path::{Path, PathBuf};

use translator_core::ModelTier;

/// One file required for a given model tier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelArtifact {
    pub role: ModelRole,
    pub file_name: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelRole {
    Detection,
    Recognition,
    Dictionary,
}

/// Artifacts required for the given PP-OCRv6 tier.
pub fn artifacts_for_tier(tier: ModelTier) -> &'static [ModelArtifact] {
    match tier {
        ModelTier::Tiny => TINY,
        ModelTier::Small => SMALL,
        ModelTier::Medium => MEDIUM,
    }
}

/// Bare registry file names for a tier (det, rec, dict).
pub fn registry_names(tier: ModelTier) -> (&'static str, &'static str, &'static str) {
    let arts = artifacts_for_tier(tier);
    let mut det = "";
    let mut rec = "";
    let mut dict = "";
    for a in arts {
        match a.role {
            ModelRole::Detection => det = a.file_name,
            ModelRole::Recognition => rec = a.file_name,
            ModelRole::Dictionary => dict = a.file_name,
        }
    }
    (det, rec, dict)
}

const TINY: &[ModelArtifact] = &[
    ModelArtifact {
        role: ModelRole::Detection,
        file_name: "pp-ocrv6_tiny_det.onnx",
    },
    ModelArtifact {
        role: ModelRole::Recognition,
        file_name: "pp-ocrv6_tiny_rec.onnx",
    },
    ModelArtifact {
        role: ModelRole::Dictionary,
        file_name: "ppocrv6_tiny_dict.txt",
    },
];

const SMALL: &[ModelArtifact] = &[
    ModelArtifact {
        role: ModelRole::Detection,
        file_name: "pp-ocrv6_small_det.onnx",
    },
    ModelArtifact {
        role: ModelRole::Recognition,
        file_name: "pp-ocrv6_small_rec.onnx",
    },
    ModelArtifact {
        role: ModelRole::Dictionary,
        file_name: "ppocrv6_dict.txt",
    },
];

const MEDIUM: &[ModelArtifact] = &[
    ModelArtifact {
        role: ModelRole::Detection,
        file_name: "pp-ocrv6_medium_det.onnx",
    },
    ModelArtifact {
        role: ModelRole::Recognition,
        file_name: "pp-ocrv6_medium_rec.onnx",
    },
    ModelArtifact {
        role: ModelRole::Dictionary,
        // Small and medium recognition share the same character dictionary.
        file_name: "ppocrv6_dict.txt",
    },
];

/// Resolved local paths for a tier under `models_dir`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPaths {
    pub det: PathBuf,
    pub rec: PathBuf,
    pub dict: PathBuf,
}

impl ModelPaths {
    pub fn from_dir(models_dir: &Path, tier: ModelTier) -> Self {
        let arts = artifacts_for_tier(tier);
        let mut det = PathBuf::new();
        let mut rec = PathBuf::new();
        let mut dict = PathBuf::new();
        for a in arts {
            let p = models_dir.join(a.file_name);
            match a.role {
                ModelRole::Detection => det = p,
                ModelRole::Recognition => rec = p,
                ModelRole::Dictionary => dict = p,
            }
        }
        Self { det, rec, dict }
    }

    pub fn all_present(&self) -> bool {
        self.det.is_file() && self.rec.is_file() && self.dict.is_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_has_three_artifacts() {
        assert_eq!(artifacts_for_tier(ModelTier::Small).len(), 3);
    }

    #[test]
    fn paths_join_file_names() {
        let paths = ModelPaths::from_dir(Path::new("models"), ModelTier::Small);
        assert!(paths.det.ends_with("pp-ocrv6_small_det.onnx"));
        assert!(paths.rec.ends_with("pp-ocrv6_small_rec.onnx"));
        assert!(paths.dict.ends_with("ppocrv6_dict.txt"));
    }

    #[test]
    fn medium_shares_dict_with_small() {
        let small = ModelPaths::from_dir(Path::new("models"), ModelTier::Small);
        let medium = ModelPaths::from_dir(Path::new("models"), ModelTier::Medium);
        assert_eq!(small.dict.file_name(), medium.dict.file_name(), "small/medium share ppocrv6_dict.txt");
    }

    #[test]
    fn all_tiers_have_registry_names() {
        for tier in [ModelTier::Tiny, ModelTier::Small, ModelTier::Medium] {
            assert_eq!(artifacts_for_tier(tier).len(), 3, "{tier}");
            let (det, rec, dict) = registry_names(tier);
            assert!(det.ends_with(".onnx"), "{tier} det");
            assert!(rec.ends_with(".onnx"), "{tier} rec");
            assert!(dict.ends_with(".txt"), "{tier} dict");
        }
    }
}
