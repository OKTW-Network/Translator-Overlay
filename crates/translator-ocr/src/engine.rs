//! PP-OCRv6 / PP-OCRv5 (+ optional PP-DocLayoutV3) engine: PaddleOCR's pipeline on ONNX Runtime
//! (WebGPU / DirectML / CPU).

use std::{
    path::Path,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use ort::{ep, logging::LogLevel, session::Session, value::Tensor};
use tracing::info;
use translator_core::{LineMergeConfig, LineMergeMode, ModelTier, OcrBlock, OcrConfig, OcrDevice, OcrModel, Rect};

use crate::{
    OcrError,
    crop::{Rgb8Crop, crop_to_rgb8},
    det::{self, DbParams, MinRect},
    filter::{filter_single_char_blocks, is_single_latin_or_digit},
    layout,
    merge::merge_line_blocks_with,
    models::{LAYOUT_ARTIFACT, ModelPaths},
    raster::{Interp, Quad, Rgb8},
    rec, reindex,
};

/// PaddleOCR's `max_side_limit`: detection never runs on a longer side than this.
const DET_MAX_SIDE: u32 = 4000;

/// Line crops recognized per inference call.
const REC_BATCH: usize = 4;

/// A recognized line: its box, text and confidence.
type Line = (Quad, String, f32);

/// One image's lines plus its layout boxes (empty without layout), each flagged `true` for
/// vertical text.
type Page = (Vec<Line>, Vec<(Rect, bool)>);

/// The ONNX sessions and what their pre/post-processing needs.
struct Models {
    det: Mutex<Session>,
    rec: Mutex<Session>,
    /// Characters behind the recognition model's output classes.
    classes: Vec<String>,
    /// PP-DocLayoutV3, loaded only for [`translator_core::LineMergeMode::Layout`].
    layout: Option<Mutex<Session>>,
    db: DbParams,
    /// Detection shrinks an image whose long side exceeds this.
    det_limit: usize,
}

/// `Session::run` takes `&mut self`; a panic mid-run leaves nothing half-written to guard.
fn lock(session: &Mutex<Session>) -> MutexGuard<'_, Session> {
    session.lock().unwrap_or_else(PoisonError::into_inner)
}

fn session(device: OcrDevice, path: &Path) -> ort::Result<Session> {
    // A GPU provider that fails to register leaves the session on the CPU.
    let gpu = match device {
        OcrDevice::Cpu => None,
        OcrDevice::Webgpu => Some(ep::WebGPU::default().build()),
        OcrDevice::Directml => Some(ep::DirectML::default().with_device_id(0).build()),
    };
    let providers: Vec<_> = gpu.into_iter().chain([ep::CPU::default().build()]).collect();
    Session::builder(translator_core::ort_environment()?)?
        .with_log_level(LogLevel::Error)?
        .with_execution_providers(providers)?
        .commit_from_file(path)
}

impl Models {
    /// Text boxes of `image`.
    fn detect(&self, image: &Rgb8) -> ort::Result<Vec<Quad>> {
        let (tensor, width, height) = det::preprocess(image, self.det_limit);
        let mut det = lock(&self.det);
        let outputs = det.run(ort::inputs![Tensor::from_array(([1, 3, height, width], tensor))?])?;
        let (shape, pred) = outputs[0].try_extract_tensor::<f32>()?;
        let &[_, _, map_height, map_width] = &shape[..] else {
            return Err(ort::Error::new(format!("detection output has shape {shape}")));
        };
        let (map_width, map_height) = (map_width as usize, map_height as usize);
        Ok(det::boxes_from_map(&pred[..map_width * map_height], map_width, map_height, image.width, image.height, &self.db))
    }

    /// Text-bearing layout boxes of `image`.
    fn layout_boxes(layout: &Mutex<Session>, image: &Rgb8) -> ort::Result<Vec<(Rect, bool)>> {
        let side = layout::SIDE as f32;
        let (width, height) = (image.width as f32, image.height as f32);
        // The image stretched to the model's input (aspect ratio is not kept): RGB in 0..1.
        let mut tensor = vec![0.0; 3 * layout::SIDE * layout::SIDE];
        image
            .resize(layout::SIDE, layout::SIDE, Interp::Cubic)
            .write_chw([0, 1, 2], [1.0 / 255.0; 3], [0.0; 3], &mut tensor, layout::SIDE);
        let mut layout_session = lock(layout);
        // The model maps its boxes back to the image through `scale_factor` (height, width).
        let outputs = layout_session.run(ort::inputs![
            "image" => Tensor::from_array(([1, 3, layout::SIDE, layout::SIDE], tensor))?,
            "scale_factor" => Tensor::from_array(([1, 2], vec![side / height, side / width]))?,
            "im_shape" => Tensor::from_array(([1, 2], vec![side, side]))?,
        ])?;
        let (shape, rows) = outputs[0].try_extract_tensor::<f32>()?;
        let &[_, columns] = &shape[..] else {
            return Err(ort::Error::new(format!("layout output has shape {shape}")));
        };
        Ok(layout::text_boxes(rows, columns as usize, width, height))
    }

