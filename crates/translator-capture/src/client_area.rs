//! Map window chrome to client-area crop offsets for Graphics Capture frames.

use windows::Win32::{
    Foundation::{HWND, RECT},
    Graphics::Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute},
    UI::{
        HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi},
        WindowsAndMessaging::{GetWindowInfo, SM_CXPADDEDBORDER, SM_CYCAPTION, SM_CYFRAME, WINDOWINFO, WS_CAPTION},
    },
};

use crate::{CaptureError, CapturedFrame};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClientAreaMetrics {
    pub window_width: u32,
    pub window_height: u32,
    pub client_width: u32,
    pub client_height: u32,
    pub offset_x: u32,
    pub offset_y: u32,
    frame_left: i32,
    frame_top: i32,
}

/// Scale toward the outer edge so a fractional title bar cannot leak into the crop.
fn scale_ceil(v: u32, num: u32, den: u32) -> u32 {
    if den == 0 || den == num {
        return v;
    }
    let n = u64::from(v).saturating_mul(u64::from(num));
    let d = u64::from(den);
    u32::try_from(n.saturating_add(d - 1) / d).unwrap_or(v)
}

fn scale_floor(v: u32, num: u32, den: u32) -> u32 {
    if den == 0 || den == num {
        return v;
    }
    u32::try_from(u64::from(v).saturating_mul(u64::from(num)) / u64::from(den)).unwrap_or(v)
}

impl ClientAreaMetrics {
    pub(crate) fn from_hwnd(hwnd: isize) -> Result<Self, CaptureError> {
        let hwnd = HWND(hwnd as *mut _);

        let mut info = WINDOWINFO {
            cbSize: size_of::<WINDOWINFO>() as u32,
            ..Default::default()
        };
        // SAFETY: `info.cbSize` is set; `hwnd` is a window handle.
        unsafe { GetWindowInfo(hwnd, &mut info) }.map_err(|e| CaptureError::Window(format!("GetWindowInfo: {e}")))?;

        let mut dwm = RECT::default();
        // SAFETY: `dwm` is a stack RECT; size matches `DWMWA_EXTENDED_FRAME_BOUNDS`.
        let dwm = if unsafe { DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, (&raw mut dwm).cast(), size_of::<RECT>() as u32) }
            .is_ok()
            && dwm.right > dwm.left
            && dwm.bottom > dwm.top
        {
            Some(dwm)
        } else {
            None
        };

        let client = info.rcClient;
        let dwm_contains_client = dwm.is_some_and(|d| {
            client.left >= d.left
                && client.top >= d.top
                && client.right <= d.right
                && client.bottom <= d.bottom
                && client.right > client.left
                && client.bottom > client.top
        });
        let outer = match dwm {
            Some(d) if dwm_contains_client => d,
            _ => info.rcWindow,
        };

        let mut metrics = Self {
            window_width: (outer.right - outer.left).max(0) as u32,
            window_height: (outer.bottom - outer.top).max(0) as u32,
            client_width: (client.right - client.left).max(0) as u32,
            client_height: (client.bottom - client.top).max(0) as u32,
            offset_x: (client.left - outer.left).max(0) as u32,
            offset_y: (client.top - outer.top).max(0) as u32,
            frame_left: outer.left,
            frame_top: outer.top,
        };

