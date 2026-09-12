//! Shared chrome: settings cards, InfoBar, title-bar status, nav-footer capture control.

use std::sync::Arc;

use parking_lot::Mutex;
use translator_core::PipelineStatus;
use windows_reactor::{
    AutomationExt, Border, Button, ButtonStyle, ChildrenControl, ContentControl, ContentDialog, ContentDialogResult, FontIcon, FontWeight,
    Grid, GridChildExt, GridLength, HorizontalAlignment, InfoBar, InfoBarSeverity, KeyedView, LayoutControl, LocalSender, Orientation,
    SlotsControl, StackPanel, Stretch, TextBlock, TextWrapping, ThemeBrush, Thickness, TooltipExt, VerticalAlignment, View, Viewbox,
    ViewboxSlot,
};

use crate::{
    pipeline::PipelineCommand,
    ui::shared::{
        AppMsg, ChromeSnap, ConfirmAction, UiCx, UiShared, do_discard, do_reload_from_disk, effective_draft, form_validation_error,
        is_settings_dirty,
    },
};

fn labeled_stack(header: TextBlock, description: Option<&str>) -> View {
    match description {
        Some(d) if !d.is_empty() => StackPanel::new()
            .spacing(2.0)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .vertical_alignment(VerticalAlignment::Center)
            .children((
                header,
                TextBlock::new()
                    .text(d)
                    .font_size(12.0)
                    .foreground(ThemeBrush::PrimaryText)
                    .opacity(0.72)
                    .text_wrapping(TextWrapping::WrapWholeWords),
            )),
        _ => StackPanel::new()
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .vertical_alignment(VerticalAlignment::Center)
            .children((header,)),
    }
}

/// Section label above a group of cards.
pub fn section_header(title: impl Into<String>) -> TextBlock {
    TextBlock::new()
        .text(title)
        .font_size(14.0)
        .font_weight(FontWeight::SEMI_BOLD)
        .margin(Thickness::new(0.0, 12.0, 0.0, 4.0))
}

/// Label for a cluster of cards inside a section (not a peer of [`section_header`]).
pub fn subsection_header(title: impl Into<String>) -> TextBlock {
    TextBlock::new()
        .text(title)
        .font_size(12.0)
        .font_weight(FontWeight::SEMI_BOLD)
        .foreground(ThemeBrush::PrimaryText)
        .opacity(0.72)
        .margin(Thickness::new(0.0, 8.0, 0.0, 2.0))
}

fn settings_row_body(header: impl Into<String>, description: Option<&str>, control: impl Into<View>) -> View {
    let left = labeled_stack(TextBlock::new().text(header).font_weight(FontWeight::SEMI_BOLD).font_size(14.0), description);

    Grid::new()
        .rows([GridLength::Auto])
        .columns([GridLength::Star(1.0), GridLength::Auto])
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .children((
            Border::new()
                .grid_row(0)
                .grid_column(0)
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .vertical_alignment(VerticalAlignment::Center)
                .margin(Thickness::new(0.0, 0.0, 16.0, 0.0))
                .content(left),
            Border::new()
                .grid_row(0)
                .grid_column(1)
                .horizontal_alignment(HorizontalAlignment::Right)
                .vertical_alignment(VerticalAlignment::Center)
                .content(control),
        ))
}

/// Standalone settings card: Header + Description on the left, control flush-right.
pub fn settings_card(key: &str, header: impl Into<String>, description: Option<&str>, control: impl Into<View>) -> View {
    settings_card_with_below(key, header, description, control, None)
}

/// [`settings_card`] with optional extra content under the header/control row.
pub fn settings_card_with_below(
    key: &str,
    header: impl Into<String>,
    description: Option<&str>,
    control: impl Into<View>,
    below: Option<View>,
) -> View {
    let row = settings_row_body(header, description, control);
    let content = if let Some(below) = below {
        StackPanel::new()
            .spacing(10.0)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .children((row, below))
    } else {
        row
    };
    Border::new()
        .automation_id(key)
        .background(ThemeBrush::CardBackground)
        .corner_radius(8.0)
        .padding(Thickness::uniform(16.0))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .content(content)
}

