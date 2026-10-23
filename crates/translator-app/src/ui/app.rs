//! Control window: title bar, navigation, page routing, and the UI-thread sync loop.
//!
//! Event handlers mutate [`UiShared`] and call [`refresh`]; the next sync copies state into
//! the controls. Change events that fire while a sync writes values are ignored ([`quiet`]).

use std::{
    cell::{Cell, RefCell},
    env,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, AtomicIsize, Ordering},
    },
    time::Duration,
};

use parking_lot::Mutex;
use rust_i18n::t;
use tokio_util::sync::CancellationToken;
use tracing::warn;
use translator_translate::list_models;
use translator_winui::{
    AppWindowTitleBar, Application, Border, Color, ElementTheme, Grid, HSTRING, HorizontalAlignment, IInspectable, Interface, Layout,
    MicaBackdrop, MicaKind, NavigationView, NavigationViewBackButtonVisible, NavigationViewItem, NavigationViewItemHeader,
    NavigationViewPaneDisplayMode, Ref, Result, ScrollViewer, SizeInt32, Symbol, SymbolIcon, TitleBar, TitleBarHeightOption, TitleBarTheme,
    UIElement, Window, XamlRoot, auto, boxed, boxed_f64, grid, keep, post, set_ui_thread, solid_brush, star, thickness, unboxed,
    window_hwnd,
};
use windows::Win32::{Foundation::HWND, UI::HiDpi::GetDpiForWindow};

use crate::{
    children,
    pipeline::install_ui_ping,
    taskbar_guard::restore_taskbar_zorder,
    ui::{
        chrome::{Footer, app_status_strip, confirm_dialog, settings_sticky_chrome, ui_language_combo},
        pages::{Dashboard, api_page, dashboard_page, hotkeys_page, ocr_page, overlay_page, translation_page},
        state::{Page, UiCx, UiShared, apply_hotkey_capture, make_shared, schedule_model_list_if_needed, stop_hotkey_edit, take_chrome},
        widgets::{Binds, Prop, text},
    },
};

thread_local! {
    static SYNCING: Cell<bool> = const { Cell::new(false) };
    static ROOT: RefCell<Option<Root>> = const { RefCell::new(None) };
    static XAML_ROOT: RefCell<Option<XamlRoot>> = const { RefCell::new(None) };
    static SCALE: Cell<f32> = const { Cell::new(1.0) };
}

const TITLE: &str = "Translator Overlay";

static PENDING: AtomicBool = AtomicBool::new(false);
static SHARED: OnceLock<Arc<Mutex<UiShared>>> = OnceLock::new();
static MODEL_FETCH: Mutex<Option<CancellationToken>> = Mutex::new(None);
static HWND_VALUE: AtomicIsize = AtomicIsize::new(0);

/// True while the UI is writing state into controls; change handlers return early.
pub fn quiet() -> bool {
    SYNCING.get()
}

/// Runs `f` as a sync: change events it triggers are not user edits.
pub fn quietly<R>(f: impl FnOnce() -> R) -> R {
    SYNCING.set(true);
    let result = f();
    SYNCING.set(false);
    result
}

/// Schedules one sync on the UI thread. Callable from any thread; repeated calls coalesce.
pub fn refresh() {
    if !PENDING.swap(true, Ordering::AcqRel) && !post(sync_root) {
        PENDING.store(false, Ordering::Release);
    }
}

/// XamlRoot of the control window, once its content has loaded.
pub fn xaml_root() -> Option<XamlRoot> {
    XAML_ROOT.with(|r| r.borrow().clone())
}

/// Physical pixels per DIP of the control window.
pub fn raster_scale() -> f32 {
    SCALE.get()
}

/// Control-window HWND, or `None` before the window exists.
pub fn control_hwnd() -> Option<isize> {
    Some(HWND_VALUE.load(Ordering::Acquire)).filter(|&h| h != 0)
}

/// Loads model ids for the API draft. A newer request cancels this one.
pub fn fetch_model_list(generation: u64, debounce: bool) {
    let token = CancellationToken::new();
    if let Some(old) = MODEL_FETCH.lock().replace(token.clone()) {
        old.cancel();
    }
    let Ok(handle) = tokio::runtime::Handle::try_current() else {
        return;
    };
    handle.spawn(async move {
        if debounce {
            tokio::select! {
                () = tokio::time::sleep(Duration::from_millis(400)) => {}
                () = token.cancelled() => return,
            }
        }
        let Some(shared) = SHARED.get() else {
            return;
        };
        let api = {
            let ui = shared.lock();
            if ui.model_list_gen != generation {
                return;
            }
            ui.draft.api.clone()
        };
        let ids = match list_models(&api, &token).await {
            Ok(ids) => ids,
            Err(e) if e.is_cancelled() => return,
            Err(e) => {
                warn!(error = %e, "model list fetch failed");
                Vec::new()
            }
        };
        post(move || {
            if let Some(shared) = SHARED.get() {
                let mut ui = shared.lock();
                if generation == ui.model_list_gen {
                    ui.model_catalog = ids;
                    ui.model_list_loading = false;
                }
            }
            refresh();
        });
    });
}

