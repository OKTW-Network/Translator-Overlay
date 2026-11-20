//! Capture preview → WinUI `Image` via a `WriteableBitmap`.

use bytes::Bytes;
use rust_i18n::t;
use translator_core::NormRect;
use translator_winui::{Image, Layout, Result, Stretch, TextBlock, bitmap};

use crate::ui::widgets::{Prop, text};

struct CachedPreview {
    sequence: u64,
    scale_cents: u32,
    regions: Vec<NormRect>,
    width: u32,
    height: u32,
}

fn scale_cents(scale: f32) -> u32 {
    (scale * 100.0).round() as u32
}

/// XAML bitmaps hold premultiplied BGRA8.
fn rgba_to_premul_bgra(rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgba.len());
    let (pixels, _) = rgba.as_chunks::<4>();
    if pixels.iter().all(|px| px[3] == 255) {
        for px in pixels {
            out.extend_from_slice(&[px[2], px[1], px[0], 255]);
        }
    } else {
        for px in pixels {
            let a = u16::from(px[3]);
            out.extend_from_slice(&[
                ((u16::from(px[2]) * a) / 255) as u8,
                ((u16::from(px[1]) * a) / 255) as u8,
                ((u16::from(px[0]) * a) / 255) as u8,
                px[3],
            ]);
        }
    }
    out
}

fn dip_size(px: u32, scale: f32) -> f32 {
    (px as f32 / scale).max(1.0)
}

/// Outlines each region with a 2 DIP stroke centered on its edge, blended over `bgra`.
fn outline_regions(bgra: &mut [u8], width: u32, height: u32, scale: f32, regions: &[NormRect]) {
    const BGR: [u32; 3] = [255, 220, 80];
    const ALPHA: u32 = 230;
    let half = scale.max(0.5);
    let mut fill = |left: f32, top: f32, right: f32, bottom: f32| {
        let clamp = |v: f32, max: u32| (v.round().max(0.0) as u32).min(max);
        for y in clamp(top, height)..clamp(bottom, height) {
            for x in clamp(left, width)..clamp(right, width) {
                let i = (y as usize * width as usize + x as usize) * 4;
                let px = &mut bgra[i..i + 4];
                for (c, &v) in BGR.iter().enumerate() {
                    px[c] = ((v * ALPHA + u32::from(px[c]) * (255 - ALPHA)) / 255) as u8;
                }
                px[3] = (ALPHA + u32::from(px[3]) * (255 - ALPHA) / 255) as u8;
            }
        }
    };
    for region in regions {
        let Some(n) = region.sanitize() else {
            continue;
        };
        let px = n.to_pixel(width, height);
        let (left, top) = (px.x, px.y);
        let right = left + px.width.max(scale);
        let bottom = top + px.height.max(scale);
        fill(left - half, top - half, right + half, top + half);
        fill(left - half, bottom - half, right + half, bottom + half);
        fill(left - half, top + half, left + half, bottom - half);
        fill(right - half, top + half, right + half, bottom - half);
    }
}

/// Latest frame and the regions to outline on it.
pub struct PreviewInput<'a> {
    pub sequence: u64,
    pub width: u32,
    pub height: u32,
    pub rgba: Option<&'a Bytes>,
    pub regions: &'a [NormRect],
}

#[derive(PartialEq, Clone, Copy)]
enum Shown {
    None,
    Image,
    Unavailable,
}

/// The preview image, or one of two placeholder lines; exactly one is visible.
pub struct Preview {
    pub image: Image,
    pub none: TextBlock,
    pub unavailable: TextBlock,
    shown: Prop<Shown>,
    size: Prop<(u64, u64)>,
    /// Frame last set on `image`. Per instance, so a rebuilt page sets its own source.
    cache: Option<CachedPreview>,
}

fn placeholder(s: &str) -> Result<TextBlock> {
    let tb = text(s)?;
    tb.SetFontSize(12.0)?;
    Ok(tb)
}

impl Preview {
    pub fn new() -> Result<Self> {
        let image = Image::new()?;
        image.SetStretch(Stretch::Uniform)?;
        Ok(Self {
            image,
            none: placeholder(&t!("preview.none"))?,
            unavailable: placeholder(&t!("preview.unavailable"))?,
            shown: Prop::default(),
            size: Prop::default(),
            cache: None,
        })
    }

