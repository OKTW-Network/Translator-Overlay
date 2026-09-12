//! windows-reactor control UI — dashboard + settings (card layout).

mod chrome;
mod controls;
mod dashboard;
mod mica;
mod nav_header;
mod preview;
mod settings;
mod shared;

use std::{
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, RecvTimeoutError},
    },
    time::Duration,
};

use windows_reactor::{
    Border, ChildrenControl, Color, Component, ComponentContext, ContentControl, Grid, GridChildExt, GridLength, HorizontalAlignment,
    LayoutControl, LocalSender, NavigationView, NavigationViewBackButtonVisible, NavigationViewItem, NavigationViewItemSlot,
    NavigationViewPaneDisplayMode, NavigationViewSlot, ScrollViewer, SlotView, SlotsControl, Symbol, SymbolIcon, TitleBar, TitleBarSlot,
    VerticalAlignment, View, ViewContext, WindowBackdrop, WindowTheme, WindowTitleBarHeight, WindowVisuals,
};

use crate::{
    pipeline::install_ui_ping,
    taskbar_guard::restore_taskbar_zorder,
    ui::{
        chrome::{app_status_strip, capture_start_stop_button, settings_sticky_chrome},
        dashboard::dashboard_page,
        settings::{api_page, ocr_page, overlay_page, translation_page},
        shared::{AppMsg, make_shared, take_chrome},
    },
};

/// Root WinUI component for the control window.
pub struct AppRoot {
    shared: Arc<parking_lot::Mutex<shared::UiShared>>,
    ping: LocalSender<AppMsg>,
    ping_rx: Arc<Mutex<Receiver<()>>>,
    page_tag: String,
    is_pane_open: bool,
}

impl AppRoot {
    fn arm_pipeline_ping(&self, context: &ComponentContext<Self>) {
        let rx = Arc::clone(&self.ping_rx);
        context.spawn_background(move |cancel| {
            loop {
                if cancel.is_cancelled() {
                    return AppMsg::Refresh;
                }
                let guard = rx.lock().unwrap_or_else(|e| e.into_inner());
                match guard.recv_timeout(Duration::from_millis(100)) {
                    Ok(()) => {
                        while guard.try_recv().is_ok() {}
                        return AppMsg::PipelineWake;
                    }
                    Err(RecvTimeoutError::Timeout) => continue,
                    Err(RecvTimeoutError::Disconnected) => return AppMsg::Refresh,
                }
            }
        });
    }
}

impl Component for AppRoot {
    type Input = ();
    type Message = AppMsg;

    fn create(_input: &(), context: &ComponentContext<Self>) -> Self {
        restore_taskbar_zorder();
        mica::apply();
        nav_header::apply();
        let (tx, rx) = std::sync::mpsc::channel();
        install_ui_ping(tx);
        let ping = context.sender();
        let root = Self {
            shared: make_shared(),
            ping,
            ping_rx: Arc::new(Mutex::new(rx)),
            page_tag: String::from("dashboard"),
            is_pane_open: true,
        };
        root.arm_pipeline_ping(context);
        root
    }

    fn update(&mut self, message: AppMsg, context: &ComponentContext<Self>) {
        match message {
            AppMsg::Refresh => {}
            AppMsg::PipelineWake => self.arm_pipeline_ping(context),
            AppMsg::SelectPage(tag) => {
                if !tag.is_empty() && tag != self.page_tag {
                    self.page_tag = tag;
                }
            }
            AppMsg::PaneOpen(open) => self.is_pane_open = open,
            AppMsg::TogglePane => self.is_pane_open = !self.is_pane_open,
        }
    }