fn sync_root() {
    PENDING.store(false, Ordering::Release);
    ROOT.with(|root| {
        // A sync already in progress (re-entered from a nested message pump) picks this up next time.
        let Ok(mut root) = root.try_borrow_mut() else {
            refresh();
            return;
        };
        let Some(root) = root.as_mut() else {
            return;
        };
        if let Err(e) = quietly(|| root.sync()) {
            warn!(error = %e, "control window sync failed");
        }
    });
}

/// Runs the control window until it closes.
pub fn run() -> Result<()> {
    translator_winui::run(|| {
        set_ui_thread()?;
        let root = quietly(Root::new)?;
        root.window.Activate()?;
        restore_taskbar_zorder();
        ROOT.with(|slot| *slot.borrow_mut() = Some(root));
        install_ui_ping(refresh);
        sync_root();
        post(restore_taskbar_zorder);
        Ok(())
    })
}

enum PageView {
    Dashboard(Box<Dashboard>),
    Settings(Binds<UiShared>),
}

const NAV: [(Page, &str, &str, Symbol); 6] = [
    (Page::Dashboard, "dashboard", "nav.dashboard", Symbol::Home),
    (Page::Api, "api", "nav.api", Symbol::Link),
    (Page::Translation, "translation", "nav.translation", Symbol::Globe),
    (Page::Ocr, "ocr", "nav.ocr", Symbol::Camera),
    (Page::Overlay, "overlay", "nav.overlay", Symbol::ViewAll),
    (Page::Hotkeys, "hotkeys", "nav.hotkeys", Symbol::Keyboard),
];

struct Root {
    cx: UiCx,
    window: Window,
    nav: NavigationView,
    items: Vec<(Page, NavigationViewItem)>,
    header: NavigationViewItemHeader,
    footer: Footer,
    binds: Binds<UiShared>,
    page: Option<(Page, PageView)>,
    locale: String,
    selected: Prop<Page>,
    pane: Prop<bool>,
}

