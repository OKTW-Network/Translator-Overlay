//! Combo, name row, delete button, and confirm dialog shared by region presets, glossaries, and
//! API profiles.

use std::borrow::Cow;

use rust_i18n::t;
use translator_winui::{
    Border, Button, ComboBox, HorizontalAlignment, Layout, Result, TextBlock, TextWrapping, Thickness, VerticalAlignment, corners,
    thickness, uniform,
};

use crate::{
    children,
    ui::{
        state::{NamedDialog, NamedPick, NamedStore, UiCx, UiShared, commit_named},
        widgets::{
            Binds, ButtonKind, Prop, Theme, button, button_style,
            cards::SEMI_BOLD,
            click, column,
            dialog::Dialog,
            fields::{Combo, TextField},
            hs, row, text, themed_border,
        },
    },
};

/// One sync's copy of a [`NamedPick`] and the item names.
pub struct NamedSnap {
    pub names: Vec<String>,
    pub pick: NamedPick,
}

impl NamedSnap {
    pub fn take(ui: &UiShared, store: &NamedStore) -> Self {
        Self {
            names: (store.names)(ui),
            pick: (store.view)(ui).clone(),
        }
    }

    /// The selected index, if it still points at an item.
    pub fn selected(&self) -> Option<usize> {
        self.pick.selected.filter(|&i| i < self.names.len())
    }
}

pub fn named_combo(b: &mut Binds<UiShared>, cx: &UiCx, store: &'static NamedStore, placeholder: &str) -> Result<ComboBox> {
    let cx = cx.clone();
    let mut combo = Combo::new(placeholder, move |idx| {
        cx.with_mut(|ui| {
            let count = (store.names)(ui).len();
            (store.pick)(ui).selected = idx.filter(|&i| i < count);
        });
    })?;
    combo.cb.width(180.0)?.min_width(140.0)?.valign(VerticalAlignment::Center)?;
    let el = combo.cb.clone();
    b.add(move |ui, _| {
        let snap = NamedSnap::take(ui, store);
        let enabled = !snap.names.is_empty();
        let selected = snap.selected();
        combo.set(snap.names, selected, enabled)
    });
    Ok(el)
}

/// Asks to delete the selected item through the confirm dialog.
pub fn named_delete_button(b: &mut Binds<UiShared>, cx: &UiCx, store: &'static NamedStore, tooltip: &str) -> Result<Button> {
    let cx = cx.clone();
    let delete = button(&t!("action.delete"), move || {
        cx.with_mut(|ui| {
            let names = (store.names)(ui);
            let pick = (store.pick)(ui);
            if let Some(name) = pick.selected.and_then(|i| names.get(i)).cloned() {
                pick.dialog = NamedDialog::Delete { name };
            }
        });
    })?;
    delete.tooltip(tooltip)?;
    let el = delete.clone();
    let mut enabled = Prop::default();
    b.add(move |ui, _| enabled.set(NamedSnap::take(ui, store).selected().is_some(), |&e| el.SetIsEnabled(e)));
    Ok(delete)
}

