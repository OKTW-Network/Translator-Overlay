//! Shared Segoe UI label painting (overlay captions + translation window), on DirectWrite.

use std::cell::RefCell;

use windows::{
    Win32::{
        Foundation::RECT,
        Graphics::{
            Direct2D::{
                Common::{D2D1_ALPHA_MODE_IGNORE, D2D1_COLOR_F, D2D1_PIXEL_FORMAT},
                D2D1_DRAW_TEXT_OPTIONS_NONE, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_FEATURE_LEVEL_DEFAULT, D2D1_RENDER_TARGET_PROPERTIES,
                D2D1_RENDER_TARGET_TYPE_DEFAULT, D2D1_RENDER_TARGET_USAGE_NONE, D2D1CreateFactory, ID2D1DCRenderTarget, ID2D1Factory,
            },
            DirectWrite::{
                DWRITE_FACTORY_TYPE_SHARED, DWRITE_FLOW_DIRECTION_RIGHT_TO_LEFT, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_WEIGHT_NORMAL, DWRITE_LINE_SPACING_METHOD_UNIFORM, DWRITE_READING_DIRECTION_TOP_TO_BOTTOM, DWRITE_TEXT_METRICS,
                DWRITE_VERTICAL_GLYPH_ORIENTATION_STACKED, DWRITE_WORD_WRAPPING_EMERGENCY_BREAK, DWriteCreateFactory, IDWriteFactory,
                IDWriteTextLayout, IDWriteTextLayout2,
            },
            Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
            Gdi::HDC,
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
    /// BCP-47 locale of the text: DirectWrite picks the CJK fallback font and the vertical
    /// punctuation for it. Empty is the system default.
    pub locale: &'a str,
    /// Columns right to left (CJK captions over vertical source text) instead of lines.
    pub vertical: bool,
    /// Distance between the tops of wrapped lines; `None` is [`line_pitch`].
    pub line_pitch: Option<i32>,
}

/// Distance between the tops of wrapped lines at Segoe UI's own line height.
pub(crate) fn line_pitch(font_px: i32) -> i32 {
    (font_px * 4 + 2) / 3
}

/// DirectWrite / Direct2D, made on first use on the painting thread.
struct Writer {
    dwrite: IDWriteFactory,
    target: ID2D1DCRenderTarget,
}

thread_local! {
    static WRITER: RefCell<Option<Writer>> = const { RefCell::new(None) };
}

/// Run `f` with the thread's [`Writer`]; a failure drops it so the next call starts fresh
/// (e.g. after `D2DERR_RECREATE_TARGET`).
fn with_writer<T>(f: impl FnOnce(&Writer) -> windows::core::Result<T>) -> Result<T, OverlayError> {
    WRITER
        .with_borrow_mut(|slot| {
            let writer = match slot.take() {
                Some(writer) => writer,
                None => {
                    // SAFETY: plain factory creation, no pointers passed in.
                    let d2d: ID2D1Factory = unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)? };
                    let props = D2D1_RENDER_TARGET_PROPERTIES {
                        r#type: D2D1_RENDER_TARGET_TYPE_DEFAULT,
                        pixelFormat: D2D1_PIXEL_FORMAT {
                            format: DXGI_FORMAT_B8G8R8A8_UNORM,
                            alphaMode: D2D1_ALPHA_MODE_IGNORE,
                        },
                        // 1 DIP = 1 surface pixel.
                        dpiX: 96.0,
                        dpiY: 96.0,
                        usage: D2D1_RENDER_TARGET_USAGE_NONE,
                        minLevel: D2D1_FEATURE_LEVEL_DEFAULT,
                    };
                    Writer {
                        // SAFETY: plain factory creation, no pointers passed in.
                        dwrite: unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)? },
                        // SAFETY: `props` outlives the call.
                        target: unsafe { d2d.CreateDCRenderTarget(&props)? },
                    }
                }
            };
            let result = f(&writer);
            if result.is_ok() {
                *slot = Some(writer);
            }
            result
        })
        .map_err(|e| OverlayError::Other(format!("text: {e}")))
}

