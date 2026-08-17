//! Win32 layered overlay window.

mod captions;
mod follow;
mod pump;
mod tick;
pub(crate) mod win32;
pub(crate) mod wnd;

use tokio::sync::mpsc;
use tracing::{debug, warn};
use translator_core::{OverlayConfig, TranslatedBlock};
use windows::{
    Win32::{
        Foundation::HWND,
        Graphics::Gdi::{DeleteObject, HFONT},
        System::{LibraryLoader::GetModuleHandleW, Threading::GetCurrentThreadId},
        UI::{
            Accessibility::HWINEVENTHOOK,
            WindowsAndMessaging::{
                CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DestroyWindow, LoadCursorW, RegisterClassExW, SW_HIDE, ShowWindow,
                UnregisterClassW, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP,
            },
        },
    },
    core::w,
};

pub(crate) use crate::host::follow::wake_overlay_thread;
use crate::{
    command::{OverlayCommand, OverlayEvent},
    error::OverlayError,
    gfx::{surface::DibSurface, text},
    host::{
        follow::{FOLLOW_SYNC, FOLLOW_TARGET, FOLLOW_THREAD, uninstall_follow_hooks},
        wnd::{CLASS_NAME, overlay_wnd_proc},
    },
    picker::{PickerEnd, RegionPicker},
    reader::{ReaderWindow, format_reader_text},
};

pub(crate) struct OverlayHost {
    pub(crate) hwnd: HWND,
    pub(crate) class_atom: u16,
    pub(crate) config: OverlayConfig,
    pub(crate) target: Option<HWND>,
    pub(crate) blocks: Vec<TranslatedBlock>,
    pub(crate) content_w: u32,
    pub(crate) content_h: u32,
    /// Paint buffer size in **OCR / capture content** pixels (not screen client).
    pub(crate) surface_w: i32,
    pub(crate) surface_h: i32,
    pub(crate) dirty: bool,
    pub(crate) surface: DibSurface,
    pub(crate) present: DibSurface,
    pub(crate) hfont: HFONT,
    pub(crate) font_px: i32,
    pub(crate) reader: Option<Box<ReaderWindow>>,
    pub(crate) picker: Option<RegionPicker>,
    pub(crate) event_tx: mpsc::UnboundedSender<OverlayEvent>,
    pub(crate) follow_hooks: [HWINEVENTHOOK; 3],
}

impl OverlayHost {
    pub fn create(config: OverlayConfig, event_tx: mpsc::UnboundedSender<OverlayEvent>) -> Result<Self, OverlayError> {
        let hinstance = unsafe { GetModuleHandleW(None) }.map_err(|e| OverlayError::Other(format!("GetModuleHandleW: {e}")))?;

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(overlay_wnd_proc),
            hInstance: hinstance.into(),
            hCursor: unsafe { LoadCursorW(None, windows::Win32::UI::WindowsAndMessaging::IDC_ARROW) }
                .map_err(|e| OverlayError::Other(format!("LoadCursorW: {e}")))?,
            lpszClassName: CLASS_NAME,
            ..Default::default()
        };

        let atom = unsafe { RegisterClassExW(&wc) };
        if atom == 0 {
            // Class may already exist from a previous run in the same process.
            // Continue — CreateWindowEx will still work if registered.
        }

