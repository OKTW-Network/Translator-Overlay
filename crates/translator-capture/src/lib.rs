//! Window capture via Windows Graphics Capture API (`windows-capture`).

mod client_area;
mod resize_watch;
mod session;

use bytes::Bytes;
use thiserror::Error;
use windows_capture::window::Window;

pub use crate::session::CaptureSession;

#[derive(Debug, Error)]
pub enum CaptureError {
    #[error("capture failed: {0}")]
    Capture(String),
    #[error("window error: {0}")]
    Window(String),
    #[error("frame conversion failed: {0}")]
    Frame(String),
    #[error("capture already running")]
    AlreadyRunning,
}

/// A captured frame in tightly packed RGBA8 (`width * height * 4`, no row padding).
#[derive(Debug, Clone)]
pub struct CapturedFrame {
    pub width: u32,
    pub height: u32,
    pub rgba: Bytes,
    pub sequence: u64,
}

impl CapturedFrame {
    pub fn new(width: u32, height: u32, rgba: impl Into<Bytes>, sequence: u64) -> Self {
        Self {
            width,
            height,
            rgba: rgba.into(),
            sequence,
        }
    }
}

/// Summary of a capturable window for UI lists.
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub title: String,
    pub hwnd: isize,
}

/// Enumerate capturable top-level windows with a non-empty title.
pub fn list_windows() -> Result<Vec<WindowInfo>, CaptureError> {
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
