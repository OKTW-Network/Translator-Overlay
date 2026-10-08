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

/// TOML serializers widen `f32` to `f64` and print the full binary expansion, so `0.02`
/// becomes `0.01999…`. Round-trip through the shortest `f32` text first.
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
/// - Fullwidth `？` and `！` fold to ASCII `?` and `!`.
/// - Wave dash `〜` and fullwidth tilde `～` fold to ASCII `~`.
/// - Ellipses of any length (`…`, `……`, `...`, `・・・`) collapse to one `…`.
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
            '〜' | '～' => '~',
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
    /// Detector lines merged into this block. `1` means a single line.
    pub source_lines: u32,
    /// Vertical span of the original ink, the union of the merged line boxes.
    /// It equals `bbox.height` for a single line. The overlay never covers less than this.
    pub source_height: f32,
}

/// OCR block after translation.
#[derive(Debug, Clone, PartialEq)]
pub struct TranslatedBlock {
    pub id: u32,
    pub source: String,
    pub translation: String,
    pub confidence: f32,
    pub bbox: Rect,
    /// Detector lines in the source. `1` means a single line, which the overlay may widen or shrink.
    pub source_lines: u32,
    /// Vertical span of the original ink. See [`OcrBlock::source_height`].
    pub source_height: f32,
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

/// ONNX Runtime device for the OCR session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OcrDevice {
    /// WebGPU execution provider (Dawn on D3D12), with CPU fallback.
    #[default]
    Webgpu,
    /// DirectML execution provider (D3D12), with CPU fallback.
    Directml,
    /// CPU execution provider only.
    Cpu,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_folds_whitespace_ellipsis_and_ocr_punct() {
        assert_eq!(normalize_ocr_text("hello   world"), "hello world");
        assert_eq!(normalize_ocr_text("  a\n\tb  "), "a b");
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
        assert_eq!(normalize_ocr_text("何？"), "何?");
        assert_eq!(normalize_ocr_text("何?"), "何?");
        assert_eq!(normalize_ocr_text("嘘！"), "嘘!");
        assert_eq!(normalize_ocr_text("嘘!"), "嘘!");
        assert_eq!(normalize_ocr_text("そう〜"), "そう~");
        assert_eq!(normalize_ocr_text("そう～"), "そう~");
        assert_eq!(normalize_ocr_text("そう~"), "そう~");
        assert_eq!(normalize_ocr_text("1〜10"), "1~10");
    }

    #[test]
    fn rect_center_and_iou() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        assert_eq!(a.center(), (5.0, 5.0));
        assert_eq!(a.iou(a), 1.0);
        assert_eq!(a.iou(Rect::new(20.0, 0.0, 10.0, 10.0)), 0.0);
        // Half overlap gives an intersection of 50 and a union of 150.
        assert!((a.iou(Rect::new(5.0, 0.0, 10.0, 10.0)) - 1.0 / 3.0).abs() < 1e-6);
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
