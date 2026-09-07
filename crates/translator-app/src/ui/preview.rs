//! Capture preview → WinUI `Image` via D2D (`CanvasImageSource`).

use std::{
    cell::{Cell, RefCell},
    hash::{Hash, Hasher},
};

use bytes::Bytes;
use translator_core::NormRect;
use windows_canvas::{AlphaMode, CanvasImageSource, ColorF, GpuDevice, Rect};
use windows_reactor::{
    Component, ComponentContext, ElementObservation, ElementRef, HorizontalAlignment, Image, LayoutControl, Stretch, TextBlock, View,
    ViewContext,
};

thread_local! {
    static GPU: RefCell<Option<GpuDevice>> = const { RefCell::new(None) };
    static CACHE: RefCell<Option<CachedPreview>> = const { RefCell::new(None) };
    static SCALE: Cell<f32> = const { Cell::new(1.0) };
}

struct CachedPreview {
    sequence: u64,
    scale_cents: u32,
    regions_key: u64,
    source: CanvasImageSource,
    width: u32,
    height: u32,
}

fn regions_key(regions: &[NormRect]) -> u64 {
    let mut h = std::hash::DefaultHasher::new();
    regions.len().hash(&mut h);
    for r in regions {
        r.x.to_bits().hash(&mut h);
        r.y.to_bits().hash(&mut h);
        r.width.to_bits().hash(&mut h);
        r.height.to_bits().hash(&mut h);
    }
    h.finish()
}

fn gpu_device() -> Option<GpuDevice> {
    GPU.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = GpuDevice::new_or_warp().ok();
        }
        slot.clone()
    })
}

fn scale_cents(scale: f32) -> u32 {
    (scale * 100.0).round() as u32
}

