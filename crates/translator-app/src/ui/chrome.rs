//! Shared chrome: settings cards, InfoBar, title-bar status, nav-footer capture control.

use std::sync::Arc;

use parking_lot::Mutex;
use windows_reactor::{
    BackgroundExt, Border, ContentDialog, ContentDialogResult, Element, Grid, GridChildExt, GridLength, HorizontalAlignment, Icon, InfoBar,
    InfoBarSeverity, KeyExt, LayoutExt, PaddingExt, StackPanel, TextBlock, TextStyleExt, ThemeRef, Thickness, TooltipExt, Updater,
    VerticalAlignment, border, button, grid, hstack, text_block, vstack,
};

use crate::{
    pipeline::PipelineCommand,
    ui::shared::{
        ConfirmAction, Snapshot, UiCx, UiShared, commit_optional_fields, do_discard, do_reload_from_disk, form_validation_error,
        is_settings_dirty,
    },
};

fn labeled_stack(header: TextBlock, description: Option<&str>) -> StackPanel {
    match description {
        Some(d) if !d.is_empty() => vstack((header, text_block(d).font_size(12.0).foreground(ThemeRef::SecondaryText).wrap())).spacing(2.0),
        _ => vstack((header,)),
    }
    .horizontal_alignment(HorizontalAlignment::Stretch)
    .vertical_alignment(VerticalAlignment::Center)
}

/// Windows Settings–style page title + optional description.
pub fn page_header(title: impl Into<String>, description: Option<&str>) -> Element {
    let title = text_block(title).font_size(28.0).bold();
    match description {
        Some(d) if !d.is_empty() => vstack((title, text_block(d).font_size(13.0).foreground(ThemeRef::SecondaryText).wrap()))
            .spacing(4.0)
            .into(),
        _ => title.into(),
    }
}

/// Section label above a group of cards.
pub fn section_header(title: impl Into<String>) -> TextBlock {
    text_block(title).font_size(14.0).semibold().margin(Thickness {
        left: 0.0,
        top: 12.0,
        right: 0.0,
        bottom: 4.0,
    })
}

/// Label for a cluster of cards inside a section (not a peer of [`section_header`]).
pub fn subsection_header(title: impl Into<String>) -> TextBlock {
    text_block(title)
        .font_size(12.0)
        .semibold()
        .foreground(ThemeRef::SecondaryText)
        .margin(Thickness {
            left: 0.0,
            top: 8.0,
            right: 0.0,
            bottom: 2.0,
        })
}

/// Shared label + control layout for settings cards.
fn settings_row_body(header: impl Into<String>, description: Option<&str>, control: impl Into<Element> + GridChildExt + LayoutExt) -> Grid {
    let left = labeled_stack(text_block(header).semibold().font_size(14.0), description);

    // Grid must Stretch: otherwise Star collapses to content and controls pack left.
    grid((
        left.grid_row(0)
            .grid_column(0)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .vertical_alignment(VerticalAlignment::Center)
            .margin(Thickness {
                left: 0.0,
                top: 0.0,
                right: 16.0,
                bottom: 0.0,
            }),
        control
            .grid_row(0)
            .grid_column(1)
            .horizontal_alignment(HorizontalAlignment::Right)
            .vertical_alignment(VerticalAlignment::Center),
    ))
    .rows([GridLength::Auto])
    .columns([GridLength::Star(1.0), GridLength::Auto])
    .horizontal_alignment(HorizontalAlignment::Stretch)
}

/// Standalone settings card: Header + Description on the left, control flush-right.
///
/// Uses Grid `Star | Auto` so the control column stays right-aligned regardless
/// of label/description length (HStack would pack after the text width).
pub fn settings_card(
    key: &str,
    header: impl Into<String>,
    description: Option<&str>,
    control: impl Into<Element> + GridChildExt + LayoutExt,
) -> Border {
    border(settings_row_body(header, description, control))
        .background(ThemeRef::CardBackground)
        .corner_radius(8.0)
        .padding(Thickness::uniform(16.0))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .with_key(key)
}

/// Card with header/description on top and full-width content below (sliders, multiline).
pub fn settings_card_stack(key: &str, header: impl Into<String>, description: Option<&str>, content: impl Into<Element>) -> Border {
    let head = labeled_stack(text_block(header).semibold().font_size(14.0), description);

    border(
        vstack((head, content.into()))
            .spacing(12.0)
            .horizontal_alignment(HorizontalAlignment::Stretch),
    )
    .background(ThemeRef::CardBackground)
    .corner_radius(8.0)
    .padding(Thickness::uniform(16.0))
    .horizontal_alignment(HorizontalAlignment::Stretch)
    .with_key(key)
}