impl Root {
    fn new() -> Result<Self> {
        let shared = make_shared();
        let _ = SHARED.set(shared.clone());
        let cx = UiCx::new(&shared);
        restore_taskbar_zorder();

        let window = Window::new()?;
        let host = Grid::new()?;
        window.SetContent(&host)?;
        let hwnd = window_hwnd(&window)?;
        HWND_VALUE.store(hwnd, Ordering::Release);

        let mica = MicaBackdrop::new()?;
        mica.SetKind(MicaKind::Base)?;
        window.SetSystemBackdrop(&mica)?;
        let app_window = window.AppWindow()?;
        let title_bar_native: AppWindowTitleBar = app_window.TitleBar()?;
        title_bar_native.SetPreferredTheme(TitleBarTheme::UseDefaultAppMode)?;
        host.SetRequestedTheme(ElementTheme::Default)?;
        if let Some(icon) = window_icon_path() {
            app_window.SetIcon(&HSTRING::from(icon.as_str()))?;
        }
        // SAFETY: `hwnd` is the live control window.
        let dpi = unsafe { GetDpiForWindow(HWND(hwnd as *mut _)) }.max(96);
        let px = |dips: f64| (dips * f64::from(dpi) / 96.0).round() as i32;
        app_window.ResizeClient(SizeInt32 {
            Width: px(1280.0),
            Height: px(800.0),
        })?;
        apply_resource_overrides()?;

        let mut binds = Binds::default();
        let title_bar = TitleBar::new()?;
        title_bar.SetTitle(&HSTRING::from(TITLE))?;
        title_bar.SetIsPaneToggleButtonVisible(true)?;
        title_bar.SetIsBackButtonVisible(false)?;
        title_bar.halign(HorizontalAlignment::Stretch)?.row(0)?.column(0)?;
        title_bar.SetContent(&app_status_strip(&mut binds)?)?;
        title_bar.SetRightHeader(&ui_language_combo(&mut binds, &cx)?)?;
        keep(title_bar.PaneToggleRequested({
            let cx = cx.clone();
            move |_, _| {
                if !quiet() {
                    cx.with_mut(|ui| ui.pane_open = !ui.pane_open);
                }
            }
        }))?;

        let nav = NavigationView::new()?;
        nav.SetPaneDisplayMode(NavigationViewPaneDisplayMode::Left)?;
        nav.SetOpenPaneLength(196.0)?;
        nav.SetIsSettingsVisible(false)?;
        nav.SetIsBackButtonVisible(NavigationViewBackButtonVisible::Collapsed)?;
        nav.SetIsPaneToggleButtonVisible(false)?;
        nav.fill()?.row(1)?.column(0)?;
        let header = NavigationViewItemHeader::new()?;
        let menu = nav.MenuItems()?;
        let mut items = Vec::new();
        for (i, &(page, tag, _, symbol)) in NAV.iter().enumerate() {
            let item = NavigationViewItem::new()?;
            item.SetTag(&boxed(tag))?;
            let icon = SymbolIcon::new()?;
            icon.SetSymbol(symbol)?;
            item.SetIcon(&icon)?;
            menu.Append(&item)?;
            // "Settings" groups every page after the dashboard.
            if i == 0 {
                menu.Append(&header)?;
            }
            items.push((page, item));
        }
        keep(nav.SelectionChanged({
            let cx = cx.clone();
            move |_, args| {
                let tag = args.as_ref().and_then(|a| a.SelectedItem().ok()).and_then(|item| {
                    let item: NavigationViewItem = item.cast().ok()?;
                    unboxed(&item.Tag().ok()?)
                });
                let Some(&(page, ..)) = tag.and_then(|tag| NAV.iter().find(|(_, t, ..)| *t == tag)) else {
                    return;
                };
                if quiet() {
                    return;
                }
                cx.with_mut(|ui| {
                    if ui.page != page {
                        if ui.page == Page::Hotkeys {
                            stop_hotkey_edit(ui);
                        }
                        ui.page = page;
                    }
                });
            }
        }))?;
        for open in [true, false] {
            let cx = cx.clone();
            let handler = move |_: Ref<NavigationView>, _: Ref<IInspectable>| {
                if !quiet() {
                    cx.with_mut(|ui| ui.pane_open = open);
                }
            };
            keep(if open { nav.PaneOpened(handler) } else { nav.PaneClosed(handler) })?;
        }
        let footer = Footer::new()?;
        nav.SetPaneFooter(&footer.root)?;

        let root = grid(&[auto(), star(1.0)], &[star(1.0)])?;
        root.fill()?;
        root.SetBackground(&solid_brush(Color::default())?)?;
        children!(root; title_bar, nav);
        children!(host; root);

        window.SetExtendsContentIntoTitleBar(true)?;
        window.SetTitleBar(&title_bar)?;
        title_bar_native.SetPreferredHeightOption(TitleBarHeightOption::Tall)?;
        window.SetTitle(&HSTRING::from(TITLE))?;
        keep(window.Closed(|_, _| {
            if let Ok(app) = Application::Current() {
                let _ = app.Exit();
            }
        }))?;
        keep(host.Loaded({
            let host = host.clone();
            move |_, _| {
                let Ok(root) = host.XamlRoot() else {
                    return;
                };
                SCALE.set(root.RasterizationScale().unwrap_or(1.0) as f32);
                let _ = keep(root.Changed(|sender, _| {
                    if let Some(root) = sender.as_ref() {
                        let scale = root.RasterizationScale().unwrap_or(1.0) as f32;
                        if (scale - SCALE.get()).abs() > f32::EPSILON {
                            SCALE.set(scale);
                            refresh();
                        }
                    }
                }));
                XAML_ROOT.with(|slot| *slot.borrow_mut() = Some(root));
                refresh();
            }
        }))?;

        let mut root = Self {
            cx,
            window,
            nav,
            items,
            header,
            footer,
            binds,
            page: None,
            locale: String::new(),
            selected: Prop::default(),
            pane: Prop::default(),
        };
        root.relabel()?;
        Ok(root)
    }

    /// Texts that only change with the UI language.
    fn relabel(&mut self) -> Result<()> {
        for ((_, item), (.., key, _)) in self.items.iter().zip(NAV) {
            item.SetContent(&text(&t!(key))?)?;
        }
        self.header.SetContent(&boxed(&t!("nav.settings")))?;
        self.locale = rust_i18n::locale().to_string();
        Ok(())
    }

