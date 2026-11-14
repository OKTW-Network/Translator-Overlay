//! PP-OCRv6 / PP-OCRv5 ONNX artifacts (file names + Hugging Face sizes/URLs).
//!
//! Missing files are fetched by the app into `models_dir` (see [`crate::download`]).
//! Load only checks that the files exist; download verifies byte length.

use std::path::{Path, PathBuf};

use translator_core::{ModelArtifact, ModelTier, OcrDevice, OcrModel, V5Det};

/// `inference.<ext>` of the official `PaddlePaddle/<repo>_onnx` Hugging Face repo at commit
/// `rev`, saved as `<repo>.<ext>`.
macro_rules! hf {
    ($repo:literal, $rev:literal, $ext:literal, $bytes:literal) => {
        ModelArtifact {
            file_name: concat!($repo, ".", $ext),
            release: concat!("https://huggingface.co/PaddlePaddle/", $repo, "_onnx/resolve/", $rev),
            remote_name: concat!("inference.", $ext),
            expected_bytes: $bytes,
        }
    };
}

/// A recognition model and the `inference.yml` that holds its dictionary:
/// `(repo, rev, onnx bytes, yml bytes)`.
macro_rules! rec {
    ($repo:literal, $rev:literal, $onnx:literal, $yml:literal) => {
        &[hf!($repo, $rev, "onnx", $onnx), hf!($repo, $rev, "yml", $yml)]
    };
}

/// Detection, recognition, and dictionary files, in that order. PP-OCRv6 comes in `tier` sizes;
/// a PP-OCRv5 script recognizer pairs with PP-OCRv5 detection of size `v5_det` (PaddleOCR
/// itself pairs it with server).
pub fn artifacts(tier: ModelTier, model: OcrModel, v5_det: V5Det) -> [&'static ModelArtifact; 3] {
    let det: &'static ModelArtifact = match (model, tier, v5_det) {
        (OcrModel::V6, ModelTier::Tiny, _) => &hf!("PP-OCRv6_tiny_det", "2ba1506c0380b8f0b03dd142459aac66d4421f6c", "onnx", 1_780_590),
        (OcrModel::V6, ModelTier::Small, _) => &hf!("PP-OCRv6_small_det", "28fe5895c24fd108c19eb3e8479f4ab385fbfc62", "onnx", 9_880_512),
        (OcrModel::V6, ModelTier::Medium, _) => &hf!("PP-OCRv6_medium_det", "61323801669c338b7891481ec7bac61ce31b576a", "onnx", 62_032_837),
        (_, _, V5Det::Mobile) => &hf!("PP-OCRv5_mobile_det", "e6f4fa85f00e168c862bc462aebca69eef9b3d3d", "onnx", 4_826_518),
        (_, _, V5Det::Server) => &hf!("PP-OCRv5_server_det", "dcf248c4dcc064c6d030db241255f9e8fbc6733a", "onnx", 88_116_791),
    };
    let [rec, dict]: &'static [ModelArtifact; 2] = match (model, tier) {
        (OcrModel::V6, ModelTier::Tiny) => rec!("PP-OCRv6_tiny_rec", "2612ab37152ae0a677521bae4e1e3d4fb4cf7c30", 4_462_639, 55_571),
        (OcrModel::V6, ModelTier::Small) => rec!("PP-OCRv6_small_rec", "b8f84f0b80c529de40b4fbb3544b84fa7233a513", 21_159_378, 150_579),
        (OcrModel::V6, ModelTier::Medium) => rec!("PP-OCRv6_medium_rec", "50c7eacafc52fa7bcf4194e8cd08e46f8558504b", 76_554_979, 150_580),
        (OcrModel::V5English, _) => rec!("en_PP-OCRv5_mobile_rec", "3fafbc3b5dcf93dd72add9f48368be8a3a2cd33b", 7_848_423, 3_964),
        (OcrModel::V5Latin, _) => rec!("latin_PP-OCRv5_mobile_rec", "89d3a50e2c27e2e7cceeab0e944c25c807d5db4f", 8_042_023, 6_817),
        (OcrModel::V5Korean, _) => rec!("korean_PP-OCRv5_mobile_rec", "5c6f574b8e2230adf4287b33e736d71b9fabd28e", 13_418_787, 96_039),
        (OcrModel::V5Cyrillic, _) => rec!("cyrillic_PP-OCRv5_mobile_rec", "2cef88145434beb8afa9dd82d77d799eb1ad7b29", 8_048_799, 6_991),
        (OcrModel::V5EastSlavic, _) => rec!("eslav_PP-OCRv5_mobile_rec", "9a32171fc5718746875e1a261818884517975013", 7_887_627, 4_538),
        (OcrModel::V5Greek, _) => rec!("el_PP-OCRv5_mobile_rec", "8152b89d2ee0e1d4c92decab75fe75e5e3d836b6", 7_808_735, 3_317),
        (OcrModel::V5Devanagari, _) => rec!("devanagari_PP-OCRv5_mobile_rec", "251aec19e36739540d35e2cc943f6aa7503b98e5", 7_912_311, 5_027),
        (OcrModel::V5Tamil, _) => rec!("ta_PP-OCRv5_mobile_rec", "0db89798af0d5218fad9ea7ace0557526e9077f1", 7_885_691, 4_579),
        (OcrModel::V5Telugu, _) => rec!("te_PP-OCRv5_mobile_rec", "f1197c83430241c58c64b0952b3a80366d04eeba", 7_898_759, 4_795),
        (OcrModel::V5Thai, _) => rec!("th_PP-OCRv5_mobile_rec", "1d4adbbafb1034a2fd6618498575b81ea7b69f69", 7_891_015, 4_667),
    };
    [det, rec, dict]
}

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

