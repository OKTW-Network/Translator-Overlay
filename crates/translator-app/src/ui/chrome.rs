//! Settings page chrome: headers, cards, InfoBar, command footer.

use std::sync::{Arc, Mutex};

use windows_reactor::*;

use super::shared::{
    ConfirmAction, Snapshot, UiShared, commit_optional_fields, do_discard, do_reload_from_disk,
    form_validation_error, is_settings_dirty,
};
use crate::pipeline::PipelineCommand;

/// Windows Settings–style page title + optional description.
pub fn page_header(title: impl Into<String>, description: Option<&str>) -> Element {
    let title = text_block(title).font_size(28.0).bold();
    match description {
        Some(d) if !d.is_empty() => vstack((
            title,
            text_block(d)
                .font_size(13.0)
                .foreground(ThemeRef::SecondaryText)
                .wrap(),
        ))
        .spacing(4.0)
        .into(),
        _ => title.into(),
    }
}

/// Section label above a group of cards.
pub fn section_header(title: impl Into<String>) -> Element {
    text_block(title)
        .font_size(14.0)
        .semibold()
        .margin(Thickness {
            left: 0.0,
            top: 12.0,
            right: 0.0,
            bottom: 4.0,
        })
        .into()
}

/// Collapsible settings group (WinUI `Expander`, SettingsExpander-style).
///
/// Community Toolkit pattern: one outer chrome for the group; put flat
/// [`settings_row`] items inside — not nested [`settings_card`]s.
///
/// `expanded` must come from UI state and be updated in `on_expanding` so the
/// panel stays open across re-renders.
pub fn settings_expander(
    key: &str,
    header: impl Into<String>,
    expanded: bool,
    on_expanding: impl Fn(bool) + 'static,
    child: impl Into<Element>,
) -> Element {
    // Content is a list of flat rows; no extra margin that would look like a
    // nested card inset.
    let body = child
        .into()
        .horizontal_alignment(HorizontalAlignment::Stretch);

    Expander::new(body)
        .header(header)
        .expanded(expanded)
        .on_expanding(on_expanding)
        .with_key(key)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .margin(Thickness {
            left: 0.0,
            top: 4.0,
            right: 0.0,
            bottom: 0.0,
        })
        .into()
}

/// Shared label + control layout for settings rows (card or flat).
fn settings_row_body(
    header: impl Into<String>,
    description: Option<&str>,
    control: impl Into<Element>,
) -> Element {
    let header_el = text_block(header).semibold().font_size(14.0);
    let left: Element = match description {
        Some(d) if !d.is_empty() => vstack((
            header_el,
            text_block(d)
                .font_size(12.0)
                .foreground(ThemeRef::SecondaryText)
                .wrap(),
        ))
        .spacing(2.0)
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .vertical_alignment(VerticalAlignment::Center)
        .into(),
        _ => header_el
            .vertical_alignment(VerticalAlignment::Center)
            .into(),
    };

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
            .into()
            .grid_row(0)
            .grid_column(1)
            .horizontal_alignment(HorizontalAlignment::Right)
            .vertical_alignment(VerticalAlignment::Center),
    ))
    .rows([GridLength::Auto])
    .columns([GridLength::Star(1.0), GridLength::Auto])
    .horizontal_alignment(HorizontalAlignment::Stretch)
    .into()
}

/// Flat settings row for use **inside** an [`settings_expander`].
///
/// Matches Community Toolkit `SettingsCard` hosted in `SettingsExpander.Items`:
/// same header/control layout as a card, but no `CardBackground` / corner radius
/// (those belong to the outer expander only). A hairline bottom border separates
/// items like the toolkit item style.
pub fn settings_row(
    key: &str,
    header: impl Into<String>,
    description: Option<&str>,
    control: impl Into<Element>,
) -> Element {
    let body = settings_row_body(header, description, control);
    border(body)
        .border_thickness(Thickness {
            left: 0.0,
            top: 0.0,
            right: 0.0,
            bottom: 1.0,
        })
        .border_brush(ThemeRef::CardStroke)
        .padding(Thickness {
            left: 16.0,
            top: 12.0,
            right: 16.0,
            bottom: 12.0,
        })
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .with_key(key)
        .into()
}