/// Card with header/description on top and full-width content below (sliders, multiline).
pub fn settings_card_stack(key: &str, header: impl Into<String>, description: Option<&str>, content: impl Into<View>) -> View {
    let head = labeled_stack(TextBlock::new().text(header).font_weight(FontWeight::SEMI_BOLD).font_size(14.0), description);

    Border::new()
        .automation_id(key)
        .background(ThemeBrush::CardBackground)
        .corner_radius(8.0)
        .padding(Thickness::uniform(16.0))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .content(
            StackPanel::new()
                .spacing(12.0)
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .children((head, content.into())),
        )
}

/// Pipeline InfoBar for the Dashboard only. Always open (Informational when idle).
pub fn status_infobar(snap: &ChromeSnap) -> InfoBar {
    let status_label = snap.status.label();
    let target = snap.target.as_deref().unwrap_or("(none)");
    let last_error = snap.last_error.as_deref().filter(|s| !s.is_empty());
    let retry_msg;
    let (title, message, severity) = match (&snap.status, last_error) {
        (PipelineStatus::RetryingTranslate { attempt, max_retries, .. }, Some(err)) => {
            retry_msg = format!("Retrying {attempt} of {max_retries}");
            (err, retry_msg.as_str(), InfoBarSeverity::Warning)
        }
        (_, Some(err)) => (err, "", InfoBarSeverity::Error),
        _ => (status_label.as_str(), target, InfoBarSeverity::Informational),
    };

    InfoBar::new()
        .title(title)
        .message(message)
        .severity(severity)
        .is_open(true)
        .is_closable(false)
}

/// Compact title-bar status: pipeline label and target window only.
pub fn app_status_strip(snap: &ChromeSnap) -> TextBlock {
    TextBlock::new()
        .text(format!("{}  ·  {}", snap.status.label(), snap.target.as_deref().unwrap_or("(none)")))
        .font_size(12.0)
        .foreground(ThemeBrush::PrimaryText)
        .opacity(0.72)
        .vertical_alignment(VerticalAlignment::Center)
}

