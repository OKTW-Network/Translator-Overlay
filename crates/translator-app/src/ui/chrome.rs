//! Window chrome: status, Start/Stop, sticky Save bar, settings page shell.

use std::{borrow::Cow, sync::Arc};

use parking_lot::Mutex;
use rust_i18n::t;
use translator_core::{PipelineStatus, SETTINGS_SAVED, UiLanguage};
use windows_reactor::{
    AutomationExt, Border, Button, ButtonStyle, ChildrenControl, ComboBox, ContentControl, ContentDialog, ContentDialogResult, FontIcon,
    FontWeight, Grid, GridChildExt, GridLength, HorizontalAlignment, InfoBar, InfoBarSeverity, KeyedView, LayoutControl, LocalSender,
    Orientation, SlotsControl, StackPanel, Stretch, TextBlock, TextTrimming, TextWrapping, ThemeBrush, Thickness, TooltipExt,
    VerticalAlignment, View, Viewbox, ViewboxSlot,
};

use crate::{
    pipeline::PipelineCommand,
    ui::{
        locale::UI_LANGUAGES,
        shared::{
            AppMsg, ChromeSnap, ConfirmAction, UiCx, UiShared, do_discard, do_reload_from_disk, effective_draft, form_validation_error,
            is_settings_dirty, send_ui_language,
        },
    },
};