/// D2D `create_bitmap_with_alpha` expects premultiplied BGRA8.
fn rgba_to_premul_bgra(rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgba.len());
    if rgba.chunks_exact(4).all(|px| px[3] == 255) {
        for px in rgba.chunks_exact(4) {
            out.extend_from_slice(&[px[2], px[1], px[0], 255]);
        }
    } else {
        for px in rgba.chunks_exact(4) {
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

fn build_source(
    device: &GpuDevice,
    rgba: &Bytes,
    width: u32,
    height: u32,
    scale: f32,
    regions: &[NormRect],
) -> windows_canvas::Result<CanvasImageSource> {
    let w = width.max(1);
    let h = height.max(1);
    let dip_w = dip_size(w, scale);
    let dip_h = dip_size(h, scale);
    let bgra = rgba_to_premul_bgra(rgba);
    let source = CanvasImageSource::new(device, dip_w, dip_h, scale)?;
    source.draw(ColorF::TRANSPARENT, |session| {
        let bitmap = session.create_bitmap_with_alpha(&bgra, w, h, AlphaMode::Premultiplied)?;
        session.draw_bitmap(&bitmap, &Rect::from_xywh(0.0, 0.0, dip_w, dip_h), 1.0);
        if !regions.is_empty() {
            let brush = session.create_solid_brush(ColorF::from_rgba8(80, 220, 255, 230))?;
            for region in regions {
                let Some(n) = region.sanitize() else {
                    continue;
                };
                let px = n.to_pixel(w, h);
                let rx = px.x / scale;
                let ry = px.y / scale;
                let rw = (px.width / scale).max(1.0);
                let rh = (px.height / scale).max(1.0);
                session.draw_rect(&Rect::from_xywh(rx, ry, rw, rh), &brush, 2.0);
            }
        }
        Ok(())
    })?;
    Ok(source)
}

#[derive(Clone)]
pub struct PreviewInput {
    pub sequence: u64,
    pub width: u32,
    pub height: u32,
    pub rgba: Option<Bytes>,
    pub regions: Vec<NormRect>,
}

impl PartialEq for PreviewInput {
    fn eq(&self, other: &Self) -> bool {
        self.sequence == other.sequence
            && self.width == other.width
            && self.height == other.height
            && regions_key(&self.regions) == regions_key(&other.regions)
    }
}

pub struct CapturePreview {
    image_ref: ElementRef<Image>,
    _scale_watch: ElementObservation,
    attach_retry: Cell<u8>,
}

impl Component for CapturePreview {
    type Input = PreviewInput;
    type Message = ();

    fn create(_input: &PreviewInput, context: &ComponentContext<Self>) -> Self {
        let image_ref = ElementRef::new();
        let sender = context.sender();
        let scale_watch = image_ref.observe_rasterization_scale(move |scale| {
            let scale = if scale > 0.0 { scale as f32 } else { 1.0 };
            let prev = scale_cents(SCALE.get());
            SCALE.set(scale);
            if scale_cents(scale) != prev {
                let _ = sender.send(());
            }
        });
        Self {
            image_ref,
            _scale_watch: scale_watch,
            attach_retry: Cell::new(0),
        }
    }

    fn input_changed(&mut self, _input: &PreviewInput, _context: &ComponentContext<Self>) {
        self.attach_retry.set(0);
    }

    fn update(&mut self, _message: Self::Message, _context: &ComponentContext<Self>) {
        self.attach_retry.set(1);
    }

    fn view(&self, input: &PreviewInput, context: &mut ViewContext<Self>) -> View {
        let Some(rgba) = input.rgba.as_ref().filter(|b| !b.is_empty() && input.width > 0 && input.height > 0) else {
            return TextBlock::new()
                .text("No capture yet")
                .font_size(12.0)
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .into();
        };

        let scale = SCALE.get();
        let cents = scale_cents(scale);
        let dip_w = f64::from(dip_size(input.width, scale));
        let dip_h = f64::from(dip_size(input.height, scale));
        let rkey = regions_key(&input.regions);

        let source = CACHE.with(|cell| {
            let mut cache = cell.borrow_mut();
            if let Some(cached) = cache.as_ref()
                && cached.sequence == input.sequence
                && cached.scale_cents == cents
                && cached.width == input.width
                && cached.height == input.height
                && cached.regions_key == rkey
            {
                return Some(cached.source.clone());
            }
            let device = gpu_device()?;
            let source = build_source(&device, rgba, input.width, input.height, scale, &input.regions).ok()?;
            *cache = Some(CachedPreview {
                sequence: input.sequence,
                scale_cents: cents,
                regions_key: rkey,
                source: source.clone(),
                width: input.width,
                height: input.height,
            });
            Some(source)
        });

        match source {
            Some(src) => {
                let image_ref = self.image_ref.clone();
                let retry = self.attach_retry.get();
                let sender = context.sender();
                context.use_effect("preview-attach", (input.sequence, cents, rkey, retry), move || {
                    if !src.attach(&image_ref) && retry == 0 {
                        let _ = sender.send(());
                    }
                    None
                });
                Image::new()
                    .element_ref(&self.image_ref)
                    .stretch(Stretch::Uniform)
                    .horizontal_alignment(HorizontalAlignment::Stretch)
                    .max_width(dip_w)
                    .max_height(dip_h)
                    .into()
            }
            None => TextBlock::new()
                .text("Preview unavailable")
                .font_size(12.0)
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .into(),
        }
    }
}

pub fn capture_preview(sequence: u64, width: u32, height: u32, rgba: Option<&Bytes>, regions: &[NormRect]) -> View {
    View::component::<CapturePreview>(PreviewInput {
        sequence,
        width,
        height,
        rgba: rgba.cloned(),
        regions: regions.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(rgba: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        for px in rgba.chunks_exact(4) {
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
    fn preview_input_eq_uses_sequence_not_rgba_bytes() {
        let a = PreviewInput {
            sequence: 7,
            width: 1,
            height: 1,
            rgba: Some(Bytes::from_static(&[1, 2, 3, 4])),
            regions: Vec::new(),
        };
        let mut b = a.clone();
        b.rgba = Some(Bytes::from_static(&[9, 9, 9, 9]));
        assert!(a == b);
        b.sequence = 8;
        assert!(a != b);
    }
}
