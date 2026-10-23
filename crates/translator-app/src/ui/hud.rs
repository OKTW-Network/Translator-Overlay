//! Always-on-top control bar: Start / Pause, Stop, and the pipeline status.

use std::sync::atomic::{AtomicIsize, Ordering};

use rust_i18n::t;
use translator_winui::{
    Button, Color, DesktopAcrylicController, DesktopAcrylicKind, ElementTheme, Grid, ICompositionSupportsSystemBackdrop,
    InputNonClientPointerSource, Interface, Layout, NonClientRegionKind, OverlappedPresenter, RectInt32, Result,
    SystemBackdropConfiguration, TextBlock, TextTrimming, TextWrapping, UIElement, VerticalAlignment, Viewbox, Window, auto, grid, keep,
    solid_brush, star, window_hwnd,
};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Dwm::{DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DwmSetWindowAttribute},
    System::Threading::GetCurrentThreadId,
    UI::{
        HiDpi::GetDpiForWindow,
        WindowsAndMessaging::{
            CallNextHookEx, GWL_EXSTYLE, GWL_STYLE, GetSystemMetrics, GetWindowLongPtrW, HCBT_ACTIVATE, HHOOK, HWND_TOPMOST, SM_CXSCREEN,
            SW_SHOWNOACTIVATE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SetWindowLongPtrW, SetWindowPos,
            SetWindowsHookExW, ShowWindow, UnhookWindowsHookEx, WH_CBT, WS_DLGFRAME, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
        },
    },
};

use crate::{
    children,
    ui::{
        app::refresh,
        chrome::{start_pause, status_text, stop_capture},
        state::{UiCx, UiShared},
        widgets::{Binds, Prop, font_icon, hs, icon_box, icon_button, text},
    },
};

// Fluent sizing in DIPs: 40px targets 8px apart, 12px before the status text.
const PAD: f64 = 8.0;
const BTN: f64 = 40.0;
const BAR_W: f64 = PAD + BTN + 8.0 + BTN + 12.0 + 200.0 + PAD;
const BAR_H: f64 = PAD * 2.0 + BTN;

/// The live control bar, for [`block_activate`].
static HUD_HWND: AtomicIsize = AtomicIsize::new(0);

/// Shows the control bar while the live config enables it.
pub fn hud_window(b: &mut Binds<UiShared>, cx: &UiCx) -> Result<()> {
    let cx = cx.clone();
    let mut hud: Option<Hud> = None;
    b.add(move |ui, snap| {
        let (enabled, text_argb, bg_argb) = {
            let s = ui.state.read();
            let o = &s.config.overlay;
            (o.hud_enabled, o.hud_colors.text, o.hud_colors.background)
        };
        if !enabled {
            hud = None;
            return Ok(());
        }
        let hud = match &mut hud {
            Some(hud) => hud,
            None => hud.insert(Hud::new(&cx)?),
        };
        let (hwnd, primary, stop, status) = (hud.hwnd, &hud.primary, &hud.stop, &hud.status);
        let (root, backdrop, drag) = (&hud.root, &hud.backdrop, &hud.drag);
        // SAFETY: `hwnd` is the live control bar.
        let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
        hud.dpi.set(dpi, |&dpi| fit(hwnd, drag, dpi))?;
        // WinUI can bury other topmost windows when the control window launches.
        // SAFETY: `hwnd` is the live control bar.
        if unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } & WS_EX_TOPMOST.0 as isize == 0 {
            // SAFETY: `hwnd` is the live control bar.
            let _ = unsafe { SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE) };
        }
        hud.colors.set((text_argb, bg_argb), |&(text, bg)| {
            let [a, r, g, b] = text.to_be_bytes();
            status.SetForeground(&solid_brush(Color { A: a, R: r, G: g, B: b })?)?;
            // Light text gets the dark theme, so hover and disabled states contrast the way the text does.
            let light = 299 * u32::from(r) + 587 * u32::from(g) + 114 * u32::from(b) >= 128_000;
            root.SetRequestedTheme(if light { ElementTheme::Dark } else { ElementTheme::Light })?;
            let [a, r, g, b] = bg.to_be_bytes();
            let solid = Color { A: 255, R: r, G: g, B: b };
            // Background alpha 0 leaves only the blur.
            let opacity = f32::from(a) / 255.0;
            backdrop.SetTintColor(solid)?;
            backdrop.SetTintOpacity(opacity)?;
            backdrop.SetLuminosityOpacity(opacity)?;
            backdrop.SetFallbackColor(solid)
        })?;
        let running = snap.auto_running && !snap.capture_paused;
        let primary_enabled = !snap.capture_busy && (snap.auto_running || snap.selected_hwnd.is_some());
        let stop_enabled = !snap.capture_busy && snap.auto_running;
        let primary_name = if running { t!("action.pause") } else { t!("action.start") };
        hud.shown.set(
            (
                status_text(&snap.status).into_owned(),
                running,
                primary_enabled,
                stop_enabled,
                primary_name.into_owned(),
                t!("action.stop").into_owned(),
                text_argb,
            ),
            |(label, running, primary_enabled, stop_enabled, primary_name, stop_name, text)| {
                // The text colour goes on the glyph itself, where the button's visual states cannot replace it,
                // at a third of its alpha while disabled. The Viewbox keeps the glyph whole in the fixed-size button.
                let [a, r, g, b] = text.to_be_bytes();
                let icon = |glyph: &str, enabled: bool| -> Result<Viewbox> {
                    let icon = font_icon(glyph)?;
                    icon.SetForeground(&solid_brush(Color {
                        A: if enabled { a } else { a / 3 },
                        R: r,
                        G: g,
                        B: b,
                    })?)?;
                    icon_box(&icon)
                };
                status.SetText(&hs(label))?;
                primary.SetContent(&icon(if *running { "\u{E769}" } else { "\u{F5B0}" }, *primary_enabled)?)?;
                primary.SetIsEnabled(*primary_enabled)?;
                primary.automation_name(primary_name)?.tooltip(primary_name)?;
                stop.SetContent(&icon("\u{EE95}", *stop_enabled)?)?;
                stop.SetIsEnabled(*stop_enabled)?;
                stop.automation_name(stop_name)?.tooltip(stop_name)?;
                Ok(())
            },
        )
    });
    Ok(())
}

