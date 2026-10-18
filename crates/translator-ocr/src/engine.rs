//! PP-OCRv6 engine backed by `oar-ocr` (ONNX Runtime + WebGPU / DirectML on Windows).

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
    /// Prefer [`Self::start_load`] when files may still need downloading.
    /// This only requires the files to exist; download verifies sizes.
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

    /// Update thresholds / merge knobs without reloading ONNX sessions.
    pub fn apply_runtime_config(&mut self, config: &OcrConfig) {
        self.confidence_threshold = config.confidence_threshold;
        self.filter_single_char = config.filter_single_char;
        self.line_merge = config.line_merge.clone();
    }

    /// One batched inference call across all images (det + rec batch on the GPU).
    async fn predict_batch(&self, images: Vec<RgbImage>) -> Result<Vec<OAROCRResult>, OcrError> {
        let inner = Arc::clone(&self.inner);
        tokio::task::spawn_blocking(move || inner.predict(images).map_err(|e| OcrError::Engine(e.to_string())))
            .await
            .unwrap_or_else(|e| Err(OcrError::Other(format!("OCR task: {e}"))))
    }

    /// Threshold / filter / merge one OCR page into blocks.
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
            // Drop single Latin letter/digit noise before merge (icons → "V"/"0").
            if self.filter_single_char && is_single_latin_or_digit(&text) {
                continue;
            }
            let (bbox, cover_h, cover_w) = region_box(&region.bounding_box);
            blocks.push(OcrBlock {
                id: i as u32,
                text,
                confidence,
                bbox,
                source_lines: 1,
                source_height: cover_h,
                source_width: cover_w,
            });
        }

        // Reading order + merge stacked lines that share column / height / gap.
        let blocks = merge_line_blocks_with(blocks, &self.line_merge, frame_w, frame_h, merge_all);

        // Re-apply after merge in case a merge edge case left a single token.
        if self.filter_single_char {
            filter_single_char_blocks(blocks)
        } else {
            reindex(blocks)
        }
    }

    /// Run OCR on each crop and offset boxes back into full-frame coordinates.
    ///
    /// Empty `regions` → whole frame (geometry rules only). Non-empty regions
    /// merge per-crop so independent boxes do not glue together. Whole-region
    /// merge uses the full frame size for thresholds, never the crop size.
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
        // Crop every region first, then one batched inference for all of them —
        // oar-ocr batches detection across images and pools recognition crops.
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

/// Overlay glyph box plus AABB cover (tilt inflates AABB on the short axis).
///
/// AABB height of a slightly rotated line is `h·cosθ + w·sinθ`, so a 3° tilt on
/// a 200×20 run adds ~10px and the overlay font jumps. The min-area quad's
/// short edges stay at the ink size. Keep AABB on the long axis so the caption
/// still covers the run.
fn region_box(bb: &BoundingBox) -> (Rect, f32, f32) {
    let (x_min, y_min, x_max, y_max) = bb.aabb();
    let x = x_min.min(x_max);
    let y = y_min.min(y_max);
    let aabb_w = (x_max - x_min).abs().max(0.0);
    let aabb_h = (y_max - y_min).abs().max(0.0);
    let short = quad_short_side(bb).unwrap_or_else(|| aabb_w.min(aabb_h));
    let (w, h) = if aabb_h > aabb_w * 1.5 {
        (short.min(aabb_w).max(0.0), aabb_h)
    } else {
        (aabb_w, short.min(aabb_h).max(0.0))
    };
    let bbox = Rect::new(x, y, w, h);
    (bbox, aabb_h.max(bbox.height), aabb_w.max(bbox.width))
}

/// Mean of the two shortest edges of a 4-point detection/recognition quad.
fn quad_short_side(bb: &BoundingBox) -> Option<f32> {
    let p = &bb.points;
    if p.len() != 4 {
        return None;
    }
    let dist = |i: usize, j: usize| {
        let dx = p[i].x - p[j].x;
        let dy = p[i].y - p[j].y;
        (dx * dx + dy * dy).sqrt()
    };
    let mut edges = [dist(0, 1), dist(1, 2), dist(2, 3), dist(3, 0)];
    edges.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let short = (edges[0] + edges[1]) * 0.5;
    if short.is_finite() && short > 0.0 { Some(short) } else { None }
}

#[cfg(test)]
mod tests {
    use oar_ocr::processors::Point;

    use super::*;

    fn quad(points: [(f32, f32); 4]) -> BoundingBox {
        BoundingBox::new(points.into_iter().map(|(x, y)| Point::new(x, y)).collect())
    }

    #[test]
    fn axis_aligned_quad_matches_aabb() {
        let bb = BoundingBox::from_coords(10.0, 20.0, 110.0, 40.0);
        let r = region_box(&bb).0;
        assert!((r.x - 10.0).abs() < 0.01);
        assert!((r.y - 20.0).abs() < 0.01);
        assert!((r.width - 100.0).abs() < 0.01);
        assert!((r.height - 20.0).abs() < 0.01);
    }

    #[test]
    fn tilted_line_height_uses_quad_short_side() {
        // 100×20 parallelogram with 10px of vertical shear. AABB is 30px tall.
        let bb = quad([(0.0, 10.0), (100.0, 20.0), (100.0, 40.0), (0.0, 30.0)]);
        let (x0, y0, x1, y1) = bb.aabb();
        assert!((y1 - y0 - 30.0).abs() < 0.01, "AABB height should be 30");
        let r = region_box(&bb).0;
        assert!((r.width - (x1 - x0)).abs() < 0.01, "keep AABB width, got {}", r.width);
        assert!((r.height - 20.0).abs() < 0.5, "glyph box is 20px, got {}", r.height);
    }

    #[test]
    fn vertical_run_width_uses_quad_short_side() {
        let bb = BoundingBox::from_coords(10.0, 0.0, 30.0, 200.0);
        let r = region_box(&bb).0;
        assert!((r.width - 20.0).abs() < 0.01);
        assert!((r.height - 200.0).abs() < 0.01);
    }

    #[test]
    fn cover_height_keeps_aabb_when_line_is_tilted() {
        let bb = quad([(0.0, 10.0), (100.0, 20.0), (100.0, 40.0), (0.0, 30.0)]);
        let (bbox, cover_h, cover_w) = region_box(&bb);
        assert!((bbox.height - 20.0).abs() < 0.5, "h={}", bbox.height);
        assert!((cover_h - 30.0).abs() < 0.5, "cover_h={}", cover_h);
        assert!((cover_w - 100.0).abs() < 0.5, "cover_w={}", cover_w);
    }

    #[test]
    fn cover_width_keeps_aabb_when_column_is_tilted() {
        // 20-wide × 200-tall parallelogram with 10px of horizontal shear. AABB is 30px wide.
        let bb = quad([(10.0, 0.0), (30.0, 0.0), (40.0, 200.0), (20.0, 200.0)]);
        let (bbox, cover_h, cover_w) = region_box(&bb);
        assert!((bbox.width - 20.0).abs() < 0.5, "w={}", bbox.width);
        assert!((bbox.height - 200.0).abs() < 0.5, "h={}", bbox.height);
        assert!((cover_w - 30.0).abs() < 0.5, "cover_w={}", cover_w);
        assert!((cover_h - 200.0).abs() < 0.5, "cover_h={}", cover_h);
    }
}
