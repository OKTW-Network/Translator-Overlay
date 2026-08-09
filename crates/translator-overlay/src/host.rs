//! Win32 layered-window host thread.

use std::{
    mem::size_of,
    sync::mpsc::Receiver,
    time::{Duration, Instant},
};

use tracing::{debug, warn};
use translator_core::{OverlayConfig, TranslatedBlock};
use windows::{
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM},
        Graphics::Gdi::{
            AC_SRC_ALPHA, AC_SRC_OVER, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS,
            ClientToScreen, CreateCompatibleDC, CreateDIBSection, CreateFontW, DEFAULT_CHARSET, DEFAULT_PITCH, DIB_RGB_COLORS, DT_CALCRECT,
            DT_EDITCONTROL, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_TOP, DT_WORDBREAK, DeleteDC, DeleteObject, DrawTextW, FF_DONTCARE,
            FW_NORMAL, GetDC, GetTextMetricsW, HALFTONE, HBITMAP, HDC, HFONT, HGDIOBJ, OUT_DEFAULT_PRECIS, ReleaseDC, SelectObject,
            SetBkMode, SetStretchBltMode, SetTextColor, StretchBlt, TEXTMETRICW, TRANSPARENT,
        },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GA_ROOT, GetAncestor,
            GetClientRect, GetForegroundWindow, HTTRANSPARENT, HWND_NOTOPMOST, HWND_TOPMOST, IDC_ARROW, IsChild, IsIconic, IsWindow,
            IsWindowVisible, LoadCursorW, MSG, PM_REMOVE, PeekMessageW, PostQuitMessage, RegisterClassExW, SW_HIDE, SW_SHOWNOACTIVATE,
            SWP_HIDEWINDOW, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SetWindowPos, ShowWindow, TranslateMessage, ULW_ALPHA,
            UnregisterClassW, UpdateLayeredWindow, WM_DESTROY, WM_NCHITTEST, WM_QUIT, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE,
            WS_EX_TOOLWINDOW, WS_EX_TRANSPARENT, WS_POPUP,
        },
    },
    core::{PCWSTR, w},
};

use crate::{
    OverlayError,
    draw::{self, Rgba, SurfaceRect},
    layout::{label_pad, place_label},
};

/// Font size + text colour for one overlay label.
struct LabelStyle {
    font_px: i32,
    color: Rgba,
}

const CLASS_NAME: PCWSTR = w!("TranslatorOverlayLayer.v1");
const TICK: Duration = Duration::from_millis(33);

pub enum OverlayCommand {
    Attach {
        target_hwnd: isize,
    },
    Detach,
    SetBlocks {
        blocks: Vec<TranslatedBlock>,
        content_width: u32,
        content_height: u32,
    },
    Clear,
    UpdateConfig(OverlayConfig),
    Shutdown,
}

pub struct OverlayHost {
    hwnd: HWND,
    class_atom: u16,
    config: OverlayConfig,
    target: Option<HWND>,
    blocks: Vec<TranslatedBlock>,
    content_w: u32,
    content_h: u32,
    /// Paint buffer size in **OCR / capture content** pixels (not screen client).
    surface_w: i32,
    surface_h: i32,
    dirty: bool,
    hdc_screen: HDC,
    hdc_mem: HDC,
    hbmp: HBITMAP,
    bits: *mut u8,
    bmp_w: i32,
    bmp_h: i32,
    /// Optional stretched present buffer when client size ≠ content size.
    hdc_present: HDC,
    hbmp_present: HBITMAP,
    present_w: i32,
    present_h: i32,
    hfont: HFONT,
    font_px: i32,
}