/// Standalone settings card: Header + Description on the left, control flush-right.
///
/// Uses Grid `Star | Auto` so the control column stays right-aligned regardless
/// of label/description length (HStack would pack after the text width).
///
/// Do **not** nest these inside an Expander — use [`settings_row`] instead.
pub fn settings_card(
    key: &str,
    header: impl Into<String>,
    description: Option<&str>,
    control: impl Into<Element>,
) -> Element {
    border(settings_row_body(header, description, control))
        .background(ThemeRef::CardBackground)
        .corner_radius(8.0)
        .padding(Thickness::uniform(16.0))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .with_key(key)
        .into()
}

/// Card with header/description on top and full-width content below (sliders, multiline).
pub fn settings_card_stack(
    key: &str,
    header: impl Into<String>,
    description: Option<&str>,
    content: impl Into<Element>,
) -> Element {
    let header_el = text_block(header).semibold().font_size(14.0);
    let head: Element = match description {
        Some(d) if !d.is_empty() => vstack((
            header_el,
            text_block(d)
                .font_size(12.0)
                .foreground(ThemeRef::SecondaryText)
                .wrap(),
        ))
        .spacing(2.0)
        .into(),
        _ => header_el.into(),
    };

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
    .into()
}

/// Pipeline error InfoBar for the Dashboard only.
///
/// Settings save success lives on the settings chrome (next to Save), not here.
/// Always mounts the same `InfoBar` (stable key) so open/close does not remount
/// the rest of the page tree.
pub fn status_infobar(snap: &Snapshot) -> Element {
    let (title, open) = if !snap.last_error.is_empty() {
        (snap.last_error.as_str(), true)
    } else {
        ("", false)
    };

    InfoBar::new(title)
        .message("")
        .severity(InfoBarSeverity::Error)
        .is_open(open)
        .is_closable(false)
        .with_key("status-infobar")
        .into()
}

/// Compact app status strip (not full runtime dump).
pub fn app_status_strip(snap: &Snapshot) -> Element {
    let ocr_time = snap
        .last_ocr_ms
        .map(|ms| format!("{ms} ms"))
        .unwrap_or_else(|| "—".into());
    text_block(format!(
        "{}  ·  {}  ·  OCR {ocr_time}  ·  API key {}",
        snap.status,
        snap.target,
        if snap.api_ready { "ready" } else { "missing" }
    ))
    .font_size(12.0)
    .foreground(ThemeRef::SecondaryText)
    .into()
}

