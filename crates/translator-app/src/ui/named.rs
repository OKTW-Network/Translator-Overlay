//! Combo, name row, delete button, and confirm dialog shared by region presets and API profiles.

use std::borrow::Cow;

use rust_i18n::t;
use windows_reactor::{
    AutomationExt, Border, Button, ButtonStyle, ChildrenControl, ComboBox, ContentControl, ContentDialog, ContentDialogResult, FontWeight,
    HorizontalAlignment, LayoutControl, Orientation, StackPanel, TextBlock, TextBox, TextWrapping, ThemeBrush, Thickness, TooltipExt,
    VerticalAlignment, View,
};

use crate::ui::shared::{NamedDialog, NamedPick, NamedStore, UiCx, UiShared, commit_named};

/// One render's copy of a [`NamedPick`] and the item names.
pub struct NamedSnap {
    pub names: Vec<String>,
    pub pick: NamedPick,
}

impl NamedSnap {
    pub fn take(ui: &mut UiShared, store: &NamedStore) -> Self {
        Self {
            names: (store.names)(ui),
            pick: (store.pick)(ui).clone(),
        }
    }

    /// The selected index, if it still points at an item.
    pub fn selected(&self) -> Option<usize> {
        self.pick.selected.filter(|&i| i < self.names.len())
    }
}

pub fn named_combo(cx: &UiCx, store: &'static NamedStore, snap: &NamedSnap, placeholder: Cow<'static, str>) -> ComboBox {
    ComboBox::new()
        .items_source(snap.names.clone())
        .selected_index(snap.selected())
        .placeholder_text(placeholder)
        .is_enabled(!snap.names.is_empty())
        .on_selection_changed({
            let cx = cx.clone();
            move |idx: Option<usize>| {
                cx.with_mut(|ui| {
                    let count = (store.names)(ui).len();
                    (store.pick)(ui).selected = idx.filter(|&i| i < count);
                });
            }
        })
        .width(180.0)
        .min_width(140.0)
        .vertical_alignment(VerticalAlignment::Center)
}

/// Asks to delete the selected item through the confirm dialog.
pub fn named_delete_button(cx: &UiCx, store: &'static NamedStore, snap: &NamedSnap, tooltip: Cow<'static, str>) -> View {
    Button::new()
        .is_enabled(snap.selected().is_some())
        .on_click({
            let cx = cx.clone();
            move || {
                cx.with_mut(|ui| {
                    let names = (store.names)(ui);
                    let pick = (store.pick)(ui);
                    if let Some(name) = pick.selected.and_then(|i| names.get(i)).cloned() {
                        pick.dialog = NamedDialog::Delete { name };
                    }
                });
            }
        })
        .content(t!("action.delete").as_ref())
        .tooltip(tooltip)
}

/// Inline "Name: [....] Save Cancel" row, shown while the pick is in [`NamedDialog::SaveName`].
pub fn name_entry_row(cx: &UiCx, store: &'static NamedStore, snap: &NamedSnap) -> View {
    if snap.pick.dialog != NamedDialog::SaveName {
        return View::empty();
    }

    let name_tb = TextBox::new()
        .text(snap.pick.name_draft.clone())
        .on_text_changed({
            let cx = cx.clone();
            move |text: String| {
                cx.with_mut(|ui| (store.pick)(ui).name_draft = text);
            }
        })
        .automation_id("txt-named-name")
        .width(180.0)
        .vertical_alignment(VerticalAlignment::Center);

    let save = Button::new()
        .style(ButtonStyle::Accent)
        .automation_id("btn-named-save")
        .on_click({
            let cx = cx.clone();
            move || {
                cx.with_mut(|ui| {
                    let name = (store.pick)(ui).name_draft.trim().to_string();
                    if name.is_empty() {
                        ui.state.write().set_error(t!(store.name_required));
                    } else if (store.names)(ui).contains(&name) {
                        (store.pick)(ui).dialog = NamedDialog::Overwrite { name };
                    } else {
                        commit_named(ui, store, name);
                    }
                });
            }
        })
        .content(t!("action.save").as_ref());

    let cancel = Button::new()
        .on_click({
            let cx = cx.clone();
            move || {
                cx.with_mut(|ui| {
                    let pick = (store.pick)(ui);
                    pick.dialog = NamedDialog::None;
                    pick.name_draft.clear();
                    (store.on_cancel)(ui);
                });
            }
        })
        .content(t!("action.cancel").as_ref());

    Border::new()
        .background(ThemeBrush::CardBackground)
        .border_brush(ThemeBrush::CardStroke)
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(4.0)
        .padding(Thickness::new(8.0, 4.0, 8.0, 4.0))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .content(
            StackPanel::new().orientation(Orientation::Horizontal).spacing(8.0).children((
                TextBlock::new()
                    .text(t!("action.name"))
                    .font_weight(FontWeight::SEMI_BOLD)
                    .vertical_alignment(VerticalAlignment::Center),
                name_tb,
                save,
                cancel,
            )),
        )
}

