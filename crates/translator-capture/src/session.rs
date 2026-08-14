//! Free-threaded capture session that publishes the latest frame.

use std::{sync::Arc, time::Duration};

use arc_swap::ArcSwapOption;
use bytes::Bytes;
use tracing::{info, warn};
use windows_capture::{
    capture::{Context, GraphicsCaptureApiHandler},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings, MinimumUpdateIntervalSettings,
        SecondaryWindowSettings, Settings,
    },
    window::Window,
};

use crate::{CaptureError, CapturedFrame, resize_watch::ResizeWatch};

/// Latest-wins slot: the capture thread overwrites; the pipeline takes the newest frame.
struct SharedLatest {
    slot: ArcSwapOption<CapturedFrame>,
}

impl SharedLatest {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            slot: ArcSwapOption::empty(),
        })
    }

    fn publish(&self, frame: CapturedFrame) {
        self.slot.store(Some(Arc::new(frame)));
    }

    fn latest(&self) -> Option<CapturedFrame> {
        self.slot.load_full().map(|frame| (*frame).clone())
    }
}

/// Copy tightly packed RGBA8 out of a mapped WGC buffer into owned [`Bytes`].
///
/// Mapped GPU memory cannot be retained. When the crate copies rows into `scratch`
/// (padded pitch), that allocation is frozen into `Bytes` to avoid a second copy.
fn pack_rgba(frame: &mut Frame<'_>, scratch: &mut Vec<u8>) -> Result<Bytes, CaptureError> {
    let buffer = frame.buffer().map_err(|e| CaptureError::Frame(e.to_string()))?;
    if buffer.has_padding() {
        let n = buffer.as_nopadding_buffer(scratch).len();
        scratch.truncate(n);
        Ok(Bytes::from(std::mem::take(scratch)))
    } else {
        Ok(Bytes::copy_from_slice(buffer.as_nopadding_buffer(scratch)))
    }
}

struct FrameHandler {
    latest: Arc<SharedLatest>,
    sequence: u64,
    scratch: Vec<u8>,
}

impl GraphicsCaptureApiHandler for FrameHandler {
    type Error = CaptureError;
    type Flags = Arc<SharedLatest>;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self {
            latest: ctx.flags,
            sequence: 0,
            scratch: Vec::new(),
        })
    }

    fn on_frame_arrived(&mut self, frame: &mut Frame<'_>, _capture_control: InternalCaptureControl) -> Result<(), Self::Error> {
        let width = frame.width();
        let height = frame.height();
        if width == 0 || height == 0 {
            return Ok(());
        }

        let rgba = pack_rgba(frame, &mut self.scratch)?;
        self.sequence += 1;
        self.latest.publish(CapturedFrame::new(width, height, rgba, self.sequence));
        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        info!("capture target closed");
        Ok(())
    }
}

#[derive(Clone)]
struct CaptureTarget {
    hwnd: isize,
    title: String,
}

struct ActiveStream {
    control: windows_capture::capture::CaptureControl<FrameHandler, CaptureError>,
    latest: Arc<SharedLatest>,
}

/// Active capture session.
pub struct CaptureSession {
    stream: Option<ActiveStream>,
    target: Option<CaptureTarget>,
    min_interval_ms: u64,
    resize: ResizeWatch,
}

impl Default for CaptureSession {
    fn default() -> Self {
        Self::new()
    }
}

impl CaptureSession {
    pub fn new() -> Self {
        Self {
            stream: None,
            target: None,
            min_interval_ms: 250,
            resize: ResizeWatch::new(),
        }
    }

    pub fn is_running(&self) -> bool {
        self.stream.as_ref().is_some_and(|s| !s.control.is_finished())
    }

    pub fn target_hwnd(&self) -> Option<isize> {
        self.target.as_ref().map(|t| t.hwnd)
    }

    pub fn target_title(&self) -> Option<String> {
        self.target.as_ref().map(|t| t.title.clone())
    }

