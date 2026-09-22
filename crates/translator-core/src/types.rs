//! Shared domain types used across crates.

use serde::{Deserialize, Serialize, Serializer};

/// Axis-aligned bounding box in capture-image pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// Clamp to a `width × height` frame and round to integer bounds `(x0, y0, x1, y1)`.
    pub fn clamped_bounds(self, width: u32, height: u32) -> (u32, u32, u32, u32) {
        let x0 = self.x.round().clamp(0.0, width as f32) as u32;
        let y0 = self.y.round().clamp(0.0, height as f32) as u32;
        let x1 = (self.x + self.width).round().clamp(0.0, width as f32) as u32;
        let y1 = (self.y + self.height).round().clamp(0.0, height as f32) as u32;
        (x0, y0, x1.max(x0), y1.max(y0))
    }

    pub fn center(self) -> (f32, f32) {
        (self.x + self.width * 0.5, self.y + self.height * 0.5)
    }

    pub fn iou(self, other: Self) -> f32 {
        let ax1 = self.x + self.width;
        let ay1 = self.y + self.height;
        let bx1 = other.x + other.width;
        let by1 = other.y + other.height;
        let ix0 = self.x.max(other.x);
        let iy0 = self.y.max(other.y);
        let ix1 = ax1.min(bx1);
        let iy1 = ay1.min(by1);
        let iw = (ix1 - ix0).max(0.0);
        let ih = (iy1 - iy0).max(0.0);
        let inter = iw * ih;
        if inter <= 0.0 {
            return 0.0;
        }
        let union = self.width * self.height + other.width * other.height - inter;
        if union <= 0.0 { 0.0 } else { inter / union }
    }

    /// True when `candidate` is a real layout change, not detector noise.
    ///
    /// OCR boxes jitter a few pixels every frame on animated backgrounds even
    /// when the readable text is unchanged. Overlay remapping must ignore that.
    pub fn is_significant_relayout(self, candidate: Self) -> bool {
        let prev_w = self.width.max(1.0);
        let prev_h = self.height.max(1.0);
        let w_ratio = candidate.width.max(1.0) / prev_w;
        let h_ratio = candidate.height.max(1.0) / prev_h;
        let size_changed = !(0.65..=1.55).contains(&w_ratio) || !(0.65..=1.55).contains(&h_ratio);

        let iou = self.iou(candidate);
        // Strong overlap with similar size → treat as jitter.
        if iou >= 0.35 && !size_changed {
            return false;
        }
        if size_changed && iou >= 0.15 {
            // Merge/split or font-scale change at roughly the same spot.
            return true;
        }

        self.center_shift_exceeds_tol(candidate)
    }

    /// Keep `self` when `candidate` is only OCR jitter; otherwise take `candidate`.
    pub fn stabilize_against(self, candidate: Self) -> Self {
        if self.is_significant_relayout(candidate) { candidate } else { self }
    }

    fn center_shift_exceeds_tol(self, candidate: Self) -> bool {
        let (cx0, cy0) = self.center();
        let (cx1, cy1) = candidate.center();
        let dx = (cx0 - cx1).abs();
        let dy = (cy0 - cy1).abs();
        let tol_x = (self.width.max(candidate.width) * 0.22).max(10.0);
        let tol_y = (self.height.max(candidate.height) * 0.40).max(8.0);
        dx > tol_x || dy > tol_y
    }
}

/// Axis-aligned region as fractions of the capture client area (`0..=1`).
///
/// Session `ocr_regions` are runtime-only (not in `config.toml`). Named sets may
/// also be stored in `region-presets.toml`. Empty list means “whole window”.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NormRect {
    #[serde(serialize_with = "serialize_f32")]
    pub x: f32,
    #[serde(serialize_with = "serialize_f32")]
    pub y: f32,
    #[serde(serialize_with = "serialize_f32")]
    pub width: f32,
    #[serde(serialize_with = "serialize_f32")]
    pub height: f32,
}

/// `toml_edit` prints `f32 as f64` with the full binary expansion (`0.02` → `0.01999…`).
fn serialize_f32<S: Serializer>(value: &f32, serializer: S) -> Result<S::Ok, S::Error> {
    let short = value.to_string().parse::<f64>().unwrap_or(*value as f64);
    serializer.serialize_f64(short)
}