    pub fn sync(&mut self, input: &PreviewInput, scale: f32) -> Result<()> {
        let shown = match input.rgba.filter(|b| !b.is_empty() && input.width > 0 && input.height > 0) {
            None => Shown::None,
            Some(rgba) => {
                let cents = scale_cents(scale);
                let current = self.cache.as_ref().is_some_and(|cached| {
                    cached.sequence == input.sequence
                        && cached.scale_cents == cents
                        && cached.width == input.width
                        && cached.height == input.height
                        && cached.regions == input.regions
                });
                let ready = current || {
                    let mut bgra = rgba_to_premul_bgra(rgba);
                    outline_regions(&mut bgra, input.width, input.height, scale, input.regions);
                    match bitmap(input.width, input.height, &bgra) {
                        Ok(image) => {
                            self.image.SetSource(&image)?;
                            self.cache = Some(CachedPreview {
                                sequence: input.sequence,
                                scale_cents: cents,
                                regions: input.regions.to_vec(),
                                width: input.width,
                                height: input.height,
                            });
                            true
                        }
                        Err(_) => false,
                    }
                };
                if ready {
                    let dip = (f64::from(dip_size(input.width, scale)), f64::from(dip_size(input.height, scale)));
                    let image = &self.image;
                    self.size.set((dip.0.to_bits(), dip.1.to_bits()), |&(w, h)| {
                        image.max_width(f64::from_bits(w))?.max_height(f64::from_bits(h))?;
                        Ok(())
                    })?;
                    Shown::Image
                } else {
                    Shown::Unavailable
                }
            }
        };
        let (image, none, unavailable) = (&self.image, &self.none, &self.unavailable);
        self.shown.set(shown, |&s| {
            image.visible(s == Shown::Image)?;
            none.visible(s == Shown::None)?;
            unavailable.visible(s == Shown::Unavailable)?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn premul_converts_opaque_mixed_zero_alpha_and_empty() {
        assert!(rgba_to_premul_bgra(&[]).is_empty());

        let opaque: &[u8] = &[10, 20, 30, 255, 200, 150, 100, 255];
        assert_eq!(rgba_to_premul_bgra(opaque), vec![30, 20, 10, 255, 100, 150, 200, 255]);

        // Zero alpha zeroes out channels; mixed alpha floors R/G/B * A / 255 and flips R<->B.
        let mixed: &[u8] = &[100, 200, 255, 0, 10, 20, 30, 128, 0, 255, 0, 51, 200, 150, 100, 255];
        assert_eq!(rgba_to_premul_bgra(mixed), vec![
            0, 0, 0, 0, // a=0 -> 0 premul channels
            15, 10, 5, 128, // 30*128/255=15, 20*128/255=10, 10*128/255=5
            0, 51, 0, 51, // 255*51/255=51
            100, 150, 200, 255,
        ]);
    }

    #[test]
    fn outline_covers_the_edge_only() {
        let (w, h) = (10, 10);
        let mut bgra = vec![0u8; w * h * 4];
        let region = NormRect::new(0.2, 0.2, 0.6, 0.6);
        outline_regions(&mut bgra, w as u32, h as u32, 1.0, &[region]);
        let alpha = |x: usize, y: usize| bgra[(y * w + x) * 4 + 3];
        // The pixel-space rect is (2,2)-(8,8); a 2 px stroke spans 1..3 and 7..9 on each axis.
        for (x, y) in [(1, 1), (2, 5), (5, 2), (8, 8), (7, 4)] {
            assert_eq!(alpha(x, y), 230, "edge pixel ({x},{y})");
        }
        for (x, y) in [(0, 0), (5, 5), (3, 3), (9, 9)] {
            assert_eq!(alpha(x, y), 0, "pixel ({x},{y}) outside the stroke");
        }
    }

    #[test]
    fn outline_regions_clamps_scales_blends_and_skips_invalid() {
        // Invalid NaN / degenerate rects leave background completely untouched.
        let mut bgra = vec![40u8; 8 * 8 * 4];
        let orig = bgra.clone();
        outline_regions(&mut bgra, 8, 8, 1.0, &[NormRect::new(f32::NAN, 0.0, 0.5, 0.5), NormRect::new(0.1, 0.1, 0.001, 0.5)]);
        assert_eq!(bgra, orig);

        // Edge-touching region at scale=2.0 clamps cleanly to 0..8 and blends BGR [255,220,80] at A=230 over bg [10,20,30,255].
        let mut scaled = vec![0u8; 8 * 8 * 4];
        for px in scaled.as_chunks_mut::<4>().0 {
            px.copy_from_slice(&[10, 20, 30, 255]);
        }
        outline_regions(&mut scaled, 8, 8, 2.0, &[NormRect::new(0.0, 0.0, 1.0, 1.0)]);
        // Corner (0, 0) blended: B=(255*230+10*25)/255=230, G=(220*230+20*25)/255=200, R=(80*230+30*25)/255=75, A=255.
        assert_eq!(&scaled[0..4], &[230, 200, 75, 255]);
        // Center (4, 4) untouched (stroke half=2.0 only reaches rows/cols 0..2 and 6..8).
        let mid = (4 * 8 + 4) * 4;
        assert_eq!(&scaled[mid..mid + 4], &[10, 20, 30, 255]);
    }
}