    /// Detect, crop and recognize every image.
    fn pages(&self, images: &[Rgb8]) -> ort::Result<Vec<Page>> {
        let mut pages: Vec<Page> = Vec::with_capacity(images.len());
        // (page, line number, box, crop)
        let mut crops = Vec::new();
        for (page, image) in images.iter().enumerate() {
            let boxes = match &self.layout {
                Some(layout) => Self::layout_boxes(layout, image)?,
                None => Vec::new(),
            };
            pages.push((Vec::new(), boxes));
            for (line, quad) in self.detect(image)?.into_iter().enumerate() {
                // PaddleOCR crops the smallest rectangle around the box; an empty one is skipped.
                if let Some(crop) = MinRect::of(&quad).and_then(|rect| image.crop_quad(&rect.corners())) {
                    crops.push((page, line, quad, crop));
                }
            }
        }

        // Crops from every image share batches, narrow to wide, so a batch pads little.
        crops.sort_by(|a, b| (a.3.width as f32 / a.3.height as f32).total_cmp(&(b.3.width as f32 / b.3.height as f32)));
        let mut lines = Vec::with_capacity(crops.len());
        let mut rec = lock(&self.rec);
        for chunk in crops.chunks(REC_BATCH) {
            let (tensor, width) = rec::batch(&chunk.iter().map(|(.., crop)| crop).collect::<Vec<_>>());
            let outputs = rec.run(ort::inputs![Tensor::from_array(([chunk.len(), 3, rec::HEIGHT, width], tensor))?])?;
            let (shape, probs) = outputs[0].try_extract_tensor::<f32>()?;
            if shape.len() != 3 || shape[2] as usize != self.classes.len() {
                return Err(ort::Error::new(format!(
                    "recognition output has shape {shape}, the dictionary has {} classes",
                    self.classes.len()
                )));
            }
            for (&(page, line, quad, _), probs) in chunk.iter().zip(probs.chunks_exact(probs.len() / chunk.len())) {
                let (text, score) = rec::decode(probs, &self.classes);
                lines.push((page, line, (quad, text, score)));
            }
        }
        lines.sort_by_key(|&(page, line, _)| (page, line));
        for (page, _, line) in lines {
            pages[page].0.push(line);
        }
        Ok(pages)
    }
}

/// Loaded PP-OCR engine (ONNX Runtime).
#[derive(Clone)]
pub struct OcrEngine {
    models: Arc<Models>,
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
        let paths = ModelPaths::from_dir(&models_dir, config.model_tier, config.model, config.v5_det);
        let layout_path = models_dir.join(LAYOUT_ARTIFACT.file_name);
        let use_layout = config.uses_layout();
        if !paths.all_present() || (use_layout && !layout_path.is_file()) {
            return Err(OcrError::Other(format!("OCR models missing under {} (run ensure_models first)", models_dir.display())));
        }

        // PaddleOCR's published `DBPostProcess` values per model family.
        let (thresh, box_thresh, unclip_ratio, max_candidates) = match (config.model, config.model_tier) {
            (OcrModel::V6, ModelTier::Tiny) => (0.2, 0.4, 1.4, 3000),
            (OcrModel::V6, _) => (0.2, 0.45, 1.4, 3000),
            _ => (0.3, 0.6, 1.5, 1000),
        };
        let db = DbParams {
            thresh,
            box_thresh,
            unclip_ratio,
            max_candidates,
        };
        let det_limit = if config.det_max_side == 0 {
            DET_MAX_SIDE
        } else {
            config.det_max_side.min(DET_MAX_SIDE)
        } as usize;
        let device = config.device;

        let files = paths.clone();
        let models = tokio::task::spawn_blocking(move || {
            let dict = std::fs::read_to_string(&files.dict).map_err(|e| OcrError::Other(format!("read {}: {e}", files.dict.display())))?;
            let load = || -> ort::Result<Models> {
                Ok(Models {
                    det: Mutex::new(session(device, &files.det)?),
                    rec: Mutex::new(session(device, &files.rec)?),
                    // CTC classes: the blank, the dictionary's entries (one per line), then a space.
                    classes: [""].into_iter().chain(dict.lines()).chain([" "]).map(str::to_owned).collect(),
                    layout: use_layout.then(|| session(device, &layout_path)).transpose()?.map(Mutex::new),
                    db,
                    det_limit,
                })
            };
            load().map_err(|e| OcrError::Engine(e.to_string()))
        })
        .await
        .unwrap_or_else(|e| Err(OcrError::Other(format!("OCR load task: {e}"))))?;

        info!(
            det = %paths.det.display(),
            rec = %paths.rec.display(),
            dict = %paths.dict.display(),
            device = ?config.device,
            layout = use_layout,
            "OCR engine loaded (ONNX Runtime)"
        );