fn resolve_named_dialog(ui: &mut UiShared, store: &NamedStore, action: NamedDialog, primary: bool) {
    if !primary {
        if matches!(action, NamedDialog::Overwrite { .. }) {
            (store.pick)(ui).dialog = NamedDialog::SaveName;
        }
        return;
    }
    match action {
        NamedDialog::Overwrite { name } => commit_named(ui, store, name),
        NamedDialog::Delete { name } => {
            {
                let pick = (store.pick)(ui);
                pick.selected = None;
                pick.name_draft.clear();
            }
            if let Err(e) = (store.delete)(ui, &name) {
                ui.state.write().set_error(e);
            }
        }
        NamedDialog::None | NamedDialog::SaveName => {}
    }
}

/// Overwrite or delete confirmation. Cancelling an overwrite returns to the name row.
pub fn named_confirm_dialog(cx: &UiCx, store: &'static NamedStore, snap: &NamedSnap) -> View {
    let (open, title, body, primary) = match &snap.pick.dialog {
        NamedDialog::Overwrite { name } => (true, t!(store.overwrite_title), t!(store.overwrite_body, name = name), t!("action.overwrite")),
        NamedDialog::Delete { name } => (true, t!(store.delete_title), t!(store.delete_body, name = name), t!("action.delete")),
        NamedDialog::None | NamedDialog::SaveName => (false, Cow::Borrowed(""), Cow::Borrowed(""), t!("action.ok")),
    };

    ContentDialog::new()
        .title(title)
        .primary_button_text(primary)
        .close_button_text(t!("action.cancel"))
        .is_open(open)
        .on_closed({
            let cx = cx.clone();
            move |result: ContentDialogResult| {
                cx.with_mut(|ui| {
                    let action = std::mem::take(&mut (store.pick)(ui).dialog);
                    resolve_named_dialog(ui, store, action, result == ContentDialogResult::Primary);
                });
            }
        })
        .content(body.as_ref())
}

/// Same overwrite/delete confirm as [`named_confirm_dialog`], as a card. Use this inside another ContentDialog.
pub fn named_confirm_inline(cx: &UiCx, store: &'static NamedStore, snap: &NamedSnap) -> View {
    let (title, body, primary) = match &snap.pick.dialog {
        NamedDialog::Overwrite { name } => (t!(store.overwrite_title), t!(store.overwrite_body, name = name), t!("action.overwrite")),
        NamedDialog::Delete { name } => (t!(store.delete_title), t!(store.delete_body, name = name), t!("action.delete")),
        NamedDialog::None | NamedDialog::SaveName => return View::empty(),
    };

    confirm_strip(
        title,
        body,
        primary,
        {
            let cx = cx.clone();
            move || {
                cx.with_mut(|ui| {
                    let action = std::mem::take(&mut (store.pick)(ui).dialog);
                    resolve_named_dialog(ui, store, action, true);
                });
            }
        },
        {
            let cx = cx.clone();
            move || {
                cx.with_mut(|ui| {
                    let action = std::mem::take(&mut (store.pick)(ui).dialog);
                    resolve_named_dialog(ui, store, action, false);
                });
            }
        },
    )
}

pub(crate) fn confirm_strip(
    title: Cow<'static, str>,
    body: Cow<'static, str>,
    primary: Cow<'static, str>,
    on_primary: impl Fn() + 'static,
    on_cancel: impl Fn() + 'static,
) -> View {
    let primary_btn = Button::new()
        .style(ButtonStyle::Accent)
        .on_click(on_primary)
        .content(primary.as_ref());
    let cancel_btn = Button::new().on_click(on_cancel).content(t!("action.cancel").as_ref());

    Border::new()
        .background(ThemeBrush::CardBackground)
        .border_brush(ThemeBrush::CardStroke)
        .border_thickness(Thickness::uniform(1.0))
        .corner_radius(4.0)
        .padding(Thickness::new(8.0, 8.0, 8.0, 8.0))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .content(
            StackPanel::new().spacing(8.0).children((
                TextBlock::new().text(title).font_weight(FontWeight::SEMI_BOLD),
                TextBlock::new().text(body).text_wrapping(TextWrapping::Wrap),
                StackPanel::new()
                    .orientation(Orientation::Horizontal)
                    .spacing(8.0)
                    .horizontal_alignment(HorizontalAlignment::Right)
                    .children((primary_btn, cancel_btn)),
            )),
        )
}
