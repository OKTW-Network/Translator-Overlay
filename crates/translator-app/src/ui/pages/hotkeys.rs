//! Hotkey settings: master switch, focus gate, and five unbound shortcuts.

use std::{borrow::Cow, sync::Arc};

use parking_lot::Mutex;
use rust_i18n::t;
use translator_core::{HotkeyAction, HotkeyConfig};
use windows_reactor::{
    Button, ButtonStyle, ChildrenControl, ContentControl, FontIcon, HorizontalAlignment, LayoutControl, LocalSender, Orientation,
    StackPanel, TextBlock, TextWrapping, ThemeBrush, TooltipExt, VerticalAlignment, View,
};

use crate::{
    hotkey,
    ui::{
        cards::{settings_card, settings_card_with_below, subsection_header},
        chrome::settings_page_shell,
        controls::{card_toggle, compact_toggle},
        shared::{AppMsg, ChromeSnap, UiCx, UiShared, hotkey_action_label, mark_dirty, stop_hotkey_edit},
    },
};

pub fn hotkeys_page(shared: &Arc<Mutex<UiShared>>, chrome: &ChromeSnap, bump: &LocalSender<AppMsg>) -> View {
    let cx = UiCx::new(shared, bump);
    let (hotkeys, listen, error) = {
        let ui = shared.lock();
        (ui.draft.hotkeys.clone(), ui.hotkey_listen, ui.hotkey_error.clone())
    };

    let enabled = hotkeys.enabled;
    let toggles = StackPanel::new().spacing(4.0).children((
        card_toggle("hk-enabled", t!("hotkeys.enabled"), Some(&t!("hotkeys.enabled_desc")), enabled, {
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
        }),
        settings_card(
            "hk-only-target",
            t!("hotkeys.only_target"),
            Some(&t!("hotkeys.only_target_desc")),
            compact_toggle(hotkeys.only_when_target_focused, enabled, {
                let cx = cx.clone();
                move |on| {
                    cx.with_mut(|ui| {
                        if ui.draft.hotkeys.only_when_target_focused != on {
                            ui.draft.hotkeys.only_when_target_focused = on;
                            mark_dirty(ui);
                        }
                    });
                }
            }),
        ),
    ));

    let rows = StackPanel::new().spacing(4.0).children((
        subsection_header(t!("hotkeys.bindings")),
        bind_row(&cx, HotkeyAction::StartPause, &hotkeys, listen, error.as_ref(), enabled),
        bind_row(&cx, HotkeyAction::Stop, &hotkeys, listen, error.as_ref(), enabled),
        bind_row(&cx, HotkeyAction::SelectRegion, &hotkeys, listen, error.as_ref(), enabled),
        bind_row(&cx, HotkeyAction::Retry, &hotkeys, listen, error.as_ref(), enabled),
        bind_row(&cx, HotkeyAction::ClearHistory, &hotkeys, listen, error.as_ref(), enabled),
    ));

    settings_page_shell(shared, chrome, bump, StackPanel::new().spacing(12.0).children((toggles, rows)))
}

fn bind_row(
    cx: &UiCx,
    action: HotkeyAction,
    hotkeys: &HotkeyConfig,
    listen: Option<HotkeyAction>,
    error: Option<&(HotkeyAction, String)>,
    enabled: bool,
) -> View {
    let chord = hotkeys.get(action);
    let listening = enabled && listen == Some(action);
    let row_error = error.filter(|(row, _)| *row == action).map(|(_, text)| text.as_str());
    let label = if listening {
        hotkey::binding_preview().map(Cow::Owned).unwrap_or_else(|| t!("hotkeys.press_key"))
    } else if let Some(chord) = chord {
        Cow::Owned(chord.label())
    } else {
        t!("hotkeys.not_set")
    };

    let mut bind = Button::new()
        .min_width(180.0)
        .width(180.0)
        .is_enabled(enabled)
        .horizontal_content_alignment(HorizontalAlignment::Center)
        .vertical_alignment(VerticalAlignment::Center);
    if listening {
        bind = bind.style(ButtonStyle::Accent);
    }
    let bind = bind
        .on_click({
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
        })
        .content(label.as_ref());

    let unbind = Button::new()
        .style(ButtonStyle::Subtle)
        .min_width(36.0)
        .min_height(36.0)
        .horizontal_content_alignment(HorizontalAlignment::Center)
        .vertical_content_alignment(VerticalAlignment::Center)
        .vertical_alignment(VerticalAlignment::Center)
        .is_enabled(enabled && chord.is_some())
        .on_click({
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
        })
        .content(FontIcon::new().glyph("\u{E74D}"))
        .tooltip(t!("hotkeys.unbind"));

    let controls = StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(8.0)
        .children((bind, unbind));

    let below = row_error.map(|text| -> View {
        TextBlock::new()
            .text(text)
            .font_size(12.0)
            .foreground(ThemeBrush::SystemCritical)
            .text_wrapping(TextWrapping::WrapWholeWords)
            .into()
    });
    let description = match action {
        HotkeyAction::StartPause => t!("hotkeys.start_pause_desc"),
        HotkeyAction::Stop => t!("hotkeys.stop_desc"),
        HotkeyAction::SelectRegion => t!("hotkeys.select_region_desc"),
        HotkeyAction::Retry => t!("hotkeys.retry_desc"),
        HotkeyAction::ClearHistory => t!("dash.clear_chat_tip"),
    };

    settings_card_with_below(
        match action {
            HotkeyAction::StartPause => "hotkey-start-pause",
            HotkeyAction::Stop => "hotkey-stop",
            HotkeyAction::SelectRegion => "hotkey-select-region",
            HotkeyAction::Retry => "hotkey-retry",
            HotkeyAction::ClearHistory => "hotkey-clear-history",
        },
        hotkey_action_label(action),
        Some(description.as_ref()),
        controls,
        below,
    )
}