/// Pipeline InfoBar for the Dashboard only. Always open (Informational when idle).
pub fn status_infobar(snap: &ChromeSnap) -> InfoBar {
    let status_label = status_text(&snap.status);
    let target = snap.target.as_deref().map(Cow::Borrowed).unwrap_or_else(|| t!("target.none"));
    let (title, message, severity) = match (&snap.status, snap.last_error.as_deref().filter(|s| !s.is_empty())) {
        (PipelineStatus::RetryingTranslate { attempt, max_retries, .. }, Some(err)) => (
            Cow::Owned(err.to_string()),
            t!("status.retrying_short", attempt = attempt, max_retries = max_retries),
            InfoBarSeverity::Warning,
        ),
        (_, Some(err)) => (Cow::Owned(err.to_string()), Cow::Borrowed(""), InfoBarSeverity::Error),
        _ => (status_label, target, InfoBarSeverity::Informational),
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
        .text(format!("{}  ·  {}", status_text(&snap.status), snap.target.as_deref().unwrap_or(&t!("target.none"))))
        .font_size(12.0)
        .foreground(ThemeBrush::PrimaryText)
        .opacity(0.72)
        .text_wrapping(TextWrapping::NoWrap)
        .text_trimming(TextTrimming::CharacterEllipsis)
        .max_lines(1)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .vertical_alignment(VerticalAlignment::Center)
}

/// Language combo for `TitleBar` `RightHeader` (fixed next to caption buttons).
pub fn ui_language_combo(shared: &Arc<Mutex<UiShared>>, bump: &LocalSender<AppMsg>) -> View {
    let selected = {
        let ui = shared.lock();
        ui.draft.ui.language.unwrap_or(UiLanguage::En)
    };
    let selected_index = UI_LANGUAGES.iter().position(|(lang, _)| *lang == selected);
    ComboBox::new()
        .items_source(UI_LANGUAGES.map(|(_, label)| label.to_string()).to_vec())
        .selected_index(selected_index)
        .min_width(132.0)
        .vertical_alignment(VerticalAlignment::Center)
        .margin(Thickness::new(8.0, 0.0, 0.0, 0.0))
        .on_selection_changed({
            let shared = shared.clone();
            let bump = bump.clone();
            move |idx: Option<usize>| {
                let Some(&(language, _)) = idx.and_then(|i| UI_LANGUAGES.get(i)) else {
                    return;
                };
                let mut ui = shared.lock();
                if ui.draft.ui.language != Some(language) {
                    send_ui_language(&mut ui, language);
                    drop(ui);
                    let _ = bump.send(AppMsg::Refresh);
                }
            }
        })
        .into()
}

/// Start/Pause (same slot) plus Stop above it in the NavigationView pane footer.
/// Style setters do not re-run; remount via `key` when Accent/Subtle or compact geometry change.
pub fn capture_start_stop_button(shared: &Arc<Mutex<UiShared>>, snap: &ChromeSnap, bump: &LocalSender<AppMsg>, pane_open: bool) -> View {
    let cx = UiCx::new(shared, bump);
    let has_window = snap.selected_hwnd.is_some();
    let paused = snap.capture_paused;
    let running = snap.auto_running && !paused;
    let session = snap.auto_running;
    let busy = snap.capture_busy;
    let primary_enabled = !busy && (session || has_window);
    let stop_enabled = !busy && session;
    let primary_label = if running { t!("action.pause") } else { t!("action.start") };
    let primary_glyph = if running { "\u{E769}" } else { "\u{F5B0}" };
    let stop_label = t!("action.stop");
    let primary_tip = if busy {
        t!("capture.waiting_stop")
    } else if running {
        t!("capture.pause")
    } else if paused {
        t!("capture.resume")
    } else if has_window {
        t!("capture.start")
    } else {
        t!("capture.select_window_first")
    };
    let stop_tip = if busy {
        t!("capture.waiting_stop")
    } else if session {
        t!("capture.stop")
    } else {
        t!("capture.stop_idle")
    };
    let key = format!(
        "cap-{}-{}-{}",
        if running {
            "pause"
        } else if paused {
            "resume"
        } else {
            "start"
        },
        if pane_open { "wide" } else { "compact" },
        if stop_enabled { "stop-on" } else { "stop-off" },
    );
    const NAV_ITEM_HEIGHT: f64 = 36.0;
    const NAV_ICON_BOX: f64 = 40.0;
    const NAV_GLYPH: f64 = 16.0;

    let glyph_icon = |glyph: &str| {
        Viewbox::new()
            .width(NAV_GLYPH)
            .height(NAV_GLYPH)
            .stretch(Stretch::Uniform)
            .horizontal_alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Center)
            .slot(ViewboxSlot::Child, FontIcon::new().glyph(glyph))
    };
    let button_content = |glyph: &str, label: &str| -> View {
        if pane_open {
            StackPanel::new()
                .orientation(Orientation::Horizontal)
                .spacing(8.0)
                .vertical_alignment(VerticalAlignment::Center)
                .children((glyph_icon(glyph), TextBlock::new().text(label).vertical_alignment(VerticalAlignment::Center)))
        } else {
            glyph_icon(glyph)
        }
    };

    let mut stop = Button::new()
        .is_enabled(stop_enabled)
        .vertical_alignment(VerticalAlignment::Bottom)
        .vertical_content_alignment(VerticalAlignment::Center)
        .height(NAV_ITEM_HEIGHT)
        .automation_name(stop_label.as_ref())
        .automation_id("btn-stop")
        .on_click({
            let cx = cx.clone();
            move || {
                {
                    let ui = cx.shared.lock();
                    let mut s = ui.state.write();
                    if !s.capture_busy && s.auto_running {
                        s.capture_busy = true;
                        drop(s);
                        let _ = ui.cmd_tx.send(PipelineCommand::StopCapture);
                    }
                }
                cx.refresh();
            }
        });
    if !pane_open {
        stop = stop.style(ButtonStyle::Subtle);
    }

    let mut primary = Button::new()
        .is_enabled(primary_enabled)
        .vertical_alignment(VerticalAlignment::Bottom)
        .vertical_content_alignment(VerticalAlignment::Center)
        .height(NAV_ITEM_HEIGHT)
        .automation_name(primary_label.as_ref())
        .automation_id("btn-start-pause")
        .on_click({
            let cx = cx.clone();
            move || {
                {
                    let ui = cx.shared.lock();
                    let s = ui.state.write();
                    if !s.capture_busy {
                        if s.auto_running && !s.capture_paused {
                            drop(s);
                            let _ = ui.cmd_tx.send(PipelineCommand::PauseCapture);
                        } else if s.capture_paused {
                            drop(s);
                            let _ = ui.cmd_tx.send(PipelineCommand::ResumeCapture);
                        } else {
                            drop(s);
                            if let Some(w) = ui.selected_idx.and_then(|i| ui.windows.get(i)).cloned() {
                                let _ = ui.cmd_tx.send(PipelineCommand::StartCapture {
                                    hwnd: w.hwnd,
                                    title: w.title,
                                });
                            }
                        }
                    }
                }
                cx.refresh();
            }
        });
    if running || paused {
        primary = primary.style(ButtonStyle::Accent);
    } else if !pane_open {
        primary = primary.style(ButtonStyle::Subtle);
    }

    if pane_open {
        stop = stop
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .margin(Thickness::new(8.0, 8.0, 8.0, 4.0));
        primary = primary
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .margin(Thickness::new(8.0, 4.0, 8.0, 8.0));
    } else {
        stop = stop
            .horizontal_content_alignment(HorizontalAlignment::Center)
            .width(NAV_ICON_BOX)
            .horizontal_alignment(HorizontalAlignment::Center)
            .margin(Thickness::new(0.0, 4.0, 0.0, 0.0));
        primary = primary
            .horizontal_content_alignment(HorizontalAlignment::Center)
            .width(NAV_ICON_BOX)
            .horizontal_alignment(HorizontalAlignment::Center)
            .margin(Thickness::new(0.0, 4.0, 0.0, 8.0));
    }

    let mut footer = Grid::new()
        .rows([GridLength::Auto, GridLength::Auto])
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .vertical_alignment(VerticalAlignment::Bottom);
    if !pane_open {
        footer = footer.width(48.0).horizontal_alignment(HorizontalAlignment::Center);
    }
    footer.keyed_children([
        KeyedView::new(
            format!("{key}-stop"),
            Border::new()
                .grid_row(0)
                .content(stop.content(button_content("\u{EE95}", stop_label.as_ref())).tooltip(stop_tip)),
        ),
        KeyedView::new(
            format!("{key}-primary"),
            Border::new().grid_row(1).content(
                primary
                    .content(button_content(primary_glyph, primary_label.as_ref()))
                    .tooltip(primary_tip),
            ),
        ),
    ])
}