/// Pipeline InfoBar for the Dashboard only. Always open (Informational when idle).
///
/// Settings save success lives on the settings chrome (next to Save), not here.
/// Always mounts the same `InfoBar` (stable key) so open/close does not remount
/// the rest of the page tree.
pub fn status_infobar(snap: &Snapshot) -> InfoBar {
    let retry_msg;
    let (title, message, severity) = if snap.retrying && !snap.last_error.is_empty() {
        retry_msg = format!("Retrying {} of {}", snap.retry_attempt, snap.retry_max);
        (snap.last_error.as_str(), retry_msg.as_str(), InfoBarSeverity::Warning)
    } else if !snap.last_error.is_empty() {
        (snap.last_error.as_str(), "", InfoBarSeverity::Error)
    } else {
        (snap.status.as_str(), snap.target.as_str(), InfoBarSeverity::Informational)
    };

    InfoBar::new(title)
        .message(message)
        .severity(severity)
        .is_open(true)
        .is_closable(false)
        .with_key("status-infobar")
}

/// Compact title-bar status: pipeline label and target window only.
pub fn app_status_strip(snap: &Snapshot) -> TextBlock {
    text_block(format!("{}  ·  {}", snap.status, snap.target))
        .font_size(12.0)
        .foreground(ThemeRef::SecondaryText)
        .vertical_alignment(VerticalAlignment::Center)
}

/// Start/Stop for the NavigationView pane footer.
///
/// Start: Default + Play. Stop: Accent + Stop. Color uses built-in button
/// styles (not a raw Background).
///
/// `pane_footer` is a single slot — `with_key` on the footer root does not
/// remount. Wrap in a Grid so the button is a keyed child and Start/Stop or
/// compact/wide actually recreate the control (`Button::accent()` Unset is a
/// no-op; compact MinWidth/Padding would otherwise leak after expand).
///
/// Compact pane is 48px; nav items use a 40px icon box. Collapsed mode is a
/// 40px chromeless control centered in a 48px footer so the glyph lines up
/// with the menu icons (a Default-styled 32px button sat left and looked huge).
pub fn capture_start_stop_button(shared: &Arc<Mutex<UiShared>>, snap: &Snapshot, bump: &Updater<u32>, pane_open: bool) -> Grid {
    let cx = UiCx::new(shared, bump);
    let has_window = snap.selected_window_idx >= 0;
    let running = snap.auto_running;
    let label = if running { "Stop" } else { "Start" };
    // Segoe Fluent filled media glyphs (`Symbol::Play` / `Stop` are outlines).
    let icon = if running {
        Icon::font("\u{EE95}") // StopSolid
    } else {
        Icon::font("\u{F5B0}") // PlaySolid
    };
    let tip = if running {
        "Stop continuous capture"
    } else if has_window {
        "Start continuous capture of the selected window"
    } else {
        "Select a window first"
    };
    let enabled = running || has_window;
    let key = match (running, pane_open) {
        (true, true) => "btn-stop-wide",
        (true, false) => "btn-stop-compact",
        (false, true) => "btn-start-wide",
        (false, false) => "btn-start-compact",
    };
    // Empty content → reactor mounts icon-only (needed in the ~48px compact pane).
    let content = if pane_open { label } else { "" };

    let mut start = button(content)
        .icon(icon)
        .tooltip(tip)
        .enabled(enabled)
        .vertical_alignment(VerticalAlignment::Center)
        .with_key(key);
    if running {
        start = start.accent();
    } else if !pane_open {
        // Same visual weight as unselected nav items (no Default fill blob).
        start = start.subtle();
    }
    start = if pane_open {
        start.horizontal_alignment(HorizontalAlignment::Stretch).margin(Thickness {
            left: 8.0,
            top: 4.0,
            right: 8.0,
            bottom: 8.0,
        })
    } else {
        start
            .min_width(40.0)
            .max_width(40.0)
            .width(40.0)
            .min_height(40.0)
            .height(40.0)
            .padding(Thickness::default())
            .horizontal_alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Center)
            .margin(Thickness {
                left: 0.0,
                top: 0.0,
                right: 0.0,
                bottom: 4.0,
            })
    };
    let start = start.on_click({
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

    let mut footer = grid((start,))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .with_key("nav-capture-footer");
    if !pane_open {
        // CompactPaneLength is 48; lock width so HorizontalAlignment::Center
        // on the button actually centers in the pane (footer otherwise sizes
        // to the button and left-aligns).
        footer = footer.width(48.0);
    }
    footer
}

/// Shared Save / Reload / Discard bar for settings pages.
///
/// Unsaved → system Accent button (same corner radius / chrome as peers).
///
/// `Button::accent()` cannot be cleared via Prop Unset (reactor no-op), so we
/// remount with a different key when dirty toggles to force a fresh Default style.
pub fn settings_actions(shared: &Arc<Mutex<UiShared>>, snap: &Snapshot, bump: &Updater<u32>) -> StackPanel {
    let cx = UiCx::new(shared, bump);
    let dirty = snap.settings_dirty;
    let has_form_error = !snap.form_error.is_empty();

    let tooltip = if has_form_error {
        format!("Cannot save: {}", snap.form_error)
    } else if dirty {
        "Unsaved changes — click to save and apply".into()
    } else {
        "Save to config.toml and apply now".into()
    };

    // Distinct keys remount the control so Accent style does not stick after Save.
    let save_key = if has_form_error {
        "btn-save-error"
    } else if dirty {
        "btn-save-dirty"
    } else {
        "btn-save"
    };

    let mut save = button("Save").tooltip(tooltip).with_key(save_key);
    if dirty || has_form_error {
        save = save.accent();
    }
    let save = save.on_click({
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
                commit_optional_fields(&mut ui);
                let cfg = ui.draft.clone();
                ui.settings_dirty = false;
                ui.form_error = None;
                // Optimistic: align live config now so dirty clears this frame
                // (pipeline ApplyConfig is async relative to the UI tick).
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
    });

    // Settings-only feedback (never routed to Dashboard InfoBar).
    let status_hint = if has_form_error {
        text_block(snap.form_error.clone())
            .font_size(12.0)
            .foreground(ThemeRef::SystemCritical)
            .with_key("settings-form-error")
    } else if !snap.settings_message.is_empty() {
        text_block(snap.settings_message.clone())
            .font_size(12.0)
            .foreground(ThemeRef::SystemSuccess)
            .with_key("settings-save-msg")
    } else {
        text_block("").font_size(12.0).with_key("settings-status-empty")
    };

    hstack((
        status_hint.vertical_alignment(VerticalAlignment::Center),
        save,
        button("Reload")
            .tooltip("Reload config.toml from disk and apply")
            .with_key("btn-reload")
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
            }),
        button("Discard")
            .tooltip("Discard edits and restore currently running settings")
            .with_key("btn-discard")
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
            }),
    ))
    .spacing(8.0)
    .with_key("settings-actions")
}

