//! PP-OCRv6 / PP-OCRv5 ONNX artifacts (file names + GitHub release sizes/URLs).
//!
//! Missing files are fetched by the app into `models_dir` (see [`crate::download`]).
//! Load only checks that the files exist; download verifies byte length.

use std::path::{Path, PathBuf};

use translator_core::{ModelArtifact, ModelTier, OcrDevice, OcrModel, V5Det};

const OAR_V070: &str = "https://github.com/GreatV/oar-ocr/releases/download/v0.7.0";
const OAR_V030: &str = "https://github.com/GreatV/oar-ocr/releases/download/v0.3.0";

/// PP-OCRv5 mobile recognition model + dictionary for one script: `(prefix, rec bytes, dict bytes)`.
macro_rules! v5 {
    ($lang:literal, $rec:literal, $dict:literal) => {
        &[
            ModelArtifact {
                file_name: concat!($lang, "_pp-ocrv5_mobile_rec.onnx"),
                release: OAR_V030,
                expected_bytes: $rec,
            },
            ModelArtifact {
                file_name: concat!("ppocrv5_", $lang, "_dict.txt"),
                release: OAR_V030,
                expected_bytes: $dict,
            },
        ]
    };
}

/// Detection, recognition, and dictionary files, in that order. PP-OCRv6 comes in `tier` sizes;
/// a PP-OCRv5 script recognizer pairs with PP-OCRv5 detection of size `v5_det` (PaddleOCR
/// itself pairs it with server).
pub fn artifacts(tier: ModelTier, model: OcrModel, v5_det: V5Det) -> [&'static ModelArtifact; 3] {
    let [det, rec, dict] = match tier {
        ModelTier::Tiny => &TINY,
        ModelTier::Small => &SMALL,
        ModelTier::Medium => &MEDIUM,
    };
    let [rec, dict]: &'static [ModelArtifact; 2] = match model {
        OcrModel::V6 => return [det, rec, dict],
        OcrModel::V5English => v5!("en", 7_876_014, 1_416),
        OcrModel::V5Latin => v5!("latin", 8_069_614, 2_616),
        OcrModel::V5Korean => v5!("korean", 13_446_374, 47_451),
        OcrModel::V5Cyrillic => v5!("cyrillic", 8_076_390, 2_781),
        OcrModel::V5EastSlavic => v5!("eslav", 7_915_218, 1_663),
        OcrModel::V5Greek => v5!("el", 7_836_326, 1_103),
        OcrModel::V5Devanagari => v5!("devanagari", 7_939_902, 1_943),
        OcrModel::V5Tamil => v5!("ta", 7_913_282, 1_723),
        OcrModel::V5Telugu => v5!("te", 7_926_350, 1_831),
        OcrModel::V5Thai => v5!("th", 7_918_606, 1_767),
    };
    let det = match v5_det {
        V5Det::Mobile => &V5_MOBILE_DET,
        V5Det::Server => &V5_SERVER_DET,
    };
    [det, rec, dict]
}

const V5_MOBILE_DET: ModelArtifact = ModelArtifact {
    file_name: "pp-ocrv5_mobile_det.onnx",
    release: OAR_V030,
    expected_bytes: 4_826_518,
};

const V5_SERVER_DET: ModelArtifact = ModelArtifact {
    file_name: "pp-ocrv5_server_det.onnx",
    release: OAR_V030,
    expected_bytes: 88_116_836,
};

