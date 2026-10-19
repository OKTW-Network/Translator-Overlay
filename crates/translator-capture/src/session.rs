//! Free-threaded capture session that publishes the latest frame.

use std::{
    sync::{Arc, Mutex, MutexGuard, PoisonError, atomic::Ordering},
    time::Duration,
};

use tracing::{info, warn};
use windows::{
    Win32::Graphics::Direct3D11::{
        D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_TEXTURE2D_DESC, D3D11_USAGE_DEFAULT, ID3D11Device, ID3D11DeviceContext,
        ID3D11Multithread, ID3D11Texture2D,
    },
    core::Interface,
};
use windows_capture::{
    capture::{Context, GraphicsCaptureApiHandler},
    d3d11::StagingTexture,
    frame::Frame,
    graphics_capture_api::{GraphicsCaptureApi, InternalCaptureControl},
    settings::{
        ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings, MinimumUpdateIntervalSettings,
        SecondaryWindowSettings, Settings,
    },
    window::Window,
};

use crate::{
    CaptureError, CapturedFrame,
    client_area::crop_frame_to_client,
    resize_watch::{IN_MOVESIZE, ResizeWatch},
};

/// Last WGC frame, copied GPU→GPU so it outlives the frame pool buffer.
struct GpuFrame {
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    texture: ID3D11Texture2D,
    desc: D3D11_TEXTURE2D_DESC,
}

/// Latest-wins slot. The capture thread only does a GPU copy per frame; the
/// CPU readback runs when the pipeline asks, at most once per sequence.
#[derive(Default)]
struct Slot {
    gpu: Option<GpuFrame>,
    staging: Option<StagingTexture>,
    sequence: u64,
    /// Last readback; current while its sequence equals `sequence`.
    frame: Option<CapturedFrame>,
}

impl Slot {
    /// Newest frame as tightly packed RGBA8. Cloning only bumps the `Bytes` refcount.
    fn read(&mut self) -> Result<Option<CapturedFrame>, CaptureError> {
        if let Some(frame) = self.frame.as_ref().filter(|f| f.sequence == self.sequence) {
            return Ok(Some(frame.clone()));
        }
        let Some(gpu) = self.gpu.as_ref() else {
            return Ok(None);
        };
        let (width, height) = (gpu.desc.Width, gpu.desc.Height);
        let staging = match &mut self.staging {
            Some(staging) => staging,
            None => self
                .staging
                .insert(StagingTexture::new(&gpu.device, width, height, gpu.desc.Format).map_err(|e| CaptureError::Frame(e.to_string()))?),
        };

        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        // SAFETY: both textures belong to `gpu.device` and share size and format;
        // `staging` is CPU-readable and unmapped outside this block.
        unsafe {
            gpu.context.CopyResource(staging.texture(), &gpu.texture);
            gpu.context.Map(staging.texture(), 0, D3D11_MAP_READ, 0, Some(&mut mapped))
        }
        .map_err(|e| CaptureError::Frame(e.to_string()))?;
        let pitch = mapped.RowPitch as usize;
        let row = width as usize * 4;
        let mut rgba = Vec::with_capacity(row * height as usize);
        for y in 0..height as usize {
            // SAFETY: the mapping spans `height` rows of `RowPitch` bytes, and `row <= RowPitch`.
            rgba.extend_from_slice(unsafe { std::slice::from_raw_parts(mapped.pData.cast::<u8>().add(y * pitch), row) });
        }
        // SAFETY: mapped above on the same context.
        unsafe { gpu.context.Unmap(staging.texture(), 0) };

        let frame = CapturedFrame::new(width, height, rgba, self.sequence);
        self.frame = Some(frame.clone());
        Ok(Some(frame))
    }
}

type LatestFrame = Arc<Mutex<Slot>>;

fn lock(latest: &LatestFrame) -> MutexGuard<'_, Slot> {
    latest.lock().unwrap_or_else(PoisonError::into_inner)
}

struct FrameHandler {
    latest: LatestFrame,
}

