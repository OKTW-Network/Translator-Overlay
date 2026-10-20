//! Shared Segoe UI label painting (overlay captions + translation window).

use std::cell::RefCell;

use windows::{
    Win32::{
        Foundation::{COLORREF, RECT},
        Graphics::{
            Direct2D::{
                Common::{D2D1_ALPHA_MODE_IGNORE, D2D1_COLOR_F, D2D1_PIXEL_FORMAT},
                D2D1_DRAW_TEXT_OPTIONS_CLIP, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT, D2D1_RENDER_TARGET_PROPERTIES,
                D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE, D2D1CreateFactory, ID2D1DCRenderTarget, ID2D1Factory,
            },
            DirectWrite::{
                DWRITE_FACTORY_TYPE_SHARED, DWRITE_FLOW_DIRECTION_RIGHT_TO_LEFT, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_WEIGHT_NORMAL, DWRITE_READING_DIRECTION_TOP_TO_BOTTOM, DWRITE_TEXT_METRICS,
                DWRITE_VERTICAL_GLYPH_ORIENTATION_STACKED, DWriteCreateFactory, IDWriteFactory, IDWriteTextLayout, IDWriteTextLayout2,
            },
            Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
            Gdi::{
                CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, DEFAULT_CHARSET, DEFAULT_PITCH, DT_EDITCONTROL, DT_END_ELLIPSIS,
                DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_TOP, DT_VCENTER, DT_WORDBREAK, DrawTextW, FF_DONTCARE, FW_NORMAL, HDC, HFONT,
                HGDIOBJ, OUT_DEFAULT_PRECIS, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
            },
        },
    },
    core::{HSTRING, Interface, w},
};
use windows_numerics::Vector2;

use crate::{
    error::OverlayError,
    gfx::draw::{Rgba, SurfaceRect, SurfaceSize, blend_over, label_pad},
};

/// Font size + text colour for one painted label.
pub(crate) struct LabelStyle<'a> {
    pub font_px: i32,
    pub color: Rgba,
    /// Single-line, vertically centered (HUD status). Overlay captions stay top-aligned wrap.
    pub vcenter: bool,
    /// Columns right to left in this BCP-47 locale (CJK captions over vertical source text).
    /// DirectWrite picks the vertical punctuation and CJK fallback font for the locale.
    pub vertical: Option<&'a str>,
}

/// DirectWrite / Direct2D for column captions, made on first use on the painting thread.
struct Columns {
    dwrite: IDWriteFactory,
    target: ID2D1DCRenderTarget,
}

thread_local! {
    static COLUMNS: RefCell<Option<Columns>> = const { RefCell::new(None) };
}

/// Run `f` with the thread's [`Columns`]; a failure drops them so the next call starts fresh
/// (e.g. after `D2DERR_RECREATE_TARGET`).
fn with_columns<T>(f: impl FnOnce(&Columns) -> windows::core::Result<T>) -> Result<T, OverlayError> {
    COLUMNS
        .with_borrow_mut(|slot| {
            let columns = match slot.take() {
                Some(columns) => columns,
                None => unsafe {
                    let d2d: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
                    let target = d2d.CreateDCRenderTarget(&D2D1_RENDER_TARGET_PROPERTIES {
                        r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                        pixelFormat: D2D1_PIXEL_FORMAT {
                            format: DXGI_FORMAT_B8G8R8A8_UNORM,
                            alphaMode: D2D1_ALPHA_MODE_IGNORE,
                        },
                        // 1 DIP = 1 surface pixel, like the GDI font sizes.
                        dpiX: 96.0,
                        dpiY: 96.0,
                        usage: D2D1_RENDER_TARGET_USAGE_NONE,
                        minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
                    })?;
                    Columns {
                        dwrite: DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?,
                        target,
                    }
                },
            };
            let result = f(&columns);
            if result.is_ok() {
                *slot = Some(columns);
            }
            result
        })
        .map_err(|e| OverlayError::Other(format!("vertical text: {e}")))
}

