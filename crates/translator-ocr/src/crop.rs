//! Crop a tightly packed RGBA8 frame to an axis-aligned pixel rect.

use translator_core::Rect;

/// Integer crop of a packed RGBA8 buffer (`width * height * 4`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaCrop {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Smallest crop we will send to the detector (px).
pub const MIN_CROP_PX: u32 = 8;

/// Crop `rgba` to `rect` (capture-pixel space), clamped to the frame.
pub fn crop_rgba(width: u32, height: u32, rgba: &[u8], rect: Rect) -> Option<RgbaCrop> {
    if width == 0 || height == 0 {
        return None;
    }
    let expected = (width as usize).checked_mul(height as usize).and_then(|n| n.checked_mul(4))?;
    if rgba.len() < expected {
        return None;
    }

    let x0 = rect.x.round().clamp(0.0, width as f32) as u32;
    let y0 = rect.y.round().clamp(0.0, height as f32) as u32;
    let x1 = (rect.x + rect.width).round().clamp(0.0, width as f32) as u32;
    let y1 = (rect.y + rect.height).round().clamp(0.0, height as f32) as u32;
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    let cw = x1 - x0;
    let ch = y1 - y0;
    if cw < MIN_CROP_PX || ch < MIN_CROP_PX {
        return None;
    }

    let mut out = Vec::with_capacity((cw as usize) * (ch as usize) * 4);
    let row_w = width as usize;
    for y in y0..y1 {
        let start = (y as usize * row_w + x0 as usize) * 4;
        let end = start + cw as usize * 4;
        out.extend_from_slice(&rgba[start..end]);
    }

    Some(RgbaCrop {
        x: x0,
        y: y0,
        width: cw,
        height: ch,
        rgba: out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, r: u8, g: u8, b: u8, a: u8) -> Vec<u8> {
        let mut v = Vec::with_capacity((w * h * 4) as usize);
        for _ in 0..(w * h) {
            v.extend_from_slice(&[r, g, b, a]);
        }
        v
    }

    #[test]
    fn crop_copies_window() {
        let w = 16u32;
        let h = 16u32;
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        // Unique pixel at (5, 3)
        let i = (3 * w + 5) as usize * 4;
        rgba[i] = 9;
        rgba[i + 1] = 8;
        rgba[i + 2] = 7;
        rgba[i + 3] = 6;
        let crop = crop_rgba(w, h, &rgba, Rect::new(4.0, 2.0, 8.0, 8.0)).unwrap();
        assert_eq!(crop.x, 4);
        assert_eq!(crop.y, 2);
        assert_eq!(crop.width, 8);
        assert_eq!(crop.height, 8);
        // source (5, 3) → crop-local (1, 1) in an 8-wide crop
        let dest = (8 + 1) * 4;
        assert_eq!(&crop.rgba[dest..dest + 4], &[9, 8, 7, 6]);
    }

    #[test]
    fn crop_rejects_tiny_and_oob() {
        let rgba = solid(32, 32, 1, 2, 3, 255);
        assert!(crop_rgba(32, 32, &rgba, Rect::new(0.0, 0.0, 4.0, 4.0)).is_none());
        assert!(crop_rgba(32, 32, &rgba, Rect::new(40.0, 40.0, 10.0, 10.0)).is_none());
        assert!(crop_rgba(32, 32, &rgba, Rect::new(0.0, 0.0, 16.0, 16.0)).is_some());
    }
}
