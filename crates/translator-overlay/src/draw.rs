//! Software compositing for overlay bitmaps (BGRA, premultiplied alpha).

use translator_core::{OverlayConfig, Rect};

/// Axis-aligned rect in surface (overlay) pixel space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl SurfaceRect {
    pub fn clamp_to(self, width: i32, height: i32) -> Option<Self> {
        if width <= 0 || height <= 0 || self.w <= 0 || self.h <= 0 {
            return None;
        }
        let x0 = self.x.max(0);
        let y0 = self.y.max(0);
        let x1 = (self.x + self.w).min(width);
        let y1 = (self.y + self.h).min(height);
        let w = x1 - x0;
        let h = y1 - y0;
        if w <= 0 || h <= 0 {
            None
        } else {
            Some(Self { x: x0, y: y0, w, h })
        }
    }
}

/// Overlay surface dimensions and BGRA row stride.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceSize {
    pub width: i32,
    pub height: i32,
    pub stride: usize,
}

impl SurfaceSize {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            stride: (width.max(0) as usize) * 4,
        }
    }
}

/// Straight (non-premultiplied) RGBA colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgba {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

/// Map a capture-space OCR rect onto the overlay surface.
pub fn map_rect_to_surface(
    bbox: Rect,
    content_w: u32,
    content_h: u32,
    surface_w: i32,
    surface_h: i32,
) -> Option<SurfaceRect> {
    if content_w == 0 || content_h == 0 || surface_w <= 0 || surface_h <= 0 {
        return None;
    }
    if bbox.width <= 0.0 || bbox.height <= 0.0 {
        return None;
    }
    // Prefer 1:1 when painting in capture/OCR space (surface == content).
    // Still support scale when sizes differ (legacy / stretch present path).
    let sx = surface_w as f32 / content_w as f32;
    let sy = surface_h as f32 / content_h as f32;
    let x = (bbox.x * sx).round() as i32;
    let y = (bbox.y * sy).round() as i32;
    let w = (bbox.width * sx).round().max(1.0) as i32;
    let h = (bbox.height * sy).round().max(1.0) as i32;
    // Only enforce readable minima after a real downscale; in 1:1 OCR space
    // keep the true line box so positions stay pixel-accurate.
    let (w, h) = if (sx - 1.0).abs() < 0.01 && (sy - 1.0).abs() < 0.01 {
        (w, h)
    } else {
        (w.max(24), h.max(22))
    };
    SurfaceRect { x, y, w, h }.clamp_to(surface_w, surface_h)
}

pub fn argb_channels(argb: u32) -> (u8, u8, u8, u8) {
    let a = ((argb >> 24) & 0xFF) as u8;
    let r = ((argb >> 16) & 0xFF) as u8;
    let g = ((argb >> 8) & 0xFF) as u8;
    let b = (argb & 0xFF) as u8;
    (a, r, g, b)
}

pub fn background_rgba(cfg: &OverlayConfig) -> Rgba {
    let (a, r, g, b) = argb_channels(cfg.background_color_argb);
    Rgba::new(r, g, b, a)
}

pub fn text_rgba(cfg: &OverlayConfig) -> Rgba {
    let (a, r, g, b) = argb_channels(cfg.text_color_argb);
    Rgba::new(r, g, b, a)
}

#[inline]
fn write_premul(buf: &mut [u8], idx: usize, r: u8, g: u8, b: u8, a: u8) {
    let af = a as u32;
    buf[idx] = ((b as u32 * af) / 255) as u8;
    buf[idx + 1] = ((g as u32 * af) / 255) as u8;
    buf[idx + 2] = ((r as u32 * af) / 255) as u8;
    buf[idx + 3] = a;
}

/// Source-over composite of a straight (non-premultiplied) RGBA colour onto a
/// premultiplied BGRA destination pixel.
#[inline]
fn blend_over(buf: &mut [u8], idx: usize, r: u8, g: u8, b: u8, a: u8) {
    if a == 0 {
        return;
    }
    if a == 255 {
        write_premul(buf, idx, r, g, b, 255);
        return;
    }
    let src_a = a as u32;
    let inv = 255 - src_a;
    let dst_b = buf[idx] as u32;
    let dst_g = buf[idx + 1] as u32;
    let dst_r = buf[idx + 2] as u32;
    let dst_a = buf[idx + 3] as u32;

    let out_b = (b as u32 * src_a + dst_b * inv) / 255;
    let out_g = (g as u32 * src_a + dst_g * inv) / 255;
    let out_r = (r as u32 * src_a + dst_r * inv) / 255;
    let out_a = src_a + (dst_a * inv) / 255;

    buf[idx] = out_b.min(255) as u8;
    buf[idx + 1] = out_g.min(255) as u8;
    buf[idx + 2] = out_r.min(255) as u8;
    buf[idx + 3] = out_a.min(255) as u8;
}