/// `text` read top to bottom, columns right to left, inside a `w × h` box.
fn column_layout(
    dwrite: &IDWriteFactory,
    text: &str,
    font_px: i32,
    locale: &str,
    w: f32,
    h: f32,
) -> windows::core::Result<IDWriteTextLayout> {
    let wide: Vec<u16> = text.encode_utf16().collect();
    unsafe {
        let format = |locale: &str| {
            dwrite.CreateTextFormat(
                w!("Segoe UI"),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                font_px as f32,
                &HSTRING::from(locale),
            )
        };
        // A free-form target ("繁體中文") is not a locale name: fall back to the system default.
        let format = format(locale).or_else(|_| format(""))?;
        format.SetReadingDirection(DWRITE_READING_DIRECTION_TOP_TO_BOTTOM)?;
        format.SetFlowDirection(DWRITE_FLOW_DIRECTION_RIGHT_TO_LEFT)?;
        let layout = dwrite.CreateTextLayout(&wide, &format, w, h)?;
        // Upright Latin and `!?` too; the default lays them on their side (UAX #50).
        layout
            .cast::<IDWriteTextLayout2>()?
            .SetVerticalGlyphOrientation(DWRITE_VERTICAL_GLYPH_ORIENTATION_STACKED)?;
        Ok(layout)
    }
}

/// Width of `text` laid out in columns `height` tall (at most `max_w` before overflowing).
pub(crate) fn measure_columns(text: &str, font_px: i32, locale: &str, height: i32, max_w: i32) -> Result<i32, OverlayError> {
    with_columns(|c| {
        let layout = column_layout(&c.dwrite, text, font_px, locale, max_w.max(1) as f32, height.max(1) as f32)?;
        let mut metrics = DWRITE_TEXT_METRICS::default();
        unsafe { layout.GetMetrics(&mut metrics)? };
        Ok(metrics.width.ceil() as i32)
    })
}

pub(crate) fn create_segoe_font(px: i32) -> Result<HFONT, OverlayError> {
    let pitch = (DEFAULT_PITCH.0 as u32) | (FF_DONTCARE.0 as u32);
    // Regular weight matches typical game/UI source text better than semibold
    // (which looks larger/heavier than the OCR ink).
    let font = unsafe {
        CreateFontW(
            -px,
            0,
            0,
            0,
            FW_NORMAL.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            pitch,
            w!("Segoe UI"),
        )
    };
    if font.is_invalid() {
        Err(OverlayError::Other("CreateFontW failed".into()))
    } else {
        Ok(font)
    }
}

