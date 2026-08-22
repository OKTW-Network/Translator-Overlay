//! Layered-window DIB (BGRA) and `UpdateLayeredWindow` present.

use std::mem::size_of;

use windows::Win32::{
    Foundation::{COLORREF, HWND, POINT, SIZE},
    Graphics::Gdi::{
        AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC, CreateDIBSection,
        DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, HALFTONE, HBITMAP, HDC, HGDIOBJ, ReleaseDC, SRCCOPY, SelectObject,
        SetStretchBltMode, StretchBlt,
    },
    UI::WindowsAndMessaging::{ULW_ALPHA, UpdateLayeredWindow},
};

use crate::error::OverlayError;

/// Off-screen 32-bit DIB selected into a memory DC, plus a screen DC for present.
///
/// `*mut u8` keeps this type `!Send` / `!Sync` — it must stay on the overlay thread.
pub(crate) struct DibSurface {
    hdc_screen: HDC,
    hdc_mem: HDC,
    hbmp: HBITMAP,
    bits: *mut u8,
    w: i32,
    h: i32,
}

impl DibSurface {
    pub(crate) fn create() -> Result<Self, OverlayError> {
        // MSDN: UpdateLayeredWindow's hdcDst is a *screen* DC (`GetDC(NULL)`), not a
        // window DC from a still-hidden layered popup. Teardown must ReleaseDC the
        // same HWND used here (`None` / NULL).
        let hdc_screen = unsafe { GetDC(None) };
        if hdc_screen.is_invalid() {
            return Err(OverlayError::Other("GetDC failed".into()));
        }
        let hdc_mem = unsafe { CreateCompatibleDC(Some(hdc_screen)) };
        if hdc_mem.is_invalid() {
            unsafe { ReleaseDC(None, hdc_screen) };
            return Err(OverlayError::Other("CreateCompatibleDC failed".into()));
        }
        Ok(Self {
            hdc_screen,
            hdc_mem,
            hbmp: HBITMAP::default(),
            bits: std::ptr::null_mut(),
            w: 0,
            h: 0,
        })
    }

    pub(crate) fn hdc(&self) -> HDC {
        self.hdc_mem
    }

    pub(crate) fn size(&self) -> (i32, i32) {
        (self.w, self.h)
    }

    pub(crate) fn ensure(&mut self, w: i32, h: i32) -> Result<(), OverlayError> {
        if w <= 0 || h <= 0 {
            return Err(OverlayError::Other("invalid bitmap size".into()));
        }
        if w == self.w && h == self.h && !self.bits.is_null() && !self.hbmp.is_invalid() {
            return Ok(());
        }

        if !self.hbmp.is_invalid() {
            let _ = unsafe { SelectObject(self.hdc_mem, HGDIOBJ::default()) };
            let _ = unsafe { DeleteObject(self.hbmp.into()) };
            self.hbmp = HBITMAP::default();
            self.bits = std::ptr::null_mut();
        }

        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: w,
                biHeight: -h,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut bits: *mut std::ffi::c_void = std::ptr::null_mut();
        let hbmp = unsafe { CreateDIBSection(Some(self.hdc_mem), &bmi, DIB_RGB_COLORS, &mut bits, None, 0) }
            .map_err(|e| OverlayError::Other(format!("CreateDIBSection: {e}")))?;

        if hbmp.is_invalid() || bits.is_null() {
            return Err(OverlayError::Other("CreateDIBSection returned null".into()));
        }

        let _ = unsafe { SelectObject(self.hdc_mem, HGDIOBJ(hbmp.0)) };
        self.hbmp = hbmp;
        self.bits = bits.cast();
        self.w = w;
        self.h = h;
        Ok(())
    }

    pub(crate) fn pixels(&mut self) -> Option<&mut [u8]> {
        if self.bits.is_null() || self.w <= 0 || self.h <= 0 {
            return None;
        }
        let len = (self.w as usize) * (self.h as usize) * 4;
        Some(unsafe { std::slice::from_raw_parts_mut(self.bits, len) })
    }

    pub(crate) fn present(&self, hwnd: HWND, x: i32, y: i32, dest_w: i32, dest_h: i32) -> Result<(), OverlayError> {
        if self.bits.is_null() || self.w <= 0 || self.h <= 0 {
            return Err(OverlayError::Other("paint bitmap missing".into()));
        }
        if dest_w <= 0 || dest_h <= 0 {
            return Err(OverlayError::Other("invalid client size".into()));
        }

        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let ppt_dst = POINT { x, y };
        let psize = SIZE { cx: dest_w, cy: dest_h };
        let ppt_src = POINT { x: 0, y: 0 };

        unsafe {
            UpdateLayeredWindow(
                hwnd,
                Some(self.hdc_screen),
                Some(&ppt_dst),
                Some(&psize),
                Some(self.hdc_mem),
                Some(&ppt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            )
        }
        .map_err(|e| OverlayError::Other(format!("UpdateLayeredWindow: {e}")))?;
        Ok(())
    }

    pub(crate) fn stretch_into(&self, dest: &mut Self, dest_w: i32, dest_h: i32) -> Result<(), OverlayError> {
        dest.ensure(dest_w, dest_h)?;
        let _ = unsafe { SetStretchBltMode(dest.hdc_mem, HALFTONE) };
        let ok = unsafe { StretchBlt(dest.hdc_mem, 0, 0, dest_w, dest_h, Some(self.hdc_mem), 0, 0, self.w, self.h, SRCCOPY) };
        if !ok.as_bool() {
            return Err(OverlayError::Other("StretchBlt failed".into()));
        }
        Ok(())
    }

    pub(crate) fn teardown(&mut self) {
        if !self.hbmp.is_invalid() {
            let _ = unsafe { SelectObject(self.hdc_mem, HGDIOBJ::default()) };
            let _ = unsafe { DeleteObject(self.hbmp.into()) };
            self.hbmp = HBITMAP::default();
            self.bits = std::ptr::null_mut();
        }
        if !self.hdc_mem.is_invalid() {
            let _ = unsafe { DeleteDC(self.hdc_mem) };
            self.hdc_mem = HDC::default();
        }
        if !self.hdc_screen.is_invalid() {
            unsafe { ReleaseDC(None, self.hdc_screen) };
            self.hdc_screen = HDC::default();
        }
        self.w = 0;
        self.h = 0;
    }
}

impl Drop for DibSurface {
    fn drop(&mut self) {
        self.teardown();
    }
}
