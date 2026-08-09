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

/// Suggested font pixel height from the **source OCR line box**.
///
/// Merged paragraphs keep a one-line-tall anchor bbox, so this usually tracks
/// the original OCR line height. Tall leftovers still clamp to a readable size.
pub fn font_height_for(rect: SurfaceRect) -> i32 {
    let h = rect.h.max(1) as f32;
    let px = if h <= 48.0 {
        // Typical single OCR line — nearly fill the line box.
        (h * 0.88).round()
    } else {
        // Unexpected tall box — comfortable body size.
        (h * 0.35).round().clamp(20.0, 28.0)
    };
    px.clamp(16.0, 48.0) as i32
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
}