/// Peak `(VRAM, RAM)` bytes with the layout model if `layout`, for a frame whose long side is
/// `det_max_side` (0 = no shrink, costed as a 1920-px frame; a larger window takes more).
/// GPU devices also take a flat runtime allowance in RAM.
pub fn memory_estimate(tier: ModelTier, model: OcrModel, v5_det: V5Det, device: OcrDevice, layout: bool, det_max_side: u32) -> (u64, u64) {
    const MIB: u64 = 1 << 20;
    const RUNTIME_MIB: u64 = 256;
    // `(GPU, CPU)` MiB per model, from its ONNX graph at the pipeline's input size (det 1×3×544×960
    // for a 960-px long side, rec 4×3×48×3200 at the width cap, layout 1×3×800×800):
    // - GPU: initializers (ORT WebGPU rounds each up to 16 bytes, outside the bucket cache), plus
    //   the activation buffers ORT's allocation planner assigns, rounded up to their buckets (an
    //   output reuses a freed buffer of the same size);
    // - CPU: initializers plus the largest set of activations alive at once, which the CPU arena
    //   reuses down to.
    let (det_weights, det, rec) = match (model, tier) {
        (OcrModel::V6, ModelTier::Tiny) => (2, (86, 34), (212, 89)),
        (OcrModel::V6, ModelTier::Small) => (9, (119, 57), (498, 249)),
        (OcrModel::V6, ModelTier::Medium) => (59, (358, 187), (1036, 335)),
        _ => {
            let (det_weights, det) = match v5_det {
                V5Det::Mobile => (5, (183, 61)),
                V5Det::Server => (84, (878, 343)),
            };
            // Korean's output layer covers a far larger dictionary than the other scripts'.
            let rec = if model == OcrModel::V5Korean { (583, 159) } else { (443, 120) };
            (det_weights, det, rec)
        }
    };
    // Detection activations (everything but the weights) grow with the input area.
    let side = u64::from(if det_max_side == 0 { 1920 } else { det_max_side });
    let grow = |mib: u64| det_weights + (mib - det_weights) * side * side / (960 * 960);
    let layout_model = (990, 380);
    let models = [(grow(det.0), grow(det.1)), rec].into_iter().chain(layout.then_some(layout_model));
    let (gpu, cpu) = models.fold((0, 0), |(g, c), (mg, mc)| (g + mg, c + mc));
    match device {
        OcrDevice::Webgpu | OcrDevice::Directml => (gpu * MIB, RUNTIME_MIB * MIB),
        OcrDevice::Cpu => (0, cpu * MIB),
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
    pub fn from_dir(models_dir: &Path, tier: ModelTier, model: OcrModel, v5_det: V5Det) -> Self {
        let [det, rec, dict] = artifacts(tier, model, v5_det).map(|a| models_dir.join(a.file_name));
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
    fn detection_memory_grows_with_input_side() {
        let at = |side| memory_estimate(ModelTier::Small, OcrModel::V6, V5Det::Mobile, OcrDevice::Webgpu, false, side).0 >> 20;
        assert_eq!(at(960), 119 + 498, "the table is measured at a 960-px long side");
        assert_eq!(at(1920), 9 + 110 * 4 + 498);
        assert_eq!(at(0), at(1920));
    }

    #[test]
    fn v5_model_pairs_with_v5_detection_at_any_tier() {
        let [det, rec, dict] = artifacts(ModelTier::Tiny, OcrModel::V5Korean, V5Det::Server);
        assert_eq!(det.download_url(), format!("{OAR_V030}/pp-ocrv5_server_det.onnx"));
        assert_eq!(rec.download_url(), format!("{OAR_V030}/korean_pp-ocrv5_mobile_rec.onnx"));
        assert_eq!(dict.download_url(), format!("{OAR_V030}/ppocrv5_korean_dict.txt"));
    }

    #[test]
    fn medium_shares_dict_with_small() {
        let small = ModelPaths::from_dir(Path::new("models"), ModelTier::Small, OcrModel::V6, V5Det::Mobile);
        let medium = ModelPaths::from_dir(Path::new("models"), ModelTier::Medium, OcrModel::V6, V5Det::Mobile);
        assert_eq!(small.dict.file_name(), medium.dict.file_name(), "small/medium share ppocrv6_dict.txt");
        assert_eq!(
            artifacts(ModelTier::Small, OcrModel::V6, V5Det::Mobile)[2].expected_bytes,
            artifacts(ModelTier::Medium, OcrModel::V6, V5Det::Mobile)[2].expected_bytes
        );
    }

    #[test]
    fn all_tiers_list_det_rec_dict_in_order() {
        for tier in [ModelTier::Tiny, ModelTier::Small, ModelTier::Medium] {
            let [det, rec, dict] = artifacts(tier, OcrModel::V6, V5Det::Mobile);
            assert!(det.file_name.ends_with("_det.onnx"), "{tier:?} det");
            assert!(rec.file_name.ends_with("_rec.onnx"), "{tier:?} rec");
            assert!(dict.file_name.ends_with(".txt"), "{tier:?} dict");
            for a in artifacts(tier, OcrModel::V6, V5Det::Mobile) {
                assert!(a.expected_bytes > 0, "{tier:?} {}", a.file_name);
                assert!(a.download_url().starts_with(OAR_V070));
            }
            let paths = ModelPaths::from_dir(Path::new("m"), tier, OcrModel::V6, V5Det::Mobile);
            assert_eq!(paths.det, Path::new("m").join(det.file_name));
            assert_eq!(paths.dict, Path::new("m").join(dict.file_name));
        }
    }

    #[test]
    fn missing_file_is_not_present() {
        let paths = ModelPaths::from_dir(Path::new("definitely-missing-models-dir"), ModelTier::Tiny, OcrModel::V6, V5Det::Mobile);
        assert!(!paths.all_present());
    }

    #[test]
    fn existing_files_count_as_present_regardless_of_size() {
        let dir = std::env::temp_dir().join(format!("translator-ocr-present-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let paths = ModelPaths::from_dir(&dir, ModelTier::Tiny, OcrModel::V6, V5Det::Mobile);
        for p in [&paths.det, &paths.rec, &paths.dict] {
            std::fs::write(p, b"x").unwrap();
        }
        assert!(paths.all_present());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
