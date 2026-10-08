//! PP-OCRv6 engine backed by `oar-ocr`, running on ONNX Runtime with WebGPU or DirectML.

use std::sync::Arc;

use image::RgbImage;
use oar_ocr::{
    core::config::{OrtExecutionProvider, OrtSessionConfig},
    oarocr::{OAROCR, OAROCRResult},
    prelude::OAROCRBuilder,
    processors::BoundingBox,
};
use tracing::info;
use translator_core::{LineMergeConfig, OcrBlock, OcrConfig, OcrDevice, Rect};

use crate::{
    OcrError,
    crop::{Rgb8Crop, crop_to_rgb8},
    filter::{filter_single_char_blocks, is_single_latin_or_digit},
    merge::merge_line_blocks_with,
    models::ModelPaths,
    reindex,
};

/// Loaded PP-OCRv6 engine (ONNX Runtime).
#[derive(Clone)]
pub struct OcrEngine {
    inner: Arc<OAROCR>,
    confidence_threshold: f32,
    filter_single_char: bool,
    line_merge: LineMergeConfig,
}

impl OcrEngine {
    /// Load ONNX models from local paths under `models_dir`.
    ///
    /// Use [`Self::start_load`] when files may still need downloading.
    /// This only requires the files to exist. The download checks their sizes.
    pub async fn load(config: &OcrConfig) -> Result<Self, OcrError> {
        let models_dir = config.models_dir_path()?;
        let paths = ModelPaths::from_dir(&models_dir, config.model_tier);
        if !paths.all_present() {
            return Err(OcrError::Other(format!("OCR models missing under {} (run ensure_models first)", models_dir.display())));
        }

        let det = paths.det.to_string_lossy().into_owned();
        let rec = paths.rec.to_string_lossy().into_owned();
        let dict = paths.dict.to_string_lossy().into_owned();
        let ort = OrtSessionConfig::new().with_execution_providers(execution_providers(config.device));

        let inner = tokio::task::spawn_blocking(move || {
            OAROCRBuilder::new(det, rec, dict)
                .region_batch_size(4)
                .image_batch_size(4)
                .ort_session(ort)
                .build()
                .map_err(|e| OcrError::Engine(e.to_string()))
        })
        .await
        .unwrap_or_else(|e| Err(OcrError::Other(format!("OCR load task: {e}"))))?;

        info!(
            det = %paths.det.display(),
            rec = %paths.rec.display(),
            dict = %paths.dict.display(),
            device = ?config.device,
            "PP-OCRv6 engine loaded (oar-ocr / ONNX Runtime)"
        );

        Ok(Self {
            inner: Arc::new(inner),
            confidence_threshold: config.confidence_threshold,
            filter_single_char: config.filter_single_char,
            line_merge: config.line_merge.clone(),
        })
    }

    /// Update the thresholds and merge settings without reloading the ONNX sessions.
    pub fn apply_runtime_config(&mut self, config: &OcrConfig) {
        self.confidence_threshold = config.confidence_threshold;
        self.filter_single_char = config.filter_single_char;
        self.line_merge = config.line_merge.clone();
    }