/// PP-DocLayoutV3 for [`translator_core::LineMergeMode::Layout`]: the official
/// Hugging Face ONNX run through `scripts/optimize-doclayout.py` (fp16, box output only).
pub const LAYOUT_ARTIFACT: ModelArtifact = ModelArtifact {
    file_name: "pp-doclayoutv3_fp16.onnx",
    release: "https://github.com/OKTW-Network/Translator-Overlay/releases/download/models-v1",
    remote_name: "pp-doclayoutv3_fp16.onnx",
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
        const HF: &str = "https://huggingface.co/PaddlePaddle";
        let [det, rec, dict] = artifacts(ModelTier::Tiny, OcrModel::V5Korean, V5Det::Server);
        assert_eq!(
            det.download_url(),
            format!("{HF}/PP-OCRv5_server_det_onnx/resolve/dcf248c4dcc064c6d030db241255f9e8fbc6733a/inference.onnx")
        );
        assert_eq!(
            rec.download_url(),
            format!("{HF}/korean_PP-OCRv5_mobile_rec_onnx/resolve/5c6f574b8e2230adf4287b33e736d71b9fabd28e/inference.onnx")
        );
        assert_eq!(
            dict.download_url(),
            format!("{HF}/korean_PP-OCRv5_mobile_rec_onnx/resolve/5c6f574b8e2230adf4287b33e736d71b9fabd28e/inference.yml")
        );
        assert_eq!(dict.file_name, "korean_PP-OCRv5_mobile_rec.yml");
    }

    #[test]
    fn all_tiers_list_det_rec_dict_in_order() {
        for tier in [ModelTier::Tiny, ModelTier::Small, ModelTier::Medium] {
            let [det, rec, dict] = artifacts(tier, OcrModel::V6, V5Det::Mobile);
            assert!(det.file_name.ends_with("_det.onnx"), "{tier:?} det");
            assert!(rec.file_name.ends_with("_rec.onnx"), "{tier:?} rec");
            assert!(dict.file_name.ends_with("_rec.yml"), "{tier:?} dict");
            for a in artifacts(tier, OcrModel::V6, V5Det::Mobile) {
                assert!(a.expected_bytes > 0, "{tier:?} {}", a.file_name);
                assert!(a.download_url().contains("/resolve/") && !a.download_url().contains("/resolve/main/"), "{}", a.download_url());
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