impl OverlayHost {
    pub fn create(config: OverlayConfig) -> Result<Self, OverlayError> {
        unsafe {
            let hinstance = GetModuleHandleW(None).map_err(|e| OverlayError::Other(format!("GetModuleHandleW: {e}")))?;

            let wc = WNDCLASSEXW {
                cbSize: size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(wnd_proc),
                hInstance: hinstance.into(),
                hCursor: LoadCursorW(None, IDC_ARROW).map_err(|e| OverlayError::Other(format!("LoadCursorW: {e}")))?,
                lpszClassName: CLASS_NAME,
                ..Default::default()
            };

            let atom = RegisterClassExW(&wc);
            if atom == 0 {
                // Class may already exist from a previous run in the same process.
                // Continue — CreateWindowEx will still work if registered.
            }

            // Not TOPMOST: only float above the target while it is in the
            // foreground; otherwise we hide so other apps are not covered.
            let hwnd = CreateWindowExW(
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
            .map_err(|e| OverlayError::Other(format!("CreateWindowExW: {e}")))?;

            let hdc_screen = GetDC(Some(hwnd));
            if hdc_screen.is_invalid() {
                let _ = DestroyWindow(hwnd);
                return Err(OverlayError::Other("GetDC failed".into()));
            }
            let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
            if hdc_mem.is_invalid() {
                ReleaseDC(Some(hwnd), hdc_screen);
                let _ = DestroyWindow(hwnd);
                return Err(OverlayError::Other("CreateCompatibleDC failed".into()));
            }

            let font_px = 16;
            let hfont = create_font(font_px)?;

            let hdc_present = CreateCompatibleDC(Some(hdc_screen));
            if hdc_present.is_invalid() {
                let _ = DeleteDC(hdc_mem);
                ReleaseDC(Some(hwnd), hdc_screen);
                let _ = DestroyWindow(hwnd);
                return Err(OverlayError::Other("CreateCompatibleDC(present) failed".into()));
            }

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
                hdc_screen,
                hdc_mem,
                hbmp: HBITMAP::default(),
                bits: std::ptr::null_mut(),
                bmp_w: 0,
                bmp_h: 0,
                hdc_present,
                hbmp_present: HBITMAP::default(),
                present_w: 0,
                present_h: 0,
                hfont,
                font_px,
            };

            let _ = ShowWindow(hwnd, SW_HIDE);
            host.ensure_bitmap(100, 100)?;
            Ok(host)
        }
    }