    fn sync(&mut self) -> Result<()> {
        if *rust_i18n::locale() != *self.locale {
            self.relabel()?;
            self.page = None;
        }
        let mut ui = self.cx.shared.lock();
        apply_hotkey_capture(&mut ui);
        if ui.page == Page::Api {
            schedule_model_list_if_needed(&mut ui);
        }
        let chrome = take_chrome(&ui);
        self.binds.run(&ui, &chrome)?;
        let nav = &self.nav;
        self.pane.set(ui.pane_open, |&open| nav.SetIsPaneOpen(open))?;
        self.footer.sync(&self.cx, &chrome, ui.pane_open)?;

        let page = ui.page;
        if self.page.as_ref().map(|(p, _)| *p) != Some(page) {
            let (content, view) = build_page(&self.cx, page)?;
            self.nav.SetContent(&content)?;
            self.page = Some((page, view));
        }
        if let Some((_, item)) = self.items.iter().find(|(p, _)| *p == page) {
            self.selected.set(page, |_| nav.SetSelectedItem(item))?;
        }
        match self.page.as_mut() {
            Some((_, PageView::Dashboard(d))) => d.sync(&ui, &chrome),
            Some((_, PageView::Settings(binds))) => binds.run(&ui, &chrome),
            None => Ok(()),
        }
    }
}

fn build_page(cx: &UiCx, page: Page) -> Result<(UIElement, PageView)> {
    if page == Page::Dashboard {
        let dashboard = dashboard_page(cx)?;
        let frame = Border::new()?;
        frame.SetPadding(thickness(24.0, 16.0, 24.0, 24.0))?;
        frame.fill()?;
        frame.SetChild(&dashboard.root)?;
        return Ok((frame.cast()?, PageView::Dashboard(Box::new(dashboard))));
    }
    let mut binds = Binds::default();
    let (body, title) = match page {
        Page::Api => (api_page(&mut binds, cx)?, t!("nav.api")),
        Page::Translation => (translation_page(&mut binds, cx)?, t!("nav.translation")),
        Page::Ocr => (ocr_page(&mut binds, cx)?, t!("nav.ocr")),
        Page::Overlay => (overlay_page(&mut binds, cx)?, t!("nav.overlay")),
        Page::Hotkeys | Page::Dashboard => (hotkeys_page(&mut binds, cx)?, t!("nav.hotkeys")),
    };
    confirm_dialog(&mut binds, cx)?;
    let sticky = Border::new()?;
    sticky.row(0)?.column(0)?.halign(HorizontalAlignment::Stretch)?;
    sticky.SetChild(&settings_sticky_chrome(&mut binds, cx, &title)?)?;
    let padded = Border::new()?;
    padded.SetPadding(thickness(24.0, 8.0, 24.0, 24.0))?;
    padded.halign(HorizontalAlignment::Stretch)?;
    padded.SetChild(&body)?;
    let scroll = ScrollViewer::new()?;
    scroll.fill()?.row(1)?.column(0)?;
    scroll.SetContent(&padded)?;
    let frame = grid(&[auto(), star(1.0)], &[star(1.0)])?;
    frame.fill()?;
    children!(frame; sticky, scroll);
    Ok((frame.cast()?, PageView::Settings(binds)))
}

/// Application-wide resource overrides: transparent NavigationView content (Mica shows between
/// cards), a narrow title-bar drag strip, and a wider ContentDialog for the glossary editor.
fn apply_resource_overrides() -> Result<()> {
    let resources = Application::Current()?.Resources()?;
    let clear = solid_brush(Color::default())?;
    for key in ["NavigationViewContentBackground", "NavigationViewContentGridBorderBrush"] {
        resources.Insert(&boxed(key), &clear)?;
    }
    // WinUI default is 48; keep a sliver so the caption buttons are not flush against the combo.
    resources.Insert(&boxed("TitleBarMinDragRegionWidth"), &boxed_f64(8.0))?;
    // Default 548 is too narrow for the two-column glossary editor.
    resources.Insert(&boxed("ContentDialogMaxWidth"), &boxed_f64(720.0))?;
    Ok(())
}

fn window_icon_path() -> Option<String> {
    const BYTES: &[u8] = include_bytes!("../../assets/icon.ico");
    if let Some(path) = env::current_exe().ok().and_then(|exe| exe.parent().map(|dir| dir.join("icon.ico")))
        && path.is_file()
    {
        return Some(path.to_string_lossy().into_owned());
    }
    let dest = env::temp_dir().join("translator-overlay-icon.ico");
    std::fs::write(&dest, BYTES).ok()?;
    Some(dest.to_string_lossy().into_owned())
}
