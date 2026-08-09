//! Free-threaded capture session that streams frames over a channel.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::Duration;

use tracing::{info, warn};
use windows_capture::capture::{Context, GraphicsCaptureApiHandler};
use windows_capture::frame::Frame;
use windows_capture::graphics_capture_api::InternalCaptureControl;
use windows_capture::settings::{
    ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
    MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
};
use windows_capture::window::Window;

use crate::{CaptureError, CapturedFrame};

/// Flags passed into the capture handler.
#[derive(Clone)]
struct HandlerFlags {
    tx: Sender<CapturedFrame>,
    sequence: Arc<AtomicU64>,
}

struct FrameHandler {
    tx: Sender<CapturedFrame>,
    sequence: Arc<AtomicU64>,
    scratch: Vec<u8>,
}

impl GraphicsCaptureApiHandler for FrameHandler {
    type Flags = HandlerFlags;
    type Error = CaptureError;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self {
            tx: ctx.flags.tx,
            sequence: ctx.flags.sequence,
            scratch: Vec::new(),
        })
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame<'_>,
        _capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let width = frame.width();
        let height = frame.height();
        if width == 0 || height == 0 {
            return Ok(());
        }

        let buffer = frame
            .buffer()
            .map_err(|e| CaptureError::Frame(e.to_string()))?;
        let pixels = buffer.as_nopadding_buffer(&mut self.scratch);
        let rgba = pixels.to_vec();

        let seq = self.sequence.fetch_add(1, Ordering::Relaxed) + 1;
        let captured = CapturedFrame {
            width,
            height,
            rgba,
            sequence: seq,
        };

        // Drop frame if consumer is slow (keep only latest via try_send pattern
        // using a bounded channel of 1 managed outside). Here we use unbounded
        // but consumer drains and keeps latest.
        if self.tx.send(captured).is_err() {
            // Receiver dropped — stop capture on next iteration by erroring out.
            return Err(CaptureError::Capture("frame receiver closed".into()));
        }
        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        info!("capture target closed");
        Ok(())
    }
}

/// Active capture session.
pub struct CaptureSession {
    control: Option<windows_capture::capture::CaptureControl<FrameHandler, CaptureError>>,
    rx: Option<Receiver<CapturedFrame>>,
    sequence: Arc<AtomicU64>,
    pub target_title: Option<String>,
    /// HWND of the capture target (for overlay tracking).
    pub target_hwnd: Option<isize>,
    pub running: bool,
}

impl Default for CaptureSession {
    fn default() -> Self {
        Self::new()
    }
}

impl CaptureSession {
    pub fn new() -> Self {
        Self {
            control: None,
            rx: None,
            sequence: Arc::new(AtomicU64::new(0)),
            target_title: None,
            target_hwnd: None,
            running: false,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running && self.control.as_ref().is_some_and(|c| !c.is_finished())
    }

    /// Start capturing a window by HWND.
    pub fn start_window(
        &mut self,
        hwnd: isize,
        title: impl Into<String>,
        min_interval_ms: u64,
    ) -> Result<(), CaptureError> {
        if self.is_running() {
            return Err(CaptureError::AlreadyRunning);
        }

        let window = Window::from_raw_hwnd(hwnd as *mut _);
        if !window.is_valid() {
            return Err(CaptureError::Window("invalid hwnd".into()));
        }

        let (tx, rx) = mpsc::channel();
        self.sequence.store(0, Ordering::Relaxed);

        let settings = Settings::new(
            window,
            CursorCaptureSettings::WithoutCursor,
            DrawBorderSettings::WithoutBorder,
            SecondaryWindowSettings::Default,
            MinimumUpdateIntervalSettings::Custom(Duration::from_millis(min_interval_ms.max(50))),
            DirtyRegionSettings::Default,
            ColorFormat::Rgba8,
            HandlerFlags {
                tx,
                sequence: Arc::clone(&self.sequence),
            },
        );

        let control = FrameHandler::start_free_threaded(settings)
            .map_err(|e| CaptureError::Capture(e.to_string()))?;

        self.control = Some(control);
        self.rx = Some(rx);
        self.target_title = Some(title.into());
        self.target_hwnd = Some(hwnd);
        self.running = true;
        info!(title = ?self.target_title, hwnd, "capture started");
        Ok(())
    }

    /// Start capturing the current foreground window.
    pub fn start_foreground(&mut self, min_interval_ms: u64) -> Result<(), CaptureError> {
        let window = Window::foreground().map_err(|e| CaptureError::Window(e.to_string()))?;
        let title = window.title().unwrap_or_else(|_| "Foreground".into());
        let hwnd = window.as_raw_hwnd() as isize;
        self.start_window(hwnd, title, min_interval_ms)
    }

    /// Stop capture if running.
    pub fn stop(&mut self) {
        if let Some(control) = self.control.take()
            && let Err(e) = control.stop()
        {
            warn!(error = %e, "error stopping capture");
        }
        self.rx = None;
        self.target_hwnd = None;
        self.running = false;
        info!("capture stopped");
    }

    /// Drain the channel and return the latest frame, if any.
    ///
    /// Frames are cropped to the **client area** (no title bar / window border).
    pub fn take_latest_frame(&mut self) -> Option<CapturedFrame> {
        let rx = self.rx.as_ref()?;
        let mut latest = None;
        loop {
            match rx.try_recv() {
                Ok(frame) => latest = Some(frame),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.running = false;
                    break;
                }
            }
        }
        latest.map(|f| self.crop_to_client(f))
    }

    /// Blocking wait for the next frame with timeout.
    ///
    /// Frames are cropped to the client area.
    pub fn recv_frame_timeout(&mut self, timeout: Duration) -> Option<CapturedFrame> {
        let rx = self.rx.as_ref()?;
        match rx.recv_timeout(timeout) {
            Ok(frame) => {
                // Drain extras, keep newest.
                let mut latest = frame;
                while let Ok(f) = rx.try_recv() {
                    latest = f;
                }
                Some(self.crop_to_client(latest))
            }
            Err(_) => None,
        }
    }

    fn crop_to_client(&self, frame: CapturedFrame) -> CapturedFrame {
        match self.target_hwnd {
            Some(hwnd) => crate::crop_frame_to_client(hwnd, frame),
            None => frame,
        }
    }
}

impl Drop for CaptureSession {
    fn drop(&mut self) {
        self.stop();
    }
}