        // DPI-unaware hosts may report client == window; invent caption only then.
        // Skip when DWM already contains rcClient (`offset_y == 0` can be real client chrome).
        let collapsed = metrics.offset_x == 0
            && metrics.offset_y == 0
            && metrics.client_width == metrics.window_width
            && metrics.client_height == metrics.window_height;
        if !dwm_contains_client && collapsed && info.dwStyle.contains(WS_CAPTION) {
            // SAFETY: metric indices are the documented caption / frame constants.
            let dpi = unsafe { GetDpiForWindow(hwnd) }.max(1);
            let caption = unsafe { GetSystemMetricsForDpi(SM_CYCAPTION, dpi) }
                + unsafe { GetSystemMetricsForDpi(SM_CYFRAME, dpi) }
                + unsafe { GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi) };
            if caption > 0 {
                metrics.offset_y = (caption as u32).min(metrics.window_height.saturating_sub(1));
            }
        }

        // Prefer DWM outer size when WGC uses that frame but rcClient was outside it.
        if let Some(dwm) = dwm.filter(|_| !dwm_contains_client) {
            let num_w = (dwm.right - dwm.left).max(0) as u32;
            let num_h = (dwm.bottom - dwm.top).max(0) as u32;
            metrics.offset_x = scale_ceil(metrics.offset_x, num_w, metrics.window_width);
            metrics.offset_y = scale_ceil(metrics.offset_y, num_h, metrics.window_height);
            metrics.client_width = scale_floor(metrics.client_width, num_w, metrics.window_width);
            metrics.client_height = scale_floor(metrics.client_height, num_h, metrics.window_height);
            metrics.window_width = num_w;
            metrics.window_height = num_h;
            metrics.frame_left = dwm.left;
            metrics.frame_top = dwm.top;
        }

        if metrics.window_width == 0 || metrics.window_height == 0 || metrics.client_width == 0 || metrics.client_height == 0 {
            return Err(CaptureError::Window("empty window/client rect".into()));
        }

        metrics.clamp();
        // WGC can sit 1px above DWM/`rcClient`; bump an existing inset, never invent one.
        if metrics.offset_y > 0 {
            metrics.offset_y = metrics.offset_y.saturating_add(1);
            metrics.client_height = metrics.client_height.saturating_sub(1).max(1);
            metrics.clamp();
        }
        Ok(metrics)
    }

    fn clamp(&mut self) {
        if self.window_width == 0 || self.window_height == 0 {
            return;
        }
        self.offset_x = self.offset_x.min(self.window_width.saturating_sub(1));
        self.offset_y = self.offset_y.min(self.window_height.saturating_sub(1));
        self.client_width = self.client_width.min(self.window_width.saturating_sub(self.offset_x)).max(1);
        self.client_height = self.client_height.min(self.window_height.saturating_sub(self.offset_y)).max(1);
    }

    fn screen_rect(&self) -> (i32, i32, i32, i32) {
        (
            self.frame_left.saturating_add_unsigned(self.offset_x),
            self.frame_top.saturating_add_unsigned(self.offset_y),
            self.client_width.max(1) as i32,
            self.client_height.max(1) as i32,
        )
    }

    pub(crate) fn crop_frame(&self, frame: &CapturedFrame) -> CapturedFrame {
        if frame.width == 0 || frame.height == 0 {
            return frame.clone();
        }

        let den_w = self.window_width.max(1);
        let den_h = self.window_height.max(1);
        let x = scale_ceil(self.offset_x, frame.width, den_w).min(frame.width.saturating_sub(1));
        let y = scale_ceil(self.offset_y, frame.height, den_h).min(frame.height.saturating_sub(1));
        let width = scale_floor(self.client_width, frame.width, den_w)
            .max(1)
            .min(frame.width.saturating_sub(x));
        let height = scale_floor(self.client_height, frame.height, den_h)
            .max(1)
            .min(frame.height.saturating_sub(y));

        if x == 0 && y == 0 && width == frame.width && height == frame.height {
            return frame.clone();
        }

        let mut rgba = vec![0u8; width as usize * height as usize * 4];
        for row in 0..height {
            let src = ((y + row) as usize * frame.width as usize + x as usize) * 4;
            let dst = row as usize * width as usize * 4;
            let n = width as usize * 4;
            if src + n <= frame.rgba.len() && dst + n <= rgba.len() {
                rgba[dst..dst + n].copy_from_slice(&frame.rgba[src..src + n]);
            }
        }

        CapturedFrame::new(width, height, rgba, frame.sequence)
    }
}

/// Physical client rect `(left, top, width, height)` — same geometry as crop.
pub fn client_screen_rect(hwnd: isize) -> Option<(i32, i32, i32, i32)> {
    ClientAreaMetrics::from_hwnd(hwnd).ok().map(|m| m.screen_rect())
}