/// Fill a rectangle with a straight RGBA colour (composited over existing).
pub fn fill_rect(buf: &mut [u8], surface: SurfaceSize, rect: SurfaceRect, color: Rgba) {
    let Some(rect) = rect.clamp_to(surface.width, surface.height) else {
        return;
    };
    for y in rect.y..(rect.y + rect.h) {
        let row = y as usize * surface.stride;
        for x in rect.x..(rect.x + rect.w) {
            let idx = row + x as usize * 4;
            blend_over(buf, idx, color.r, color.g, color.b, color.a);
        }
    }
}

/// Clear buffer to fully transparent.
pub fn clear(buf: &mut [u8]) {
    buf.fill(0);
}

/// Per-glyph box size in surface pixels (short side of the OCR rect).
///
/// - horizontal lines: height ≈ glyph size
/// - vertical / stacked columns: width ≈ glyph size (height is the full run)
pub fn char_box_px(rect: SurfaceRect) -> i32 {
    rect.w.max(1).min(rect.h.max(1))
}

/// Initial CreateFontW character-height estimate from the OCR line box.
///
/// Conservative on purpose: GDI/Segoe UI glyphs read larger than the raw
/// CreateFont height suggests, and vertical OCR boxes are often padded.
/// The overlay host further shrinks with GetTextMetrics so the cell fits.
pub fn font_height_for(rect: SurfaceRect) -> i32 {
    let w = rect.w.max(1) as f32;
    let h = rect.h.max(1) as f32;
    let char_box = w.min(h);
    // Tall thin = vertical run; detector width is usually looser than ink.
    let vertical = h > w * 1.5;
    let fill = if vertical { 0.58 } else { 0.68 };
    let px = (char_box * fill).round();
    px.clamp(8.0, 128.0) as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use translator_core::Rect;

    #[test]
    fn map_rect_scales() {
        let r = Rect::new(10.0, 20.0, 100.0, 40.0);
        let s = map_rect_to_surface(r, 200, 100, 400, 200).unwrap();
        assert_eq!(s.x, 20);
        assert_eq!(s.y, 40);
        assert_eq!(s.w, 200);
        assert_eq!(s.h, 80);
    }

    #[test]
    fn map_rect_rejects_empty() {
        assert!(map_rect_to_surface(Rect::new(0.0, 0.0, 0.0, 10.0), 100, 100, 100, 100).is_none());
        assert!(map_rect_to_surface(Rect::new(0.0, 0.0, 10.0, 10.0), 0, 100, 100, 100).is_none());
    }

    #[test]
    fn fill_rect_sets_alpha() {
        let mut buf = vec![0u8; 4 * 4 * 4];
        fill_rect(
            &mut buf,
            SurfaceSize::new(4, 4),
            SurfaceRect {
                x: 1,
                y: 1,
                w: 2,
                h: 2,
            },
            Rgba::new(255, 0, 0, 128),
        );
        // pixel (1,1): row stride 16, 4 bytes per pixel
        let i = 16 + 4;
        assert_eq!(buf[i + 3], 128);
        assert_eq!(buf[i + 2], 128); // premultiplied red
        // pixel (0,0) untouched
        assert_eq!(buf[3], 0);
    }

    #[test]
    fn argb_parse() {
        assert_eq!(argb_channels(0xC800_00FF), (0xC8, 0x00, 0x00, 0xFF));
    }

    #[test]
    fn font_height_tracks_line_box() {
        // Small UI label (~12px OCR) must not jump to a fixed 16px floor.
        let small = SurfaceRect {
            x: 0,
            y: 0,
            w: 80,
            h: 12,
        };
        let small_px = font_height_for(small);
        assert!(
            (8..=10).contains(&small_px),
            "small line box → ~8px font, got {small_px}"
        );

        // Typical body line — under box height so glyphs don't overflow ink.
        let body = SurfaceRect {
            x: 0,
            y: 0,
            w: 200,
            h: 28,
        };
        let body_px = font_height_for(body);
        assert!(
            (16..=22).contains(&body_px),
            "body line → ~19px font, got {body_px}"
        );
        assert!(body_px < 28, "font must stay under line box height");

        // Large title scales with box (not crushed to 20–28).
        let title = SurfaceRect {
            x: 0,
            y: 0,
            w: 300,
            h: 64,
        };
        let title_px = font_height_for(title);
        assert!(
            (40..=50).contains(&title_px),
            "title line → ~43px font, got {title_px}"
        );
    }

    #[test]
    fn font_height_vertical_uses_width() {
        // Tall thin column (vertical CJK / stacked UI): char size ≈ width, not height.
        let vertical = SurfaceRect {
            x: 0,
            y: 0,
            w: 24,
            h: 220,
        };
        assert_eq!(char_box_px(vertical), 24);
        let px = font_height_for(vertical);
        // Vertical fill is more conservative (~0.58 of width).
        assert!(
            (12..=16).contains(&px),
            "vertical text should size from width (~14px), got {px}"
        );
        // Must not treat full column height as font size.
        assert!(px < 40, "vertical text must not use full height, got {px}");
    }
}