    /// Start capturing a window by HWND.
    pub fn start_window(&mut self, hwnd: isize, title: impl Into<String>, min_interval_ms: u64) -> Result<(), CaptureError> {
        if self.is_running() {
            return Err(CaptureError::AlreadyRunning);
        }
        self.stop_stream_inner(true);

        let window = Window::from_raw_hwnd(hwnd as *mut _);
        if !window.is_valid() {
            return Err(CaptureError::Window("invalid hwnd".into()));
        }

        let latest = SharedLatest::new();
        let settings = Settings::new(
            window,
            CursorCaptureSettings::WithoutCursor,
            DrawBorderSettings::WithoutBorder,
            SecondaryWindowSettings::Default,
            MinimumUpdateIntervalSettings::Custom(Duration::from_millis(min_interval_ms.max(50))),
            DirtyRegionSettings::Default,
            ColorFormat::Rgba8,
            Arc::clone(&latest),
        );

        let control = FrameHandler::start_free_threaded(settings).map_err(|e| CaptureError::Capture(e.to_string()))?;
        let title = title.into();
        info!(%title, hwnd, "capture started");
        self.min_interval_ms = min_interval_ms;
        self.stream = Some(ActiveStream { control, latest });
        self.target = Some(CaptureTarget { hwnd, title });
        self.resize.set_target(hwnd);
        Ok(())
    }

    /// Start capturing the current foreground window.
    pub fn start_foreground(&mut self, min_interval_ms: u64) -> Result<(), CaptureError> {
        let window = Window::foreground().map_err(|e| CaptureError::Window(e.to_string()))?;
        let title = window.title().unwrap_or_else(|_| "Foreground".into());
        let hwnd = window.as_raw_hwnd() as isize;
        self.start_window(hwnd, title, min_interval_ms)
    }

    /// Stop the capture stream but keep the target window (overlay tracking).
    pub fn stop_stream_keep_target(&mut self) {
        self.stop_stream_inner(true);
    }

    /// Stop capture if running and clear the target window.
    pub fn stop(&mut self) {
        self.stop_stream_inner(false);
    }

    fn stop_stream_inner(&mut self, keep_target: bool) {
        if let Some(stream) = self.stream.take() {
            if let Err(e) = stream.control.stop() {
                warn!(error = %e, "error stopping capture");
            }
            info!(keep_target, "capture stopped");
        }
        if !keep_target {
            self.target = None;
            self.resize.set_target(0);
        }
    }

    /// Recreate the WGC session after a target-window resize settles.
    ///
    /// Graphics Capture keeps the buffer layout from session start; a fresh
    /// session matches a manual Stop + Start. Edge-drag waits for
    /// `EVENT_SYSTEM_MOVESIZEEND`. Maximize / snap restart on the first
    /// size-changing `EVENT_OBJECT_LOCATIONCHANGE`.
    pub fn sync_stream(&mut self) -> bool {
        if !self.is_running() || !self.resize.take_pending() {
            return false;
        }

        match self.restart_stream() {
            Ok(()) => true,
            Err(e) => {
                warn!(error = %e, "failed to restart capture after target resize");
                false
            }
        }
    }

    fn restart_stream(&mut self) -> Result<(), CaptureError> {
        let target = self
            .target
            .clone()
            .ok_or_else(|| CaptureError::Window("no capture target".into()))?;
        let interval = self.min_interval_ms;
        info!(hwnd = target.hwnd, "restarting capture after target resize");
        self.stop_stream_inner(true);
        self.start_window(target.hwnd, target.title, interval)
    }

    /// Latest published frame, cropped to the client area. Does not consume the slot.
    pub fn latest_frame(&self) -> Option<CapturedFrame> {
        let frame = self.stream.as_ref()?.latest.latest()?;
        Some(self.crop_to_client(frame))
    }

    fn crop_to_client(&self, frame: CapturedFrame) -> CapturedFrame {
        match self.target_hwnd() {
            Some(hwnd) => crate::client_area::crop_frame_to_client(hwnd, frame),
            None => frame,
        }
    }
}

impl Drop for CaptureSession {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(seq: u64) -> CapturedFrame {
        CapturedFrame::new(1, 1, Bytes::from_static(&[1, 2, 3, 4]), seq)
    }

    #[test]
    fn latest_overwrites_unread() {
        let slot = SharedLatest::new();
        slot.publish(frame(1));
        slot.publish(frame(2));
        let got = slot.latest().expect("frame");
        assert_eq!(got.sequence, 2);
        assert_eq!(slot.latest().expect("still there").sequence, 2);
        slot.publish(frame(3));
        assert_eq!(slot.latest().expect("replaced").sequence, 3);
    }
}