/// Start/Stop for the NavigationView pane footer.
/// Style setters do not re-run; remount via `key` when Accent/Subtle or compact geometry change.
pub fn capture_start_stop_button(shared: &Arc<Mutex<UiShared>>, snap: &ChromeSnap, bump: &LocalSender<AppMsg>, pane_open: bool) -> View {
    let cx = UiCx::new(shared, bump);
    let has_window = snap.selected_hwnd.is_some();
    let running = snap.auto_running;
    let enabled = running || has_window;
    let label = if running { "Stop" } else { "Start" };
    let glyph = if running { "\u{EE95}" } else { "\u{F5B0}" };
    let tip = if running {
        "Stop continuous capture"
    } else if has_window {
        "Start continuous capture of the selected window"
    } else {
        "Select a window first"
    };
    let key = match (running, pane_open) {
        (true, true) => "btn-stop-wide",
        (true, false) => "btn-stop-compact",
        (false, true) => "btn-start-wide",
        (false, false) => "btn-start-compact",
    };
    // NavigationViewItemOnLeftMinHeight / OnLeftIconBoxHeight.
    const NAV_ITEM_HEIGHT: f64 = 36.0;
    const NAV_ICON_BOX: f64 = 40.0;
    const NAV_GLYPH: f64 = 16.0;
    let glyph_icon = Viewbox::new()
        .width(NAV_GLYPH)
        .height(NAV_GLYPH)
        .stretch(Stretch::Uniform)
        .horizontal_alignment(HorizontalAlignment::Center)
        .vertical_alignment(VerticalAlignment::Center)
        .slot(ViewboxSlot::Child, FontIcon::new().glyph(glyph));
    let content: View = if pane_open {
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .vertical_alignment(VerticalAlignment::Center)
            .children((glyph_icon, TextBlock::new().text(label).vertical_alignment(VerticalAlignment::Center)))
    } else {
        glyph_icon
    };

    let mut start = Button::new()
        .is_enabled(enabled)
        .vertical_alignment(VerticalAlignment::Bottom)
        .vertical_content_alignment(VerticalAlignment::Center)
        .height(NAV_ITEM_HEIGHT)
        .automation_name(label)
        .automation_id("btn-start-stop")
        .on_click({
            let cx = cx.clone();
            move || {
                {
                    let ui = cx.shared.lock();
                    if ui.state.read().auto_running {
                        let _ = ui.cmd_tx.send(PipelineCommand::StopCapture);
                    } else if let Some(w) = ui.selected_idx.and_then(|i| ui.windows.get(i)).cloned() {
                        let _ = ui.cmd_tx.send(PipelineCommand::StartCapture {
                            hwnd: w.hwnd,
                            title: w.title,
                        });
                    }
                }
                cx.refresh();
            }
        });
    if running {
        start = start.style(ButtonStyle::Accent);
    } else if !pane_open {
        start = start.style(ButtonStyle::Subtle);
    }
    start = if pane_open {
        start
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .margin(Thickness::new(8.0, 4.0, 8.0, 8.0))
    } else {
        start
            .horizontal_content_alignment(HorizontalAlignment::Center)
            .width(NAV_ICON_BOX)
            .horizontal_alignment(HorizontalAlignment::Center)
            .margin(Thickness::new(0.0, 4.0, 0.0, 8.0))
    };
    let mut footer = Grid::new()
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .vertical_alignment(VerticalAlignment::Bottom);
    if !pane_open {
        footer = footer.width(48.0).horizontal_alignment(HorizontalAlignment::Center);
    }
    footer.keyed_children([KeyedView::new(key, start.content(content).tooltip(tip))])
}

/// Shared Save / Reload / Discard bar for settings pages.
pub fn settings_actions(shared: &Arc<Mutex<UiShared>>, snap: &ChromeSnap, bump: &LocalSender<AppMsg>) -> View {
    let cx = UiCx::new(shared, bump);
    let dirty = snap.settings_dirty;
    let has_form_error = snap.form_error.is_some();

    let tooltip = if let Some(err) = snap.form_error.as_deref() {
        format!("Cannot save: {err}")
    } else if dirty {
        "Unsaved changes — click to save and apply".into()
    } else {
        "Save to config.toml and apply now".into()
    };

    let mut save = Button::new();
    if dirty || has_form_error {
        save = save.style(ButtonStyle::Accent);
    }
    let save = save
        .on_click({
            let cx = cx.clone();
            move || {
                {
                    let mut ui = cx.shared.lock();
                    if let Some(err) = form_validation_error(&ui) {
                        ui.form_error = Some(err);
                        drop(ui);
                        cx.refresh();
                        return;
                    }
                    ui.draft = effective_draft(&ui);
                    let cfg = ui.draft.clone();
                    ui.form_error = None;
                    {
                        let mut s = ui.state.write();
                        s.config = cfg.clone();
                        s.settings_message = None;
                        s.last_error = None;
                    }
                    let _ = ui.cmd_tx.send(PipelineCommand::ApplyConfig(Box::new(cfg)));
                }
                cx.refresh();
            }
        })
        .content("Save")
        .tooltip(tooltip);

    let status_hint: View = if let Some(err) = snap.form_error.as_ref() {
        TextBlock::new()
            .text(err.clone())
            .font_size(12.0)
            .foreground(ThemeBrush::SystemCritical)
            .vertical_alignment(VerticalAlignment::Center)
            .into()
    } else if let Some(msg) = snap.settings_message.as_ref().filter(|s| !s.is_empty()) {
        TextBlock::new()
            .text(msg.clone())
            .font_size(12.0)
            .foreground(ThemeBrush::Accent)
            .vertical_alignment(VerticalAlignment::Center)
            .into()
    } else {
        TextBlock::new()
            .text("")
            .font_size(12.0)
            .vertical_alignment(VerticalAlignment::Center)
            .into()
    };

    StackPanel::new().orientation(Orientation::Horizontal).spacing(8.0).children((
        status_hint,
        save,
        Button::new()
            .on_click({
                let cx = cx.clone();
                move || {
                    cx.with_mut(|ui| {
                        if is_settings_dirty(ui) {
                            ui.confirm = ConfirmAction::Reload;
                        } else {
                            do_reload_from_disk(ui);
                        }
                    });
                }
            })
            .content("Reload")
            .tooltip("Reload config.toml from disk and apply"),
        Button::new()
            .on_click({
                let cx = cx.clone();
                move || {
                    cx.with_mut(|ui| {
                        if is_settings_dirty(ui) {
                            ui.confirm = ConfirmAction::Discard;
                        } else {
                            do_discard(ui);
                        }
                    });
                }
            })
            .content("Discard")
            .tooltip("Discard edits and restore currently running settings"),
    ))
}

