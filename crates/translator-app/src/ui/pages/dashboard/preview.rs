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

    fn reference(rgba: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        for px in rgba.as_chunks::<4>().0 {
            out.push((u32::from(px[2]) * u32::from(px[3]) / 255) as u8);
            out.push((u32::from(px[1]) * u32::from(px[3]) / 255) as u8);
            out.push((u32::from(px[0]) * u32::from(px[3]) / 255) as u8);
            out.push(px[3]);
        }
        out
    }

    #[test]
    fn premul_matches_reference_for_opaque_and_mixed_alpha() {
        let opaque: &[u8] = &[10, 20, 30, 255, 200, 150, 100, 255];
        let mixed: &[u8] = &[10, 20, 30, 128, 200, 150, 100, 255, 0, 255, 0, 51];
        for rgba in [opaque, mixed] {
            assert_eq!(rgba_to_premul_bgra(rgba), reference(rgba));
        }
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
}