impl NormRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// Flip negative size, clamp to `[0, 1]`, drop if too small to OCR.
    pub fn sanitize(self) -> Option<Self> {
        let mut x = self.x;
        let mut y = self.y;
        let mut w = self.width;
        let mut h = self.height;
        if !x.is_finite() || !y.is_finite() || !w.is_finite() || !h.is_finite() {
            return None;
        }
        if w < 0.0 {
            x += w;
            w = -w;
        }
        if h < 0.0 {
            y += h;
            h = -h;
        }
        let x1 = (x + w).clamp(0.0, 1.0);
        let y1 = (y + h).clamp(0.0, 1.0);
        x = x.clamp(0.0, 1.0);
        y = y.clamp(0.0, 1.0);
        w = (x1 - x).max(0.0);
        h = (y1 - y).max(0.0);
        // ~0.4% of a side is too thin for useful OCR.
        if w < 0.004 || h < 0.004 {
            return None;
        }
        Some(Self { x, y, width: w, height: h })
    }

    pub fn to_pixel(self, frame_w: u32, frame_h: u32) -> Rect {
        let fw = frame_w as f32;
        let fh = frame_h as f32;
        Rect::new(self.x * fw, self.y * fh, self.width * fw, self.height * fh)
    }

    pub fn from_pixel(rect: Rect, frame_w: u32, frame_h: u32) -> Self {
        let fw = frame_w.max(1) as f32;
        let fh = frame_h.max(1) as f32;
        Self {
            x: rect.x / fw,
            y: rect.y / fh,
            width: rect.width / fw,
            height: rect.height / fh,
        }
    }
}

/// Collapse whitespace and punctuation flicker so OCR matching stays stable.
///
/// Used by stability fingerprints, block persistence, sticky remap, and the
/// translation cache. Does not rewrite `OcrBlock.text` sent to the model.
///
/// - Fullwidth `？` / `！` fold to ASCII `?` / `!`.
/// - Ellipsis-length thrash (`…` / `……` / `...` / `・・・`) collapses to one `…`.
/// - A trailing collapsed `…` is dropped unless the whole line is only `…`.
pub fn normalize_ocr_text(s: &str) -> String {
    let collapsed = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let is_ellipsis = |c: char| matches!(c, '…' | '⋯' | '‥' | '︙');
    let is_unit = |c: char| is_ellipsis(c) || matches!(c, '.' | '．' | '。' | '・' | '･' | '·');

    let mut out = String::with_capacity(collapsed.len());
    let mut chars = collapsed.chars().peekable();
    while let Some(c) = chars.next() {
        let c = match c {
            '？' => '?',
            '！' => '!',
            other => other,
        };
        if is_unit(c) {
            if is_ellipsis(c) || chars.peek().copied().is_some_and(is_unit) {
                while chars.peek().copied().is_some_and(is_unit) {
                    let _ = chars.next();
                }
                out.push('…');
            } else {
                out.push(c);
            }
        } else {
            out.push(c);
        }
    }

    if out != "…" {
        while out.ends_with('…') {
            out.pop();
        }
        let end = out.trim_end().len();
        out.truncate(end);
    }
    out
}

/// One OCR text region.
#[derive(Debug, Clone, PartialEq)]
pub struct OcrBlock {
    pub id: u32,
    pub text: String,
    pub confidence: f32,
    pub bbox: Rect,
    /// Detector lines merged into this block (`1` = single line).
    pub source_lines: u32,
}

/// OCR block after translation.
#[derive(Debug, Clone, PartialEq)]
pub struct TranslatedBlock {
    pub id: u32,
    pub source: String,
    pub translation: String,
    pub confidence: f32,
    pub bbox: Rect,
    /// Detector lines in the source (`1` = single line; overlay may widen/shrink).
    pub source_lines: u32,
}