/// Sticky top bar: page title + Save actions (outside scroll).
pub fn settings_sticky_chrome(title: &str, shared: &Arc<Mutex<UiShared>>, snap: &ChromeSnap, bump: &LocalSender<AppMsg>) -> View {
    let title_el = TextBlock::new().text(title).font_size(28.0).font_weight(FontWeight::BOLD);

    Border::new()
        .padding(Thickness::new(24.0, 16.0, 24.0, 12.0))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .content(
            Grid::new()
                .rows([GridLength::Auto])
                .columns([GridLength::Star(1.0), GridLength::Auto])
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .children((
                    Border::new()
                        .grid_row(0)
                        .grid_column(0)
                        .horizontal_alignment(HorizontalAlignment::Stretch)
                        .vertical_alignment(VerticalAlignment::Center)
                        .margin(Thickness::new(0.0, 0.0, 16.0, 0.0))
                        .content(title_el),
                    Border::new()
                        .grid_row(0)
                        .grid_column(1)
                        .vertical_alignment(VerticalAlignment::Center)
                        .horizontal_alignment(HorizontalAlignment::Right)
                        .content(settings_actions(shared, snap, bump)),
                )),
        )
}

fn confirm_dialog(shared: &Arc<Mutex<UiShared>>, snap: &ChromeSnap, bump: &LocalSender<AppMsg>) -> View {
    let open = snap.confirm != ConfirmAction::None;
    let (title, body, primary) = match snap.confirm {
        ConfirmAction::Reload => {
            ("Reload from disk?", "Unsaved edits will be lost. The file on disk will replace the current draft.", "Reload")
        }
        ConfirmAction::Discard => ("Discard changes?", "Unsaved edits will be lost. Running settings will be restored.", "Discard"),
        ConfirmAction::None => ("", "", "OK"),
    };

    let cx = UiCx::new(shared, bump);
    ContentDialog::new()
        .title(title)
        .primary_button_text(primary)
        .close_button_text("Cancel")
        .is_open(open)
        .on_closed(move |result: ContentDialogResult| {
            cx.with_mut(|ui| {
                let action = ui.confirm;
                ui.confirm = ConfirmAction::None;
                if result == ContentDialogResult::Primary {
                    match action {
                        ConfirmAction::Reload => do_reload_from_disk(ui),
                        ConfirmAction::Discard => do_discard(ui),
                        ConfirmAction::None => {}
                    }
                }
            });
        })
        .content(body)
}

/// Standard settings page body (cards only).
pub fn settings_page_shell(shared: &Arc<Mutex<UiShared>>, snap: &ChromeSnap, bump: &LocalSender<AppMsg>, body: impl Into<View>) -> View {
    StackPanel::new()
        .spacing(12.0)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .children((body.into(), confirm_dialog(shared, snap, bump)))
}