/// Draw wrapped text into a BGRA DIB already selected into `hdc`.
pub(crate) fn draw_text_label(
    hdc: HDC,
    hfont: HFONT,
    buf: &mut [u8],
    surface: SurfaceSize,
    rect: SurfaceRect,
    text: &str,
    style: LabelStyle<'_>,
) -> Result<(), OverlayError> {
    if text.is_empty() {
        return Ok(());
    }
    let _ = unsafe { SelectObject(hdc, HGDIOBJ(hfont.0)) };
    let _ = unsafe { SetBkMode(hdc, TRANSPARENT) };

    let pad = label_pad(style.font_px);
    let mut text_rect = if style.vcenter {
        RECT {
            left: rect.x + pad,
            top: rect.y,
            right: (rect.x + rect.w - pad).max(rect.x + pad + 1),
            bottom: (rect.y + rect.h).max(rect.y + 1),
        }
    } else {
        RECT {
            left: rect.x + pad,
            top: rect.y + pad,
            right: (rect.x + rect.w - pad).max(rect.x + pad + 1),
            bottom: (rect.y + rect.h - pad).max(rect.y + pad + 1),
        }
    };

    let rw = (text_rect.right - text_rect.left).max(0) as usize;
    let rh = (text_rect.bottom - text_rect.top).max(0) as usize;
    if rw == 0 || rh == 0 {
        return Ok(());
    }

    let mut bg_copy = vec![0u8; rw * rh * 4];
    for row in 0..rh {
        let src_y = text_rect.top as usize + row;
        if src_y >= surface.height as usize {
            break;
        }
        let src = src_y * surface.stride + text_rect.left as usize * 4;
        let dst = row * rw * 4;
        let count = rw * 4;
        if src + count <= buf.len() {
            bg_copy[dst..dst + count].copy_from_slice(&buf[src..src + count]);
            for px in buf[src..src + count].as_chunks_mut::<4>().0 {
                px.fill(0);
            }
        }
    }

    let _ = unsafe { SetTextColor(hdc, COLORREF(0x00FF_FFFF)) };
    if let Some(locale) = style.vertical {
        // White on black like the GDI path, clipped to `text_rect` so the coverage pass sees all ink.
        with_columns(|c| unsafe {
            let layout = column_layout(&c.dwrite, text, style.font_px, locale, rw as f32, rh as f32)?;
            c.target.BindDC(hdc, &text_rect)?;
            let white = c.target.CreateSolidColorBrush(
                &D2D1_COLOR_F {
                    r: 1.0,
                    g: 1.0,
                    b: 1.0,
                    a: 1.0,
                },
                None,
            )?;
            c.target.BeginDraw();
            c.target.Clear(Some(&D2D1_COLOR_F {
                a: 1.0,
                ..Default::default()
            }));
            c.target
                .DrawTextLayout(Vector2 { X: 0.0, Y: 0.0 }, &layout, &white, D2D1_DRAW_TEXT_OPTIONS_CLIP);
            c.target.EndDraw(None, None)
        })?;
    } else {
        let mut wide: Vec<u16> = text.encode_utf16().collect();
        let flags = if style.vcenter {
            DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX
        } else {
            DT_LEFT | DT_TOP | DT_WORDBREAK | DT_EDITCONTROL | DT_NOPREFIX
        };
        unsafe { DrawTextW(hdc, &mut wide, &mut text_rect, flags) };
    }

    let color = style.color;
    for row in 0..rh {
        let y = text_rect.top as usize + row;
        if y >= surface.height as usize {
            break;
        }
        for col in 0..rw {
            let x = text_rect.left as usize + col;
            if x >= surface.width as usize {
                break;
            }
            let idx = y * surface.stride + x * 4;
            let midx = row * rw * 4 + col * 4;
            let coverage = buf[idx].max(buf[idx + 1]).max(buf[idx + 2]) as u32;
            buf[idx..idx + 4].copy_from_slice(&bg_copy[midx..midx + 4]);
            if coverage > 8 {
                let fa = ((color.a as u32 * coverage) / 255) as u8;
                blend_over(buf, idx, Rgba { a: fa, ..color });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_grow_sideways_as_height_shrinks() {
        let text = "一二三四五六";
        let one = measure_columns(text, 20, "zh-TW", 1000, 1000).unwrap();
        let two = measure_columns(text, 20, "zh-TW", 70, 1000).unwrap();
        assert!((15..=40).contains(&one), "one column of 20px glyphs, got {one}");
        assert!(two > one * 3 / 2, "two columns must be wider than one: {two} vs {one}");
    }

    #[test]
    fn latin_punctuation_stays_upright_in_columns() {
        let mut dib = crate::gfx::surface::DibSurface::create().unwrap();
        dib.ensure(80, 160).unwrap();
        let surface = SurfaceSize::new(80, 160);
        let font = create_segoe_font(40).unwrap();
        let hdc = dib.hdc();
        let buf = dib.pixels().unwrap();
        buf.fill(0);
        let rect = SurfaceRect { x: 0, y: 0, w: 80, h: 160 };
        let style = LabelStyle {
            font_px: 40,
            color: Rgba::from_argb(0xFFFF_FFFF),
            vcenter: false,
            vertical: Some("zh-TW"),
        };
        draw_text_label(hdc, font, buf, surface, rect, "!!!", style).unwrap();
        let ink_x: Vec<usize> = (0..160 * 80)
            .filter(|i| buf[i / 80 * surface.stride + i % 80 * 4 + 3] > 64)
            .map(|i| i % 80)
            .collect();
        let width = ink_x.iter().max().unwrap() - ink_x.iter().min().unwrap();
        // Upright `!` is a thin stroke; on its side it spans most of the em.
        assert!(width < 12, "`!` must stand upright, ink is {width}px wide");
    }

    #[test]
    fn free_form_target_language_still_lays_out() {
        // DirectWrite rejects non-locale names with E_INVALIDARG.
        assert!(measure_columns("一二三", 20, "繁體中文", 1000, 1000).is_ok());
    }
}