/// PP-OCRv6 model size tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ModelTier {
    Tiny,
    #[default]
    Small,
    Medium,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_collapses_whitespace() {
        assert_eq!(normalize_ocr_text("hello   world"), "hello world");
        assert_eq!(normalize_ocr_text("  a\n\tb  "), "a b");
    }

    #[test]
    fn normalize_collapses_ellipsis_length() {
        assert_eq!(normalize_ocr_text("…"), "…");
        assert_eq!(normalize_ocr_text("……"), "…");
        assert_eq!(normalize_ocr_text("………"), "…");
        assert_eq!(normalize_ocr_text("..."), "…");
        assert_eq!(normalize_ocr_text("・・・"), "…");
        assert_eq!(normalize_ocr_text("待って………"), "待って");
        assert_eq!(normalize_ocr_text("待って…"), "待って");
        assert_eq!(normalize_ocr_text("待って"), "待って");
        assert_eq!(normalize_ocr_text("待って…ください"), "待って…ください");
        assert_eq!(normalize_ocr_text("セリフ。"), "セリフ。");
        assert_eq!(normalize_ocr_text("Hello."), "Hello.");
    }

    #[test]
    fn normalize_folds_fullwidth_question_and_bang() {
        assert_eq!(normalize_ocr_text("何？"), "何?");
        assert_eq!(normalize_ocr_text("何?"), "何?");
        assert_eq!(normalize_ocr_text("嘘！"), "嘘!");
        assert_eq!(normalize_ocr_text("嘘!"), "嘘!");
    }

    #[test]
    fn stabilize_keeps_bbox_under_small_jitter() {
        let prev = Rect::new(100.0, 200.0, 180.0, 28.0);
        // Typical detector noise: a few px shift + width wobble.
        let jitter = Rect::new(103.0, 197.0, 176.0, 30.0);
        assert!(!prev.is_significant_relayout(jitter));
        assert_eq!(prev.stabilize_against(jitter), prev);
    }

    #[test]
    fn stabilize_adopts_real_move() {
        let prev = Rect::new(100.0, 200.0, 180.0, 28.0);
        let moved = Rect::new(100.0, 320.0, 180.0, 28.0);
        assert!(prev.is_significant_relayout(moved));
        assert_eq!(prev.stabilize_against(moved), moved);
    }

    #[test]
    fn stabilize_adopts_large_size_change() {
        let prev = Rect::new(100.0, 200.0, 80.0, 24.0);
        // Paragraph merge expands width/height at similar origin.
        let merged = Rect::new(98.0, 198.0, 240.0, 72.0);
        assert!(prev.is_significant_relayout(merged));
        assert_eq!(prev.stabilize_against(merged), merged);
    }

    #[test]
    fn iou_full_and_none() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert!((a.iou(a) - 1.0).abs() < 1e-5);
        let b = Rect::new(20.0, 20.0, 10.0, 10.0);
        assert_eq!(a.iou(b), 0.0);
    }

    #[test]
    fn norm_rect_sanitize_flips_and_clamps() {
        let flipped = NormRect::new(0.4, 0.5, -0.2, -0.1).sanitize().unwrap();
        assert!((flipped.x - 0.2).abs() < 1e-5);
        assert!((flipped.y - 0.4).abs() < 1e-5);
        assert!((flipped.width - 0.2).abs() < 1e-5);
        assert!((flipped.height - 0.1).abs() < 1e-5);
        assert!(NormRect::new(0.0, 0.0, 0.001, 0.5).sanitize().is_none());
        assert!(NormRect::new(f32::NAN, 0.0, 0.2, 0.2).sanitize().is_none());
    }

    #[test]
    fn norm_rect_pixel_roundtrip() {
        let n = NormRect::new(0.1, 0.2, 0.3, 0.4);
        let px = n.to_pixel(1000, 500);
        assert!((px.x - 100.0).abs() < 1e-3);
        assert!((px.y - 100.0).abs() < 1e-3);
        assert!((px.width - 300.0).abs() < 1e-3);
        assert!((px.height - 200.0).abs() < 1e-3);
        let back = NormRect::from_pixel(px, 1000, 500);
        assert!((back.x - n.x).abs() < 1e-5);
        assert!((back.width - n.width).abs() < 1e-5);
    }
}
