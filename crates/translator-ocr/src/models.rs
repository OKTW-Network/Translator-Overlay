//! PP-OCRv6 ONNX artifacts (file names + GitHub release sizes/URLs).
//!
//! Missing files are fetched by the app into `models_dir` (see [`crate::download`]).
//! Load only checks that the files exist; download verifies byte length.

use std::path::{Path, PathBuf};

use translator_core::ModelTier;

const OAR_V070: &str = "https://github.com/GreatV/oar-ocr/releases/download/v0.7.0";

/// One file required for a given model tier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelArtifact {
    pub file_name: &'static str,
    /// GitHub release download prefix that hosts the file.
    pub release: &'static str,
    /// Exact byte length from that GitHub release.
    pub expected_bytes: u64,
}

impl ModelArtifact {
    pub fn download_url(&self) -> String {
        format!("{}/{}", self.release, self.file_name)
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
        release: OAR_V070,
        expected_bytes: 1_780_590,
    },
    ModelArtifact {
        file_name: "pp-ocrv6_tiny_rec.onnx",
        release: OAR_V070,
        expected_bytes: 4_462_639,
    },
    ModelArtifact {
        file_name: "ppocrv6_tiny_dict.txt",
        release: OAR_V070,
        expected_bytes: 27_156,
    },
];

const SMALL: [ModelArtifact; 3] = [
    ModelArtifact {
        file_name: "pp-ocrv6_small_det.onnx",
        release: OAR_V070,
        expected_bytes: 9_880_512,
    },
    ModelArtifact {
        file_name: "pp-ocrv6_small_rec.onnx",
        release: OAR_V070,
        expected_bytes: 21_159_378,
    },
    ModelArtifact {
        file_name: "ppocrv6_dict.txt",
        release: OAR_V070,
        expected_bytes: 74_947,
    },
];

const MEDIUM: [ModelArtifact; 3] = [
    ModelArtifact {
        file_name: "pp-ocrv6_medium_det.onnx",
        release: OAR_V070,
        expected_bytes: 62_032_837,
    },
    ModelArtifact {
        file_name: "pp-ocrv6_medium_rec.onnx",
        release: OAR_V070,
        expected_bytes: 76_554_979,
    },
    // Small and medium recognition share the same character dictionary.
    ModelArtifact {
        file_name: "ppocrv6_dict.txt",
        release: OAR_V070,
        expected_bytes: 74_947,
    },
];

/// PP-DocLayoutV3 for [`translator_core::LineMergeMode::Layout`]: the official
/// Hugging Face ONNX run through `scripts/optimize-doclayout.py` (fp16, box output only).
pub const LAYOUT_ARTIFACT: ModelArtifact = ModelArtifact {
    file_name: "pp-doclayoutv3_fp16.onnx",
    release: "https://github.com/OKTW-Network/Translator-Overlay/releases/download/models-v1",
    expected_bytes: 65_090_091,
};

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
                assert!(a.download_url().starts_with(OAR_V070));
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