/// Inline "Name: [....] Save Cancel" row, shown while the pick is in [`NamedDialog::SaveName`].
pub fn name_entry_row(b: &mut Binds<UiShared>, cx: &UiCx, store: &'static NamedStore) -> Result<Border> {
    let name_tb = TextField::new("", {
        let cx = cx.clone();
        move |text| cx.with_mut(|ui| (store.pick)(ui).name_draft = text)
    })?;
    name_tb
        .tb
        .automation_id("txt-named-name")?
        .width(180.0)?
        .valign(VerticalAlignment::Center)?;

    let save = button(&t!("action.save"), {
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
    })?;
    button_style(&save, ButtonKind::Accent)?;
    save.automation_id("btn-named-save")?;

    let cancel = button(&t!("action.cancel"), {
        let cx = cx.clone();
        move || {
            cx.with_mut(|ui| {
                let pick = (store.pick)(ui);
                pick.dialog = NamedDialog::None;
                pick.name_draft.clear();
                (store.on_cancel)(ui);
            });
        }
    })?;

    let label = text(&t!("action.name"))?;
    label.SetFontWeight(SEMI_BOLD)?;
    label.valign(VerticalAlignment::Center)?;
    let line = row(8.0)?;
    children!(line; label, name_tb.tb, save, cancel);
    let root = outlined(thickness(8.0, 4.0, 8.0, 4.0))?;
    root.SetChild(&line)?;

    let el = root.clone();
    let mut shown = Prop::default();
    b.add(move |ui, _| {
        let pick = (store.view)(ui);
        let open = pick.dialog == NamedDialog::SaveName;
        if open {
            name_tb.set(&pick.name_draft)?;
        }
        shown.set(open, |&o| el.visible(o).map(drop))
    });
    Ok(root)
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

/// Title, body, and primary label for an overwrite / delete confirmation, if one is pending.
fn confirm_texts(store: &NamedStore, dialog: &NamedDialog) -> Option<(Cow<'static, str>, Cow<'static, str>, Cow<'static, str>)> {
    match dialog {
        NamedDialog::Overwrite { name } => Some((t!(store.overwrite_title), t!(store.overwrite_body, name = name), t!("action.overwrite"))),
        NamedDialog::Delete { name } => Some((t!(store.delete_title), t!(store.delete_body, name = name), t!("action.delete"))),
        NamedDialog::None | NamedDialog::SaveName => None,
    }
}

/// Overwrite or delete confirmation. Cancelling an overwrite returns to the name row.
pub fn named_confirm_dialog(b: &mut Binds<UiShared>, cx: &UiCx, store: &'static NamedStore) -> Result<()> {
    let cx = cx.clone();
    let mut dialog = Dialog::new(move |primary| {
        cx.with_mut(|ui| {
            let action = std::mem::take(&mut (store.pick)(ui).dialog);
            resolve_named_dialog(ui, store, action, primary);
        });
    })?;
    b.add(move |ui, _| {
        let texts = confirm_texts(store, &(store.view)(ui).dialog);
        if let Some((title, body, primary)) = &texts {
            dialog.labels(title, primary, &t!("action.cancel"))?;
            dialog.body(body)?;
        }
        dialog.show(texts.is_some())
    });
    Ok(())
}

/// Same overwrite/delete confirm as [`named_confirm_dialog`], as a card. Use this inside another ContentDialog.
pub fn named_confirm_inline(b: &mut Binds<UiShared>, cx: &UiCx, store: &'static NamedStore) -> Result<Border> {
    let resolve = |primary: bool| {
        let cx = cx.clone();
        move || {
            cx.with_mut(|ui| {
                let action = std::mem::take(&mut (store.pick)(ui).dialog);
                resolve_named_dialog(ui, store, action, primary);
            });
        }
    };
    let primary_label = text("")?;
    let primary = Button::new()?;
    button_style(&primary, ButtonKind::Accent)?;
    primary.SetContent(&primary_label)?;
    click(&primary, resolve(true))?;
    let cancel = button(&t!("action.cancel"), resolve(false))?;

    let title = TextBlock::new()?;
    title.SetFontWeight(SEMI_BOLD)?;
    let body = TextBlock::new()?;
    body.SetTextWrapping(TextWrapping::Wrap)?;
    let buttons = row(8.0)?;
    buttons.halign(HorizontalAlignment::Right)?;
    children!(buttons; primary, cancel);
    let stack = column(8.0)?;
    children!(stack; title, body, buttons);
    let root = outlined(uniform(8.0))?;
    root.SetChild(&stack)?;

    let el = root.clone();
    let mut shown = Prop::default();
    b.add(move |ui, _| {
        let texts = confirm_texts(store, &(store.view)(ui).dialog);
        if let Some((title_text, body_text, primary_text)) = &texts {
            title.SetText(&hs(title_text))?;
            body.SetText(&hs(body_text))?;
            primary_label.SetText(&hs(primary_text))?;
        }
        shown.set(texts.is_some(), |&o| el.visible(o).map(drop))
    });
    Ok(root)
}

/// Stretched card with a 1px stroke and small corners, for rows inside a card list or dialog.
fn outlined(padding: Thickness) -> Result<Border> {
    let root = themed_border(Some(Theme::CardBackground), Some(Theme::CardStroke))?;
    root.halign(HorizontalAlignment::Stretch)?;
    root.SetBorderThickness(uniform(1.0))?;
    root.SetCornerRadius(corners(4.0))?;
    root.SetPadding(padding)?;
    Ok(root)
}
