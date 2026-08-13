//! Map window chrome to client-area crop offsets for Graphics Capture frames.

use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::{
        Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute},
        Gdi::ClientToScreen,
    },
    UI::WindowsAndMessaging::{GetClientRect, GetWindowRect},
};

use crate::{CaptureError, CapturedFrame};

/// Client area expressed relative to the outer window rect (physical pixels).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClientAreaMetrics {
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
}

#[derive(Debug, Clone, Copy)]
struct CropRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl CropRect {
    fn covers_frame(self, frame_w: u32, frame_h: u32) -> bool {
        self.x == 0 && self.y == 0 && self.width == frame_w && self.height == frame_h
    }
}

fn rect_size(rect: RECT) -> (u32, u32) {
    ((rect.right - rect.left).max(0) as u32, (rect.bottom - rect.top).max(0) as u32)
}

fn client_rect(hwnd: HWND) -> Result<RECT, CaptureError> {
    let mut client = RECT::default();
    // SAFETY: `hwnd` is a window handle; `client` is a valid out-param.
    unsafe { GetClientRect(hwnd, &mut client) }.map_err(|e| CaptureError::Window(format!("GetClientRect: {e}")))?;
    Ok(client)
}

fn client_origin_screen(hwnd: HWND, client: RECT) -> Result<POINT, CaptureError> {
    let mut origin = POINT {
        x: client.left,
        y: client.top,
    };
    // SAFETY: `hwnd` is a window handle; `origin` is a valid in/out POINT.
    if !unsafe { ClientToScreen(hwnd, &mut origin) }.as_bool() {
        return Err(CaptureError::Window("ClientToScreen failed".into()));
    }
    Ok(origin)
}

fn extended_frame_bounds(hwnd: HWND) -> Option<RECT> {
    let mut extended = RECT::default();
    let attribute = (&raw mut extended).cast();
    let attribute_size = size_of::<RECT>() as u32;
    // SAFETY: `extended` is a stack RECT; size matches `DWMWA_EXTENDED_FRAME_BOUNDS`.
    let ok = unsafe { DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, attribute, attribute_size) };
    if ok.is_ok() {
        let (w, h) = rect_size(extended);
        if w > 0 && h > 0 {
            return Some(extended);
        }
    }
    None
}

fn window_rect(hwnd: HWND) -> Result<RECT, CaptureError> {
    let mut window = RECT::default();
    // SAFETY: `hwnd` is a window handle; `window` is a valid out-param.
    unsafe { GetWindowRect(hwnd, &mut window) }.map_err(|e| CaptureError::Window(format!("GetWindowRect: {e}")))?;
    Ok(window)
}

/// Outer rect for scaling capture frames: DWM extended frame when available.
fn outer_window_rect(hwnd: HWND) -> Result<RECT, CaptureError> {
    match extended_frame_bounds(hwnd) {
        Some(rect) => Ok(rect),
        None => window_rect(hwnd),
    }
}

impl ClientAreaMetrics {
    /// Read metrics for `hwnd`.
    pub(crate) fn from_hwnd(hwnd: isize) -> Result<Self, CaptureError> {
        let hwnd = HWND(hwnd as *mut _);
        let window = outer_window_rect(hwnd)?;
        let client = client_rect(hwnd)?;
        let origin = client_origin_screen(hwnd, client)?;

        let (window_width, window_height) = rect_size(window);
        let (mut client_width, mut client_height) = rect_size(client);
        if window_width == 0 || window_height == 0 || client_width == 0 || client_height == 0 {
            return Err(CaptureError::Window("empty window/client rect".into()));
        }

        let offset_x = (origin.x - window.left).max(0) as u32;
        let offset_y = (origin.y - window.top).max(0) as u32;
        let offset_x = offset_x.min(window_width.saturating_sub(1));
        let offset_y = offset_y.min(window_height.saturating_sub(1));
        client_width = client_width.min(window_width.saturating_sub(offset_x));
        client_height = client_height.min(window_height.saturating_sub(offset_y));

        Ok(Self {
            window_width,
            window_height,
            client_width,
            client_height,
            offset_x,
            offset_y,
        })
    }

    fn scaled_crop(&self, frame_w: u32, frame_h: u32) -> CropRect {
        let scale_x = frame_w as f32 / self.window_width.max(1) as f32;
        let scale_y = frame_h as f32 / self.window_height.max(1) as f32;

        let x = (self.offset_x as f32 * scale_x).round() as u32;
        let y = (self.offset_y as f32 * scale_y).round() as u32;
        let x = x.min(frame_w.saturating_sub(1));
        let y = y.min(frame_h.saturating_sub(1));
        let width = (self.client_width as f32 * scale_x).round() as u32;
        let height = (self.client_height as f32 * scale_y).round() as u32;
        CropRect {
            x,
            y,
            width: width.max(1).min(frame_w.saturating_sub(x)),
            height: height.max(1).min(frame_h.saturating_sub(y)),
        }
    }

    /// Crop a full-window capture frame down to the client area.
    ///
    /// Graphics Capture frame size may differ slightly from the outer window
    /// rect (DPI / shadows); offsets are scaled to the frame dimensions.
    pub(crate) fn crop_frame(&self, frame: &CapturedFrame) -> CapturedFrame {
        if frame.width == 0 || frame.height == 0 {
            return frame.clone();
        }

        let crop = self.scaled_crop(frame.width, frame.height);
        if crop.covers_frame(frame.width, frame.height) {
            return frame.clone();
        }

        let mut rgba = vec![0u8; crop.width as usize * crop.height as usize * 4];
        for row in 0..crop.height {
            let src_y = (crop.y + row) as usize;
            let src = (src_y * frame.width as usize + crop.x as usize) * 4;
            let dst = row as usize * crop.width as usize * 4;
            let n = crop.width as usize * 4;
            if src + n <= frame.rgba.len() && dst + n <= rgba.len() {
                rgba[dst..dst + n].copy_from_slice(&frame.rgba[src..src + n]);
            }
        }

        CapturedFrame::new(crop.width, crop.height, rgba, frame.sequence)
    }
}

/// Crop `frame` to the client area of `hwnd` when possible.
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
        }
    }

    #[test]
    fn crop_frame_extracts_subrect() {
        // 4x3 frame, client is 2x2 starting at (1,1)
        let mut rgba = vec![0u8; 4 * 3 * 4];
        let i = (4 + 1) * 4;
        rgba[i] = 255;
        rgba[i + 1] = 0;
        rgba[i + 2] = 0;
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
}
