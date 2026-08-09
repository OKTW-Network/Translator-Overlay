//! Pure overlay label placement helpers (no GDI).

use crate::draw::{SurfaceRect, SurfaceSize};

/// Label padding scales with font size so small UI text is not over-padded.
pub fn label_pad(font_px: i32) -> i32 {
    (font_px / 6).clamp(2, 6)
}

/// Place a label at the OCR origin; shift up only if it would go past the bottom.
pub fn place_label(base: SurfaceRect, box_w: i32, box_h: i32, surface: SurfaceSize) -> SurfaceRect {
    let box_w = box_w.clamp(1, surface.width.max(1));
    let box_h = box_h.min(surface.height.max(1)).max(1);
    let mut x = base.x;
    let mut y = base.y;
    if x + box_w > surface.width {
        x = (surface.width - box_w).max(0);
    }
    if y + box_h > surface.height {
        y = (surface.height - box_h).max(0);
    }
    SurfaceRect { x, y, w: box_w, h: box_h }
}