pub(crate) fn crop_frame_to_client(hwnd: isize, frame: CapturedFrame) -> CapturedFrame {
    match ClientAreaMetrics::from_hwnd(hwnd) {
        Ok(m) => m.crop_frame(&frame),
        Err(_) => frame,
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use super::*;

    fn metrics(window_w: u32, window_h: u32, client_w: u32, client_h: u32, offset_x: u32, offset_y: u32) -> ClientAreaMetrics {
        ClientAreaMetrics {
            window_width: window_w,
            window_height: window_h,
            client_width: client_w,
            client_height: client_h,
            offset_x,
            offset_y,
            frame_left: 0,
            frame_top: 0,
        }
    }

    #[test]
    fn crop_frame_extracts_subrect() {
        let mut rgba = vec![0u8; 4 * 3 * 4];
        let i = (4 + 1) * 4;
        rgba[i] = 255;
        rgba[i + 3] = 255;

        let frame = CapturedFrame::new(4, 3, rgba, 1);
        let cropped = metrics(4, 3, 2, 2, 1, 1).crop_frame(&frame);
        assert_eq!(cropped.width, 2);
        assert_eq!(cropped.height, 2);
        assert_eq!(cropped.rgba[0], 255);
        assert_eq!(cropped.rgba[3], 255);
    }

    #[test]
    fn crop_frame_noop_reuses_bytes() {
        let rgba = Bytes::from(vec![7u8; 4 * 3 * 4]);
        let frame = CapturedFrame::new(4, 3, rgba, 2);
        let cropped = metrics(4, 3, 4, 3, 0, 0).crop_frame(&frame);
        assert_eq!(cropped.rgba.as_ptr(), frame.rgba.as_ptr());
        assert_eq!(cropped.sequence, 2);
    }

    #[test]
    fn crop_scales_title_bar_with_ceil() {
        // 31 × 900/600 = 46.5 → ceil 47; floor sizes match.
        let cropped = metrics(800, 600, 784, 553, 8, 31).crop_frame(&CapturedFrame::new(1200, 900, vec![0u8; 1200 * 900 * 4], 0));
        assert_eq!(cropped.width, 1176);
        assert_eq!(cropped.height, 829);

        let y = scale_ceil(31, 899, 600);
        assert_eq!(y, 47);
        assert_eq!(scale_ceil(31, 900, 900), 31);
        assert_eq!(scale_floor(553, 900, 900), 553);
    }

    #[test]
    fn collapsed_caption_and_top_safety() {
        let mut m = metrics(800, 600, 800, 600, 0, 0);
        m.offset_y = 31;
        m.clamp();
        assert_eq!(m.client_height, 569);
        assert_eq!(m.screen_rect(), (0, 31, 800, 569));

        // Already-inset client is left alone by the collapsed path.
        let inset = metrics(800, 600, 784, 553, 8, 31);
        assert_ne!(inset.client_height, inset.window_height);

        // Top safety bumps an existing inset.
        let mut m = metrics(800, 600, 784, 553, 8, 31);
        m.offset_y = m.offset_y.saturating_add(1);
        m.client_height = m.client_height.saturating_sub(1).max(1);
        m.clamp();
        assert_eq!(m.offset_y, 32);
        assert_eq!(m.client_height, 552);

        let mut flush = metrics(800, 600, 800, 600, 0, 0);
        if flush.offset_y > 0 {
            flush.offset_y = flush.offset_y.saturating_add(1);
        }
        assert_eq!(flush.offset_y, 0);
    }

    #[test]
    fn remap_onto_dwm_ceils_title() {
        let mut m = metrics(800, 600, 784, 553, 8, 31);
        let num_w = 1200u32;
        let num_h = 900u32;
        m.offset_x = scale_ceil(m.offset_x, num_w, m.window_width);
        m.offset_y = scale_ceil(m.offset_y, num_h, m.window_height);
        m.client_width = scale_floor(m.client_width, num_w, m.window_width);
        m.client_height = scale_floor(m.client_height, num_h, m.window_height);
        m.window_width = num_w;
        m.window_height = num_h;
        m.frame_left = 10;
        m.frame_top = 20;
        m.clamp();
        assert_eq!(m.offset_x, 12);
        assert_eq!(m.offset_y, 47);
        assert_eq!(m.client_width, 1176);
        assert_eq!(m.client_height, 829);
        assert_eq!(m.screen_rect(), (22, 67, 1176, 829));
    }
}
