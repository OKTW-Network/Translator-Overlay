//! PP-OCRv6 engine backed by `oar-ocr` (ONNX Runtime + DirectML on Windows).

use std::{
    path::Path,
    sync::{Arc, Once},
};

use image::{DynamicImage, RgbaImage};
use oar_ocr::{
    core::config::{OrtExecutionProvider, OrtSessionConfig},
    prelude::OAROCRBuilder,
    processors::BoundingBox,
};
use tracing::info;
use translator_core::{LineMergeConfig, OcrBlock, OcrConfig, Rect};

use crate::{OcrError, models::ModelPaths};

/// Loaded PP-OCRv6 engine (ONNX Runtime).
pub struct OcrEngine {
    // OAROCR is not exposed as a public type alias in all versions; use the builder output type.
    inner: Arc<oar_ocr::oarocr::OAROCR>,
    confidence_threshold: f32,
    filter_single_char: bool,
    line_merge: LineMergeConfig,
    model_paths: ModelPaths,
}

impl std::fmt::Debug for OcrEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OcrEngine")
            .field("confidence_threshold", &self.confidence_threshold)
            .field("filter_single_char", &self.filter_single_char)
            .field("line_merge_enabled", &self.line_merge.enabled)
            .field("det", &self.model_paths.det)
            .field("rec", &self.model_paths.rec)
            .finish()
    }
}

impl OcrEngine {
    /// Point oar-ocr auto-download / cache at our app `models_dir` (portable).
    pub fn configure_model_home(models_dir: &Path) -> Result<(), OcrError> {
        std::fs::create_dir_all(models_dir).map_err(|e| OcrError::Other(format!("create models dir {}: {e}", models_dir.display())))?;
        // oar-ocr resolves bare registry names under $OAR_HOME.
        // SAFETY: process-wide env used only for OCR model cache root.
        unsafe {
            std::env::set_var("OAR_HOME", models_dir);
        }
        Ok(())
    }

    /// Load ONNX models via oar-ocr.
    ///
    /// Uses full paths when files already exist under `models_dir`. Otherwise
    /// passes bare registry names so oar-ocr can auto-download into `OAR_HOME`.
    pub async fn load(config: &OcrConfig) -> Result<Self, OcrError> {
        let models_dir = config.models_dir_path()?;
        Self::configure_model_home(&models_dir)?;

        let paths = ModelPaths::from_dir(&models_dir, config.model_tier);
        let (det, rec, dict) = model_source_args(&paths);
        let ort = OrtSessionConfig::new().with_execution_providers(default_execution_providers());

        let inner = tokio::task::spawn_blocking(move || {
            OAROCRBuilder::new(det, rec, dict)
                .region_batch_size(4)
                .ort_session(ort)
                .build()
                .map_err(|e| OcrError::Engine(e.to_string()))
        })
        .await
        .unwrap_or_else(|e| Err(OcrError::Other(format!("OCR load task: {e}"))))?;

        log_gpu_once();

        info!(
            det = %paths.det.display(),
            rec = %paths.rec.display(),
            dict = %paths.dict.display(),
            "PP-OCRv6 engine loaded (oar-ocr / ONNX Runtime, DirectML→CPU)"
        );

        Ok(Self {
            inner: Arc::new(inner),
            confidence_threshold: config.confidence_threshold,
            filter_single_char: config.filter_single_char,
            line_merge: config.line_merge.clone(),
            model_paths: paths,
        })
    }

    /// Update thresholds / merge knobs without reloading ONNX sessions.
    pub fn apply_runtime_config(&mut self, config: &OcrConfig) {
        self.confidence_threshold = config.confidence_threshold;
        self.filter_single_char = config.filter_single_char;
        self.line_merge = config.line_merge.clone();
    }

    pub fn model_paths(&self) -> &ModelPaths {
        &self.model_paths
    }

    /// Run OCR on a dynamic image.
    async fn recognize(&self, image: &DynamicImage) -> Result<Vec<OcrBlock>, OcrError> {
        // oar-ocr predict takes RGB8 ImageBuffer.
        let rgb = image.to_rgb8();
        let inner = Arc::clone(&self.inner);
        let results = tokio::task::spawn_blocking(move || inner.predict(vec![rgb]).map_err(|e| OcrError::Engine(e.to_string())))
            .await
            .unwrap_or_else(|e| Err(OcrError::Other(format!("OCR task: {e}"))))?;

        let Some(page) = results.into_iter().next() else {
            return Ok(Vec::new());
        };

        let mut blocks = Vec::new();
        for (i, region) in page.text_regions.into_iter().enumerate() {
            let Some((text, confidence)) = region.text_with_confidence() else {
                continue;
            };
            if confidence < self.confidence_threshold {
                continue;
            }
            let text = text.trim().to_string();
            if text.is_empty() {
                continue;
            }
            // Drop single Latin letter/digit noise before merge (icons → "V"/"0").
            if self.filter_single_char && crate::filter::is_single_latin_or_digit(&text) {
                continue;
            }
            let bbox = aabb_to_rect(&region.bounding_box);
            blocks.push(OcrBlock {
                id: i as u32,
                text,
                confidence,
                bbox,
                source_lines: 1,
            });
        }

        // Reading order + merge multi-line paragraphs into single blocks.
        let blocks = crate::merge::merge_line_blocks_with(blocks, &self.line_merge);

        // Re-apply after merge in case a merge edge case left a single token.
        let blocks = if self.filter_single_char {
            crate::filter::filter_single_char_blocks(blocks)
        } else {
            reindex_ids(blocks)
        };

        Ok(blocks)
    }