/// Shared Save / Reload / Discard bar for settings pages.
pub fn settings_actions(shared: &Arc<Mutex<UiShared>>, snap: &ChromeSnap, bump: &LocalSender<AppMsg>) -> View {
    let cx = UiCx::new(shared, bump);
    let dirty = snap.settings_dirty;
    let has_form_error = snap.form_error.is_some();

    let tooltip = if let Some(err) = snap.form_error.as_deref() {
        t!("save.cannot", error = err)
    } else if dirty {
        t!("save.unsaved")
    } else {
        t!("save.idle_tip")
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
        .content(t!("action.save").as_ref())
        .tooltip(tooltip);

    let status_hint: View = if let Some(err) = snap.form_error.as_ref() {
        TextBlock::new()
            .text(err.clone())
            .font_size(12.0)
            .foreground(ThemeBrush::SystemCritical)
            .vertical_alignment(VerticalAlignment::Center)
            .into()
    } else if let Some(msg) = snap.settings_message.as_ref().filter(|s| !s.is_empty()) {
        let msg = if msg == SETTINGS_SAVED {
            t!("saved")
        } else {
            Cow::Owned(msg.clone())
        };
        TextBlock::new()
            .text(msg)
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
            .content(t!("action.reload").as_ref())
            .tooltip(t!("save.reload_tip")),
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
            .content(t!("action.discard").as_ref())
            .tooltip(t!("save.discard_tip")),
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
        ConfirmAction::Reload => (t!("confirm.reload_title"), t!("confirm.reload_body"), t!("action.reload")),
        ConfirmAction::Discard => (t!("confirm.discard_title"), t!("confirm.discard_body"), t!("action.discard")),
        ConfirmAction::None => (Cow::Borrowed(""), Cow::Borrowed(""), t!("action.ok")),
    };

    let cx = UiCx::new(shared, bump);
    ContentDialog::new()
        .title(title)
        .primary_button_text(primary)
        .close_button_text(t!("action.cancel"))
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
        .content(body.as_ref())
}

pub(crate) fn status_text(status: &PipelineStatus) -> Cow<'static, str> {
    match status {
        PipelineStatus::Idle => t!("status.idle"),
        PipelineStatus::Capturing => t!("status.capturing"),
        PipelineStatus::RunningOcr => t!("status.running_ocr"),
        PipelineStatus::WaitingForStable { elapsed_ms } => t!("status.waiting_stable", elapsed_ms = elapsed_ms),
        PipelineStatus::DownloadingModels {
            file,
            file_index,
            file_count,
            percent,
        } => {
            t!("status.downloading", file = file, file_index = file_index, file_count = file_count, percent = percent)
        }
        PipelineStatus::LoadingModels => t!("status.loading_models"),
        PipelineStatus::Translating => t!("status.translating"),
        PipelineStatus::RetryingTranslate { attempt, max_retries, .. } => {
            t!("status.retrying", attempt = attempt, max_retries = max_retries)
        }
        PipelineStatus::Cancelled => t!("status.cancelled"),
        PipelineStatus::OverlayActive => t!("status.overlay_active"),
        PipelineStatus::Paused => t!("status.paused"),
        PipelineStatus::Error { message } => t!("status.error", message = message),
    }
}

/// Standard settings page body (cards only).
pub fn settings_page_shell(shared: &Arc<Mutex<UiShared>>, snap: &ChromeSnap, bump: &LocalSender<AppMsg>, body: impl Into<View>) -> View {
    StackPanel::new()
        .spacing(12.0)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .children((body.into(), confirm_dialog(shared, snap, bump)))
}