        // Not TOPMOST: only float above the target while it is in the
        // foreground; otherwise we hide so other apps are not covered.
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                CLASS_NAME,
                w!("Translator Overlay"),
                WS_POPUP,
                CW_USEDEFAULT,
                0,
                100,
                100,
                None,
                None,
                Some(hinstance.into()),
                None,
            )
        }
        .map_err(|e| OverlayError::Other(format!("CreateWindowExW: {e}")))?;

        let surface = match DibSurface::create(hwnd) {
            Ok(s) => s,
            Err(e) => {
                let _ = unsafe { DestroyWindow(hwnd) };
                return Err(e);
            }
        };
        let present = match DibSurface::create(hwnd) {
            Ok(s) => s,
            Err(e) => {
                drop(surface);
                let _ = unsafe { DestroyWindow(hwnd) };
                return Err(e);
            }
        };

        let font_px = 16;
        let hfont = match text::create_segoe_font(font_px) {
            Ok(font) => font,
            Err(e) => {
                drop(present);
                drop(surface);
                let _ = unsafe { DestroyWindow(hwnd) };
                return Err(e);
            }
        };

        let mut host = Self {
            hwnd,
            class_atom: atom,
            config,
            target: None,
            blocks: Vec::new(),
            content_w: 0,
            content_h: 0,
            surface_w: 0,
            surface_h: 0,
            dirty: true,
            surface,
            present,
            hfont,
            font_px,
            reader: None,
            picker: None,
            event_tx,
            follow_hooks: [HWINEVENTHOOK::default(); 3],
        };

        FOLLOW_THREAD.store(unsafe { GetCurrentThreadId() }, std::sync::atomic::Ordering::Release);

        let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
        host.surface.ensure(100, 100)?;
        match ReaderWindow::create(&host.config) {
            Ok(reader) => host.reader = Some(reader),
            Err(e) => {
                host.teardown();
                return Err(e);
            }
        }
        Ok(host)
    }

    pub(crate) fn handle(&mut self, cmd: OverlayCommand) {
        match cmd {
            OverlayCommand::Attach { target_hwnd } => {
                let hwnd = HWND(target_hwnd as *mut _);
                if unsafe { windows::Win32::UI::WindowsAndMessaging::IsWindow(Some(hwnd)) }.as_bool() {
                    self.target = Some(hwnd);
                    FOLLOW_TARGET.store(target_hwnd, std::sync::atomic::Ordering::Release);
                    self.dirty = true;
                    debug!(?target_hwnd, "overlay attached");
                } else {
                    warn!(?target_hwnd, "attach ignored — invalid hwnd");
                    self.target = None;
                    FOLLOW_TARGET.store(0, std::sync::atomic::Ordering::Release);
                }
            }
            OverlayCommand::Detach => {
                if self.picker.is_some() {
                    self.finish_picker(PickerEnd::Cancel);
                }
                self.target = None;
                FOLLOW_TARGET.store(0, std::sync::atomic::Ordering::Release);
                self.hide();
            }
            OverlayCommand::SetBlocks {
                blocks,
                content_width,
                content_height,
            } => {
                let size_changed = self.content_w != content_width || self.content_h != content_height;
                self.blocks = blocks;
                self.content_w = content_width;
                self.content_h = content_height;
                if size_changed {
                    self.surface_w = content_width as i32;
                    self.surface_h = content_height as i32;
                }
                self.dirty = true;
                if let Some(reader) = self.reader.as_mut() {
                    reader.set_text(&format_reader_text(&self.blocks));
                }
            }
            OverlayCommand::Clear => {
                self.blocks.clear();
                self.content_w = 0;
                self.content_h = 0;
                self.dirty = true;
                self.hide();
                if let Some(reader) = self.reader.as_mut() {
                    reader.set_text("");
                }
            }
            OverlayCommand::UpdateConfig(cfg) => {
                self.config = cfg;
                self.dirty = true;
                if let Some(reader) = self.reader.as_mut() {
                    reader.apply_config(&self.config);
                }
                if !self.config.enabled && self.picker.is_none() {
                    self.hide();
                }
            }
            OverlayCommand::BeginRegionSelect { regions } => self.begin_picker(regions),
            OverlayCommand::CancelRegionSelect => {
                if self.picker.is_some() {
                    self.finish_picker(PickerEnd::Cancel);
                }
            }
            OverlayCommand::ConfirmRegionSelect => {
                if self.picker.is_some() {
                    self.finish_picker(PickerEnd::Confirm);
                }
            }
            OverlayCommand::ClearRegionSelect => {
                if let Some(p) = self.picker.as_mut() {
                    p.regions.clear();
                    p.selected = None;
                    let _ = self.event_tx.send(OverlayEvent::RegionSelectUpdated(Vec::new()));
                    self.dirty = true;
                }
            }
            OverlayCommand::Shutdown => {}
        }
    }

    pub(crate) fn present_to_client(&mut self, x: i32, y: i32, client_w: i32, client_h: i32) -> Result<(), OverlayError> {
        let (bw, bh) = self.surface.size();
        if bw == client_w && bh == client_h {
            self.surface.present(self.hwnd, x, y, client_w, client_h)
        } else {
            self.surface.stretch_into(&mut self.present, client_w, client_h)?;
            self.present.present(self.hwnd, x, y, client_w, client_h)
        }
    }

    pub(crate) fn teardown(&mut self) {
        FOLLOW_TARGET.store(0, std::sync::atomic::Ordering::Release);
        FOLLOW_THREAD.store(0, std::sync::atomic::Ordering::Release);
        FOLLOW_SYNC.store(false, std::sync::atomic::Ordering::Release);
        uninstall_follow_hooks(&mut self.follow_hooks);
        if let Some(mut reader) = self.reader.take() {
            reader.teardown();
        }
        if !self.hwnd.is_invalid() {
            let _ = unsafe { DestroyWindow(self.hwnd) };
            self.hwnd = HWND::default();
        }
        if !self.hfont.is_invalid() {
            let _ = unsafe { DeleteObject(self.hfont.into()) };
            self.hfont = HFONT::default();
        }
        self.surface.teardown();
        self.present.teardown();
        if self.class_atom != 0 {
            if let Ok(hi) = unsafe { GetModuleHandleW(None) } {
                let _ = unsafe { UnregisterClassW(CLASS_NAME, Some(hi.into())) };
            }
            self.class_atom = 0;
        }
    }
}

impl Drop for OverlayHost {
    fn drop(&mut self) {
        self.teardown();
    }
}
