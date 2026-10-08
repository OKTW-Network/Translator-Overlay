//! Software compositing for overlay bitmaps in BGRA with premultiplied alpha.

use translator_core::Rect;

/// Axis-aligned rect in overlay surface pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl SurfaceRect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

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
        if w <= 0 || h <= 0 { None } else { Some(Self::new(x0, y0, w, h)) }
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

/// Label padding scales with font size so small UI text is not over-padded.
pub fn label_pad(font_px: i32) -> i32 {
    (font_px / 6).clamp(2, 6)
}

/// Place a label at the OCR origin, and shift it up only if it would go past the bottom.
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
    SurfaceRect::new(x, y, box_w, box_h)
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

    /// Colour from a config value in `#AARRGGBB` form.
    pub const fn from_argb(argb: u32) -> Self {
        let [a, r, g, b] = argb.to_be_bytes();
        Self { r, g, b, a }
    }
}

/// Map a capture-space OCR rect onto the overlay surface.
pub fn map_rect_to_surface(bbox: Rect, content_w: u32, content_h: u32, surface_w: i32, surface_h: i32) -> Option<SurfaceRect> {
    if content_w == 0 || content_h == 0 || surface_w <= 0 || surface_h <= 0 {
        return None;
    }
    if bbox.width <= 0.0 || bbox.height <= 0.0 {
        return None;
    }
    // Painting normally happens 1:1 in capture pixels, where the surface equals the content.
    // Scaling still works when the sizes differ.
    let sx = surface_w as f32 / content_w as f32;
    let sy = surface_h as f32 / content_h as f32;
    let x = (bbox.x * sx).round() as i32;
    let y = (bbox.y * sy).round() as i32;
    let w = (bbox.width * sx).round().max(1.0) as i32;
    let h = (bbox.height * sy).round().max(1.0) as i32;
    // Enforce readable minimum sizes only after a real downscale. At 1:1, keep the
    // true line box so positions stay pixel-accurate.
    let (w, h) = if (sx - 1.0).abs() < 0.01 && (sy - 1.0).abs() < 0.01 {
        (w, h)
    } else {
        (w.max(24), h.max(22))
    };
    SurfaceRect::new(x, y, w, h).clamp_to(surface_w, surface_h)
}

/// Source-over composite of a straight (non-premultiplied) RGBA colour onto a
/// premultiplied BGRA destination pixel.
#[inline]
pub(crate) fn blend_over(buf: &mut [u8], idx: usize, color: Rgba) {
    let Rgba { r, g, b, a } = color;
    if a == 0 {
        return;
    }
    if a == 255 {
        buf[idx..idx + 4].copy_from_slice(&[b, g, r, 255]);
        return;
    }
    let src_a = a as u32;
    let inv = 255 - src_a;
    let over = |src: u8, dst: u8| ((src as u32 * src_a + dst as u32 * inv) / 255).min(255) as u8;
    buf[idx] = over(b, buf[idx]);
    buf[idx + 1] = over(g, buf[idx + 1]);
    buf[idx + 2] = over(r, buf[idx + 2]);
    buf[idx + 3] = (src_a + (buf[idx + 3] as u32 * inv) / 255).min(255) as u8;
}

/// Stroke an axis-aligned rectangle, `thickness` pixels inward from the edges of `rect`.
pub fn stroke_rect(buf: &mut [u8], surface: SurfaceSize, rect: SurfaceRect, color: Rgba, thickness: i32) {
    let t = thickness.max(1);
    let Some(SurfaceRect { x, y, w, h }) = rect.clamp_to(surface.width, surface.height) else {
        return;
    };
    for edge in [
        SurfaceRect::new(x, y, w, t),
        SurfaceRect::new(x, (y + h - t).max(y), w, t),
        SurfaceRect::new(x, y, t, h),
        SurfaceRect::new((x + w - t).max(x), y, t, h),
    ] {
        fill_rect(buf, surface, edge, color);
    }
}

/// Composite a straight RGBA colour over a rectangle.
pub fn fill_rect(buf: &mut [u8], surface: SurfaceSize, rect: SurfaceRect, color: Rgba) {
    let Some(rect) = rect.clamp_to(surface.width, surface.height) else {
        return;
    };
    for y in rect.y..(rect.y + rect.h) {
        let row = y as usize * surface.stride;
        for x in rect.x..(rect.x + rect.w) {
            blend_over(buf, row + x as usize * 4, color);
        }
    }
}

