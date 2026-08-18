//! Capture preview → WinUI `Image` via D2D (`CanvasImageSource`).

use std::{
    any::Any,
    cell::{Cell, RefCell},
};

use bytes::Bytes;
use translator_core::NormRect;
use windows_canvas::{AlphaMode, ColorF, GpuDevice, Rect};
use windows_reactor::{CanvasImageSource, Element, HorizontalAlignment, Image, KeyExt, LayoutExt, Stretch, Updater, text_block};

thread_local! {
    static GPU: RefCell<Option<GpuDevice>> = const { RefCell::new(None) };
    static CACHE: RefCell<Option<CachedPreview>> = const { RefCell::new(None) };
    /// Last `XamlRoot.RasterizationScale` (96 DPI → 1.0). `1.0` until first callback.
    static SCALE: Cell<f32> = const { Cell::new(1.0) };
    /// Keeps `ImageHandle::on_rasterization_scale_changed` subscribed.
    static SCALE_WATCH: RefCell<Option<Box<dyn Any>>> = const { RefCell::new(None) };
}

struct CachedPreview {
    sequence: u64,
    /// Rounded scale × 100 (e.g. 150 for 1.5×).
    scale_cents: u32,
    regions_key: u64,
    source: CanvasImageSource,
    width: u32,
    height: u32,
}

fn regions_key(regions: &[NormRect]) -> u64 {
    let mut h = regions.len() as u64;
    for r in regions {
        h = h.wrapping_mul(16777619) ^ u64::from(r.x.to_bits());
        h = h.wrapping_mul(16777619) ^ u64::from(r.y.to_bits());
        h = h.wrapping_mul(16777619) ^ u64::from(r.width.to_bits());
        h = h.wrapping_mul(16777619) ^ u64::from(r.height.to_bits());
    }
    h
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

fn watch_rasterization_scale(bump: Updater<u32>) -> impl Fn(windows_reactor::ImageHandle) + 'static {
    move |handle| {
        let bump = bump.clone();
        if let Ok(revoker) = handle.on_rasterization_scale_changed(move |scale| {
            let scale = if scale > 0.0 { scale as f32 } else { 1.0 };
            let prev = scale_cents(SCALE.get());
            SCALE.set(scale);
            if scale_cents(scale) != prev {
                bump.call(|n| n.wrapping_add(1));
            }
        }) {
            SCALE_WATCH.with(|cell| *cell.borrow_mut() = Some(Box::new(revoker)));
        }
    }
}

/// D2D `create_bitmap_with_alpha` expects premultiplied BGRA8.
fn rgba_to_premul_bgra(rgba: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rgba.len());
    for px in rgba.as_chunks::<4>().0 {
        let r = u16::from(px[0]);
        let g = u16::from(px[1]);
        let b = u16::from(px[2]);
        let a = u16::from(px[3]);
        out.push(((b * a) / 255) as u8);
        out.push(((g * a) / 255) as u8);
        out.push(((r * a) / 255) as u8);
        out.push(a as u8);
    }
    out
}

/// DIP size so that at `scale`, physical pixels match `px` capture resolution.
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
) -> windows_reactor::Result<CanvasImageSource> {
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

pub fn capture_preview(sequence: u64, width: u32, height: u32, rgba: Option<&Bytes>, regions: &[NormRect], bump: &Updater<u32>) -> Element {
    let Some(rgba) = rgba.filter(|b| !b.is_empty() && width > 0 && height > 0) else {
        return text_block("No capture yet")
            .font_size(12.0)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .into();
    };

    let scale = SCALE.get();
    let cents = scale_cents(scale);
    let dip_w = f64::from(dip_size(width, scale));
    let dip_h = f64::from(dip_size(height, scale));

    let source = CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        let rkey = regions_key(regions);
        if let Some(cached) = cache.as_ref()
            && cached.sequence == sequence
            && cached.scale_cents == cents
            && cached.width == width
            && cached.height == height
            && cached.regions_key == rkey
        {
            return Some(cached.source.clone());
        }

        let device = gpu_device()?;
        let source = build_source(&device, rgba, width, height, scale, regions).ok()?;
        *cache = Some(CachedPreview {
            sequence,
            scale_cents: cents,
            regions_key: rkey,
            source: source.clone(),
            width,
            height,
        });
        Some(source)
    });

    match source {
        Some(src) => Image::new(src.image_source())
            .stretch(Stretch::Uniform)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            // Cap in DIPs so on-screen physical size ≤ capture resolution.
            .max_width(dip_w)
            .max_height(dip_h)
            .on_mounted(watch_rasterization_scale(bump.clone()))
            .with_key("capture-preview")
            .into(),
        None => text_block("Preview unavailable")
            .font_size(12.0)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .into(),
    }
}