    /// Run OCR on each crop and offset boxes back into full-frame coordinates.
    ///
    /// Empty `regions` → whole frame. Line merge stays per-crop so independent
    /// boxes do not glue together.
    pub async fn recognize_rgba_regions(&self, width: u32, height: u32, rgba: &[u8], regions: &[Rect]) -> Result<Vec<OcrBlock>, OcrError> {
        if regions.is_empty() {
            return self.recognize_rgba(width, height, rgba).await;
        }

        let mut all = Vec::new();
        for region in regions {
            let Some(crop) = crate::crop::crop_rgba(width, height, rgba, *region) else {
                continue;
            };
            let ox = crop.x as f32;
            let oy = crop.y as f32;
            let mut blocks = self.recognize_rgba(crop.width, crop.height, &crop.rgba).await?;
            for block in &mut blocks {
                block.bbox.x += ox;
                block.bbox.y += oy;
            }
            all.extend(blocks);
        }
        Ok(reindex_ids(all))
    }

    /// Run OCR on RGBA pixel buffer (e.g. capture frame).
    async fn recognize_rgba(&self, width: u32, height: u32, rgba: &[u8]) -> Result<Vec<OcrBlock>, OcrError> {
        let expected = (width as usize)
            .checked_mul(height as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or_else(|| OcrError::Image("frame dimensions overflow".into()))?;
        if rgba.len() < expected {
            return Err(OcrError::Image(format!("buffer too small: {} < {}", rgba.len(), expected)));
        }

        let rgba_img =
            RgbaImage::from_raw(width, height, rgba[..expected].to_vec()).ok_or_else(|| OcrError::Image("invalid RGBA buffer".into()))?;
        let dyn_img = DynamicImage::ImageRgba8(rgba_img);
        self.recognize(&dyn_img).await
    }

    /// Join block texts for UI display.
    pub fn blocks_to_text(blocks: &[OcrBlock]) -> String {
        blocks.iter().map(|b| b.text.as_str()).collect::<Vec<_>>().join("\n")
    }
}

fn default_execution_providers() -> Vec<OrtExecutionProvider> {
    // Prefer DirectML GPU on Windows; always fall back to CPU.
    vec![OrtExecutionProvider::DirectML { device_id: Some(0) }, OrtExecutionProvider::CPU]
}

fn model_source_args(paths: &ModelPaths) -> (String, String, String) {
    if paths.all_present() {
        return (
            paths.det.to_string_lossy().into_owned(),
            paths.rec.to_string_lossy().into_owned(),
            paths.dict.to_string_lossy().into_owned(),
        );
    }

    // Bare registry names: oar-ocr auto-download into $OAR_HOME (= models_dir).
    (
        paths
            .det
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("pp-ocrv6_small_det.onnx")
            .to_string(),
        paths
            .rec
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("pp-ocrv6_small_rec.onnx")
            .to_string(),
        paths
            .dict
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("ppocrv6_dict.txt")
            .to_string(),
    )
}

fn aabb_to_rect(bb: &BoundingBox) -> Rect {
    let (x_min, y_min, x_max, y_max) = bb.aabb();
    let left = x_min.min(x_max);
    let top = y_min.min(y_max);
    let width = (x_max - x_min).abs().max(0.0);
    let height = (y_max - y_min).abs().max(0.0);
    Rect::new(left, top, width, height)
}

fn reindex_ids(mut blocks: Vec<OcrBlock>) -> Vec<OcrBlock> {
    for (i, b) in blocks.iter_mut().enumerate() {
        b.id = i as u32;
    }
    blocks
}

fn log_gpu_once() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        info!("OCR execution providers: DirectML (device 0) → CPU fallback");
    });
}

#[cfg(test)]
mod tests {
    use oar_ocr::processors::BoundingBox;

    use super::*;

    #[test]
    fn aabb_maps_to_rect() {
        let bb = BoundingBox::from_coords(10.0, 20.0, 50.0, 40.0);
        let r = aabb_to_rect(&bb);
        assert!((r.x - 10.0).abs() < 1e-3);
        assert!((r.y - 20.0).abs() < 1e-3);
        assert!((r.width - 40.0).abs() < 1e-3);
        assert!((r.height - 20.0).abs() < 1e-3);
    }
}
