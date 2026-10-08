//! Crop a tightly packed RGBA8 frame for OCR input.

use translator_core::Rect;

/// Integer crop of a packed RGB8 buffer (`width * height * 3`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rgb8Crop {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub rgb: Vec<u8>,
}

/// Smallest crop side in pixels that is sent to the detector.
pub const MIN_CROP_PX: u32 = 8;

/// Crop `rgba` to `rect`, given in capture pixels, and drop alpha in the same pass.
///
/// OCR detection and recognition take RGB8. Cropping straight to RGB8 avoids a
/// separate RGBA crop buffer and a second conversion pass.
pub fn crop_to_rgb8(width: u32, height: u32, rgba: &[u8], rect: Rect) -> Option<Rgb8Crop> {
    if width == 0 || height == 0 {
        return None;
    }
    let expected = (width as usize).checked_mul(height as usize).and_then(|n| n.checked_mul(4))?;
    if rgba.len() < expected {
        return None;
    }

    let (x0, y0, x1, y1) = rect.clamped_bounds(width, height);
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    if x1 - x0 < MIN_CROP_PX || y1 - y0 < MIN_CROP_PX {
        return None;
    }

    let cw = (x1 - x0) as usize;
    let mut out = Vec::with_capacity(cw * (y1 - y0) as usize * 3);
    let row_w = width as usize;
    for y in y0..y1 {
        let row = &rgba[(y as usize * row_w + x0 as usize) * 4..(y as usize * row_w + x1 as usize) * 4];
        for px in row.as_chunks::<4>().0 {
            out.extend_from_slice(&px[..3]);
        }
    }

    Some(Rgb8Crop {
        x: x0,
        y: y0,
        width: x1 - x0,
        height: y1 - y0,
        rgb: out,
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
    fn crop_copies_window_and_drops_alpha() {
        let w = 16u32;
        let h = 16u32;
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        // A unique pixel at (5, 3).
        let i = (3 * w + 5) as usize * 4;
        rgba[i] = 9;
        rgba[i + 1] = 8;
        rgba[i + 2] = 7;
        rgba[i + 3] = 6;
        let crop = crop_to_rgb8(w, h, &rgba, Rect::new(4.0, 2.0, 8.0, 8.0)).unwrap();
        assert_eq!(crop.x, 4);
        assert_eq!(crop.y, 2);
        assert_eq!(crop.width, 8);
        assert_eq!(crop.height, 8);
        // Source (5, 3) lands at (1, 1) in the 8-wide crop, at 3 bytes per pixel.
        let dest = (8 + 1) * 3;
        assert_eq!(&crop.rgb[dest..dest + 3], &[9, 8, 7]);
    }

    #[test]
    fn crop_rejects_tiny_and_oob() {
        let rgba = solid(32, 32, 1, 2, 3, 255);
        assert!(crop_to_rgb8(32, 32, &rgba, Rect::new(0.0, 0.0, 4.0, 4.0)).is_none());
        assert!(crop_to_rgb8(32, 32, &rgba, Rect::new(40.0, 40.0, 10.0, 10.0)).is_none());
        assert!(crop_to_rgb8(32, 32, &rgba, Rect::new(0.0, 0.0, 16.0, 16.0)).is_some());
        assert!(crop_to_rgb8(32, 32, &rgba[..rgba.len() - 1], Rect::new(0.0, 0.0, 16.0, 16.0)).is_none(), "short buffer");
    }

    #[test]
    fn whole_frame_crop_keeps_rgb_order() {
        let rgba = [1u8, 2, 3, 255, 4, 5, 6, 128].repeat(32);
        let crop = crop_to_rgb8(8, 8, &rgba, Rect::new(0.0, 0.0, 8.0, 8.0)).unwrap();
        assert_eq!((crop.x, crop.y, crop.width, crop.height), (0, 0, 8, 8));
        assert_eq!(&crop.rgb[..6], &[1, 2, 3, 4, 5, 6]);
        assert_eq!(crop.rgb.len(), 8 * 8 * 3);
    }
}