/// Sticky top bar: page title + description + Save actions (outside scroll).
pub fn settings_sticky_chrome(
    title: &str,
    description: Option<&str>,
    shared: &Arc<Mutex<UiShared>>,
    snap: &Snapshot,
    bump: &Updater<u32>,
) -> Grid {
    let title_el = text_block(title).font_size(28.0).bold();
    let head = match description {
        Some(d) if !d.is_empty() => vstack((title_el, text_block(d).font_size(13.0).foreground(ThemeRef::SecondaryText).wrap()))
            .spacing(4.0)
            .vertical_alignment(VerticalAlignment::Center),
        _ => vstack((title_el,)).vertical_alignment(VerticalAlignment::Center),
    };

    let actions = settings_actions(shared, snap, bump)
        .vertical_alignment(VerticalAlignment::Center)
        .horizontal_alignment(HorizontalAlignment::Right);

    // Title (left, grows) | Save / Reload / Discard (right), fixed above scroll.
    // Transparent — no solid bar over Mica.
    grid((
        head.grid_row(0)
            .grid_column(0)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .vertical_alignment(VerticalAlignment::Center)
            .margin(Thickness {
                left: 0.0,
                top: 0.0,
                right: 16.0,
                bottom: 0.0,
            }),
        actions.grid_row(0).grid_column(1),
    ))
    .rows([GridLength::Auto])
    .columns([GridLength::Star(1.0), GridLength::Auto])
    .padding(Thickness {
        left: 24.0,
        top: 16.0,
        right: 24.0,
        bottom: 12.0,
    })
    .horizontal_alignment(HorizontalAlignment::Stretch)
    .with_key("settings-sticky-chrome")
}

/// Confirm Reload / Discard when there are unsaved changes.
fn confirm_dialog(shared: &Arc<Mutex<UiShared>>, snap: &Snapshot, bump: &Updater<u32>) -> ContentDialog {
    let open = snap.confirm != ConfirmAction::None;
    let (title, body, primary) = match snap.confirm {
        ConfirmAction::Reload => {
            ("Reload from disk?", "Unsaved edits will be lost. The file on disk will replace the current draft.", "Reload")
        }
        ConfirmAction::Discard => ("Discard changes?", "Unsaved edits will be lost. Running settings will be restored.", "Discard"),
        ConfirmAction::None => ("", "", "OK"),
    };

    let cx = UiCx::new(shared, bump);
    ContentDialog::new(title)
        .content(body)
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
        .with_key("settings-confirm-dialog")
}

/// Standard settings page body (cards only).
///
/// Title + Save live in [`settings_sticky_chrome`] (fixed above the scroll area).
pub fn settings_page_shell(
    shared: &Arc<Mutex<UiShared>>,
    snap: &Snapshot,
    bump: &Updater<u32>,
    body: impl Into<Element> + LayoutExt,
) -> StackPanel {
    vstack((body.horizontal_alignment(HorizontalAlignment::Stretch), confirm_dialog(shared, snap, bump)))
        .spacing(12.0)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .with_key("settings-page-shell")
}
