//! Hotkey settings: master switch, focus gate, and five unbound shortcuts.

use std::borrow::Cow;

use rust_i18n::t;
use translator_core::HotkeyAction;
use translator_winui::{Border, Button, Layout, Result, StackPanel, TextWrapping};

use crate::{
    children, hotkey,
    ui::{
        state::{UiCx, UiShared, hotkey_action_label, mark_dirty, stop_hotkey_edit},
        widgets::{
            Binds, ButtonKind, Prop, Theme, button_style,
            cards::{settings_card, settings_card_with_below, subsection_header},
            click, column,
            controls::{card_toggle, compact_toggle},
            foreground, hs, icon_button, row, text, ui,
        },
    },
};

pub fn hotkeys_page(b: &mut Binds<UiShared>, cx: &UiCx) -> Result<StackPanel> {
    let enabled_card =
        card_toggle(b, ("hk-enabled", &t!("hotkeys.enabled"), Some(&t!("hotkeys.enabled_desc"))), |ui| ui.draft.hotkeys.enabled, {
            let cx = cx.clone();
            move |on| {
                cx.with_mut(|ui| {
                    if ui.draft.hotkeys.enabled != on {
                        ui.draft.hotkeys.enabled = on;
                        if !on {
                            stop_hotkey_edit(ui);
                        }
                        mark_dirty(ui);
                    }
                });
            }
        })?;
    let mut only_target = compact_toggle({
        let cx = cx.clone();
        move |on| {
            cx.with_mut(|ui| {
                if ui.draft.hotkeys.only_when_target_focused != on {
                    ui.draft.hotkeys.only_when_target_focused = on;
                    mark_dirty(ui);
                }
            });
        }
    })?;
    let only_target_card =
        settings_card("hk-only-target", &t!("hotkeys.only_target"), Some(&t!("hotkeys.only_target_desc")), &only_target.ts)?;
    b.add(move |ui, _| only_target.set(ui.draft.hotkeys.only_when_target_focused, ui.draft.hotkeys.enabled));
    let toggles = column(4.0)?;
    children!(toggles; enabled_card, only_target_card.root);

    let rows = column(4.0)?;
    children!(rows; subsection_header(&t!("hotkeys.bindings"))?);
    for action in [
        HotkeyAction::StartPause,
        HotkeyAction::Stop,
        HotkeyAction::SelectRegion,
        HotkeyAction::Retry,
        HotkeyAction::ClearHistory,
    ] {
        children!(rows; bind_row(b, cx, action)?);
    }

    let body = column(12.0)?;
    children!(body; toggles, rows);
    Ok(body)
}

fn bind_row(b: &mut Binds<UiShared>, cx: &UiCx, action: HotkeyAction) -> Result<Border> {
    let label = text("")?;
    let bind = Button::new()?;
    bind.min_width(180.0)?.width(180.0)?;
    bind.SetContent(&label)?;
    click(&bind, {
        let cx = cx.clone();
        move || {
            cx.with_mut(|ui| {
                if ui.hotkey_listen == Some(action) {
                    ui.hotkey_listen = None;
                    hotkey::cancel_listen();
                    return;
                }
                ui.hotkey_error = None;
                ui.hotkey_listen_gen = hotkey::begin_listen(Some(action));
                ui.hotkey_listen = Some(action);
            });
        }
    })?;

    let unbind = icon_button("\u{E74D}", {
        let cx = cx.clone();
        move || {
            cx.with_mut(|ui| {
                if ui.draft.hotkeys.get(action).is_some() {
                    ui.draft.hotkeys.set(action, None);
                    mark_dirty(ui);
                }
                if ui.hotkey_listen == Some(action) {
                    ui.hotkey_listen = None;
                    hotkey::cancel_listen();
                }
                if ui.hotkey_error.as_ref().is_some_and(|(row, _)| *row == action) {
                    ui.hotkey_error = None;
                }
            });
        }
    })?;
    unbind.tooltip(&t!("hotkeys.unbind"))?;

    let controls = row(8.0)?;
    children!(controls; bind, unbind);
    let error = text("")?;
    error.SetFontSize(12.0)?;
    foreground(&error, Theme::SystemCritical)?;
    error.SetTextWrapping(TextWrapping::WrapWholeWords)?;
    error.visible(false)?;
    let description = match action {
        HotkeyAction::StartPause => t!("hotkeys.start_pause_desc"),
        HotkeyAction::Stop => t!("hotkeys.stop_desc"),
        HotkeyAction::SelectRegion => t!("hotkeys.select_region_desc"),
        HotkeyAction::Retry => t!("hotkeys.retry_desc"),
        HotkeyAction::ClearHistory => t!("dash.clear_chat_tip"),
    };
    let key = match action {
        HotkeyAction::StartPause => "hotkey-start-pause",
        HotkeyAction::Stop => "hotkey-stop",
        HotkeyAction::SelectRegion => "hotkey-select-region",
        HotkeyAction::Retry => "hotkey-retry",
        HotkeyAction::ClearHistory => "hotkey-clear-history",
    };
    let card = settings_card_with_below(key, &hotkey_action_label(action), Some(&description), &controls, Some(&ui(&error)?))?;

    let mut shown = Prop::default();
    b.add(move |ui, _| {
        let enabled = ui.draft.hotkeys.enabled;
        let chord = ui.draft.hotkeys.get(action);
        let listening = enabled && ui.hotkey_listen == Some(action);
        let text = if listening {
            hotkey::binding_preview().map(Cow::Owned).unwrap_or_else(|| t!("hotkeys.press_key"))
        } else if let Some(chord) = chord {
            Cow::Owned(chord.label())
        } else {
            t!("hotkeys.not_set")
        };
        let row_error = ui
            .hotkey_error
            .as_ref()
            .filter(|(row, _)| *row == action)
            .map(|(_, text)| text.clone());
        shown.set((text.into_owned(), listening, enabled, chord.is_some(), row_error), |(text, listening, enabled, bound, row_error)| {
            label.SetText(&hs(text))?;
            button_style(&bind, if *listening { ButtonKind::Accent } else { ButtonKind::Default })?;
            bind.SetIsEnabled(*enabled)?;
            unbind.SetIsEnabled(*enabled && *bound)?;
            error.SetText(&hs(row_error.as_deref().unwrap_or_default()))?;
            error.visible(row_error.is_some())?;
            Ok(())
        })
    });
    Ok(card.root)
}