        Ok(Self {
            models: Arc::new(models),
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

    /// Threshold / filter / merge one OCR page into blocks.
    fn blocks_from_page(&self, (lines, layout): Page, frame_h: u32, merge_all: bool) -> Vec<OcrBlock> {
        let mut blocks = Vec::new();
        for (i, (quad, text, confidence)) in lines.into_iter().enumerate() {
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
            let (bbox, cover_h, cover_w) = region_box(&quad);
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

        // Reading order + merge lines by geometry rules or layout boxes.
        let blocks = merge_line_blocks_with(blocks, &self.line_merge, frame_h, merge_all, &layout);

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
            (regions, self.line_merge.mode == LineMergeMode::Region)
        };
        // Crop every region first: their lines are recognized in shared batches.
        let (origins, images): (Vec<(u32, u32)>, Vec<Rgb8>) = regions
            .iter()
            .filter_map(|region| crop_to_rgb8(width, height, rgba, *region))
            .map(|Rgb8Crop { x, y, width, height, rgb }| {
                let image = Rgb8 {
                    width: width as usize,
                    height: height as usize,
                    data: rgb,
                };
                ((x, y), image)
            })
            .unzip();
        if images.is_empty() {
            return Ok(Vec::new());
        }

        let models = Arc::clone(&self.models);
        let results = tokio::task::spawn_blocking(move || models.pages(&images))
            .await
            .map_err(|e| OcrError::Other(format!("OCR task: {e}")))?
            .map_err(|e| OcrError::Engine(e.to_string()))?;

        let mut all = Vec::new();
        for ((ox, oy), page) in origins.into_iter().zip(results) {
            let mut blocks = self.blocks_from_page(page, height, merge_all);
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

/// Overlay glyph box plus AABB cover (tilt inflates AABB on the short axis).
///
/// AABB height of a slightly rotated line is `h·cosθ + w·sinθ`, so a 3° tilt on
/// a 200×20 run adds ~10px and the overlay font jumps. The detector's min-area
/// quad wobbles a few degrees even on axis-aligned text; its edges stay at the
/// ink size. Keep AABB on the long axis so the caption still covers the run.
fn region_box(quad: &Quad) -> (Rect, f32, f32) {
    let bound = |axis: usize, pick: fn(f32, f32) -> f32| quad.iter().map(|p| p[axis]).reduce(pick).unwrap_or(0.0);
    let (x, y) = (bound(0, f32::min), bound(1, f32::min));
    let aabb_w = bound(0, f32::max) - x;
    let aabb_h = bound(1, f32::max) - y;
    let (quad_w, quad_h) = quad_sides(quad).unwrap_or((aabb_w, aabb_h));
    let (w, h) = if aabb_h > aabb_w * 1.5 {
        (quad_w.min(aabb_w).max(0.0), aabb_h)
    } else {
        (aabb_w, quad_h.min(aabb_h).max(0.0))
    };
    let bbox = Rect::new(x, y, w, h);
    (bbox, aabb_h.max(bbox.height), aabb_w.max(bbox.width))
}

/// Quad (width, height): mean of the near-horizontal and near-vertical opposite
/// edge pairs. Picks by direction, not length, so a 16×22 cell keeps height 22.
fn quad_sides(p: &Quad) -> Option<(f32, f32)> {
    let dist = |i: usize, j: usize| (p[i][0] - p[j][0]).hypot(p[i][1] - p[j][1]);
    let a = (dist(0, 1) + dist(2, 3)) * 0.5;
    let b = (dist(1, 2) + dist(3, 0)) * 0.5;
    let a_horizontal = (p[1][0] - p[0][0]).abs() >= (p[1][1] - p[0][1]).abs();
    let (w, h) = if a_horizontal { (a, b) } else { (b, a) };
    (w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0).then_some((w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad(points: [(f32, f32); 4]) -> Quad {
        points.map(|(x, y)| [x, y])
    }

    fn upright(x0: f32, y0: f32, x1: f32, y1: f32) -> Quad {
        quad([(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
    }

    #[test]
    fn axis_aligned_quad_matches_aabb() {
        let bb = upright(10.0, 20.0, 110.0, 40.0);
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
        let r = region_box(&bb).0;
        assert!((r.width - 100.0).abs() < 0.01, "keep AABB width, got {}", r.width);
        assert!((r.height - 20.0).abs() < 0.5, "glyph box is 20px, got {}", r.height);
    }

    #[test]
    fn narrow_horizontal_cell_keeps_height() {
        // w < h but not a vertical run: height must not come from the shorter width.
        let bb = upright(0.0, 0.0, 16.0, 22.0);
        let r = region_box(&bb).0;
        assert!((r.width - 16.0).abs() < 0.01, "w={}", r.width);
        assert!((r.height - 22.0).abs() < 0.01, "h={}", r.height);
    }

    #[test]
    fn vertical_run_width_uses_quad_short_side() {
        let bb = upright(10.0, 0.0, 30.0, 200.0);
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
