//! PP-OCRv6 ONNX artifacts, with their file names and GitHub release sizes and URLs.
//!
//! The app fetches missing files into `models_dir` (see [`crate::download`]).
//! Loading checks only that the files exist. The download checks the byte length.

use std::path::{Path, PathBuf};

use translator_core::ModelTier;

const RELEASE_BASE: &str = "https://github.com/GreatV/oar-ocr/releases/download/v0.7.0";

/// One file required for a given model tier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelArtifact {
    pub file_name: &'static str,
    /// Exact byte length from the GreatV/oar-ocr v0.7.0 GitHub release.
    pub expected_bytes: u64,
}

impl ModelArtifact {
    pub fn download_url(&self) -> String {
        format!("{RELEASE_BASE}/{}", self.file_name)
    }
}

/// Detection, recognition, and dictionary files for `tier`, in that order.
pub fn artifacts_for_tier(tier: ModelTier) -> &'static [ModelArtifact; 3] {
    match tier {
        ModelTier::Tiny => &TINY,
        ModelTier::Small => &SMALL,
        ModelTier::Medium => &MEDIUM,
    }
}

const TINY: [ModelArtifact; 3] = [
    ModelArtifact {
        file_name: "pp-ocrv6_tiny_det.onnx",
        expected_bytes: 1_780_590,
    },
    ModelArtifact {
        file_name: "pp-ocrv6_tiny_rec.onnx",
        expected_bytes: 4_462_639,
    },
    ModelArtifact {
        file_name: "ppocrv6_tiny_dict.txt",
        expected_bytes: 27_156,
    },
];

const SMALL: [ModelArtifact; 3] = [
    ModelArtifact {
        file_name: "pp-ocrv6_small_det.onnx",
        expected_bytes: 9_880_512,
    },
    ModelArtifact {
        file_name: "pp-ocrv6_small_rec.onnx",
        expected_bytes: 21_159_378,
    },
    ModelArtifact {
        file_name: "ppocrv6_dict.txt",
        expected_bytes: 74_947,
    },
];

const MEDIUM: [ModelArtifact; 3] = [
    ModelArtifact {
        file_name: "pp-ocrv6_medium_det.onnx",
        expected_bytes: 62_032_837,
    },
    ModelArtifact {
        file_name: "pp-ocrv6_medium_rec.onnx",
        expected_bytes: 76_554_979,
    },
    // Small and medium recognition share the same character dictionary.
    ModelArtifact {
        file_name: "ppocrv6_dict.txt",
        expected_bytes: 74_947,
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
        let [det, rec, dict] = artifacts_for_tier(tier).each_ref().map(|a| models_dir.join(a.file_name));
        Self { det, rec, dict }
    }

    /// True when every artifact exists as a regular file. Size is not checked.
    pub fn all_present(&self) -> bool {
        self.det.is_file() && self.rec.is_file() && self.dict.is_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medium_shares_dict_with_small() {
        let small = ModelPaths::from_dir(Path::new("models"), ModelTier::Small);
        let medium = ModelPaths::from_dir(Path::new("models"), ModelTier::Medium);
        assert_eq!(small.dict.file_name(), medium.dict.file_name(), "small/medium share ppocrv6_dict.txt");
        assert_eq!(artifacts_for_tier(ModelTier::Small)[2].expected_bytes, artifacts_for_tier(ModelTier::Medium)[2].expected_bytes);
    }

    #[test]
    fn all_tiers_list_det_rec_dict_in_order() {
        for tier in [ModelTier::Tiny, ModelTier::Small, ModelTier::Medium] {
            let [det, rec, dict] = artifacts_for_tier(tier);
            assert!(det.file_name.ends_with("_det.onnx"), "{tier:?} det");
            assert!(rec.file_name.ends_with("_rec.onnx"), "{tier:?} rec");
            assert!(dict.file_name.ends_with(".txt"), "{tier:?} dict");
            for a in artifacts_for_tier(tier) {
                assert!(a.expected_bytes > 0, "{tier:?} {}", a.file_name);
                assert!(a.download_url().starts_with(RELEASE_BASE));
            }
            let paths = ModelPaths::from_dir(Path::new("m"), tier);
            assert_eq!(paths.det, Path::new("m").join(det.file_name));
            assert_eq!(paths.dict, Path::new("m").join(dict.file_name));
        }
    }

    #[test]
    fn missing_file_is_not_present() {
        let paths = ModelPaths::from_dir(Path::new("definitely-missing-models-dir"), ModelTier::Tiny);
        assert!(!paths.all_present());
    }

    #[test]
    fn existing_files_count_as_present_regardless_of_size() {
        let dir = std::env::temp_dir().join(format!("translator-ocr-present-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let paths = ModelPaths::from_dir(&dir, ModelTier::Tiny);
        for p in [&paths.det, &paths.rec, &paths.dict] {
            std::fs::write(p, b"x").unwrap();
        }
        assert!(paths.all_present());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
