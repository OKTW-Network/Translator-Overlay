//! Shared Segoe UI label painting (overlay captions + translation window).

use windows::{
    Win32::{
        Foundation::{COLORREF, RECT},
        Graphics::Gdi::{
            CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, CreateFontW, DEFAULT_CHARSET, DEFAULT_PITCH, DT_EDITCONTROL, DT_END_ELLIPSIS, DT_LEFT,
            DT_NOPREFIX, DT_SINGLELINE, DT_TOP, DT_VCENTER, DT_WORDBREAK, DrawTextW, FF_DONTCARE, FW_NORMAL, HDC, HFONT, HGDIOBJ,
            OUT_DEFAULT_PRECIS, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
        },
    },
    core::w,
};

use crate::{
    error::OverlayError,
    gfx::draw::{Rgba, SurfaceRect, SurfaceSize, blend_over, label_pad},
};

/// Font size + text colour for one painted label.
pub(crate) struct LabelStyle {
    pub font_px: i32,
    pub color: Rgba,
    /// Single-line, vertically centered (HUD status). Overlay captions stay top-aligned wrap.
    pub vcenter: bool,
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
    style: LabelStyle,
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

    let mut wide: Vec<u16> = text.encode_utf16().collect();
    let _ = unsafe { SetTextColor(hdc, COLORREF(0x00FF_FFFF)) };
    let flags = if style.vcenter {
        DT_LEFT | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX
    } else {
        DT_LEFT | DT_TOP | DT_WORDBREAK | DT_EDITCONTROL | DT_NOPREFIX
    };
    unsafe { DrawTextW(hdc, &mut wide, &mut text_rect, flags) };

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
