//! windows-reactor control UI — dashboard + settings (card layout).

mod chrome;
mod controls;
mod dashboard;
mod nav_header;
mod page_api;
mod page_ocr;
mod page_overlay;
mod page_translation;
mod preview;
mod shared;

use std::time::Duration;

use windows_reactor::{
    Backdrop, BackgroundExt, Color, DispatcherTimer, Element, GridChildExt, GridLength, HorizontalAlignment, KeyExt, LayoutExt,
    NavViewItem, NavigationView, NavigationViewPaneDisplayMode, PaddingExt, RenderCx, RequestedTheme, ResourceExt, Symbol, Thickness,
    TitleBar, VerticalAlignment, border, grid, scroll_viewer, set_backdrop, set_requested_theme,
};

use crate::ui::{
    chrome::{app_status_strip, capture_start_stop_button, settings_sticky_chrome},
    dashboard::dashboard_page,
    page_api::api_page,
    page_ocr::ocr_page,
    page_overlay::overlay_page,
    page_translation::translation_page,
    shared::{make_shared, take_snapshot},
};

/// Entry render function for the control window.
pub fn app(cx: &mut RenderCx) -> Element {
    cx.use_effect((), || {
        set_requested_theme(RequestedTheme::Default);
        set_backdrop(Some(Backdrop::Mica));
        // WinUI 3 Activate can knock the taskbar out of the topmost stack (#11091).
        crate::taskbar_guard::restore_taskbar_zorder();
    });
    let _scheme = cx.use_color_scheme();

    let shared = cx.use_ref(make_shared());
    let (tick, bump_tick) = cx.use_reducer(0_u32);
    let _ = tick;
    let (page_tag, set_page) = cx.use_state(String::from("dashboard"));
    let (is_pane_open, set_pane_open) = cx.use_state(true);

    cx.use_effect_with_cleanup((), {
        let bump_tick = bump_tick.clone();
        move || {
            let timer = DispatcherTimer::new(Duration::from_millis(250), move || {
                crate::ui::nav_header::retarget();
                bump_tick.call(|n| n.wrapping_add(1));
            })
            .ok();
            Some(move || drop(timer))
        }
    });

    let shared_arc = shared.borrow().clone();
    let snap = take_snapshot(&shared_arc);

    // Critical: every page needs a distinct key so the reconciler does not
    // positionally reuse StackPanel children across tab switches.
    let page: Element = match page_tag.as_str() {
        "api" => api_page(&shared_arc, &snap, &bump_tick).with_key("page-api").into(),
        "translation" => translation_page(&shared_arc, &snap, &bump_tick).with_key("page-translation").into(),
        "ocr" => ocr_page(&shared_arc, &snap, &bump_tick).with_key("page-ocr").into(),
        "overlay" => overlay_page(&shared_arc, &snap, &bump_tick).with_key("page-overlay").into(),
        _ => dashboard_page(&shared_arc, &snap, &bump_tick).with_key("page-dashboard"),
    };

    // Settings pages: pin title + Save above the scroll (color = unsaved).
    let settings_meta: Option<(&str, Option<&str>)> = match page_tag.as_str() {
        "api" => Some(("API", Some("Choose how to reach a translation model."))),
        "translation" => Some(("Translation", Some("Languages and translation context."))),
        "ocr" => Some(("OCR", Some("Text recognition and capture timing."))),
        "overlay" => Some(("Overlay", Some("Where and how translations are shown."))),
        _ => None,
    };

    let page_padding = Thickness {
        left: 24.0,
        top: if settings_meta.is_some() { 8.0 } else { 16.0 },
        right: 24.0,
        bottom: 24.0,
    };

    // Settings: Grid Auto+* so ScrollViewer gets a bounded height.
    // A VStack measures children with infinite height → ScrollViewer never scrolls.
    // Dashboard: same Auto+* fill (page is already a Grid); do not wrap it in an
    // unbounded ScrollViewer or the workspace Star row collapses.
    let content: Element = if let Some((title, description)) = settings_meta {
        let scrolled = scroll_viewer(
            border(page)
                .padding(page_padding)
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .with_key(format!("body-{}", page_tag.as_str())),
        )
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .vertical_alignment(VerticalAlignment::Stretch)
        .with_key(format!("scroll-{}", page_tag.as_str()));
        grid((
            settings_sticky_chrome(title, description, &shared_arc, &snap, &bump_tick)
                .grid_row(0)
                .grid_column(0)
                .horizontal_alignment(HorizontalAlignment::Stretch),
            scrolled.grid_row(1).grid_column(0),
        ))
        .rows([GridLength::Auto, GridLength::Star(1.0)])
        .columns([GridLength::Star(1.0)])
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .vertical_alignment(VerticalAlignment::Stretch)
        .with_key(format!("settings-frame-{}", page_tag.as_str()))
        .into()
    } else {
        border(page)
            .padding(page_padding)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .vertical_alignment(VerticalAlignment::Stretch)
            .with_key(format!("body-{}", page_tag.as_str()))
            .into()
    };

    let nav_items = [
        NavViewItem::new("Dashboard").tag("dashboard").icon(Symbol::Home),
        NavViewItem::header("Settings"),
        NavViewItem::new("API").tag("api").icon(Symbol::Link),
        NavViewItem::new("Translation").tag("translation").icon(Symbol::Globe),
        NavViewItem::new("OCR").tag("ocr").icon(Symbol::Camera),
        NavViewItem::new("Overlay").tag("overlay").icon(Symbol::ViewAll),
    ];

    let current_tag = page_tag.clone();
    // Fluent card pattern on Mica: clear NavigationView content-layer fill + border
    // so Mica shows between settings cards (cards keep CardBackground).
    // Brush overrides only — Thickness/CornerRadius resource boxing can crash.
    // Pane hamburger lives on TitleBar (WinUI Gallery shell).
    let nav = NavigationView::new(nav_items, content)
        .pane_display_mode(NavigationViewPaneDisplayMode::Left)
        .open_pane_length(168.0)
        .pane_open(is_pane_open)
        .on_pane_open_changed({
            let set_pane_open = set_pane_open.clone();
            move |open| set_pane_open.call(open)
        })
        .selected_tag(page_tag.as_str())
        .on_selection_changed({
            move |tag: String| {
                if !tag.is_empty() && tag != current_tag {
                    set_page.call(tag);
                }
            }
        })
        .pane_footer(capture_start_stop_button(&shared_arc, &snap, &bump_tick, is_pane_open))
        .settings_visible(false)
        .back_button_visible(false)
        .pane_toggle_button_visible(false)
        .background(Color::transparent())
        .resource_overrides(|r| {
            r.set("NavigationViewContentBackground", Color::transparent())
                .set("NavigationViewContentGridBorderBrush", Color::transparent())
        })
        .with_key("main-nav");

    let title_bar = TitleBar::new("Translator Overlay")
        .pane_toggle_button_visible(true)
        .back_button_visible(false)
        .on_pane_toggle_requested(move || set_pane_open.call(!is_pane_open))
        .content(app_status_strip(&snap))
        .tall(true)
        .with_key("app-title-bar");

    grid((
        title_bar
            .grid_row(0)
            .grid_column(0)
            .horizontal_alignment(HorizontalAlignment::Stretch),
        nav.grid_row(1)
            .grid_column(0)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .vertical_alignment(VerticalAlignment::Stretch),
    ))
    .rows([GridLength::Auto, GridLength::Star(1.0)])
    .columns([GridLength::Star(1.0)])
    .horizontal_alignment(HorizontalAlignment::Stretch)
    .vertical_alignment(VerticalAlignment::Stretch)
    .into()
}