/// `text` inside a `w × h` box: wrapped lines whose tops are `pitch` px apart, or with no
/// pitch, columns read top to bottom and laid right to left.
fn text_layout(
    dwrite: &IDWriteFactory,
    text: &str,
    font_px: i32,
    locale: &str,
    pitch: Option<i32>,
    w: f32,
    h: f32,
) -> windows::core::Result<IDWriteTextLayout> {
    let wide: Vec<u16> = text.encode_utf16().collect();
    let format = |locale: &str| {
        // SAFETY: the family and locale strings outlive the call.
        unsafe {
            dwrite.CreateTextFormat(
                w!("Segoe UI"),
                None,
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                font_px as f32,
                &HSTRING::from(locale),
            )
        }
    };
    // A free-form target ("繁體中文") is not a locale name: fall back to the system default.
    let format = format(locale).or_else(|_| format(""))?;
    match pitch {
        Some(pitch) => {
            // Each line is its em square, which CJK glyphs fill (baseline 0.88 em down), whatever
            // fallback font a line uses; Latin descenders hang below it.
            // SAFETY: `format` is a live text format.
            unsafe { format.SetLineSpacing(DWRITE_LINE_SPACING_METHOD_UNIFORM, pitch as f32, font_px as f32 * 0.88)? };
            // Break inside a word wider than the box rather than overflow it.
            // SAFETY: `format` is a live text format.
            unsafe { format.SetWordWrapping(DWRITE_WORD_WRAPPING_EMERGENCY_BREAK)? };
        }
        None => {
            // SAFETY: `format` is a live text format.
            unsafe { format.SetReadingDirection(DWRITE_READING_DIRECTION_TOP_TO_BOTTOM)? };
            // SAFETY: `format` is a live text format.
            unsafe { format.SetFlowDirection(DWRITE_FLOW_DIRECTION_RIGHT_TO_LEFT)? };
        }
    }
    // SAFETY: `wide` and `format` outlive the call.
    let layout = unsafe { dwrite.CreateTextLayout(&wide, &format, w, h)? };
    if pitch.is_none() {
        let layout = layout.cast::<IDWriteTextLayout2>()?;
        // Upright Latin and `!?` too; the default lays them on their side (UAX #50).
        // SAFETY: `layout` is a live text layout.
        unsafe { layout.SetVerticalGlyphOrientation(DWRITE_VERTICAL_GLYPH_ORIENTATION_STACKED)? };
    }
    Ok(layout)
}

fn metrics(layout: &IDWriteTextLayout) -> windows::core::Result<DWRITE_TEXT_METRICS> {
    let mut metrics = DWRITE_TEXT_METRICS::default();
    // SAFETY: `metrics` is a valid out-parameter for the call.
    unsafe { layout.GetMetrics(&mut metrics)? };
    Ok(metrics)
}

/// Width of `text` laid out in columns `height` tall (at most `max_w` before overflowing).
pub(crate) fn measure_columns(text: &str, font_px: i32, locale: &str, height: i32, max_w: i32) -> Result<i32, OverlayError> {
    with_writer(|c| {
        let layout = text_layout(&c.dwrite, text, font_px, locale, None, max_w.max(1) as f32, height.max(1) as f32)?;
        Ok(metrics(&layout)?.width.ceil() as i32)
    })
}

/// Extent of `text` wrapped into lines at most `max_w` wide: the widest line, and the height
/// from the top of the first line's em square to the bottom of the last one's, the lines
/// [`line_pitch`] apart.
pub(crate) fn measure_lines(text: &str, font_px: i32, locale: &str, max_w: i32) -> Result<(i32, i32), OverlayError> {
    with_writer(|c| {
        let pitch = line_pitch(font_px);
        let layout = text_layout(&c.dwrite, text, font_px, locale, Some(pitch), max_w.max(1) as f32, 65536.0)?;
        let metrics = metrics(&layout)?;
        let lines = (metrics.lineCount as i32).max(1);
        Ok(((metrics.width.ceil() as i32).max(1), (lines - 1) * pitch + font_px))
    })
}