/// First estimate of the CreateFontW character height from the OCR line box.
///
/// It runs small on purpose. Segoe UI glyphs in GDI look larger than the raw
/// CreateFont height suggests, and vertical OCR boxes are often padded. The
/// overlay host then shrinks the font with GetTextMetrics until the cell fits.
pub fn font_height_for(rect: SurfaceRect) -> i32 {
    let w = rect.w.max(1) as f32;
    let h = rect.h.max(1) as f32;
    let char_box = w.min(h);
    // A tall, thin box is a vertical run. The detector's width is usually looser than the ink.
    let vertical = h > w * 1.5;
    let fill = if vertical { 0.58 } else { 0.68 };
    let px = (char_box * fill).round();
    px.clamp(8.0, 128.0) as i32
}

#[cfg(test)]
mod tests {
    use translator_core::Rect;

    use super::*;

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
    fn stroke_rect_draws_border() {
        let mut buf = vec![0u8; 8 * 8 * 4];
        let surface = SurfaceSize::new(8, 8);
        stroke_rect(&mut buf, surface, SurfaceRect { x: 1, y: 1, w: 6, h: 6 }, Rgba::new(0, 0, 255, 255), 1);
        // The stroke's top-left pixel is (1, 1) on a stride of 8.
        let i = (8 + 1) * 4;
        assert_eq!(buf[i], 255); // B
        assert_eq!(buf[i + 3], 255);
        // The interior pixel (3, 3) is untouched.
        let mid = (3 * 8 + 3) * 4;
        assert_eq!(buf[mid + 3], 0);
    }

    #[test]
    fn fill_rect_sets_alpha() {
        let mut buf = vec![0u8; 4 * 4 * 4];
        fill_rect(&mut buf, SurfaceSize::new(4, 4), SurfaceRect { x: 1, y: 1, w: 2, h: 2 }, Rgba::new(255, 0, 0, 128));
        // Pixel (1, 1) with a row stride of 16 and 4 bytes per pixel.
        let i = 16 + 4;
        assert_eq!(buf[i + 3], 128);
        assert_eq!(buf[i + 2], 128); // premultiplied red
        // Pixel (0, 0) is untouched.
        assert_eq!(buf[3], 0);
    }

    #[test]
    fn font_height_tracks_line_box() {
        // A small UI label, about 12 px in OCR, must not jump to a fixed 16 px floor.
        let small = SurfaceRect { x: 0, y: 0, w: 80, h: 12 };
        let small_px = font_height_for(small);
        assert!((8..=10).contains(&small_px), "small line box → ~8px font, got {small_px}");

        // A typical body line stays under the box height so glyphs do not overflow the ink.
        let body = SurfaceRect { x: 0, y: 0, w: 200, h: 28 };
        let body_px = font_height_for(body);
        assert!((16..=22).contains(&body_px), "body line → ~19px font, got {body_px}");
        assert!(body_px < 28, "font must stay under line box height");

        // A large title scales with its box instead of being squeezed into 20 to 28 px.
        let title = SurfaceRect { x: 0, y: 0, w: 300, h: 64 };
        let title_px = font_height_for(title);
        assert!((40..=50).contains(&title_px), "title line → ~43px font, got {title_px}");
    }

    #[test]
    fn font_height_vertical_uses_width() {
        // In a tall, thin column (vertical CJK or stacked UI), the character size follows the width, not the height.
        let vertical = SurfaceRect { x: 0, y: 0, w: 24, h: 220 };
        let px = font_height_for(vertical);
        // Vertical text fills less, about 0.58 of the width.
        assert!((12..=16).contains(&px), "vertical text should size from width (~14px), got {px}");
        // The full column height must not become the font size.
        assert!(px < 40, "vertical text must not use full height, got {px}");
    }

    #[test]
    fn place_label_shifts_up_past_the_bottom() {
        let surface = SurfaceSize::new(200, 100);
        let base = SurfaceRect {
            x: 10,
            y: 80,
            w: 80,
            h: 16,
        };
        let placed = place_label(base, 80, 40, surface);
        assert_eq!(placed.y, 60);
        assert_eq!(placed.h, 40);
    }
}
