//! Map window chrome to client-area crop offsets for Graphics Capture frames.

use windows::Win32::Foundation::{HWND, POINT, RECT};
use windows::Win32::Graphics::Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, GetWindowRect};

use crate::{CaptureError, CapturedFrame};

/// Client area expressed relative to the outer window rect (physical pixels).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientAreaMetrics {
    /// Outer window width / height used to scale into capture frames.
    /// Prefer DWM extended frame bounds (matches Graphics Capture better than
    /// `GetWindowRect`, which often includes invisible drop-shadow margins).
    pub window_width: u32,
    pub window_height: u32,
    /// Client size from `GetClientRect`.
    pub client_width: u32,
    pub client_height: u32,
    /// Offset of client origin within the outer window rect.
    pub offset_x: u32,
    pub offset_y: u32,
    /// Client top-left in screen coordinates.
    pub screen_x: i32,
    pub screen_y: i32,
}

impl ClientAreaMetrics {
    /// Read metrics for `hwnd`.
    pub fn from_hwnd(hwnd: isize) -> Result<Self, CaptureError> {
        unsafe {
            let hwnd = HWND(hwnd as *mut _);

            // Prefer DWM extended frame bounds — Graphics Capture usually
            // matches this visual rect, not the larger GetWindowRect box.
            let window = outer_window_rect(hwnd)?;

            let mut client = RECT::default();
            GetClientRect(hwnd, &mut client)
                .map_err(|e| CaptureError::Window(format!("GetClientRect: {e}")))?;

            let mut origin = POINT {
                x: client.left,
                y: client.top,
            };
            if !ClientToScreen(hwnd, &mut origin).as_bool() {
                return Err(CaptureError::Window("ClientToScreen failed".into()));
            }

            let window_width = (window.right - window.left).max(0) as u32;
            let window_height = (window.bottom - window.top).max(0) as u32;
            let client_width = (client.right - client.left).max(0) as u32;
            let client_height = (client.bottom - client.top).max(0) as u32;

            if window_width == 0 || window_height == 0 || client_width == 0 || client_height == 0 {
                return Err(CaptureError::Window("empty window/client rect".into()));
            }

            // Client origin relative to the same outer rect used for scaling.
            let offset_x = (origin.x - window.left).max(0) as u32;
            let offset_y = (origin.y - window.top).max(0) as u32;

            // Clamp offsets so crop stays inside the window.
            let offset_x = offset_x.min(window_width.saturating_sub(1));
            let offset_y = offset_y.min(window_height.saturating_sub(1));
            let client_width = client_width.min(window_width.saturating_sub(offset_x));
            let client_height = client_height.min(window_height.saturating_sub(offset_y));

            Ok(Self {
                window_width,
                window_height,
                client_width,
                client_height,
                offset_x,
                offset_y,
                screen_x: origin.x,
                screen_y: origin.y,
            })
        }
    }

    /// Crop a full-window capture frame down to the client area.
    ///
    /// Graphics Capture frame size may differ slightly from the outer window
    /// rect (DPI / shadows); offsets are scaled to the frame dimensions.
    pub fn crop_frame(&self, frame: &CapturedFrame) -> CapturedFrame {
        if frame.width == 0 || frame.height == 0 {
            return frame.clone();
        }

        let scale_x = frame.width as f32 / self.window_width.max(1) as f32;
        let scale_y = frame.height as f32 / self.window_height.max(1) as f32;

        let mut x0 = (self.offset_x as f32 * scale_x).round() as u32;
        let mut y0 = (self.offset_y as f32 * scale_y).round() as u32;
        let mut cw = (self.client_width as f32 * scale_x).round() as u32;
        let mut ch = (self.client_height as f32 * scale_y).round() as u32;

        x0 = x0.min(frame.width.saturating_sub(1));
        y0 = y0.min(frame.height.saturating_sub(1));
        cw = cw.max(1).min(frame.width.saturating_sub(x0));
        ch = ch.max(1).min(frame.height.saturating_sub(y0));

        // Already client-sized (borderless / full-screen client).
        if x0 == 0 && y0 == 0 && cw == frame.width && ch == frame.height {
            return frame.clone();
        }

        let mut rgba = vec![0u8; (cw as usize) * (ch as usize) * 4];
        for row in 0..ch {
            let src_y = (y0 + row) as usize;
            let src = (src_y * frame.width as usize + x0 as usize) * 4;
            let dst = (row as usize * cw as usize) * 4;
            let n = cw as usize * 4;
            if src + n <= frame.rgba.len() && dst + n <= rgba.len() {
                rgba[dst..dst + n].copy_from_slice(&frame.rgba[src..src + n]);
            }
        }

        CapturedFrame {
            width: cw,
            height: ch,
            rgba,
            sequence: frame.sequence,
        }
    }
}

/// Outer rect for scaling capture frames: DWM extended frame when available.
unsafe fn outer_window_rect(hwnd: HWND) -> Result<RECT, CaptureError> {
    unsafe {
        let mut extended = RECT::default();
        let ok = DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut extended as *mut RECT as *mut _,
            std::mem::size_of::<RECT>() as u32,
        );
        if ok.is_ok() {
            let w = extended.right - extended.left;
            let h = extended.bottom - extended.top;
            if w > 0 && h > 0 {
                return Ok(extended);
            }
        }

        let mut window = RECT::default();
        GetWindowRect(hwnd, &mut window)
            .map_err(|e| CaptureError::Window(format!("GetWindowRect: {e}")))?;
        Ok(window)
    }
}

/// Crop `frame` to the client area of `hwnd` when possible.
pub fn crop_frame_to_client(hwnd: isize, frame: CapturedFrame) -> CapturedFrame {
    match ClientAreaMetrics::from_hwnd(hwnd) {
        Ok(m) => m.crop_frame(&frame),
        Err(_) => frame,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_frame_extracts_subrect() {
        // 4x3 frame, client is 2x2 starting at (1,1)
        let mut rgba = vec![0u8; 4 * 3 * 4];
        // mark pixel (1,1) as red
        // pixel (x=1, y=1) in a 4-wide row of RGBA
        let i = (4 + 1) * 4;
        rgba[i] = 255;
        rgba[i + 1] = 0;
        rgba[i + 2] = 0;
        rgba[i + 3] = 255;

        let frame = CapturedFrame {
            width: 4,
            height: 3,
            rgba,
            sequence: 1,
        };
        let m = ClientAreaMetrics {
            window_width: 4,
            window_height: 3,
            client_width: 2,
            client_height: 2,
            offset_x: 1,
            offset_y: 1,
            screen_x: 0,
            screen_y: 0,
        };
        let cropped = m.crop_frame(&frame);
        assert_eq!(cropped.width, 2);
        assert_eq!(cropped.height, 2);
        // top-left of crop is original (1,1)
        assert_eq!(cropped.rgba[0], 255);
        assert_eq!(cropped.rgba[3], 255);
    }
}
