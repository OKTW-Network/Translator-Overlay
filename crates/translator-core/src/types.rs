//! Shared domain types used across crates.

use serde::{Deserialize, Serialize};

/// Axis-aligned bounding box in capture-image pixel coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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

        let (cx0, cy0) = self.center();
        let (cx1, cy1) = candidate.center();
        let dx = (cx0 - cx1).abs();
        let dy = (cy0 - cy1).abs();
        let tol_x = (self.width.max(candidate.width) * 0.22).max(10.0);
        let tol_y = (self.height.max(candidate.height) * 0.40).max(8.0);
        dx > tol_x || dy > tol_y
    }

    /// Keep `self` when `candidate` is only OCR jitter; otherwise take `candidate`.
    pub fn stabilize_against(self, candidate: Self) -> Self {
        if self.is_significant_relayout(candidate) { candidate } else { self }
    }
}

fn default_source_lines() -> u32 {
    1
}

/// One OCR text region.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OcrBlock {
    pub id: u32,
    pub text: String,
    pub confidence: f32,
    pub bbox: Rect,
    /// Detector lines merged into this block (`1` = single line).
    #[serde(default = "default_source_lines")]
    pub source_lines: u32,
}

/// OCR block after translation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TranslatedBlock {
    pub id: u32,
    pub source: String,
    pub translation: String,
    pub confidence: f32,
    pub bbox: Rect,
    /// Detector lines in the source (`1` = single line; overlay may widen/shrink).
    #[serde(default = "default_source_lines")]
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

impl ModelTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Tiny => "tiny",
            Self::Small => "small",
            Self::Medium => "medium",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "tiny" => Some(Self::Tiny),
            "small" => Some(Self::Small),
            "medium" => Some(Self::Medium),
            _ => None,
        }
    }
}

impl std::fmt::Display for ModelTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