    fn view(&self, _input: &(), context: &mut ViewContext<Self>) -> View {
        context.window_title("Translator Overlay");
        context.window_visuals(
            WindowVisuals::new()
                .backdrop(WindowBackdrop::Mica)
                .theme(WindowTheme::System)
                .client_size(1280.0, 800.0),
        );
        context.use_effect("taskbar-zorder", (), || {
            restore_taskbar_zorder();
            None
        });

        let chrome = take_chrome(&self.shared);
        let bump = &self.ping;
        let page_tag = self.page_tag.as_str();
        let is_pane_open = self.is_pane_open;

        let page: View = match page_tag {
            "api" => api_page(&self.shared, &chrome, bump),
            "translation" => translation_page(&self.shared, &chrome, bump),
            "ocr" => ocr_page(&self.shared, &chrome, bump),
            "overlay" => overlay_page(&self.shared, &chrome, bump),
            _ => dashboard_page(&self.shared, &chrome, bump),
        };

        let settings_meta: Option<&str> = match page_tag {
            "api" => Some("API"),
            "translation" => Some("Translation"),
            "ocr" => Some("OCR"),
            "overlay" => Some("Overlay"),
            _ => None,
        };

        let page_padding = windows_reactor::Thickness::new(24.0, if settings_meta.is_some() { 8.0 } else { 16.0 }, 24.0, 24.0);

        let content: View = if let Some(title) = settings_meta {
            let scrolled = ScrollViewer::new()
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .vertical_alignment(VerticalAlignment::Stretch)
                .grid_row(1)
                .grid_column(0)
                .content(
                    Border::new()
                        .padding(page_padding)
                        .horizontal_alignment(HorizontalAlignment::Stretch)
                        .content(page),
                );
            Grid::new()
                .rows([GridLength::Auto, GridLength::Star(1.0)])
                .columns([GridLength::Star(1.0)])
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .vertical_alignment(VerticalAlignment::Stretch)
                .children((
                    Border::new()
                        .grid_row(0)
                        .grid_column(0)
                        .horizontal_alignment(HorizontalAlignment::Stretch)
                        .content(settings_sticky_chrome(title, &self.shared, &chrome, bump)),
                    scrolled,
                ))
        } else {
            Border::new()
                .padding(page_padding)
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .vertical_alignment(VerticalAlignment::Stretch)
                .content(page)
        };

        let nav_items = [
            ("nav-dashboard", nav_item("dashboard", "Dashboard", Symbol::Home, page_tag == "dashboard")),
            ("nav-api", nav_item("api", "API", Symbol::Link, page_tag == "api")),
            ("nav-translation", nav_item("translation", "Translation", Symbol::Globe, page_tag == "translation")),
            ("nav-ocr", nav_item("ocr", "OCR", Symbol::Camera, page_tag == "ocr")),
            ("nav-overlay", nav_item("overlay", "Overlay", Symbol::ViewAll, page_tag == "overlay")),
        ];

        let nav = NavigationView::new()
            .pane_display_mode(NavigationViewPaneDisplayMode::Left)
            .open_pane_length(196.0)
            .is_pane_open(is_pane_open)
            .on_is_pane_open_changed(context.callback(AppMsg::PaneOpen))
            .on_selected_tag_changed(context.callback(|tag: Option<String>| AppMsg::SelectPage(tag.unwrap_or_default())))
            .is_settings_visible(false)
            .is_back_button_visible(NavigationViewBackButtonVisible::Collapsed)
            .is_pane_toggle_button_visible(false)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .vertical_alignment(VerticalAlignment::Stretch)
            .grid_row(1)
            .grid_column(0)
            .slots([
                SlotView::collection(NavigationViewSlot::MenuItems, nav_items),
                SlotView::new(NavigationViewSlot::Content, content),
                SlotView::new(NavigationViewSlot::PaneFooter, capture_start_stop_button(&self.shared, &chrome, bump, is_pane_open)),
            ]);

        let title_bar = TitleBar::new()
            .title("Translator Overlay")
            .preferred_height(WindowTitleBarHeight::Tall)
            .is_pane_toggle_button_visible(true)
            .is_back_button_visible(false)
            .on_pane_toggle_requested(context.message(AppMsg::TogglePane))
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .grid_row(0)
            .grid_column(0)
            .slot(TitleBarSlot::Content, app_status_strip(&chrome));

        Grid::new()
            .rows([GridLength::Auto, GridLength::Star(1.0)])
            .columns([GridLength::Star(1.0)])
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .vertical_alignment(VerticalAlignment::Stretch)
            .background(Color::transparent())
            .children((title_bar, nav))
    }
}

fn nav_item(tag: &str, label: &str, symbol: Symbol, selected: bool) -> View {
    NavigationViewItem::new().tag(tag).is_selected(selected).slots([
        SlotView::new(NavigationViewItemSlot::Icon, SymbolIcon::new().symbol(symbol)),
        SlotView::new(NavigationViewItemSlot::Content, label),
    ])
}