impl GraphicsCaptureApiHandler for FrameHandler {
    type Error = CaptureError;
    type Flags = LatestFrame;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(Self { latest: ctx.flags })
    }

    fn on_frame_arrived(&mut self, frame: &mut Frame<'_>, _capture_control: InternalCaptureControl) -> Result<(), Self::Error> {
        // Title-bar / edge drag: do not copy the WGC buffer. Overlay
        // follow keeps the last captions; OCR waits until MOVESIZEEND.
        if IN_MOVESIZE.load(Ordering::Acquire) {
            return Ok(());
        }

        let width = frame.width();
        let height = frame.height();
        if width == 0 || height == 0 {
            return Ok(());
        }

        let mut slot = lock(&self.latest);
        if slot.gpu.as_ref().is_none_or(|g| g.desc.Width != width || g.desc.Height != height) {
            let context = frame.device_context().clone();
            // The pipeline thread reads back on this same immediate context.
            let multithread = context
                .cast::<ID3D11Multithread>()
                .map_err(|e| CaptureError::Frame(e.to_string()))?;
            // SAFETY: plain flag toggle on a live context.
            let _ = unsafe { multithread.SetMultithreadProtected(true) };
            let desc = D3D11_TEXTURE2D_DESC {
                Usage: D3D11_USAGE_DEFAULT,
                BindFlags: 0,
                CPUAccessFlags: 0,
                MiscFlags: 0,
                ..*frame.desc()
            };
            let device = frame.device().clone();
            let mut texture = None;
            // SAFETY: `desc` is a valid single-mip 2D texture description.
            unsafe { device.CreateTexture2D(&desc, None, Some(&mut texture)) }.map_err(|e| CaptureError::Frame(e.to_string()))?;
            let texture = texture.ok_or_else(|| CaptureError::Frame("CreateTexture2D returned no texture".into()))?;
            slot.gpu = Some(GpuFrame {
                device,
                context,
                texture,
                desc,
            });
            slot.staging = None;
        }
        if let Some(gpu) = slot.gpu.as_ref() {
            // SAFETY: same device, size, and format as the frame texture.
            unsafe { gpu.context.CopyResource(&gpu.texture, frame.as_raw_texture()) };
        }
        slot.sequence += 1;
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
    min_interval: Duration,
}

struct ActiveStream {
    control: windows_capture::capture::CaptureControl<FrameHandler, CaptureError>,
    latest: LatestFrame,
}

/// Active capture session.
pub struct CaptureSession {
    stream: Option<ActiveStream>,
    target: Option<CaptureTarget>,
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

    /// Interactive title-bar drag or edge resize is in progress.
    pub fn in_movesize(&self) -> bool {
        self.resize.in_movesize()
    }

    /// Latest published raw frame sequence (O(1), no crop / Win32 calls).
    ///
    /// Lets callers detect "no new frame" before paying for client-area crop.
    pub fn latest_sequence(&self) -> Option<u64> {
        let sequence = lock(&self.stream.as_ref()?.latest).sequence;
        (sequence > 0).then_some(sequence)
    }