/// Draw `text` into `rect` of a BGRA DIB already selected into `hdc`, inset by the label padding.
pub(crate) fn draw_text_label(
    hdc: HDC,
    buf: &mut [u8],
    surface: SurfaceSize,
    rect: SurfaceRect,
    text: &str,
    style: LabelStyle<'_>,
) -> Result<(), OverlayError> {
    let cols = (rect.w.max(0) as usize).min((surface.width as usize).saturating_sub(rect.x as usize));
    let rows = (rect.h.max(0) as usize).min((surface.height as usize).saturating_sub(rect.y as usize));
    if text.is_empty() || cols == 0 || rows == 0 {
        return Ok(());
    }
    let at = |row: usize, col: usize| (rect.y as usize + row) * surface.stride + (rect.x as usize + col) * 4;

    let mut bg_copy = vec![0u8; cols * rows * 4];
    for row in 0..rows {
        bg_copy[row * cols * 4..][..cols * 4].copy_from_slice(&buf[at(row, 0)..][..cols * 4]);
    }

    // White on black over the whole label, so ink that hangs out of the text box (descenders)
    // lands in the padding and the coverage pass below sees all of it.
    let pad = label_pad(style.font_px);
    let pitch = (!style.vertical).then(|| style.line_pitch.unwrap_or_else(|| line_pitch(style.font_px)));
    let bound = RECT {
        left: rect.x,
        top: rect.y,
        right: rect.x + rect.w,
        bottom: rect.y + rect.h,
    };
    let white = D2D1_COLOR_F {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    let black = D2D1_COLOR_F {
        a: 1.0,
        ..Default::default()
    };
    with_writer(|c| {
        let (w, h) = ((rect.w - pad * 2).max(1), (rect.h - pad * 2).max(1));
        let layout = text_layout(&c.dwrite, text, style.font_px, style.locale, pitch, w as f32, h as f32)?;
        // SAFETY: `hdc` is the live memory DC holding the DIB; `bound` outlives the call.
        unsafe { c.target.BindDC(hdc, &bound)? };
        // SAFETY: `white` outlives the call.
        let brush = unsafe { c.target.CreateSolidColorBrush(&white, None)? };
        let origin = Vector2 {
            X: pad as f32,
            Y: pad as f32,
        };
        // SAFETY: one BeginDraw / EndDraw pair on the bound target; `layout`, `brush` and
        // `black` outlive it.
        unsafe {
            c.target.BeginDraw();
            c.target.Clear(Some(&black));
            c.target.DrawTextLayout(origin, &layout, &brush, D2D1_DRAW_TEXT_OPTIONS_NONE);
            c.target.EndDraw(None, None)
        }
    })?;

    // Lift the coverage off the surface, put the background back and blend the text colour on it.
    let color = style.color;
    for row in 0..rows {
        for col in 0..cols {
            let idx = at(row, col);
            let coverage = buf[idx].max(buf[idx + 1]).max(buf[idx + 2]) as u32;
            buf[idx..idx + 4].copy_from_slice(&bg_copy[(row * cols + col) * 4..][..4]);
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
    fn wrapped_lines_are_one_pitch_apart() {
        let text = "one two three four five six";
        let (wide, one) = measure_lines(text, 20, "en", 1000).unwrap();
        assert_eq!(one, 20, "one line is its em square");
        let (narrow, wrapped) = measure_lines(text, 20, "en", wide / 2).unwrap();
        assert!(narrow <= wide / 2, "wrapped lines must fit the width: {narrow} vs {}", wide / 2);
        assert!(wrapped > 20 && (wrapped - 20) % line_pitch(20) == 0, "extra lines add one pitch each, got {wrapped}");
        // A word wider than the box breaks inside instead of overflowing.
        let (long, _) = measure_lines("Donaudampfschifffahrt", 20, "en", 60).unwrap();
        assert!(long <= 60, "a long word must break to fit, got {long}");
    }

    #[test]
    fn cjk_line_fills_its_em_square() {
        let mut dib = crate::gfx::surface::DibSurface::create().unwrap();
        dib.ensure(200, 52).unwrap();
        let surface = SurfaceSize::new(200, 52);
        let hdc = dib.hdc();
        let buf = dib.pixels().unwrap();
        buf.fill(0);
        // 40px font plus 6px padding above and below.
        let rect = SurfaceRect { x: 0, y: 0, w: 200, h: 52 };
        let style = LabelStyle {
            font_px: 40,
            color: Rgba::from_argb(0xFFFF_FFFF),
            locale: "zh-TW",
            vertical: false,
            line_pitch: None,
        };
        draw_text_label(hdc, buf, surface, rect, "國語", style).unwrap();
        let ink_y: Vec<usize> = (0..52 * 200)
            .filter(|i| buf[i / 200 * surface.stride + i % 200 * 4 + 3] > 64)
            .map(|i| i / 200)
            .collect();
        let (top, bottom) = (*ink_y.iter().min().unwrap(), *ink_y.iter().max().unwrap());
        assert!(top >= 6 && bottom < 46, "ink must stay inside the em square, got rows {top}..={bottom}");
        assert!((24..=28).contains(&((top + bottom) / 2)), "ink must be centred in the label, got rows {top}..={bottom}");
    }

    #[test]
    fn latin_punctuation_stays_upright_in_columns() {
        let mut dib = crate::gfx::surface::DibSurface::create().unwrap();
        dib.ensure(80, 160).unwrap();
        let surface = SurfaceSize::new(80, 160);
        let hdc = dib.hdc();
        let buf = dib.pixels().unwrap();
        buf.fill(0);
        let rect = SurfaceRect { x: 0, y: 0, w: 80, h: 160 };
        let style = LabelStyle {
            font_px: 40,
            color: Rgba::from_argb(0xFFFF_FFFF),
            locale: "zh-TW",
            vertical: true,
            line_pitch: None,
        };
        draw_text_label(hdc, buf, surface, rect, "!!!", style).unwrap();
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