    /// Run one batched inference over all images. Detection and recognition both batch on the GPU.
    async fn predict_batch(&self, images: Vec<RgbImage>) -> Result<Vec<OAROCRResult>, OcrError> {
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || inner.predict(images).map_err(|e| OcrError::Engine(e.to_string())))
            .await
            .unwrap_or_else(|e| Err(OcrError::Other(format!("OCR task: {e}"))))
    }

    /// Turn one OCR page into blocks by applying the threshold, the filters, and the merge.
    fn blocks_from_page(&self, page: OAROCRResult, frame_w: u32, frame_h: u32, merge_all: bool) -> Vec<OcrBlock> {
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
            // Drop single Latin letters and digits before the merge. Icons often read as "V" or "0".
            if self.filter_single_char && is_single_latin_or_digit(&text) {
                continue;
            }
            let bbox = aabb_to_rect(&region.bounding_box);
            blocks.push(OcrBlock {
                id: i as u32,
                text,
                confidence,
                bbox,
                source_lines: 1,
                source_height: bbox.height,
            });
        }

        // Merge stacked lines that share a column, height, and gap, and sort into reading order.
        let blocks = merge_line_blocks_with(blocks, &self.line_merge, frame_w, frame_h, merge_all);

        // Filter again in case an edge case in the merge left a single character.
        if self.filter_single_char {
            filter_single_char_blocks(blocks)
        } else {
            reindex(blocks)
        }
    }

    /// Run OCR on each crop and offset boxes back into full-frame coordinates.
    ///
    /// Empty `regions` means the whole frame, merged by the geometry rules only.
    /// Each region merges on its own, so separate boxes do not glue together. The
    /// whole-region merge still measures thresholds against the full frame, never the crop.
    pub async fn recognize_rgba_regions(&self, width: u32, height: u32, rgba: &[u8], regions: &[Rect]) -> Result<Vec<OcrBlock>, OcrError> {
        let expected = (width as usize).checked_mul(height as usize).and_then(|n| n.checked_mul(4));
        if expected.is_none_or(|n| rgba.len() < n) {
            return Err(OcrError::Image(format!("{width}x{height} frame needs more than {} bytes", rgba.len())));
        }
        let whole_frame = [Rect::new(0.0, 0.0, width as f32, height as f32)];
        let (regions, merge_all) = if regions.is_empty() {
            (&whole_frame[..], false)
        } else {
            (regions, self.line_merge.merge_whole_region)
        };
        // Crop every region first and run one batched inference over all of them.
        // oar-ocr batches detection across images and pools the recognition crops.
        let crops: Vec<(u32, u32, RgbImage)> = regions
            .iter()
            .filter_map(|region| crop_to_rgb8(width, height, rgba, *region))
            .filter_map(|crop| {
                let Rgb8Crop { x, y, width, height, rgb } = crop;
                RgbImage::from_raw(width, height, rgb).map(|img| (x, y, img))
            })
            .collect();
        if crops.is_empty() {
            return Ok(Vec::new());
        }

        let origins: Vec<(u32, u32)> = crops.iter().map(|(x, y, _)| (*x, *y)).collect();
        let results = self.predict_batch(crops.into_iter().map(|(_, _, img)| img).collect()).await?;

        let mut all = Vec::new();
        for ((ox, oy), page) in origins.into_iter().zip(results) {
            let mut blocks = self.blocks_from_page(page, width, height, merge_all);
            for block in &mut blocks {
                block.bbox.x += ox as f32;
                block.bbox.y += oy as f32;
            }
            all.extend(blocks);
        }
        Ok(reindex(all))
    }

    /// Join block texts for UI display.
    pub fn blocks_to_text(blocks: &[OcrBlock]) -> String {
        blocks.iter().map(|b| b.text.as_str()).collect::<Vec<_>>().join("\n")
    }
}

fn execution_providers(device: OcrDevice) -> Vec<OrtExecutionProvider> {
    match device {
        OcrDevice::Cpu => vec![OrtExecutionProvider::CPU],
        OcrDevice::Webgpu => vec![OrtExecutionProvider::WebGPU, OrtExecutionProvider::CPU],
        OcrDevice::Directml => vec![OrtExecutionProvider::DirectML { device_id: Some(0) }, OrtExecutionProvider::CPU],
    }
}

fn aabb_to_rect(bb: &BoundingBox) -> Rect {
    let (x_min, y_min, x_max, y_max) = bb.aabb();
    // `f32::max` turns a NaN extent from a degenerate polygon into 0.
    Rect::new(x_min.min(x_max), y_min.min(y_max), (x_max - x_min).abs().max(0.0), (y_max - y_min).abs().max(0.0))
}
