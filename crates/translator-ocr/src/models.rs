//! PP-OCRv6 ONNX artifacts (file names + GitHub release sizes/URLs).
//!
//! Missing or wrong-sized files are fetched by the app into `models_dir`
//! (see [`crate::download`]). `oar-ocr` loads only local absolute paths.

use std::path::{Path, PathBuf};

use translator_core::ModelTier;

const RELEASE_BASE: &str = "https://github.com/GreatV/oar-ocr/releases/download/v0.7.0";

/// One file required for a given model tier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelArtifact {
    pub role: ModelRole,
    pub file_name: &'static str,
    /// Exact byte length from the GreatV/oar-ocr v0.7.0 GitHub release.
    pub expected_bytes: u64,
}

impl ModelArtifact {
    pub fn download_url(&self) -> String {
        format!("{RELEASE_BASE}/{}", self.file_name)
    }
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
        expected_bytes: 1_780_590,
    },
    ModelArtifact {
        role: ModelRole::Recognition,
        file_name: "pp-ocrv6_tiny_rec.onnx",
        expected_bytes: 4_462_639,
    },
    ModelArtifact {
        role: ModelRole::Dictionary,
        file_name: "ppocrv6_tiny_dict.txt",
        expected_bytes: 27_156,
    },
];

const SMALL: &[ModelArtifact] = &[
    ModelArtifact {
        role: ModelRole::Detection,
        file_name: "pp-ocrv6_small_det.onnx",
        expected_bytes: 9_880_512,
    },
    ModelArtifact {
        role: ModelRole::Recognition,
        file_name: "pp-ocrv6_small_rec.onnx",
        expected_bytes: 21_159_378,
    },
    ModelArtifact {
        role: ModelRole::Dictionary,
        file_name: "ppocrv6_dict.txt",
        expected_bytes: 74_947,
    },
];

const MEDIUM: &[ModelArtifact] = &[
    ModelArtifact {
        role: ModelRole::Detection,
        file_name: "pp-ocrv6_medium_det.onnx",
        expected_bytes: 62_032_837,
    },
    ModelArtifact {
        role: ModelRole::Recognition,
        file_name: "pp-ocrv6_medium_rec.onnx",
        expected_bytes: 76_554_979,
    },
    ModelArtifact {
        role: ModelRole::Dictionary,
        // Small and medium recognition share the same character dictionary.
        file_name: "ppocrv6_dict.txt",
        expected_bytes: 74_947,
    },
];

/// Resolved local paths for a tier under `models_dir`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPaths {
    pub tier: ModelTier,
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
        Self { tier, det, rec, dict }
    }

    /// True when every artifact exists and matches its expected byte length.
    pub fn all_present(&self) -> bool {
        for a in artifacts_for_tier(self.tier) {
            let path = match a.role {
                ModelRole::Detection => &self.det,
                ModelRole::Recognition => &self.rec,
                ModelRole::Dictionary => &self.dict,
            };
            if !file_has_expected_size(path, a.expected_bytes) {
                return false;
            }
        }
        true
    }
}

/// Whether `path` is a regular file whose length equals `expected_bytes`.
pub fn file_has_expected_size(path: &Path, expected_bytes: u64) -> bool {
    match std::fs::metadata(path) {
        Ok(meta) => meta.is_file() && meta.len() == expected_bytes,
        Err(_) => false,
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
        assert_eq!(artifacts_for_tier(ModelTier::Small)[2].expected_bytes, artifacts_for_tier(ModelTier::Medium)[2].expected_bytes);
    }

    #[test]
    fn all_tiers_have_registry_names_and_sizes() {
        for tier in [ModelTier::Tiny, ModelTier::Small, ModelTier::Medium] {
            assert_eq!(artifacts_for_tier(tier).len(), 3, "{tier}");
            let (det, rec, dict) = registry_names(tier);
            assert!(det.ends_with(".onnx"), "{tier} det");
            assert!(rec.ends_with(".onnx"), "{tier} rec");
            assert!(dict.ends_with(".txt"), "{tier} dict");
            for a in artifacts_for_tier(tier) {
                assert!(a.expected_bytes > 0, "{tier} {}", a.file_name);
                assert!(a.download_url().starts_with(RELEASE_BASE));
            }
        }
    }

    #[test]
    fn missing_file_is_not_present() {
        let paths = ModelPaths::from_dir(Path::new("definitely-missing-models-dir"), ModelTier::Tiny);
        assert!(!paths.all_present());
    }
}