    /// Start capturing a window by HWND.
    ///
    /// `min_interval` throttles WGC frame delivery where the OS supports it.
    pub fn start_window(&mut self, hwnd: isize, title: impl Into<String>, min_interval: Duration) -> Result<(), CaptureError> {
        if self.is_running() {
            return Err(CaptureError::AlreadyRunning);
        }
        self.stop_stream_inner(true);

        let window = Window::from_raw_hwnd(hwnd as *mut _);
        if !window.is_valid() {
            return Err(CaptureError::Window("invalid hwnd".into()));
        }

        let latest = LatestFrame::default();
        // windows-capture fails the whole Start if a non-Default setting's WinRT
        // property is missing (IsCursorCaptureEnabled / IsBorderRequired).
        let cursor = if GraphicsCaptureApi::is_cursor_settings_supported().unwrap_or(false) {
            CursorCaptureSettings::WithoutCursor
        } else {
            CursorCaptureSettings::Default
        };
        let border = if GraphicsCaptureApi::is_border_settings_supported().unwrap_or(false) {
            DrawBorderSettings::WithoutBorder
        } else {
            DrawBorderSettings::Default
        };
        let interval = if GraphicsCaptureApi::is_minimum_update_interval_supported().unwrap_or(false) {
            MinimumUpdateIntervalSettings::Custom(min_interval)
        } else {
            MinimumUpdateIntervalSettings::Default
        };
        let settings = Settings::new(
            window,
            cursor,
            border,
            SecondaryWindowSettings::Default,
            interval,
            DirtyRegionSettings::Default,
            ColorFormat::Rgba8,
            Arc::clone(&latest),
        );

        let control = FrameHandler::start_free_threaded(settings).map_err(|e| CaptureError::Capture(e.to_string()))?;
        let title = title.into();
        info!(%title, hwnd, ?cursor, ?border, ?interval, "capture started");
        self.stream = Some(ActiveStream { control, latest });
        self.target = Some(CaptureTarget { hwnd, title, min_interval });
        self.resize.set_target(hwnd);
        Ok(())
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

    /// Apply a new WGC frame throttle, restarting a running stream when it changed.
    ///
    /// Returns `true` when the stream restarted (frame sequence renumbered).
    pub fn set_min_interval(&mut self, min_interval: Duration) -> bool {
        let Some(target) = self.target.as_mut().filter(|t| t.min_interval != min_interval) else {
            return false;
        };
        target.min_interval = min_interval;
        // Unsupported OS: the stream ignores the interval, so a restart buys nothing.
        if !self.is_running() || !GraphicsCaptureApi::is_minimum_update_interval_supported().unwrap_or(false) {
            return false;
        }
        match self.restart_stream() {
            Ok(()) => true,
            Err(e) => {
                warn!(error = %e, "failed to restart capture after interval change");
                false
            }
        }
    }

    fn restart_stream(&mut self) -> Result<(), CaptureError> {
        let target = self
            .target
            .clone()
            .ok_or_else(|| CaptureError::Window("no capture target".into()))?;
        info!(hwnd = target.hwnd, "restarting capture");
        self.stop_stream_inner(true);
        self.start_window(target.hwnd, target.title, target.min_interval)
    }

    /// Latest published frame, cropped to the client area. Does not consume the slot.
    ///
    /// Returns `None` during interactive move/size so callers do not crop via
    /// DWM or feed OCR a frame from before the drag.
    pub fn latest_frame(&self) -> Option<CapturedFrame> {
        if self.in_movesize() {
            return None;
        }
        let frame = match lock(&self.stream.as_ref()?.latest).read() {
            Ok(frame) => frame?,
            Err(e) => {
                warn!(error = %e, "capture frame readback failed");
                return None;
            }
        };
        Some(self.crop_to_client(frame))
    }

    fn crop_to_client(&self, frame: CapturedFrame) -> CapturedFrame {
        match self.target_hwnd() {
            Some(hwnd) => crop_frame_to_client(hwnd, frame),
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
    use windows::Win32::Graphics::{
        Direct3D11::D3D11_SUBRESOURCE_DATA,
        Dxgi::Common::{DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_SAMPLE_DESC},
    };
    use windows_capture::d3d11::create_d3d_device;

    use super::*;

    #[test]
    fn read_unpads_rows_and_caches_per_sequence() {
        let (device, context) = create_d3d_device().expect("d3d device");
        // 3 px wide → 12-byte rows; staging pitch is padded well past that.
        let (width, height) = (3u32, 2u32);
        let pixels: Vec<u8> = (0..(width * height * 4) as u8).collect();
        let desc = D3D11_TEXTURE2D_DESC {
            Width: width,
            Height: height,
            MipLevels: 1,
            ArraySize: 1,
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_DEFAULT,
            ..Default::default()
        };
        let init = D3D11_SUBRESOURCE_DATA {
            pSysMem: pixels.as_ptr().cast(),
            SysMemPitch: width * 4,
            SysMemSlicePitch: 0,
        };
        let mut texture = None;
        unsafe { device.CreateTexture2D(&desc, Some(&init), Some(&mut texture)) }.expect("texture");
        let mut slot = Slot {
            gpu: Some(GpuFrame {
                device,
                context,
                texture: texture.expect("texture"),
                desc,
            }),
            sequence: 1,
            ..Slot::default()
        };

        let first = slot.read().expect("read").expect("frame");
        assert_eq!((first.width, first.height, first.sequence), (width, height, 1));
        assert_eq!(&first.rgba[..], &pixels[..]);

        let again = slot.read().expect("read").expect("frame");
        assert_eq!(again.rgba.as_ptr(), first.rgba.as_ptr(), "same sequence must not read back again");

        slot.sequence = 2;
        let next = slot.read().expect("read").expect("frame");
        assert_eq!(next.sequence, 2);
        assert_eq!(&next.rgba[..], &pixels[..]);
    }
}