/// DIPs to pixels at `dpi`.
fn px(dips: f64, dpi: u32) -> i32 {
    (dips * f64::from(dpi) / 96.0).round() as i32
}

/// Sizes the bar and its drag region for `dpi`; the content rescales when the bar moves to another monitor.
fn fit(hwnd: HWND, drag: &InputNonClientPointerSource, dpi: u32) -> Result<()> {
    // Frameless, so the window size is the client size.
    let (w, h) = (px(BAR_W, dpi), px(BAR_H, dpi));
    // SAFETY: `hwnd` is the live control bar.
    let _ = unsafe { SetWindowPos(hwnd, None, 0, 0, w, h, SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED) };
    // Everything right of the buttons drags the bar. Not `SetTitleBar`: extending content into the
    // title bar keeps a 1px strip along the top that the content never covers.
    let buttons = px(PAD + BTN + 8.0 + BTN, dpi);
    drag.SetRegionRects(NonClientRegionKind::Caption, &[RectInt32 {
        X: buttons,
        Y: 0,
        Width: w - buttons,
        Height: h,
    }])
}

/// UI-thread CBT hook that refuses to activate the control bar, so clicking it leaves the game focused.
/// WinUI activates its window on every click, ignoring `WS_EX_NOACTIVATE` and `WM_MOUSEACTIVATE`.
unsafe extern "system" fn block_activate(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HCBT_ACTIVATE as i32 && wparam.0 as isize == HUD_HWND.load(Ordering::Relaxed) {
        return LRESULT(1);
    }
    // SAFETY: forwards the hook's own arguments to the next hook.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

/// A closed WinUI window cannot be shown again, so each show builds a new one.
struct Hud {
    window: Window,
    hwnd: HWND,
    hook: Option<HHOOK>,
    backdrop: DesktopAcrylicController,
    root: Grid,
    primary: Button,
    stop: Button,
    status: TextBlock,
    drag: InputNonClientPointerSource,
    /// DPI that [`fit`] last sized the bar for.
    dpi: Prop<u32>,
    /// Control-bar text and background ARGB.
    colors: Prop<(u32, u32)>,
    shown: Prop<(String, bool, bool, bool, String, String, u32)>,
}

impl Drop for Hud {
    fn drop(&mut self) {
        if let Some(hook) = self.hook {
            // SAFETY: `hook` came from `SetWindowsHookExW` and is unhooked once.
            let _ = unsafe { UnhookWindowsHookEx(hook) };
        }
        HUD_HWND.store(0, Ordering::Relaxed);
        let _ = self.backdrop.Close();
        let _ = self.window.Close();
    }
}

impl Hud {
    fn new(cx: &UiCx) -> Result<Self> {
        let primary = icon_button("\u{F5B0}", {
            let cx = cx.clone();
            move || start_pause(&cx)
        })?;
        primary.width(BTN)?.height(BTN)?.column(0)?.automation_id("hud-start-pause")?;
        let stop = icon_button("\u{EE95}", {
            let cx = cx.clone();
            move || stop_capture(&cx)
        })?;
        stop.width(BTN)?
            .height(BTN)?
            .column(1)?
            .margin(8.0, 0.0, 0.0, 0.0)?
            .automation_id("hud-stop")?;
        let status = text("")?;
        status.SetTextWrapping(TextWrapping::NoWrap)?;
        status.SetTextTrimming(TextTrimming::CharacterEllipsis)?;
        status.SetMaxLines(1)?;
        status.valign(VerticalAlignment::Center)?.column(2)?.margin(12.0, 0.0, 0.0, 0.0)?;
        let root = grid(&[star(1.0)], &[auto(), auto(), star(1.0)])?;
        root.margin(PAD, PAD, PAD, PAD)?;
        children!(root; primary, stop, status);
        // A move to a monitor with another scale changes the XamlRoot; the next sync refits the bar.
        keep(root.Loaded(|sender, _| {
            if let Some(xaml) = sender
                .as_ref()
                .and_then(|s| s.cast::<UIElement>().ok())
                .and_then(|e| e.XamlRoot().ok())
            {
                let _ = keep(xaml.Changed(|_, _| refresh()));
            }
        }))?;

        let window = Window::new()?;
        window.SetContent(&root)?;
        window.SetTitle(&hs(&t!("overlay.hud")))?;
        let app_window = window.AppWindow()?;
        let presenter: OverlappedPresenter = app_window.Presenter()?.cast()?;
        // A frame draws a 1px line along the top.
        presenter.SetBorderAndTitleBar(false, false)?;
        presenter.SetIsResizable(false)?;
        presenter.SetIsMaximizable(false)?;
        presenter.SetIsMinimizable(false)?;

        let backdrop = DesktopAcrylicController::new()?;
        backdrop.SetKind(DesktopAcrylicKind::Thin)?;
        let config = SystemBackdropConfiguration::new()?;
        // The bar never activates, and an inactive window's acrylic falls back to a solid colour.
        config.SetIsInputActive(true)?;
        backdrop.SetSystemBackdropConfiguration(&config)?;
        backdrop.AddSystemBackdropTarget(&window.cast::<ICompositionSupportsSystemBackdrop>()?)?;

        let hwnd = HWND(window_hwnd(&window)? as *mut _);
        // No taskbar button or Alt+Tab entry; `WS_EX_NOACTIVATE` also keeps the system move loop from activating it.
        // SAFETY: `hwnd` is the live control bar.
        let ex = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } | (WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0) as isize;
        // SAFETY: `hwnd` is the live control bar.
        unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex) };
        // The presenter leaves a 3px dialog frame on a borderless window; the size below applies the change.
        // SAFETY: `hwnd` is the live control bar.
        let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) } & !(WS_DLGFRAME.0 as isize);
        // SAFETY: `hwnd` is the live control bar.
        unsafe { SetWindowLongPtrW(hwnd, GWL_STYLE, style) };
        HUD_HWND.store(hwnd.0 as isize, Ordering::Relaxed);
        // SAFETY: a thread-local hook on this (UI) thread; `block_activate` lives for the whole program.
        let hook = unsafe { SetWindowsHookExW(WH_CBT, Some(block_activate), None, GetCurrentThreadId()) }.ok();
        // Square corners: DWM's rounded corners always come with a drop shadow.
        let square = DWMWCP_DONOTROUND;
        // SAFETY: `hwnd` is the live control bar; `square` is the value type this attribute takes and outlives the call.
        let _ =
            unsafe { DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, (&raw const square).cast(), size_of_val(&square) as u32) };
        let drag = InputNonClientPointerSource::GetForWindowId(app_window.Id()?)?;
        // SAFETY: `hwnd` is the live control bar.
        let dpi = unsafe { GetDpiForWindow(hwnd) }.max(96);
        let mut fitted = Prop::default();
        fitted.set(dpi, |&dpi| fit(hwnd, &drag, dpi))?;
        // SAFETY: reads a system metric; no pointers involved.
        let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let (x, y) = ((screen_w - px(BAR_W, dpi)).max(0) / 2, px(12.0, dpi));
        // SAFETY: `hwnd` is the live control bar.
        let _ = unsafe { SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, 0, 0, SWP_NOSIZE | SWP_NOACTIVATE) };
        // Not `Activate`, which would take focus from the game.
        // SAFETY: `hwnd` is the live control bar.
        let _ = unsafe { ShowWindow(hwnd, SW_SHOWNOACTIVATE) };

        Ok(Self {
            window,
            hwnd,
            hook,
            backdrop,
            root,
            primary,
            stop,
            status,
            drag,
            dpi: fitted,
            colors: Prop::default(),
            shown: Prop::default(),
        })
    }
}