    pub fn run(&mut self, rx: Receiver<OverlayCommand>) {
        let mut last_tick = Instant::now();
        loop {
            loop {
                match rx.try_recv() {
                    Ok(OverlayCommand::Shutdown) => {
                        self.teardown();
                        return;
                    }
                    Ok(cmd) => self.handle(cmd),
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.teardown();
                        return;
                    }
                }
            }

            unsafe {
                let mut msg = MSG::default();
                while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).as_bool() {
                    if msg.message == WM_QUIT {
                        self.teardown();
                        return;
                    }
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }

            if last_tick.elapsed() >= TICK {
                last_tick = Instant::now();
                self.tick();
            } else {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }

    fn handle(&mut self, cmd: OverlayCommand) {
        match cmd {
            OverlayCommand::Attach { target_hwnd } => {
                let hwnd = HWND(target_hwnd as *mut _);
                if unsafe { IsWindow(Some(hwnd)).as_bool() } {
                    self.target = Some(hwnd);
                    self.dirty = true;
                    debug!(?target_hwnd, "overlay attached");
                } else {
                    warn!(?target_hwnd, "attach ignored — invalid hwnd");
                    self.target = None;
                }
            }
            OverlayCommand::Detach => {
                self.target = None;
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
            }
            OverlayCommand::Clear => {
                self.blocks.clear();
                self.content_w = 0;
                self.content_h = 0;
                self.dirty = true;
                self.hide();
            }
            OverlayCommand::UpdateConfig(cfg) => {
                self.config = cfg;
                self.dirty = true;
            }
            OverlayCommand::Shutdown => {}
        }
    }

    fn tick(&mut self) {
        let Some(target) = self.target else {
            return;
        };

        unsafe {
            if !IsWindow(Some(target)).as_bool() {
                debug!("target window gone — detaching overlay");
                self.target = None;
                self.hide();
                return;
            }
            if IsIconic(target).as_bool() || !IsWindowVisible(target).as_bool() {
                self.hide();
                return;
            }

            // Only show while the capture target (or one of its children) is
            // the foreground window. Use TOPMOST only in that window so the
            // overlay is not stuck under the target (HWND_TOP is not enough
            // for many apps) and is cleared when focus leaves.
            let fg = GetForegroundWindow();
            if !is_target_in_foreground(target, fg) {
                self.hide();
                return;
            }

            // Align to **client area** (matches cropped capture frames).
            let Some((x, y, client_w, client_h)) = client_screen_rect(target) else {
                return;
            };

            if self.blocks.is_empty() || self.content_w == 0 || self.content_h == 0 {
                self.hide();
                return;
            }

            // Paint in capture/OCR pixel space (1:1 with bboxes), then stretch to
            // the live client rect. Avoids DPI / DWM size drift mis-mapping boxes.
            let paint_w = self.content_w as i32;
            let paint_h = self.content_h as i32;
            let size_changed = paint_w != self.surface_w || paint_h != self.surface_h;
            if size_changed {
                self.surface_w = paint_w;
                self.surface_h = paint_h;
                self.dirty = true;
            }

            if self.dirty {
                if let Err(e) = self.repaint() {
                    warn!(error = %e, "overlay repaint failed");
                    return;
                }
                self.dirty = false;
            }

            // TOPMOST only while target is focused — otherwise other apps get covered.
            let _ = SetWindowPos(self.hwnd, Some(HWND_TOPMOST), x, y, client_w, client_h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);

            if let Err(e) = self.present(x, y, client_w, client_h) {
                warn!(error = %e, "UpdateLayeredWindow failed");
            }
        }
    }

    fn hide(&mut self) {
        unsafe {
            // Drop topmost so we never stay above unrelated apps after hide.
            let _ = SetWindowPos(self.hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_HIDEWINDOW);
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    fn ensure_bitmap(&mut self, w: i32, h: i32) -> Result<(), OverlayError> {
        if w <= 0 || h <= 0 {
            return Err(OverlayError::Other("invalid bitmap size".into()));
        }
        if w == self.bmp_w && h == self.bmp_h && !self.bits.is_null() && !self.hbmp.is_invalid() {
            return Ok(());
        }

        unsafe {
            if !self.hbmp.is_invalid() {
                let _ = SelectObject(self.hdc_mem, HGDIOBJ::default());
                let _ = DeleteObject(self.hbmp.into());
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
            let hbmp = CreateDIBSection(Some(self.hdc_mem), &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
                .map_err(|e| OverlayError::Other(format!("CreateDIBSection: {e}")))?;

            if hbmp.is_invalid() || bits.is_null() {
                return Err(OverlayError::Other("CreateDIBSection returned null".into()));
            }

            let _ = SelectObject(self.hdc_mem, HGDIOBJ(hbmp.0));
            self.hbmp = hbmp;
            self.bits = bits.cast();
            self.bmp_w = w;
            self.bmp_h = h;
        }
        Ok(())
    }

    fn repaint(&mut self) -> Result<(), OverlayError> {
        let w = self.surface_w.max(1);
        let h = self.surface_h.max(1);
        self.ensure_bitmap(w, h)?;

        let len = (w as usize) * (h as usize) * 4;
        let buf = unsafe { std::slice::from_raw_parts_mut(self.bits, len) };
        draw::clear(buf);

        let bg = draw::background_rgba(&self.config);
        let fg = draw::text_rgba(&self.config);
        let surface = draw::SurfaceSize::new(w, h);

        // Layout labels first so paint order is stable.
        let content_w = self.content_w;
        let content_h = self.content_h;
        let pending: Vec<(SurfaceRect, String, u32)> = self
            .blocks
            .iter()
            .filter_map(|block| {
                let text = block.translation.trim();
                if text.is_empty() {
                    return None;
                }
                let base = draw::map_rect_to_surface(block.bbox, content_w, content_h, w, h)?;
                Some((base, text.to_string(), block.source_lines.max(1)))
            })
            .collect();

        // (rect, text, font_px) — single-line sources may shrink/widen; merged
        // paragraphs keep the OCR column width.
        let mut labels: Vec<(SurfaceRect, String, i32)> = Vec::with_capacity(pending.len());
        for (base, text, source_lines) in pending {
            let (expanded, font_px) = self.layout_label(base, &text, source_lines, surface)?;
            labels.push((expanded, text, font_px));
        }

        for (rect, _, _) in &labels {
            draw::fill_rect(buf, surface, *rect, bg);
        }
        for (rect, text, font_px) in &labels {
            self.draw_text_label(buf, surface, *rect, text, LabelStyle {
                font_px: *font_px,
                color: fg,
            })?;
        }
        Ok(())
    }

    /// Pick CreateFont height so the GDI cell fits inside the OCR glyph box.
    ///
    /// Fixed ratios still overshoot (Segoe UI cell > requested height; vertical
    /// OCR width is padded). Shrink until ascent+descent ≤ ~90% of short side.
    fn fit_font_to_source_box(&mut self, base: SurfaceRect) -> Result<i32, OverlayError> {
        let target = draw::char_box_px(base);
        // Leave a little air so ClearType stems don't look larger than source ink.
        let max_cell = ((target as f32) * 0.90).round().max(8.0) as i32;
        let mut px = draw::font_height_for(base);

        for _ in 0..6 {
            self.ensure_font(px)?;
            let cell = unsafe {
                let _ = SelectObject(self.hdc_mem, HGDIOBJ(self.hfont.0));
                let mut tm = TEXTMETRICW::default();
                if GetTextMetricsW(self.hdc_mem, &mut tm).as_bool() {
                    (tm.tmAscent + tm.tmDescent).max(1)
                } else {
                    px
                }
            };
            if cell <= max_cell {
                break;
            }
            let next = ((px as f32) * (max_cell as f32) / (cell as f32)).floor().max(8.0) as i32;
            if next >= px {
                px = (px - 1).max(8);
            } else {
                px = next;
            }
        }
        Ok(px)
    }

    /// Layout one overlay label.
    ///
    /// - Merged paragraph (`source_lines > 1`): lock width to OCR column; wrap.
    /// - Single line: shrink font (down to ~70%) to fit source width, then allow
    ///   width growth up to 1.75x if the translation is still longer.
    fn layout_label(
        &mut self,
        base: SurfaceRect,
        text: &str,
        source_lines: u32,
        surface: draw::SurfaceSize,
    ) -> Result<(SurfaceRect, i32), OverlayError> {
        let max_w = (surface.width - base.x).max(1);
        let source_w = base.w.clamp(1, max_w);
        let mut font_px = self.fit_font_to_source_box(base)?;

        if source_lines <= 1 {
            // 1) Shrink font so the translation can stay one line inside source_w.
            let min_font = ((font_px as f32) * 0.70).round().max(8.0) as i32;
            loop {
                let pad = label_pad(font_px);
                let natural = self.measure_single_line(text, font_px)?;
                let need_w = natural.0 + pad * 2;
                if need_w <= source_w || font_px <= min_font {
                    break;
                }
                font_px = (font_px - 1).max(min_font);
            }

            // 2) If still wider than source at min font, grow width (cap 1.75×).
            let pad = label_pad(font_px);
            let natural = self.measure_single_line(text, font_px)?;
            let need_w = (natural.0 + pad * 2).max(1);
            let expand_cap = ((source_w as f32) * 1.75).round() as i32;
            let box_w = if need_w <= source_w {
                source_w
            } else {
                need_w.min(expand_cap).min(max_w).max(source_w)
            };

            // Height: one line if it fits; otherwise wrap within the chosen width.
            let text_h = if need_w <= box_w {
                natural.1
            } else {
                self.measure_wrapped(text, font_px, (box_w - pad * 2).max(8))?.1
            };
            let box_h = (text_h + pad * 2).max(font_px + pad * 2);
            return Ok((place_label(base, box_w, box_h, surface), font_px));
        }

        // Multi-line source: keep OCR column width; wrap height only.
        let pad = label_pad(font_px);
        let box_w = source_w;
        let text_h = self.measure_wrapped(text, font_px, (box_w - pad * 2).max(8))?.1;
        let box_h = (text_h + pad * 2).max(font_px + pad * 2);
        Ok((place_label(base, box_w, box_h, surface), font_px))
    }

    /// Unwrapped single-line extent (width, height) in pixels.
    fn measure_single_line(&mut self, text: &str, font_px: i32) -> Result<(i32, i32), OverlayError> {
        self.ensure_font(font_px)?;
        unsafe {
            let _ = SelectObject(self.hdc_mem, HGDIOBJ(self.hfont.0));
            let mut calc = RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            };
            let mut wide: Vec<u16> = text.encode_utf16().collect();
            let flags = DT_LEFT | DT_TOP | DT_SINGLELINE | DT_NOPREFIX | DT_CALCRECT;
            let measured_h = DrawTextW(self.hdc_mem, &mut wide, &mut calc, flags);
            let w = (calc.right - calc.left).max(1);
            let h = if measured_h > 0 { measured_h } else { font_px + 2 };
            Ok((w, h))
        }
    }

    /// Word-wrapped extent for a fixed text area width.
    fn measure_wrapped(&mut self, text: &str, font_px: i32, text_area_w: i32) -> Result<(i32, i32), OverlayError> {
        self.ensure_font(font_px)?;
        unsafe {
            let _ = SelectObject(self.hdc_mem, HGDIOBJ(self.hfont.0));
            let mut calc = RECT {
                left: 0,
                top: 0,
                right: text_area_w.max(8),
                bottom: 0,
            };
            let mut wide: Vec<u16> = text.encode_utf16().collect();
            let flags = DT_LEFT | DT_TOP | DT_WORDBREAK | DT_EDITCONTROL | DT_NOPREFIX | DT_CALCRECT;
            let measured_h = DrawTextW(self.hdc_mem, &mut wide, &mut calc, flags);
            let w = (calc.right - calc.left).max(1);
            let h = if measured_h > 0 { measured_h } else { font_px + 2 };
            Ok((w, h))
        }
    }

    fn ensure_font(&mut self, font_px: i32) -> Result<(), OverlayError> {
        if font_px == self.font_px && !self.hfont.is_invalid() {
            return Ok(());
        }
        unsafe {
            if !self.hfont.is_invalid() {
                let _ = DeleteObject(self.hfont.into());
            }
            self.hfont = create_font(font_px)?;
            self.font_px = font_px;
        }
        Ok(())
    }

    fn draw_text_label(
        &mut self,
        buf: &mut [u8],
        surface: draw::SurfaceSize,
        rect: SurfaceRect,
        text: &str,
        style: LabelStyle,
    ) -> Result<(), OverlayError> {
        self.ensure_font(style.font_px)?;
        unsafe {
            let _ = SelectObject(self.hdc_mem, HGDIOBJ(self.hfont.0));
            let _ = SetBkMode(self.hdc_mem, TRANSPARENT);

            let pad = label_pad(style.font_px);
            let mut text_rect = RECT {
                left: rect.x + pad,
                top: rect.y + pad,
                right: (rect.x + rect.w - pad).max(rect.x + pad + 1),
                bottom: (rect.y + rect.h - pad).max(rect.y + pad + 1),
            };

            let rw = (text_rect.right - text_rect.left).max(0) as usize;
            let rh = (text_rect.bottom - text_rect.top).max(0) as usize;
            if rw == 0 || rh == 0 {
                return Ok(());
            }

            let mut bg_copy = vec![0u8; rw * rh * 4];
            for row in 0..rh {
                let src_y = text_rect.top as usize + row;
                if src_y >= surface.height as usize {
                    break;
                }
                let src = src_y * surface.stride + text_rect.left as usize * 4;
                let dst = row * rw * 4;
                let count = rw * 4;
                if src + count <= buf.len() {
                    bg_copy[dst..dst + count].copy_from_slice(&buf[src..src + count]);
                    for px in buf[src..src + count].chunks_exact_mut(4) {
                        px.fill(0);
                    }
                }
            }

            let mut wide: Vec<u16> = text.encode_utf16().collect();
            let _ = SetTextColor(self.hdc_mem, COLORREF(0x00FF_FFFF));
            // No DT_END_ELLIPSIS — rect was expanded to fit the full translation.
            let flags = DT_LEFT | DT_TOP | DT_WORDBREAK | DT_EDITCONTROL | DT_NOPREFIX;
            DrawTextW(self.hdc_mem, &mut wide, &mut text_rect, flags);

            let color = style.color;
            for row in 0..rh {
                let y = text_rect.top as usize + row;
                if y >= surface.height as usize {
                    break;
                }
                for col in 0..rw {
                    let x = text_rect.left as usize + col;
                    if x >= surface.width as usize {
                        break;
                    }
                    let idx = y * surface.stride + x * 4;
                    let midx = row * rw * 4 + col * 4;
                    let coverage = buf[idx].max(buf[idx + 1]).max(buf[idx + 2]) as u32;
                    buf[idx] = bg_copy[midx];
                    buf[idx + 1] = bg_copy[midx + 1];
                    buf[idx + 2] = bg_copy[midx + 2];
                    buf[idx + 3] = bg_copy[midx + 3];
                    if coverage > 8 {
                        let fa = ((color.a as u32 * coverage) / 255) as u8;
                        blend_premul(buf, idx, color.r, color.g, color.b, fa);
                    }
                }
            }
        }
        Ok(())
    }

    fn present(&mut self, x: i32, y: i32, client_w: i32, client_h: i32) -> Result<(), OverlayError> {
        if self.bits.is_null() || self.bmp_w <= 0 || self.bmp_h <= 0 {
            return Err(OverlayError::Other("paint bitmap missing".into()));
        }
        if client_w <= 0 || client_h <= 0 {
            return Err(OverlayError::Other("invalid client size".into()));
        }

        unsafe {
            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };
            let ppt_dst = POINT { x, y };
            let psize = SIZE {
                cx: client_w,
                cy: client_h,
            };
            let ppt_src = POINT { x: 0, y: 0 };

            // Same size: present paint buffer directly (common path).
            if self.bmp_w == client_w && self.bmp_h == client_h {
                UpdateLayeredWindow(
                    self.hwnd,
                    Some(self.hdc_screen),
                    Some(&ppt_dst),
                    Some(&psize),
                    Some(self.hdc_mem),
                    Some(&ppt_src),
                    COLORREF(0),
                    Some(&blend),
                    ULW_ALPHA,
                )
                .map_err(|e| OverlayError::Other(format!("UpdateLayeredWindow: {e}")))?;
                return Ok(());
            }

            // Stretch content-space paint into client-sized present buffer.
            self.ensure_present_bitmap(client_w, client_h)?;
            let _ = SetStretchBltMode(self.hdc_present, HALFTONE);
            let ok = StretchBlt(
                self.hdc_present,
                0,
                0,
                client_w,
                client_h,
                Some(self.hdc_mem),
                0,
                0,
                self.bmp_w,
                self.bmp_h,
                windows::Win32::Graphics::Gdi::SRCCOPY,
            );
            if !ok.as_bool() {
                return Err(OverlayError::Other("StretchBlt failed".into()));
            }

            UpdateLayeredWindow(
                self.hwnd,
                Some(self.hdc_screen),
                Some(&ppt_dst),
                Some(&psize),
                Some(self.hdc_present),
                Some(&ppt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            )
            .map_err(|e| OverlayError::Other(format!("UpdateLayeredWindow: {e}")))?;
        }
        Ok(())
    }

    fn ensure_present_bitmap(&mut self, w: i32, h: i32) -> Result<(), OverlayError> {
        if w <= 0 || h <= 0 {
            return Err(OverlayError::Other("invalid present size".into()));
        }
        if w == self.present_w && h == self.present_h && !self.hbmp_present.is_invalid() {
            return Ok(());
        }

        unsafe {
            if !self.hbmp_present.is_invalid() {
                let _ = SelectObject(self.hdc_present, HGDIOBJ::default());
                let _ = DeleteObject(self.hbmp_present.into());
                self.hbmp_present = HBITMAP::default();
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
            let hbmp = CreateDIBSection(Some(self.hdc_present), &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
                .map_err(|e| OverlayError::Other(format!("CreateDIBSection(present): {e}")))?;

            if hbmp.is_invalid() || bits.is_null() {
                return Err(OverlayError::Other("CreateDIBSection(present) returned null".into()));
            }

            let _ = SelectObject(self.hdc_present, HGDIOBJ(hbmp.0));
            self.hbmp_present = hbmp;
            self.present_w = w;
            self.present_h = h;
        }
        Ok(())
    }

    fn teardown(&mut self) {
        unsafe {
            if !self.hwnd.is_invalid() {
                let _ = DestroyWindow(self.hwnd);
                self.hwnd = HWND::default();
            }
            if !self.hfont.is_invalid() {
                let _ = DeleteObject(self.hfont.into());
                self.hfont = HFONT::default();
            }
            if !self.hbmp.is_invalid() {
                let _ = DeleteObject(self.hbmp.into());
                self.hbmp = HBITMAP::default();
                self.bits = std::ptr::null_mut();
            }
            if !self.hbmp_present.is_invalid() {
                let _ = DeleteObject(self.hbmp_present.into());
                self.hbmp_present = HBITMAP::default();
            }
            if !self.hdc_mem.is_invalid() {
                let _ = DeleteDC(self.hdc_mem);
                self.hdc_mem = HDC::default();
            }
            if !self.hdc_present.is_invalid() {
                let _ = DeleteDC(self.hdc_present);
                self.hdc_present = HDC::default();
            }
            if !self.hdc_screen.is_invalid() {
                ReleaseDC(None, self.hdc_screen);
                self.hdc_screen = HDC::default();
            }
            if self.class_atom != 0 {
                if let Ok(hi) = GetModuleHandleW(None) {
                    let _ = UnregisterClassW(CLASS_NAME, Some(hi.into()));
                }
                self.class_atom = 0;
            }
        }
    }
}

impl Drop for OverlayHost {
    fn drop(&mut self) {
        self.teardown();
    }
}

/// True when `fg` is the capture target or a child / same top-level tree.
fn is_target_in_foreground(target: HWND, fg: HWND) -> bool {
    unsafe {
        if fg.is_invalid() || target.is_invalid() {
            return false;
        }
        if fg == target {
            return true;
        }
        // Focused child control inside the target window.
        if IsChild(target, fg).as_bool() {
            return true;
        }
        // Same top-level root (e.g. owned popups under the target).
        let fg_root = GetAncestor(fg, GA_ROOT);
        if !fg_root.is_invalid() && fg_root == target {
            return true;
        }
        false
    }
}

/// Client-area rectangle in screen coordinates (left, top, width, height).
fn client_screen_rect(target: HWND) -> Option<(i32, i32, i32, i32)> {
    unsafe {
        let mut client = RECT::default();
        if GetClientRect(target, &mut client).is_err() {
            return None;
        }
        let mut tl = POINT {
            x: client.left,
            y: client.top,
        };
        let mut br = POINT {
            x: client.right,
            y: client.bottom,
        };
        if !ClientToScreen(target, &mut tl).as_bool() || !ClientToScreen(target, &mut br).as_bool() {
            return None;
        }
        let w = (br.x - tl.x).max(1);
        let h = (br.y - tl.y).max(1);
        Some((tl.x, tl.y, w, h))
    }
}

fn create_font(px: i32) -> Result<HFONT, OverlayError> {
    unsafe {
        let pitch = (DEFAULT_PITCH.0 as u32) | (FF_DONTCARE.0 as u32);
        // Regular weight matches typical game/UI source text better than semibold
        // (which looks larger/heavier than the OCR ink).
        let font = CreateFontW(
            -px,
            0,
            0,
            0,
            FW_NORMAL.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            pitch,
            w!("Segoe UI"),
        );
        if font.is_invalid() {
            Err(OverlayError::Other("CreateFontW failed".into()))
        } else {
            Ok(font)
        }
    }
}

fn blend_premul(buf: &mut [u8], idx: usize, r: u8, g: u8, b: u8, a: u8) {
    if a == 0 {
        return;
    }
    if a == 255 {
        buf[idx] = b;
        buf[idx + 1] = g;
        buf[idx + 2] = r;
        buf[idx + 3] = 255;
        return;
    }
    let src_a = a as u32;
    let inv = 255 - src_a;
    let dst_b = buf[idx] as u32;
    let dst_g = buf[idx + 1] as u32;
    let dst_r = buf[idx + 2] as u32;
    let dst_a = buf[idx + 3] as u32;
    let sb = (b as u32 * src_a) / 255;
    let sg = (g as u32 * src_a) / 255;
    let sr = (r as u32 * src_a) / 255;
    buf[idx] = (sb + (dst_b * inv) / 255).min(255) as u8;
    buf[idx + 1] = (sg + (dst_g * inv) / 255).min(255) as u8;
    buf[idx + 2] = (sr + (dst_r * inv) / 255).min(255) as u8;
    buf[idx + 3] = (src_a + (dst_a * inv) / 255).min(255) as u8;
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match msg {
            WM_NCHITTEST => LRESULT(HTTRANSPARENT as isize),
            WM_DESTROY => {
                PostQuitMessage(0);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