/// Shared Save / Reload / Discard bar for settings pages.
///
/// Unsaved → system Accent button (same corner radius / chrome as peers).
///
/// `Button::accent()` cannot be cleared via Prop Unset (reactor no-op), so we
/// remount with a different key when dirty toggles to force a fresh Default style.
pub fn settings_actions(
    shared: &Arc<Mutex<UiShared>>,
    snap: &Snapshot,
    bump: &Updater<u32>,
) -> Element {
    let s_save = Arc::clone(shared);
    let s_reload = Arc::clone(shared);
    let bump_save = bump.clone();
    let bump_reload = bump.clone();
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
    let save = save.on_click(move || {
        if let Ok(mut ui) = s_save.lock() {
            if let Some(err) = form_validation_error(&ui) {
                ui.form_error = Some(err);
                bump_save.call(|n| n.wrapping_add(1));
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
        bump_save.call(|n| n.wrapping_add(1));
    });

    // Settings-only feedback (never routed to Dashboard InfoBar).
    let status_hint: Element = if has_form_error {
        text_block(snap.form_error.clone())
            .font_size(12.0)
            .foreground(ThemeRef::SystemCritical)
            .with_key("settings-form-error")
            .into()
    } else if !snap.settings_message.is_empty() {
        text_block(snap.settings_message.clone())
            .font_size(12.0)
            .foreground(ThemeRef::SystemSuccess)
            .with_key("settings-save-msg")
            .into()
    } else {
        text_block("")
            .font_size(12.0)
            .with_key("settings-status-empty")
            .into()
    };

    hstack((
        status_hint.vertical_alignment(VerticalAlignment::Center),
        save,
        button("Reload")
            .tooltip("Reload config.toml from disk and apply")
            .with_key("btn-reload")
            .on_click(move || {
                if let Ok(mut ui) = s_reload.lock() {
                    if is_settings_dirty(&ui) {
                        ui.confirm = ConfirmAction::Reload;
                    } else {
                        do_reload_from_disk(&mut ui);
                    }
                }
                bump_reload.call(|n| n.wrapping_add(1));
            }),
        button("Discard")
            .tooltip("Discard edits and restore currently running settings")
            .with_key("btn-discard")
            .on_click({
                let s = Arc::clone(shared);
                let bump = bump.clone();
                move || {
                    if let Ok(mut ui) = s.lock() {
                        if is_settings_dirty(&ui) {
                            ui.confirm = ConfirmAction::Discard;
                        } else {
                            do_discard(&mut ui);
                        }
                    }
                    bump.call(|n| n.wrapping_add(1));
                }
            }),
    ))
    .spacing(8.0)
    .with_key("settings-actions")
    .into()
}

/// Sticky top bar: page title + description + Save actions (outside scroll).
pub fn settings_sticky_chrome(
    title: &str,
    description: Option<&str>,
    shared: &Arc<Mutex<UiShared>>,
    snap: &Snapshot,
    bump: &Updater<u32>,
) -> Element {
    let title_el = text_block(title).font_size(28.0).bold();
    let head: Element = match description {
        Some(d) if !d.is_empty() => vstack((
            title_el,
            text_block(d)
                .font_size(13.0)
                .foreground(ThemeRef::SecondaryText)
                .wrap(),
        ))
        .spacing(4.0)
        .vertical_alignment(VerticalAlignment::Center)
        .into(),
        _ => title_el
            .vertical_alignment(VerticalAlignment::Center)
            .into(),
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
    .into()
}

/// Confirm Reload / Discard when there are unsaved changes.
fn confirm_dialog(shared: &Arc<Mutex<UiShared>>, snap: &Snapshot, bump: &Updater<u32>) -> Element {
    let open = snap.confirm != ConfirmAction::None;
    let (title, body, primary) = match snap.confirm {
        ConfirmAction::Reload => (
            "Reload from disk?",
            "Unsaved edits will be lost. The file on disk will replace the current draft.",
            "Reload",
        ),
        ConfirmAction::Discard => (
            "Discard changes?",
            "Unsaved edits will be lost. Running settings will be restored.",
            "Discard",
        ),
        ConfirmAction::None => ("", "", "OK"),
    };

    let s = Arc::clone(shared);
    let bump_c = bump.clone();
    ContentDialog::new(title)
        .content(body)
        .primary_button_text(primary)
        .close_button_text("Cancel")
        .is_open(open)
        .on_closed(move |result: ContentDialogResult| {
            if let Ok(mut ui) = s.lock() {
                let action = ui.confirm;
                ui.confirm = ConfirmAction::None;
                if result == ContentDialogResult::Primary {
                    match action {
                        ConfirmAction::Reload => do_reload_from_disk(&mut ui),
                        ConfirmAction::Discard => do_discard(&mut ui),
                        ConfirmAction::None => {}
                    }
                }
            }
            bump_c.call(|n| n.wrapping_add(1));
        })
        .with_key("settings-confirm-dialog")
        .into()
}

/// Standard settings page body (cards only).
///
/// Title + Save live in [`settings_sticky_chrome`] (fixed above the scroll area).
pub fn settings_page_shell(
    shared: &Arc<Mutex<UiShared>>,
    snap: &Snapshot,
    bump: &Updater<u32>,
    body: Element,
) -> Element {
    vstack((
        body.horizontal_alignment(HorizontalAlignment::Stretch),
        confirm_dialog(shared, snap, bump),
    ))
    .spacing(12.0)
    .horizontal_alignment(HorizontalAlignment::Stretch)
    .with_key("settings-page-shell")
    .into()
}
