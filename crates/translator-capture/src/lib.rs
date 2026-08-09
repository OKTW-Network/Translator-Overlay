//! Window capture via Windows Graphics Capture API (`windows-capture`).

mod client_area;
mod session;

pub use client_area::*;
pub use session::*;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("capture failed: {0}")]
    Capture(String),
    #[error("window error: {0}")]
    Window(String),
    #[error("no capture target selected")]
    NoTarget,
    #[error("frame conversion failed: {0}")]
    Frame(String),
    #[error("capture already running")]
    AlreadyRunning,
    #[error("user cancelled window picker")]
    Cancelled,
}

/// A captured frame in RGBA8 (no padding).
#[derive(Debug, Clone)]
pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub sequence: u64,
}

impl CapturedFrame {
    /// Approximate memory size in bytes.
    pub fn byte_len(&self) -> usize {
        self.rgba.len()
    }

    /// Downscale for UI preview (nearest-neighbor, max edge `max_edge`).
    pub fn thumbnail(&self, max_edge: u32) -> CapturedFrame {
        if self.width == 0 || self.height == 0 {
            return self.clone();
        }
        let scale = (max_edge as f32 / self.width.max(self.height) as f32).min(1.0);
        if (scale - 1.0).abs() < f32::EPSILON {
            return self.clone();
        }
        let tw = ((self.width as f32) * scale).max(1.0) as u32;
        let th = ((self.height as f32) * scale).max(1.0) as u32;
        let mut out = vec![0u8; (tw * th * 4) as usize];
        for y in 0..th {
            for x in 0..tw {
                let sx = (x as f32 / tw as f32 * self.width as f32) as u32;
                let sy = (y as f32 / th as f32 * self.height as f32) as u32;
                let si = ((sy * self.width + sx) * 4) as usize;
                let di = ((y * tw + x) * 4) as usize;
                out[di..di + 4].copy_from_slice(&self.rgba[si..si + 4]);
            }
        }
        CapturedFrame {
            width: tw,
            height: th,
            rgba: out,
            sequence: self.sequence,
        }
    }

    /// Encode as PNG bytes (for preview / debug).
    pub fn to_png_bytes(&self) -> Result<Vec<u8>, CaptureError> {
        use image::ImageEncoder;
        let mut buf = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        encoder
            .write_image(
                &self.rgba,
                self.width,
                self.height,
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| CaptureError::Frame(e.to_string()))?;
        Ok(buf)
    }
}

/// Summary of a capturable window for UI lists.
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub title: String,
    pub hwnd: isize,
}

/// Enumerate capturable top-level windows.
pub fn list_windows() -> Result<Vec<WindowInfo>, CaptureError> {
    use windows_capture::window::Window;

    let windows = Window::enumerate().map_err(|e| CaptureError::Window(e.to_string()))?;
    let mut out = Vec::new();
    for w in windows {
        if !w.is_valid() {
            continue;
        }
        let title = w.title().unwrap_or_default();
        if title.trim().is_empty() {
            continue;
        }
        out.push(WindowInfo {
            title,
            hwnd: w.as_raw_hwnd() as isize,
        });
    }
    out.sort_by_key(|a| a.title.to_lowercase());
    Ok(out)
}

/// Resolve a window by exact title.
pub fn window_from_title(title: &str) -> Result<WindowInfo, CaptureError> {
    use windows_capture::window::Window;
    let w = Window::from_name(title).map_err(|e| CaptureError::Window(e.to_string()))?;
    Ok(WindowInfo {
        title: w.title().unwrap_or_else(|_| title.to_string()),
        hwnd: w.as_raw_hwnd() as isize,
    })
}

/// Resolve a window whose title contains `partial`.
pub fn window_from_contains(partial: &str) -> Result<WindowInfo, CaptureError> {
    use windows_capture::window::Window;
    let w = Window::from_contains_name(partial).map_err(|e| CaptureError::Window(e.to_string()))?;
    Ok(WindowInfo {
        title: w.title().unwrap_or_else(|_| partial.to_string()),
        hwnd: w.as_raw_hwnd() as isize,
    })
}

/// Current foreground window.
pub fn foreground_window() -> Result<WindowInfo, CaptureError> {
    use windows_capture::window::Window;
    let w = Window::foreground().map_err(|e| CaptureError::Window(e.to_string()))?;
    Ok(WindowInfo {
        title: w.title().unwrap_or_else(|_| "Foreground".into()),
        hwnd: w.as_raw_hwnd() as isize,
    })
}
